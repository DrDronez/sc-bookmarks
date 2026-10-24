use super::*;
impl MapView {
    pub(super) fn show_overview(
        &mut self,
        ui: &mut egui::Ui,
        bookmarks: &[Bookmark],
        selected: Option<usize>,
    ) -> (Option<usize>, Option<usize>) {
        ui.horizontal_wrapped(|ui| {
            for system in ["Stanton", "Pyro", "Nyx"] {
                if ui.selectable_label(self.system == system, system).clicked() {
                    self.system = system.into();
                }
            }
            if ui.button("Open selected system").clicked() {
                self.overview = false;
                self.fit = true;
            }
        });
        ui.weak("Click a system to focus · Double-click to explore · Jump links are schematic; each system has its own scale");
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(
                ui.available_width().max(1.),
                ui.available_height().max(180.),
            ),
            egui::Sense::click(),
        );
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4., Color32::from_rgb(5, 13, 22));
        for s in &self.data.background {
            painter.circle_filled(
                rect.min + egui::vec2(s[0] * rect.width(), s[1] * rect.height()),
                0.7,
                Color32::from_gray(40 + (s[2] * 65.) as u8),
            );
        }
        let systems = ["Stanton", "Pyro", "Nyx"];
        let centers = [[0.25, 0.34], [0.75, 0.34], [0.5, 0.74]]
            .map(|[x, y]| rect.min + egui::vec2(rect.width() * x, rect.height() * y));
        let radius = (rect.width() * 0.205).min(rect.height() * 0.205).max(35.);
        let target = selected.and_then(|i| bookmarks.get(i));
        let current = self.ship.as_ref().map(|s| s.0.as_str());
        let mut cameras = Vec::new();
        for system in systems {
            let extent = self
                .display
                .markers
                .iter()
                .filter(|m| m.system == system)
                .map(|m| m.xyz_km[0].hypot(m.xyz_km[1]))
                .fold(1_f64, f64::max)
                .max(
                    self.data
                        .features
                        .belts
                        .iter()
                        .filter(|b| b.system == system && b.parent.is_empty())
                        .map(|b| b.outer_km)
                        .fold(0_f64, f64::max),
                );
            cameras.push(Camera {
                center: [0.; 3],
                yaw: 0.,
                tilt: std::f64::consts::FRAC_PI_2,
                scale: radius as f64 * 0.86 / extent,
            });
        }
        let at = |index: usize, xyz| {
            let [x, y] = cameras[index].project(xyz);
            centers[index] + egui::vec2(x as f32, y as f32)
        };
        // Lines connect the gateway coordinates, not the centers of unrelated coordinate frames.
        for (a, b) in [(0, 1), (1, 2), (0, 2)] {
            let ma =
                self.display.markers.iter().find(|m| {
                    m.system == systems[a] && m.name == format!("{} Gateway", systems[b])
                });
            let mb =
                self.display.markers.iter().find(|m| {
                    m.system == systems[b] && m.name == format!("{} Gateway", systems[a])
                });
            if let (Some(ma), Some(mb)) = (ma, mb) {
                let pa = at(a, ma.xyz_km);
                let pb = at(b, mb.xyz_km);
                let active = target.is_some_and(|t| {
                    (current == Some(systems[a]) && t.system == systems[b])
                        || (current == Some(systems[b]) && t.system == systems[a])
                });
                let color = if active {
                    AMBER
                } else {
                    Color32::from_rgb(48, 108, 126)
                };
                painter.line_segment(
                    [pa, pb],
                    Stroke::new(if active { 2_f32 } else { 1_f32 }, color),
                );
                let middle = pa + (pb - pa) * 0.5;
                let text = format!("{} / {}", systems[a], systems[b]);
                let galley = painter.layout_no_wrap(text, egui::FontId::monospace(10.), color);
                let bg = egui::Rect::from_center_size(middle, galley.size() + egui::vec2(12., 8.));
                painter.rect_filled(bg, 3., Color32::from_rgb(7, 20, 29));
                painter.galley(bg.min + egui::vec2(6., 4.), galley, color);
            }
        }
        let mut gateway_hits = Vec::new();
        for (i, system) in systems.iter().enumerate() {
            let center = centers[i];
            let focus = self.system == *system;
            painter.circle_filled(
                center,
                radius,
                Color32::from_rgba_unmultiplied(9, 26, 36, 220),
            );
            painter.circle_stroke(
                center,
                radius,
                Stroke::new(
                    if focus { 1.8_f32 } else { 0.7_f32 },
                    if focus {
                        CYAN
                    } else {
                        Color32::from_rgb(32, 66, 82)
                    },
                ),
            );
            // Inset maps retain actual relative positions and do not animate.
            for m in self
                .display
                .markers
                .iter()
                .filter(|m| m.system == *system && map::is_primary(system, &m.name))
            {
                let origin = map::orbit_center(&self.display, m);
                painter.circle_stroke(
                    at(i, origin),
                    ((m.xyz_km[0] - origin[0]).hypot(m.xyz_km[1] - origin[1]) * cameras[i].scale)
                        as f32,
                    Stroke::new(0.6_f32, Color32::from_rgb(35, 74, 85)),
                );
                let p = at(i, m.xyz_km);
                painter.circle_filled(p, 3., CYAN);
                painter.text(
                    p + egui::vec2(5., -5.),
                    egui::Align2::LEFT_BOTTOM,
                    &m.name,
                    egui::FontId::proportional(10.),
                    Color32::from_rgb(155, 191, 201),
                );
            }
            if *system == "Stanton" {
                for (_, xyz) in self.data.dust.iter().step_by(5) {
                    painter.circle_filled(at(i, *xyz), 0.5, Color32::from_rgb(75, 87, 75));
                }
            }
            for belt in self
                .data
                .features
                .belts
                .iter()
                .filter(|b| b.system == *system && b.parent.is_empty())
            {
                for p in belt.particles.iter().step_by(3) {
                    painter.circle_filled(at(i, *p), 0.65, Color32::from_rgb(111, 132, 117));
                }
                let pos = at(i, [0., -(belt.inner_km + belt.outer_km) / 2., 0.]);
                painter.text(
                    pos,
                    egui::Align2::CENTER_BOTTOM,
                    &belt.name,
                    egui::FontId::proportional(9.),
                    AMBER,
                );
            }
            if *system == "Nyx" {
                if let Some(delamar) = self
                    .display
                    .markers
                    .iter()
                    .find(|m| m.system == "Nyx" && m.name == "Delamar")
                {
                    let pos = at(i, delamar.xyz_km);
                    painter.circle_filled(pos, 3., CYAN);
                    painter.text(
                        pos + egui::vec2(5., 7.),
                        egui::Align2::LEFT_TOP,
                        "Delamar / Levski",
                        egui::FontId::proportional(10.),
                        CYAN,
                    );
                }
            }
            let star = at(i, self.data.stars[*system].xyz_km);
            painter.circle_filled(star, 10., Color32::from_rgba_unmultiplied(255, 192, 90, 20));
            painter.circle_filled(star, 4., AMBER);
            for (index, m) in self.display.markers.iter().enumerate().filter(|(_, m)| {
                m.system == *system && systems.iter().any(|s| m.name == format!("{s} Gateway"))
            }) {
                let p = at(i, m.xyz_km);
                painter.rect_stroke(
                    egui::Rect::from_center_size(p, egui::vec2(8., 8.)),
                    1.,
                    Stroke::new(1.3_f32, CYAN),
                    egui::StrokeKind::Inside,
                );
                gateway_hits.push((p, index));
                if self.marker == Some(index) {
                    painter.text(
                        p + egui::vec2(7., 7.),
                        egui::Align2::LEFT_TOP,
                        &m.name,
                        egui::FontId::proportional(11.),
                        CYAN,
                    );
                }
            }
            if let Some(t) = target.filter(|t| t.system == *system) {
                let p = at(i, t.xyz_km);
                painter.circle_stroke(p, 6., Stroke::new(2_f32, AMBER));
                painter.text(
                    p + egui::vec2(8., -10.),
                    egui::Align2::LEFT_BOTTOM,
                    "DESTINATION",
                    egui::FontId::monospace(10.),
                    AMBER,
                );
            }
            if let Some((_, xyz)) = self.ship.as_ref().filter(|s| s.0 == *system) {
                let p = at(i, *xyz);
                painter.circle_filled(p, 4., Color32::WHITE);
                painter.text(
                    p + egui::vec2(7., 8.),
                    egui::Align2::LEFT_TOP,
                    "YOU",
                    egui::FontId::monospace(10.),
                    Color32::WHITE,
                );
            }
            let name = center - egui::vec2(0., radius + 10.);
            painter.text(
                name,
                egui::Align2::CENTER_BOTTOM,
                system.to_uppercase(),
                egui::FontId::proportional(19.),
                if focus { CYAN } else { Color32::WHITE },
            );
            let status = match (
                current == Some(*system),
                target.is_some_and(|t| t.system == *system),
            ) {
                (true, true) => "CURRENT / DESTINATION",
                (true, false) => "CURRENT SYSTEM",
                (false, true) => "DESTINATION SYSTEM",
                _ => "",
            };
            painter.text(
                center + egui::vec2(0., radius + 8.),
                egui::Align2::CENTER_TOP,
                status,
                egui::FontId::monospace(10.),
                AMBER,
            );
        }
        if response.clicked() || response.double_clicked() {
            if let Some(cursor) = response.interact_pointer_pos() {
                if let Some((_, index)) = gateway_hits
                    .iter()
                    .filter(|(p, _)| p.distance(cursor) < 12.)
                    .min_by(|a, b| a.0.distance(cursor).total_cmp(&b.0.distance(cursor)))
                {
                    self.marker = Some(*index);
                    self.system = self.display.markers[*index].system.clone();
                } else if let Some(i) = (0..3).find(|&i| centers[i].distance(cursor) < radius + 28.)
                {
                    self.system = systems[i].into();
                    self.marker = None;
                }
                if response.double_clicked() {
                    self.overview = false;
                    self.fit = true;
                }
            }
        }
        (None, None)
    }
}
