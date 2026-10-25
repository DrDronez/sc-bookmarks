//! Non-activating, click-through desktop panels. Only our windows are modified.
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::*,
        UI::WindowsAndMessaging::*,
    },
    core::w,
};
#[derive(Clone, Debug, PartialEq)]
pub struct Panel {
    pub rect: crate::overlay::Rect,
    pub title: String,
    pub lines: String,
    pub warning: bool,
    pub floating: bool,
    pub angles: Option<(f64, f64)>,
    pub theme: crate::overlay::Theme,
    pub projected: bool,
}
pub struct Overlays {
    tx: mpsc::SyncSender<Vec<Panel>>,
    hotkey: Arc<AtomicU32>,
    hidden: Arc<AtomicBool>,
    key_status: Arc<std::sync::Mutex<String>>,
    health: std::sync::Arc<std::sync::Mutex<Result<usize, String>>>,
}
impl Default for Overlays {
    fn default() -> Self {
        let (tx, rx) = mpsc::sync_channel::<Vec<Panel>>(1);
        let health = std::sync::Arc::new(std::sync::Mutex::new(Ok(0)));
        let report = health.clone();
        let hotkey = Arc::new(AtomicU32::new(0));
        let desired = hotkey.clone();
        let hidden = Arc::new(AtomicBool::new(false));
        let hidden_worker = hidden.clone();
        let key_status = Arc::new(std::sync::Mutex::new(String::new()));
        let key_report = key_status.clone();
        thread::spawn(move || unsafe {
            windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext(
                windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            );
            let class = WNDCLASSW {
                lpfnWndProc: Some(procedure),
                lpszClassName: w!("SCBMObserveOverlay"),
                ..Default::default()
            };
            let atom = RegisterClassW(&class);
            if atom == 0 {
                let e = windows::core::Error::from_thread();
                if windows::Win32::Foundation::GetLastError()
                    != windows::Win32::Foundation::ERROR_CLASS_ALREADY_EXISTS
                {
                    *report.lock().unwrap() = Err(format!("Register overlay: {e}"));
                    return;
                }
            }
            let mut windows: Vec<(HWND, Box<Panel>)> = Vec::new();
            let mut z_order_at = std::time::Instant::now();
            let mut foreground = HWND::default();
            let mut targets: Vec<Option<(f64, f64)>> = Vec::new();
            let mut animation_at = std::time::Instant::now();
            let mut registered = 0_u32;
            let mut active_count = 0;
            loop {
                let key = desired.load(Ordering::Relaxed);
                if key != registered {
                    let _ = UnregisterHotKey(None, 73);
                    *key_report.lock().unwrap() = if key == 0 {
                        String::new()
                    } else {
                        match RegisterHotKey(
                            None,
                            73,
                            HOT_KEY_MODIFIERS((key >> 16) | MOD_NOREPEAT.0),
                            key & 0xffff,
                        ) {
                            Ok(()) => String::new(),
                            Err(e) => format!(
                                "Overlay hotkey unavailable; choose another combination in Settings: {e}"
                            ),
                        }
                    };
                    registered = key;
                }
                match rx.recv_timeout(Duration::from_millis(67)) {
                    Ok(panels) => {
                        let count = panels.len().min(9);
                        active_count = count;
                        targets.resize(count, None);
                        for (hwnd, _) in windows.iter().skip(count) {
                            let _ = ShowWindow(*hwnd, SW_HIDE);
                        }
                        for (i, mut p) in panels.into_iter().take(9).enumerate() {
                            targets[i] = p.angles;
                            if let Some((_, old)) = windows.get(i) {
                                if old.title == p.title
                                    && old.angles.is_some()
                                    && p.angles.is_some()
                                {
                                    p.angles = old.angles;
                                }
                            }
                            if i == windows.len() {
                                let mut data = Box::new(p.clone());
                                let Ok(hwnd) = CreateWindowExW(
                                    WS_EX_TOPMOST
                                        | WS_EX_LAYERED
                                        | WS_EX_TRANSPARENT
                                        | WS_EX_NOACTIVATE
                                        | WS_EX_TOOLWINDOW,
                                    w!("SCBMObserveOverlay"),
                                    w!("SC Bookmarks overlay"),
                                    WS_POPUP,
                                    0,
                                    0,
                                    1,
                                    1,
                                    None,
                                    None,
                                    None,
                                    None,
                                ) else {
                                    *report.lock().unwrap() = Err(format!(
                                        "Create overlay: {}",
                                        windows::core::Error::from_thread()
                                    ));
                                    break;
                                };
                                SetWindowLongPtrW(
                                    hwnd,
                                    GWLP_USERDATA,
                                    (&mut *data as *mut Panel) as isize,
                                );
                                // Capture targets the game window, so our separate window needs
                                // no capture exclusion (which also obscured visual verification).
                                windows.push((hwnd, data));
                            }
                            let (hwnd, data) = &mut windows[i];
                            if hidden_worker.load(Ordering::Relaxed) {
                                **data = p;
                                let _ = ShowWindow(*hwnd, SW_HIDE);
                                continue;
                            }
                            if **data == p && IsWindowVisible(*hwnd).as_bool() {
                                // Game activation can change z-order even when the text has not changed.
                                let _ = SetWindowPos(
                                    *hwnd,
                                    Some(HWND_TOPMOST),
                                    0,
                                    0,
                                    0,
                                    0,
                                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                                );
                                continue;
                            }
                            **data = p;
                            let r = data.rect;
                            let _ = SetWindowPos(
                                *hwnd,
                                Some(HWND_TOPMOST),
                                r.left,
                                r.top,
                                r.width as i32,
                                r.height as i32,
                                SWP_NOACTIVATE | SWP_SHOWWINDOW,
                            );
                            if let Err(e) = present(*hwnd, data) {
                                *report.lock().unwrap() = Err(e);
                            }
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                // Animate only the small marker bitmap, independently of map/UI/OCR.
                if animation_at.elapsed() >= Duration::from_millis(67) {
                    let dt = animation_at.elapsed().as_secs_f64();
                    animation_at = std::time::Instant::now();
                    for ((hwnd, p), target) in windows.iter_mut().zip(&targets) {
                        if let (Some(current), Some(goal)) = (p.angles, *target) {
                            let next = smooth_angles(current, goal, dt);
                            if next != current {
                                p.angles = Some(next);
                                if let Err(e) = present(*hwnd, p) {
                                    *report.lock().unwrap() = Err(e);
                                }
                            }
                        }
                    }
                }
                // eframe can throttle its event loop when occluded/minimized. Keep
                // the external overlay above a newly activated game independently
                // of UI repaint, OCR completion, or changes to the displayed text.
                let active = GetForegroundWindow();
                if active != foreground || z_order_at.elapsed() >= Duration::from_millis(250) {
                    foreground = active;
                    z_order_at = std::time::Instant::now();
                    for (hwnd, _) in &windows {
                        if IsWindowVisible(*hwnd).as_bool() {
                            if let Err(e) = SetWindowPos(
                                *hwnd,
                                Some(HWND_TOPMOST),
                                0,
                                0,
                                0,
                                0,
                                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                            ) {
                                *report.lock().unwrap() =
                                    Err(format!("Overlay foreground placement: {e}"));
                            }
                        }
                    }
                }
                if report.lock().unwrap().is_ok() {
                    *report.lock().unwrap() = Ok(windows
                        .iter()
                        .filter(|(hwnd, _)| IsWindowVisible(*hwnd).as_bool())
                        .count());
                }
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_HOTKEY && msg.wParam.0 == 73 {
                        let was = hidden_worker.fetch_xor(true, Ordering::Relaxed);
                        for (hwnd, p) in windows.iter().take(active_count) {
                            if was {
                                let _ = present(*hwnd, p);
                                let _ = ShowWindow(*hwnd, SW_SHOWNOACTIVATE);
                            } else {
                                let _ = ShowWindow(*hwnd, SW_HIDE);
                            }
                        }
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            let _ = UnregisterHotKey(None, 73);
            for (hwnd, _) in windows {
                let _ = DestroyWindow(hwnd);
            }
        });
        Self {
            tx,
            health,
            hotkey,
            hidden,
            key_status,
        }
    }
}
impl Overlays {
    pub fn configure_hotkey(&self, key: u32) {
        self.hotkey.store(key, Ordering::Relaxed);
    }
    pub fn hotkey_status(&self) -> String {
        self.key_status.lock().unwrap().clone()
    }
    pub fn hidden(&self) -> bool {
        self.hidden.load(Ordering::Relaxed)
    }
    pub fn health(&self) -> Result<usize, String> {
        self.health.lock().unwrap().clone()
    }
    pub fn update(&self, panels: Vec<Panel>) {
        let _ = self.tx.try_send(panels);
    }
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if msg == WM_NCHITTEST {
            return LRESULT(HTTRANSPARENT as isize);
        }
        if msg == WM_MOUSEACTIVATE {
            return LRESULT(MA_NOACTIVATE as isize);
        }
        if msg == WM_PAINT {
            let mut ps = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut ps);
            let _ = EndPaint(hwnd, &ps);
            return LRESULT(0);
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

unsafe fn draw(dc: HDC, p: &Panel) {
    unsafe {
        let mut r = RECT {
            left: 0,
            top: 0,
            right: p.rect.width as i32,
            bottom: p.rect.height as i32,
        };
        if p.projected {
            draw_projected(dc, p);
            return;
        }
        if p.floating {
            draw_floating(dc, p);
            return;
        }
        let bg = CreateSolidBrush(COLORREF(0x120d08));
        FillRect(dc, &r, bg);
        let _ = DeleteObject(bg.into());
        let accent = p.theme.colors().1;
        let subdued = ((accent & 0xfefefe) >> 1) + 0x202020;
        let edge = CreateSolidBrush(COLORREF(subdued));
        FrameRect(dc, &r, edge);
        let _ = DeleteObject(edge.into());
        SetBkMode(dc, TRANSPARENT);
        let font = CreateFontW(
            -13,
            0,
            0,
            0,
            500,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH.0 as u32,
            w!("Segoe UI"),
        );
        let old = SelectObject(dc, font.into());
        SetTextColor(dc, COLORREF(p.theme.colors().1));
        r.left = 8;
        r.right -= 8;
        r.top = 6;
        r.bottom = 24;
        let mut title: Vec<u16> = p.title.encode_utf16().collect();
        DrawTextW(
            dc,
            &mut title,
            &mut r,
            DT_CENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        r.top = 28;
        r.bottom = p.rect.height as i32 - 6;
        SetTextColor(
            dc,
            COLORREF(if p.warning {
                0x6677ff
            } else {
                p.theme.colors().0
            }),
        );
        let mut text: Vec<u16> = p.lines.encode_utf16().collect();
        DrawTextW(dc, &mut text, &mut r, DT_LEFT | DT_WORDBREAK);
        SelectObject(dc, old);
        let _ = DeleteObject(font.into());
    }
}
// Draw opaque outlined glyphs onto a transparent DIB. Near-black outlines are
// distinct from the zero-filled transparent background, including on bright HUDs.
unsafe fn floating_text(dc: HDC, text: &str, width: i32, top: i32, size: i32, color: u32) {
    unsafe {
        let font = CreateFontW(
            -size,
            0,
            0,
            0,
            700,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            NONANTIALIASED_QUALITY,
            DEFAULT_PITCH.0 as u32,
            w!("Segoe UI"),
        );
        let old = SelectObject(dc, font.into());
        let mut chars: Vec<u16> = text.encode_utf16().collect();
        for (dx, dy) in [
            (-2, 0),
            (2, 0),
            (0, -2),
            (0, 2),
            (-1, -1),
            (1, 1),
            (-1, 1),
            (1, -1),
            (0, 0),
        ] {
            SetTextColor(
                dc,
                COLORREF(if dx == 0 && dy == 0 { color } else { 0x010101 }),
            );
            let mut r = RECT {
                left: 4 + dx,
                top: top + dy,
                right: width - 4 + dx,
                bottom: top + size + 8 + dy,
            };
            DrawTextW(
                dc,
                &mut chars,
                &mut r,
                DT_CENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
            );
        }
        SelectObject(dc, old);
        let _ = DeleteObject(font.into());
    }
}
fn smooth_angles(current: (f64, f64), target: (f64, f64), dt: f64) -> (f64, f64) {
    let alpha = 1. - (-dt.max(0.) / 0.18).exp();
    let yaw_delta = (target.0 - current.0 + 180.).rem_euclid(360.) - 180.;
    if yaw_delta.abs() < 0.05 && (target.1 - current.1).abs() < 0.05 {
        return target;
    }
    (
        (current.0 + yaw_delta * alpha + 180.).rem_euclid(360.) - 180.,
        current.1 + (target.1 - current.1) * alpha,
    )
}
/// Matches the Python FlightOverlay's clamped, normalized 45x36 marker.
fn arrow_offset(yaw: f64, pitch: f64) -> (f64, f64) {
    let dx = (yaw / 20.).clamp(-1., 1.);
    let dy = -(pitch / 20.).clamp(-1., 1.);
    let length = dx.hypot(dy);
    if length == 0. {
        (0., 0.)
    } else {
        (dx / length * 45., dy / length * 36.)
    }
}
unsafe fn draw_floating(dc: HDC, p: &Panel) {
    use windows::Win32::Foundation::POINT;
    unsafe {
        SetBkMode(dc, TRANSPARENT);
        let width = p.rect.width as i32;
        if p.theme != crate::overlay::Theme::Classic {
            draw_accents(dc, width, p.theme);
        }
        // Text y values in the Python canvas are centers; GDI uses top edges.
        floating_text(dc, &p.title, width, 9, 16, 0xe0cca8);
        let lines: Vec<_> = p.lines.lines().collect();
        if let Some(distance) = lines.first() {
            floating_text(
                dc,
                distance,
                width,
                33,
                28,
                if p.warning {
                    0x65caff
                } else {
                    p.theme.colors().0
                },
            );
        }
        if let Some((yaw, pitch)) = p.angles {
            floating_text(
                dc,
                "AIM TOWARD THE BOOKMARK",
                width,
                71,
                15,
                p.theme.colors().1,
            );
            let cx = width / 2;
            let pen = CreatePen(
                PS_SOLID,
                if yaw.abs() < 2. && pitch.abs() < 2. {
                    3
                } else {
                    6
                },
                COLORREF(if yaw.abs() < 2. && pitch.abs() < 2. {
                    0xacff78
                } else {
                    p.theme.colors().0
                }),
            );
            let oldpen = SelectObject(dc, pen.into());
            let brush = CreateSolidBrush(COLORREF(p.theme.colors().0));
            let oldbrush = SelectObject(dc, GetStockObject(NULL_BRUSH));
            if yaw.abs() < 2. && pitch.abs() < 2. {
                let _ = Ellipse(dc, cx - 16, 104, cx + 16, 136);
                floating_text(dc, "ON COURSE", width, 150, 16, 0xacff78);
            } else {
                let (dx, dy) = arrow_offset(yaw, pitch);
                let len = dx.hypot(dy);
                let ux = dx / len;
                let uy = dy / len;
                let tip = (cx as f64 + dx, 120. + dy);
                let _ = MoveToEx(dc, cx, 120, None);
                let _ = LineTo(
                    dc,
                    (tip.0 - ux * 12.).round() as i32,
                    (tip.1 - uy * 12.).round() as i32,
                );
                SelectObject(dc, brush.into());
                let point = |x: f64, y: f64| POINT {
                    x: x.round() as i32,
                    y: y.round() as i32,
                };
                let head = [
                    point(tip.0, tip.1),
                    point(tip.0 - ux * 19. - uy * 7., tip.1 - uy * 19. + ux * 7.),
                    point(tip.0 - ux * 16., tip.1 - uy * 16.),
                    point(tip.0 - ux * 19. + uy * 7., tip.1 - uy * 19. - ux * 7.),
                ];
                let thin = CreatePen(PS_SOLID, 1, COLORREF(p.theme.colors().0));
                SelectObject(dc, thin.into());
                let _ = Polygon(dc, &head);
                SelectObject(dc, pen.into());
                let _ = DeleteObject(thin.into());
                floating_text(
                    dc,
                    &format!(
                        "{:.1}° {}   {:.1}° {}",
                        yaw.abs(),
                        if yaw >= 0. { "RIGHT" } else { "LEFT" },
                        pitch.abs(),
                        if pitch >= 0. { "UP" } else { "DOWN" }
                    ),
                    width,
                    150,
                    16,
                    0xfff6e4,
                );
            }
            SelectObject(dc, oldpen);
            SelectObject(dc, oldbrush);
            let _ = DeleteObject(pen.into());
            let _ = DeleteObject(brush.into());
            if let Some(eta) = lines
                .iter()
                .find(|l| l.starts_with("ETA") || l.starts_with("Not closing"))
            {
                floating_text(dc, eta, width, 183, 13, 0xdaf3a8);
            }

            if let Some(note) = lines
                .iter()
                .find(|l| l.starts_with("Holding") || l.starts_with("Position estimated"))
            {
                floating_text(dc, note, width, 235, 12, 0x65caff);
            }
        } else {
            // QT/status mode has no steering marker; preserve countdown and alert text.
            for (i, line) in lines
                .iter()
                .skip(1)
                .filter(|l| !l.contains("strafe"))
                .enumerate()
            {
                floating_text(
                    dc,
                    line,
                    width,
                    77 + i as i32 * 35,
                    16,
                    if p.warning { 0x6969ff } else { 0x65caff },
                );
            }
        }
    }
}
#[cfg(test)]
mod marker_tests {
    use super::{arrow_offset, smooth_angles};
    #[test]
    fn smoothing_wraps_and_converges() {
        let next = smooth_angles((179., 0.), (-179., 20.), 0.05);
        assert!(next.0.abs() > 179.);
        assert!(next.1 > 0. && next.1 < 20.);
        let mut p = (0., 0.);
        for _ in 0..40 {
            p = smooth_angles(p, (20., -10.), 0.05);
        }
        assert_eq!(p, (20., -10.));
    }
    #[test]
    fn original_marker_geometry() {
        assert_eq!(arrow_offset(20., 0.), (45., 0.));
        assert_eq!(arrow_offset(0., 20.), (0., -36.));
        let (x, y) = arrow_offset(-20., -20.);
        assert!((x + 45. / 2_f64.sqrt()).abs() < 1e-9);
        assert!((y - 36. / 2_f64.sqrt()).abs() < 1e-9);
        assert_eq!(arrow_offset(100., 100.), arrow_offset(20., 20.));
    }
}
unsafe fn present(hwnd: HWND, p: &Panel) -> Result<(), String> {
    use windows::Win32::Foundation::{POINT, SIZE};
    unsafe {
        let width = p.rect.width.min(8192) as i32;
        let height = p.rect.height.min(8192) as i32;
        if width <= 0 || height <= 0 {
            return Err("Empty overlay surface".into());
        }
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return Err(format!(
                "Overlay DC: {}",
                windows::core::Error::from_thread()
            ));
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(b) => b,
            Err(e) => {
                let _ = DeleteDC(dc);
                return Err(format!("Overlay bitmap: {e}"));
            }
        };
        let old = SelectObject(dc, bitmap.into());
        std::ptr::write_bytes(bits, 0, (width * height * 4) as usize);
        draw(dc, p);
        let _ = GdiFlush();
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u8, (width * height * 4) as usize);
        // GDI leaves alpha zero. Make only the RS card fill translucent;
        // keep glyphs/borders opaque and premultiply the background for Windows.
        for pixel in pixels.chunks_exact_mut(4) {
            if !p.floating && !p.projected && pixel[..3] == [0x12, 0x0d, 0x08] {
                for channel in &mut pixel[..3] {
                    *channel = (u16::from(*channel) * 140 / 255) as u8;
                }
                pixel[3] = 140;
                continue;
            }
            pixel[3] = if p.floating && pixel[..3] == [0, 0, 0] {
                0
            } else {
                255
            };
        }
        let dst = POINT {
            x: p.rect.left,
            y: p.rect.top,
        };
        let src = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: width,
            cy: height,
        };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let result = UpdateLayeredWindow(
            hwnd,
            None,
            Some(&dst),
            Some(&size),
            Some(dc),
            Some(&src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        )
        .map_err(|e| format!("Overlay composition: {e}"));
        SelectObject(dc, old);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(dc);
        result
    }
}

unsafe fn draw_accents(dc: HDC, width: i32, theme: crate::overlay::Theme) {
    unsafe {
        let pen = CreatePen(
            PS_SOLID,
            if matches!(
                theme,
                crate::overlay::Theme::Drake | crate::overlay::Theme::Argo
            ) {
                2
            } else {
                1
            },
            COLORREF(theme.colors().1),
        );
        let old = SelectObject(dc, pen.into());
        for side in [-1, 1] {
            let x = width / 2 + side * 100;
            let _ = MoveToEx(dc, x - side * 12, 70, None);
            let _ = LineTo(dc, x, 70);
            let _ = LineTo(
                dc,
                x,
                if theme == crate::overlay::Theme::Misc {
                    85
                } else {
                    155
                },
            );
            if theme == crate::overlay::Theme::Argo {
                // Squared industrial brackets with short paired index marks.
                let _ = LineTo(dc, x - side * 12, 155);
                for y in [108, 116] {
                    let _ = MoveToEx(dc, x, y, None);
                    let _ = LineTo(dc, x - side * 7, y);
                }
            }
            if theme == crate::overlay::Theme::Anvil || theme == crate::overlay::Theme::Aegis {
                let _ = MoveToEx(dc, x, 120, None);
                let _ = LineTo(dc, x - side * 8, 120);
            }
        }
        SelectObject(dc, old);
        let _ = DeleteObject(pen.into());
    }
}
unsafe fn draw_projected(dc: HDC, p: &Panel) {
    use windows::Win32::Foundation::POINT;
    unsafe {
        let cx = p.rect.width as i32 / 2;
        let points = [
            POINT { x: cx, y: 8 },
            POINT { x: cx + 13, y: 23 },
            POINT { x: cx, y: 38 },
            POINT { x: cx - 13, y: 23 },
            POINT { x: cx, y: 8 },
        ];
        let oldbrush = SelectObject(dc, GetStockObject(NULL_BRUSH));
        let border = CreatePen(PS_SOLID, 5, COLORREF(0x010101));
        let old = SelectObject(dc, border.into());
        let _ = Polyline(dc, &points);
        let pen = CreatePen(PS_SOLID, 2, COLORREF(p.theme.colors().0));
        SelectObject(dc, pen.into());
        let _ = Polyline(dc, &points);
        SelectObject(dc, old);
        SelectObject(dc, oldbrush);
        let _ = DeleteObject(pen.into());
        let _ = DeleteObject(border.into());
        floating_text(
            dc,
            &p.lines,
            p.rect.width as i32,
            43,
            13,
            p.theme.colors().0,
        );
    }
}
