use eframe::egui;
use sc_bookmarks::{
    capture::{Session, Status},
    ocr_worker::{Observation, Worker},
};
use std::time::{Duration, Instant};
pub struct CaptureView {
    session: Option<Session>,
    worker: Option<Worker>,
    texture: Option<egui::TextureHandle>,
    last: Option<Instant>,
    info: String,
    stopping: bool,
    pub observation: Option<Observation>,
    pub error: String,
    pub generation: u64,
    scan: Option<sc_bookmarks::scan_worker::Worker>,
    pending: Option<sc_bookmarks::scan_worker::Kind>,
    busy: bool,
    last_scan: Option<Instant>,
    scan_delay: u64,
    pub scans_enabled: bool,
    pub scan_result: Option<sc_bookmarks::scan_worker::ResultFrame>,
}
impl Default for CaptureView {
    fn default() -> Self {
        Self {
            session: None,
            worker: None,
            texture: None,
            last: None,
            info: String::new(),
            stopping: false,
            observation: None,
            error: String::new(),
            generation: 0,
            scan: None,
            pending: None,
            busy: false,
            last_scan: None,
            scan_delay: 5,
            scans_enabled: true,
            scan_result: None,
        }
    }
}
impl CaptureView {
    pub fn start(&mut self) {
        if self.session.as_ref().is_some_and(|s| !s.finished()) {
            return;
        }
        let runtime = std::env::current_exe()
            .unwrap_or_default()
            .with_file_name("runtime");
        #[cfg(debug_assertions)]
        let runtime = if runtime.is_dir() {
            runtime
        } else {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runtime")
        };
        self.generation = self.generation.wrapping_add(1);
        self.scan = Some(sc_bookmarks::scan_worker::Worker::start(runtime.clone()));
        self.worker = Some(Worker::start(runtime));
        self.session = Some(Session::start());
        self.stopping = false;
        self.last = None;
        self.observation = None;
        self.error.clear();
        self.pending = None;
        self.busy = false;
        self.scan_result = None;
    }
    pub fn poll(&mut self, ctx: &egui::Context, preview: bool) {
        if let Some(session) = &self.session {
            if let Some(pixels) = session.take() {
                if self.pending.is_some() && pixels.size == pixels.source_size {
                    // Full scan frames still contain navigation text. Feed a small
                    // debug crop to navigation so RS scans never steal its sample.
                    if let Some(worker) = &self.worker {
                        worker.submit(pixels.navigation_crop());
                    }
                    if let (Some(kind), Some(worker)) = (self.pending.take(), &self.scan) {
                        self.busy = worker.submit(kind, pixels);
                    }
                } else {
                    self.last = Some(pixels.stamp);
                    self.info = format!(
                        "Source {} × {} · sample {} × {}",
                        pixels.source_size[0],
                        pixels.source_size[1],
                        pixels.size[0],
                        pixels.size[1]
                    );
                    if preview {
                        let image = egui::ColorImage::from_rgba_unmultiplied(
                            [pixels.size[0] as usize, pixels.size[1] as usize],
                            &pixels.rgba,
                        );
                        if let Some(texture) = &mut self.texture {
                            texture.set(image, egui::TextureOptions::LINEAR);
                        } else {
                            self.texture = Some(ctx.load_texture(
                                "capture-preview",
                                image,
                                egui::TextureOptions::LINEAR,
                            ));
                        }
                    }
                    if let Some(worker) = &self.worker {
                        worker.submit(pixels);
                    }
                }
            }
            if !session.finished() {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
        if let Some(worker) = &self.scan {
            if let Some(result) = worker.poll() {
                self.busy = false;
                match result {
                    Ok(v) => {
                        self.scan_delay = if v
                            .contacts
                            .as_ref()
                            .is_some_and(|r| r.as_ref().is_ok_and(|c| !c.is_empty()))
                        {
                            1
                        } else {
                            5
                        };
                        self.last_scan = Some(Instant::now());
                        self.scan_result = Some(v);
                    }
                    Err(e) => self.error = e,
                }
            }
        }
        if self.scans_enabled
            && self.session.as_ref().is_some_and(|s| !s.finished())
            && !self.busy
            && self.pending.is_none()
            && self
                .last_scan
                .is_none_or(|t| t.elapsed().as_secs() >= self.scan_delay.max(1))
        {
            self.request_scan(sc_bookmarks::scan_worker::Kind::Contacts);
        }
        if let Some(worker) = &self.worker {
            if let Some(result) = worker.poll() {
                match result {
                    Ok(value) => {
                        // Heading-only updates must not erase the last coordinate failure.
                        if let Some(e) = &value.error {
                            self.error = e.clone();
                        } else if value.position.is_some() {
                            self.error.clear();
                        }
                        self.observation = Some(value);
                    }
                    Err(e) => self.error = e,
                }
            }
        }
    }
    pub fn request_scan(&mut self, kind: sc_bookmarks::scan_worker::Kind) {
        if self.busy || self.pending.is_some() {
            self.error = "Scan already in progress".into();
            return;
        }
        if let Some(s) = &self.session {
            if !s.finished() {
                s.request_full_frame();
                self.pending = Some(kind);
                self.last_scan = Some(Instant::now());
                return;
            }
        }
        self.error = "Start capture before scanning".into();
    }
    pub fn show(&mut self, ui: &mut egui::Ui, system: &str, shard: &str) {
        ui.heading("SCREEN CAPTURE");
        ui.checkbox(&mut self.scans_enabled, "Identify RS contacts");
        ui.label("Reads visible game pixels. Enable r_displayinfo 3 in the game for coordinates.");
        let finished = self.session.as_ref().is_none_or(Session::finished);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(finished, egui::Button::new("Start capture"))
                .clicked()
            {
                self.start();
            }
            if ui
                .add_enabled(!finished && !self.stopping, egui::Button::new("Stop"))
                .clicked()
            {
                if let Some(s) = &self.session {
                    s.stop();
                    self.stopping = true;
                }
                self.worker = None;
                self.scan = None;
                self.pending = None;
                self.busy = false;
            }
        });
        if let Some(session) = &self.session {
            ui.label(match session.status() {
                Status::Starting => "Starting capture…".into(),
                Status::Running if self.stopping => "Stopping capture…".into(),
                Status::Running => "Capture and OCR active".into(),
                Status::Stopped => "Capture stopped".into(),
                Status::Failed(e) => format!("Capture failed: {e}"),
            });
        }
        if !self.error.is_empty() {
            ui.colored_label(egui::Color32::YELLOW, &self.error);
        }
        if let Some(o) = &self.observation {
            if let Some(p) = o.position {
                ui.label(format!(
                    "Root XYZ km: {:.4}, {:.4}, {:.4}",
                    p[0], p[1], p[2]
                ));
            }
            ui.label(format!(
                "CamDir: {:?} · System: {} · Shard: {}",
                o.camera,
                if system.is_empty() { "Unknown" } else { system },
                if shard.is_empty() { "Unknown" } else { shard }
            ));
        }
        ui.label(&self.info);
        if let Some(last) = self.last {
            ui.label(format!(
                "Last frame {:.1}s ago",
                last.elapsed().as_secs_f64()
            ));
        }
        if let Some(texture) = &self.texture {
            let available = ui.available_size();
            let size = texture.size_vec2();
            let scale = (available.x / size.x)
                .min(available.y / size.y)
                .clamp(0.01, 1.);
            ui.image((texture.id(), size * scale));
        }
    }
}
