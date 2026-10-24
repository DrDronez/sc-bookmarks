//! Read-only HWND geometry; never changes focus, position, or game input.
use crate::overlay::Rect;
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::{
        Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute},
        Gdi::ClientToScreen,
    },
    UI::{
        HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext},
        WindowsAndMessaging::{GetClientRect, GetWindowRect, IsIconic, IsWindow},
    },
};
pub struct Geometry {
    pub client: Rect,
    pub outer: Rect,
}
/// Query on a per-monitor-aware thread, then restore its prior DPI context.
pub fn query(handle: usize) -> Option<Geometry> {
    unsafe {
        let hwnd = HWND(handle as *mut std::ffi::c_void);
        if !IsWindow(Some(hwnd)).as_bool() || IsIconic(hwnd).as_bool() {
            return None;
        }
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let result = (|| {
            let mut client = RECT::default();
            let mut origin = POINT::default();
            let mut outer = RECT::default();
            GetClientRect(hwnd, &mut client).ok()?;
            if !ClientToScreen(hwnd, &mut origin).as_bool() {
                return None;
            }
            if DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut outer as *mut RECT as *mut _,
                std::mem::size_of::<RECT>() as u32,
            )
            .is_err()
            {
                GetWindowRect(hwnd, &mut outer).ok()?;
            }
            let width = u32::try_from(client.right - client.left).ok()?;
            let height = u32::try_from(client.bottom - client.top).ok()?;
            if width == 0 || height == 0 {
                return None;
            }
            Some(Geometry {
                client: Rect {
                    left: origin.x,
                    top: origin.y,
                    width,
                    height,
                },
                outer: Rect {
                    left: outer.left,
                    top: outer.top,
                    width: u32::try_from(outer.right - outer.left).ok()?,
                    height: u32::try_from(outer.bottom - outer.top).ok()?,
                },
            })
        })();
        if !old.0.is_null() {
            SetThreadDpiAwarenessContext(old);
        }
        result
    }
}
/// Frame/client mapping can briefly disagree during resize. Skip that sample
/// rather than assigning incorrect desktop coordinates to an overlay or scan.
pub fn client_crop(frame: [u32; 2], geometry: &Geometry) -> Option<[u32; 4]> {
    let c = geometry.client;
    let o = geometry.outer;
    if frame[0].abs_diff(c.width) <= 2 && frame[1].abs_diff(c.height) <= 2 {
        return Some([0, 0, frame[0], frame[1]]);
    }
    if frame[0].abs_diff(o.width) > 3 || frame[1].abs_diff(o.height) > 3 {
        return None;
    }
    let x = u32::try_from(i64::from(c.left) - i64::from(o.left)).ok()?;
    let y = u32::try_from(i64::from(c.top) - i64::from(o.top)).ok()?;
    let right = x.checked_add(c.width)?;
    let bottom = y.checked_add(c.height)?;
    (right <= frame[0] && bottom <= frame[1]).then_some([x, y, right, bottom])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_and_border_frames_on_negative_monitor() {
        let g = Geometry {
            client: Rect {
                left: -1912,
                top: 30,
                width: 1904,
                height: 1042,
            },
            outer: Rect {
                left: -1920,
                top: 0,
                width: 1920,
                height: 1080,
            },
        };
        assert_eq!(client_crop([1920, 1080], &g), Some([8, 30, 1912, 1072]));
        assert_eq!(client_crop([1904, 1042], &g), Some([0, 0, 1904, 1042]));
        assert!(client_crop([1280, 720], &g).is_none());
    }
    #[test]
    fn rejects_invalid_offsets() {
        let g = Geometry {
            client: Rect {
                left: 0,
                top: 0,
                width: 1000,
                height: 600,
            },
            outer: Rect {
                left: 10,
                top: 0,
                width: 1020,
                height: 640,
            },
        };
        assert!(client_crop([1020, 640], &g).is_none());
    }
}
