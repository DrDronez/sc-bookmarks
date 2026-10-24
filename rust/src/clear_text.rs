use eframe::egui;

/// Text input with a reserved clear control inside its frame.
pub struct ClearText<'a> {
    text: &'a mut String,
    multiline: bool,
    width: Option<f32>,
    rows: usize,
    hint: &'a str,
}
impl<'a> ClearText<'a> {
    pub fn singleline(text: &'a mut String) -> Self {
        Self {
            text,
            multiline: false,
            width: None,
            rows: 1,
            hint: "",
        }
    }
    pub fn multiline(text: &'a mut String) -> Self {
        Self {
            multiline: true,
            rows: 4,
            ..Self::singleline(text)
        }
    }
    pub fn desired_width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
    pub fn desired_rows(mut self, rows: usize) -> Self {
        self.rows = rows;
        self
    }
    pub fn hint_text(mut self, hint: &'a str) -> Self {
        self.hint = hint;
        self
    }
}
impl egui::Widget for ClearText<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let width = self
            .width
            .unwrap_or(ui.spacing().text_edit_width)
            .min(ui.available_width())
            .max(48.);
        let visuals = ui.visuals().widgets.inactive;
        egui::Frame::new()
            .fill(ui.visuals().extreme_bg_color)
            .stroke(visuals.bg_stroke)
            .corner_radius(visuals.corner_radius)
            .inner_margin(3.)
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.;
                    let edit = if self.multiline {
                        egui::TextEdit::multiline(self.text)
                    } else {
                        egui::TextEdit::singleline(self.text)
                    };
                    let mut response = ui.add(
                        edit.frame(false)
                            .hint_text(self.hint)
                            .desired_rows(self.rows)
                            .desired_width((width - 28.).max(20.)),
                    );
                    let clear = ui
                        .add_enabled(
                            !self.text.is_empty(),
                            egui::Button::new("×")
                                .frame(false)
                                .min_size(egui::vec2(18., 18.)),
                        )
                        .on_hover_text("Clear text");
                    if clear.clicked() {
                        self.text.clear();
                        response.mark_changed();
                        response.request_focus();
                    }
                    response
                })
                .inner
            })
            .inner
    }
}
