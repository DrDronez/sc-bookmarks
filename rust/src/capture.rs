//! Read-only Windows Graphics Capture. No input, injection or game file access.
//! One pending image; callbacks discard excess frames before mapping GPU pixels.
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    settings::*,
    window::Window,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
pub struct Pixels {
    pub desktop: crate::overlay::Rect,
    pub stamp: Instant,
    pub source_size: [u32; 2],
    pub origin: [u32; 2],
    pub size: [u32; 2],
    pub rgba: Vec<u8>,
}
impl Pixels {
    /// Crop a full scan sample without changing its capture timestamp or geometry.
    pub fn navigation_crop(&self) -> Self {
        let [x, y, r, b] = crate::overlay::debug_box(self.size[0], self.size[1]);
        let width = r - x;
        let height = b - y;
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for row in y..b {
            let start = ((row * self.size[0] + x) * 4) as usize;
            rgba.extend_from_slice(&self.rgba[start..start + (width * 4) as usize]);
        }
        Self {
            desktop: self.desktop,
            stamp: self.stamp,
            source_size: self.source_size,
            origin: [self.origin[0] + x, self.origin[1] + y],
            size: [width, height],
            rgba,
        }
    }
}
#[derive(Clone, Debug)]
pub enum Status {
    Starting,
    Running,
    Stopped,
    Failed(String),
}
pub struct Shared {
    image: Mutex<Option<Pixels>>,
    status: Mutex<Status>,
    full_requested: AtomicBool,
}
impl Shared {
    fn new() -> Self {
        Self {
            image: Mutex::new(None),
            status: Mutex::new(Status::Starting),
            full_requested: AtomicBool::new(false),
        }
    }
    fn status(&self, status: Status) {
        if let Ok(mut s) = self.status.lock() {
            *s = status;
        }
    }
}
struct Handler {
    shared: Arc<Shared>,
    handle: usize,
    last: Option<Instant>,
}
impl GraphicsCaptureApiHandler for Handler {
    type Flags = (Arc<Shared>, usize);
    type Error = Error;
    fn new(ctx: Context<Self::Flags>) -> Result<Self, Error> {
        ctx.flags.0.status(Status::Running);
        Ok(Self {
            shared: ctx.flags.0,
            handle: ctx.flags.1,
            last: None,
        })
    }
    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _control: InternalCaptureControl,
    ) -> Result<(), Error> {
        let now = Instant::now();
        if self
            .last
            .is_some_and(|last| now.duration_since(last) < Duration::from_millis(250))
        {
            return Ok(());
        }
        self.last = Some(now);
        let Ok(mut slot) = self.shared.image.try_lock() else {
            return Ok(());
        };
        if slot.is_some() {
            return Ok(());
        }
        let source_size = [frame.width(), frame.height()];
        if source_size.contains(&0) || source_size[0] > 16384 || source_size[1] > 8640 {
            return Err("Capture dimensions outside supported bounds".into());
        }
        let Some(geometry) = crate::window_geometry::query(self.handle) else {
            return Ok(());
        };
        let Some([client_x, client_y, client_right, client_bottom]) =
            crate::window_geometry::client_crop(source_size, &geometry)
        else {
            return Ok(());
        };
        let source_size = [client_right - client_x, client_bottom - client_y];
        let full = self.shared.full_requested.swap(false, Ordering::Relaxed);
        let [x, y, right, bottom] = if full {
            [0, 0, source_size[0], source_size[1]]
        } else {
            crate::overlay::debug_box(source_size[0], source_size[1])
        };
        let width = right - x;
        let height = bottom - y;
        let length = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .filter(|n| *n <= 256 * 1024 * 1024)
            .ok_or("Capture frame exceeds memory limit")?;
        let mut buffer = frame.buffer_crop(
            client_x + x,
            client_y + y,
            client_x + right,
            client_y + bottom,
        )?;
        let pitch = buffer.row_pitch() as usize;
        let raw = buffer.as_raw_buffer();
        let rgba = copy_rows(raw, width as usize, height as usize, pitch, length)?;
        *slot = Some(Pixels {
            desktop: geometry.client,
            stamp: now,
            source_size,
            origin: [x, y],
            size: [width, height],
            rgba,
        });
        Ok(())
    }
    fn on_closed(&mut self) -> Result<(), Error> {
        self.shared.status(Status::Stopped);
        Ok(())
    }
}
fn copy_rows(
    raw: &[u8],
    width: usize,
    height: usize,
    pitch: usize,
    length: usize,
) -> Result<Vec<u8>, Error> {
    let row = width.checked_mul(4).ok_or("Row size overflow")?;
    if pitch < row
        || height == 0
        || pitch
            .checked_mul(height - 1)
            .and_then(|n| n.checked_add(row))
            .is_none_or(|n| n > raw.len())
        || row.checked_mul(height) != Some(length)
    {
        return Err("Invalid capture buffer dimensions".into());
    }
    let mut rgba = vec![0; length];
    for y in 0..height {
        rgba[y * row..(y + 1) * row].copy_from_slice(&raw[y * pitch..y * pitch + row]);
    }
    Ok(rgba)
}
pub struct Session {
    shared: Arc<Shared>,
    stop: mpsc::Sender<()>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Session {
    pub fn start() -> Self {
        let shared = Arc::new(Shared::new());
        let flags = shared.clone();
        let (stop, rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let result = (|| -> Result<(), Error> {
                // Exact title match avoids capturing unrelated windows containing the game name.
                let window = Window::from_name("Star Citizen ")
                    .or_else(|_| Window::from_name("Star Citizen"))?;
                let handle = window.as_raw_hwnd() as usize;
                let settings = Settings::new(
                    window,
                    CursorCaptureSettings::WithoutCursor,
                    DrawBorderSettings::Default,
                    SecondaryWindowSettings::Default,
                    // Do not set MinUpdateInterval: unavailable on older supported Windows builds.
                    // Our callback gates work before pixel readback instead.
                    MinimumUpdateIntervalSettings::Default,
                    DirtyRegionSettings::Default,
                    ColorFormat::Rgba8,
                    (flags.clone(), handle),
                );
                let control = Handler::start_free_threaded(settings)?;
                loop {
                    if control.is_finished() {
                        control.wait()?;
                        break;
                    }
                    match rx.recv_timeout(Duration::from_millis(250)) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                            control.stop()?;
                            break;
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                }
                Ok(())
            })();
            flags.status(match result {
                Ok(()) => Status::Stopped,
                Err(e) => Status::Failed(e.to_string()),
            });
        });
        Self {
            shared,
            stop,
            worker: Some(worker),
        }
    }
    pub fn take(&self) -> Option<Pixels> {
        self.shared.image.lock().ok()?.take()
    }
    pub fn request_full_frame(&self) {
        self.shared.full_requested.store(true, Ordering::Relaxed);
    }
    pub fn status(&self) -> Status {
        self.shared
            .status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(Status::Failed("Capture state unavailable".into()))
    }
    pub fn stop(&self) {
        let _ = self.stop.send(());
    }
    pub fn finished(&self) -> bool {
        self.worker.as_ref().is_none_or(|w| w.is_finished())
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
        if self.finished() {
            if let Some(w) = self.worker.take() {
                let _ = w.join();
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scan_crop_preserves_capture_time_and_client_origin() {
        let p = Pixels {
            desktop: crate::overlay::Rect {
                left: 20,
                top: 30,
                width: 1920,
                height: 1080,
            },
            stamp: Instant::now(),
            source_size: [1920, 1080],
            origin: [0, 0],
            size: [1920, 1080],
            rgba: vec![42; 1920 * 1080 * 4],
        };
        let crop = p.navigation_crop();
        let [x, y, r, b] = crate::overlay::debug_box(1920, 1080);
        assert_eq!(crop.stamp, p.stamp);
        assert_eq!(crop.desktop, p.desktop);
        assert_eq!(crop.origin, [x, y]);
        assert_eq!(crop.size, [r - x, b - y]);
        assert_eq!(crop.rgba.len(), ((r - x) * (b - y) * 4) as usize);
        assert!(crop.rgba.iter().all(|v| *v == 42));
    }
    #[test]
    fn strips_row_padding_without_channel_swap() {
        let raw = [1, 2, 3, 4, 99, 99, 99, 99, 5, 6, 7, 8];
        assert_eq!(
            copy_rows(&raw, 1, 2, 8, 8).unwrap(),
            vec![1, 2, 3, 4, 5, 6, 7, 8]
        );
        assert!(copy_rows(&raw, 2, 2, 8, 16).is_err());
    }
    #[test]
    fn queue_holds_only_one_image() {
        let shared = Shared::new();
        for i in 0..5 {
            let mut slot = shared.image.lock().unwrap();
            if slot.is_none() {
                *slot = Some(Pixels {
                    desktop: crate::overlay::Rect {
                        left: 0,
                        top: 0,
                        width: 1,
                        height: 1,
                    },
                    stamp: Instant::now(),
                    source_size: [1, 1],
                    origin: [0, 0],
                    size: [1, 1],
                    rgba: vec![i; 4],
                });
            }
        }
        assert_eq!(
            shared.image.lock().unwrap().take().unwrap().rgba,
            vec![0; 4]
        );
        assert!(shared.image.lock().unwrap().is_none());
    }
}
