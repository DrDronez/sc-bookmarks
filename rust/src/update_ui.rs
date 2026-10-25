use eframe::egui;
use sc_bookmarks::update_check::{self, Update};
use std::sync::mpsc::{self, Receiver};

pub struct UpdateNotice {
    pending: Option<Receiver<Option<Update>>>,
    update: Option<Update>,
}

impl UpdateNotice {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        // A failed check (including offline/rate-limited GitHub) stays silent.
        // The worker never blocks capture, rendering, or application startup.
        let _ = std::thread::Builder::new()
            .name("release-check".into())
            .spawn(move || {
                let _ = tx.send(update_check::check().ok().flatten());
            });
        Self {
            pending: Some(rx),
            update: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(update) => {
                    self.update = update;
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => self.pending = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let Some(update) = &self.update else {
            return;
        };
        let mut open = true;
        let mut dismiss = false;
        egui::Window::new("Update available")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(430.)
            .show(ctx, |ui| {
                ui.label(format!("SC Bookmarks {} is available.", update.version));
                ui.small(format!("You're running {}.", env!("CARGO_PKG_VERSION")));
                ui.add_space(4.);
                for change in &update.changes {
                    ui.label(format!("• {change}"));
                }
                ui.add_space(4.);
                ui.horizontal(|ui| {
                    ui.hyperlink_to("Open GitHub repository", update_check::REPOSITORY);
                    dismiss = ui.button("Dismiss").clicked();
                });
            });
        if !open || dismiss {
            self.update = None;
        }
    }
}
