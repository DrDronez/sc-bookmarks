use eframe::egui::{self, Color32, FontId, Stroke, TextStyle};
pub const BG: Color32 = Color32::from_rgb(7, 14, 22);
pub const PANEL: Color32 = Color32::from_rgb(15, 29, 40);
pub const EDGE: Color32 = Color32::from_rgb(40, 66, 81);
pub const CYAN: Color32 = Color32::from_rgb(104, 223, 235);
pub const AMBER: Color32 = Color32::from_rgb(239, 189, 112);
pub const MUTED: Color32 = Color32::from_rgb(140, 166, 181);
pub fn install(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(14.));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(13.));
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(23.));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(11.));
    style.spacing.item_spacing = egui::vec2(10., 9.);
    style.spacing.button_padding = egui::vec2(13., 8.);
    style.spacing.interact_size.y = 30.;
    style.visuals = egui::Visuals::dark();
    let v = &mut style.visuals;
    v.panel_fill = BG;
    v.window_fill = PANEL;
    v.extreme_bg_color = Color32::from_rgb(6, 17, 26);
    v.faint_bg_color = Color32::from_rgb(17, 35, 47);
    v.override_text_color = Some(Color32::from_rgb(226, 237, 242));
    v.window_stroke = Stroke::new(1_f32, EDGE);
    v.window_corner_radius = egui::CornerRadius::same(3);
    v.selection.bg_fill = Color32::from_rgb(30, 76, 92);
    v.selection.stroke = Stroke::new(1_f32, CYAN);
    for w in [&mut v.widgets.inactive, &mut v.widgets.noninteractive] {
        w.bg_fill = PANEL;
        w.weak_bg_fill = PANEL;
        w.bg_stroke = Stroke::new(1_f32, EDGE);
        w.fg_stroke = Stroke::new(1_f32, MUTED);
        w.corner_radius = egui::CornerRadius::same(2);
    }
    v.widgets.hovered.bg_fill = Color32::from_rgb(30, 66, 82);
    v.widgets.hovered.weak_bg_fill = v.widgets.hovered.bg_fill;
    v.widgets.hovered.bg_stroke = Stroke::new(1_f32, CYAN);
    v.widgets.hovered.corner_radius = egui::CornerRadius::same(2);
    v.widgets.active.bg_fill = Color32::from_rgb(36, 87, 103);
    v.widgets.active.weak_bg_fill = v.widgets.active.bg_fill;
    v.widgets.active.bg_stroke = Stroke::new(1_f32, CYAN);
    v.widgets.active.corner_radius = egui::CornerRadius::same(2);
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_style_of(egui::Theme::Dark, style.clone());
    ctx.set_style_of(egui::Theme::Light, style);
}
pub fn eyebrow(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(11.).color(CYAN).strong());
}
pub fn tab(ui: &mut egui::Ui, selected: bool, text: &str) -> bool {
    ui.add(
        egui::Button::new(egui::RichText::new(text).color(if selected { CYAN } else { MUTED }))
            .fill(if selected {
                Color32::from_rgb(22, 48, 62)
            } else {
                BG
            })
            .stroke(Stroke::new(1_f32, if selected { CYAN } else { EDGE })),
    )
    .clicked()
}
