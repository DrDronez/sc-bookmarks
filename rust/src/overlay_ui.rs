use eframe::egui;
use sc_bookmarks::overlay::{self, Rect, Settings};
use std::path::PathBuf;
pub struct OverlaySettingsView {
    path: PathBuf,
    pub runtime_status: String,
    pub saved: Settings,
    draft: Option<Settings>,
    name: String,
    pending: Option<(String, bool)>,
    error: String,
    blocked: bool,
    preview_size: [u32; 2],
}
impl OverlaySettingsView {
    pub fn new(path: PathBuf) -> Self {
        let (saved, error, blocked) = match Settings::load(&path) {
            Ok(s) => (s, String::new(), false),
            Err(e) => (
                Settings::default(),
                format!("Settings could not load: {e}. Saving disabled to preserve the file."),
                true,
            ),
        };
        Self {
            path,
            runtime_status: String::new(),
            saved,
            draft: None,
            name: String::new(),
            pending: None,
            error,
            blocked,
            preview_size: [1920, 1080],
        }
    }
    pub fn position(&self) -> [f64; 2] {
        self.draft.as_ref().unwrap_or(&self.saved).position
    }
    pub fn open(&mut self) {
        self.draft = Some(self.saved.clone());
        self.pending = None;
        if !self.blocked {
            self.error.clear();
        }
    }
    pub fn show(&mut self, ctx: &egui::Context) {
        let Some(draft) = self.draft.as_mut() else {
            return;
        };
        let mut open = true;
        let mut save = false;
        let mut cancel = false;
        egui::Window::new("Settings — overlays")
            .open(&mut open)
            .default_width(520.)
            .resizable(true)
            .vscroll(true)
            .show(ctx, |ui| {
                ui.checkbox(&mut draft.hide_capture_border,"Hide yellow capture border (if Windows allows)");
                ui.weak("Applies on the next capture start. Capture falls back to a border if unsupported.");
                ui.separator();
                ui.heading("OVERLAY APPEARANCE");
                egui::ComboBox::from_id_salt("overlay_theme").selected_text(draft.theme.label()).show_ui(ui, |ui| {
                    for theme in overlay::Theme::ALL {ui.selectable_value(&mut draft.theme,theme,theme.label());}
                });
                ui.weak("Independent manufacturer-inspired styling; no game artwork.");
                ui.checkbox(&mut draft.bookmark_3d,"3D bookmark test (within 5 km)");
                if draft.bookmark_3d {
                    ui.label("Experimental projected diamond. Uses camera direction; normal arrow remains for offscreen targets.");
                    ui.add(egui::Slider::new(&mut draft.vertical_fov,30. ..=120.).text("Vertical FOV °"));
                    ui.weak("Match the camera's vertical FOV (default 60°). Zoom/FOV changes require adjustment during this test.");
                }
                ui.separator();
                ui.heading("OVERLAY HOTKEY");
                ui.label(&self.runtime_status);
                let mut enabled=draft.hotkey!=0;
                if ui.checkbox(&mut enabled,"Enable global toggle").changed() {draft.hotkey=if enabled {0x79} else {0};}
                if enabled {
                    let mut key=draft.hotkey&0xffff;
                    let mut modifiers=draft.hotkey>>16;
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("overlay_key").selected_text(format!("F{}",key-0x6f)).show_ui(ui,|ui|{for k in 0x75..=0x7b {ui.selectable_value(&mut key,k,format!("F{}",k-0x6f));}});
                        for (bit,label) in [(2,"Ctrl"),(1,"Alt"),(4,"Shift")] {let mut on=modifiers&bit!=0;if ui.checkbox(&mut on,label).changed(){if on {modifiers|=bit;}else{modifiers&=!bit;}}}
                    });
                    draft.hotkey=key|(modifiers<<16);
                }
                ui.weak("Toggles all overlay visuals while the game is focused. Capture and alerts continue. Applied on Save; conflicts are reported.");
                ui.separator();
                ui.heading("OVERLAY POSITION");
                ui.label(
                    "Drag the cyan box or use the sliders. Placement follows the game client area.",
                );
                ui.weak("Placement uses the game window; saved presets work across resolutions.");
                egui::ComboBox::from_id_salt("preview_size")
                    .selected_text(format!(
                        "Preview {} × {}",
                        self.preview_size[0], self.preview_size[1]
                    ))
                    .show_ui(ui, |ui| {
                        for size in [
                            [1280, 720],
                            [1920, 1080],
                            [2560, 1440],
                            [3440, 1440],
                            [3840, 2160],
                            [5120, 1440],
                        ] {
                            ui.selectable_value(
                                &mut self.preview_size,
                                size,
                                format!("{} × {}", size[0], size[1]),
                            );
                        }
                    });
                let (area, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width().min(520.), 220.),
                    egui::Sense::click_and_drag(),
                );
                let scale = (area.width() / self.preview_size[0] as f32)
                    .min(area.height() / self.preview_size[1] as f32);
                let game = egui::Rect::from_center_size(
                    area.center(),
                    egui::vec2(
                        self.preview_size[0] as f32 * scale,
                        self.preview_size[1] as f32 * scale,
                    ),
                );
                if response.clicked() || response.dragged() {
                    if let Some(cursor) = response.interact_pointer_pos() {
                        draft.position = overlay::drag_position(
                            self.preview_size,
                            [
                                (cursor.x - game.left()) as f64 / scale as f64,
                                (cursor.y - game.top()) as f64 / scale as f64,
                            ],
                        );
                    }
                }
                let painter = ui.painter();
                painter.rect_filled(game, 0., egui::Color32::from_rgb(9, 19, 29));
                painter.rect_stroke(
                    game,
                    0.,
                    egui::Stroke::new(1_f32, egui::Color32::GRAY),
                    egui::StrokeKind::Inside,
                );
                if let Ok(p) = overlay::placement(
                    Rect {
                        left: 0,
                        top: 0,
                        width: self.preview_size[0],
                        height: self.preview_size[1],
                    },
                    draft.position,
                ) {
                    let panel = egui::Rect::from_min_size(
                        game.min + egui::vec2(p.left as f32 * scale, p.top as f32 * scale),
                        egui::vec2(p.width as f32 * scale, p.height as f32 * scale),
                    );
                    painter.rect_filled(panel, 2., egui::Color32::from_rgb(36, 78, 98));
                    painter.rect_stroke(
                        panel,
                        2.,
                        egui::Stroke::new(1_f32, {let c=draft.theme.colors().0; egui::Color32::from_rgb(c as u8,(c>>8) as u8,(c>>16) as u8)}),
                        egui::StrokeKind::Inside,
                    );
                    painter.text(
                        panel.center(),
                        egui::Align2::CENTER_CENTER,
                        "OVERLAY",
                        egui::FontId::proportional(10.),
                        egui::Color32::WHITE,
                    );
                }
                ui.add(egui::Slider::new(&mut draft.position[0], 0. ..=100.).text("Horizontal %"));
                ui.add(egui::Slider::new(&mut draft.position[1], 0. ..=100.).text("Vertical %"));
                if ui.button("Center overlay").clicked() {
                    draft.position = [50.; 2];
                }
                ui.separator();
                ui.heading("SHIP / POSITION PRESETS");
                egui::ComboBox::from_id_salt("overlay_preset")
                    .selected_text(if self.name.is_empty() {
                        "Select preset"
                    } else {
                        &self.name
                    })
                    .show_ui(ui, |ui| {
                        for (name, position) in &draft.presets {
                            if ui.selectable_label(self.name == *name, name).clicked() {
                                self.name = name.clone();
                                draft.position = *position;
                                self.pending = None;
                            }
                        }
                    });
                ui.horizontal(|ui| {
                    ui.label("Preset name");
                    if ui.text_edit_singleline(&mut self.name).changed() {
                        self.pending = None;
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Save as preset").clicked() {
                        let name = self.name.trim().to_owned();
                        if name.is_empty() {
                            self.error = "Enter a preset name.".into();
                        } else if draft.presets.contains_key(&name) {
                            self.pending = Some((name, false));
                        } else {
                            draft.presets.insert(name.clone(), draft.position);
                            self.name = name;
                            self.error.clear();
                        }
                    }
                    if ui.button("Delete preset").clicked()
                        && draft.presets.contains_key(&self.name)
                    {
                        self.pending = Some((self.name.clone(), true));
                    }
                });
                if let Some((name, delete)) = self.pending.clone() {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} preset ‘{name}’?",
                            if delete { "Delete" } else { "Replace" }
                        ));
                        if ui.button("Confirm").clicked() {
                            if delete {
                                draft.presets.remove(&name);
                                self.name.clear();
                            } else {
                                draft.presets.insert(name, draft.position);
                            }
                            self.pending = None;
                        }
                        if ui.button("Keep existing").clicked() {
                            self.pending = None;
                        }
                    });
                }
                if !self.error.is_empty() {
                    ui.colored_label(egui::Color32::LIGHT_RED, &self.error);
                }
                ui.separator();
                ui.label("Save settings keeps the position and presets for the next launch.");
                ui.horizontal(|ui| {
                    save = ui
                        .add_enabled(!self.blocked, egui::Button::new("Save settings"))
                        .clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if save {
            match draft.save(&self.path) {
                Ok(()) => {
                    self.saved = draft.clone();
                    self.draft = None;
                }
                Err(e) => self.error = format!("Could not save settings: {e}"),
            }
        }
        if cancel || !open {
            self.draft = None;
            self.pending = None;
        }
    }
}
