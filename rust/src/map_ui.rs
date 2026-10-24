#[path = "map_overview.rs"]
mod overview;
use eframe::egui::{self, Color32, Pos2, Stroke};
use sc_bookmarks::{
    Bookmark,
    map::{self, Camera, Data},
    routes::{self, Catalog},
};
const CYAN: Color32 = Color32::from_rgb(104, 223, 235);
const AMBER: Color32 = Color32::from_rgb(239, 189, 112);
pub struct MapView {
    data: Data,
    pub refinery_points: Vec<(String, Bookmark)>,
    refineries: bool,
    refinery_selected: Option<String>,
    pending_refinery: Option<Bookmark>,
    display: Catalog,
    overview: bool,
    camera: Camera,
    system: String,
    marker: Option<usize>,
    orbits: bool,
    halo: bool,
    belts: bool,
    stations: bool,
    selected_station: Option<usize>,
    band: u32,
    pub ship: Option<(String, [f64; 3])>,
    fit: bool,
    route_key: Option<(String, [f64; 3])>,
    route_points: Vec<[f64; 3]>,
    route_names: Vec<String>,
}
impl Default for MapView {
    fn default() -> Self {
        Self {
            refinery_points: Vec::new(),
            refineries: true,
            refinery_selected: None,
            pending_refinery: None,
            data: Data::load().expect("Embedded map data"),
            display: {
                let mut base = routes::catalog().expect("Map catalog");
                let extra: Catalog = serde_json::from_str(include_str!(
                    "../../route-data/map-reference-points.json"
                ))
                .expect("Map reference points");
                for point in extra.markers {
                    if !base
                        .markers
                        .iter()
                        .any(|m| m.system == point.system && m.name == point.name)
                    {
                        base.markers.push(point);
                    }
                }
                base
            },
            overview: true,
            camera: Camera::default(),
            system: "Stanton".into(),
            marker: None,
            orbits: true,
            halo: true,
            belts: true,
            stations: true,
            selected_station: None,
            band: 0,
            ship: None,
            fit: true,
            route_key: None,
            route_points: Vec::new(),
            route_names: Vec::new(),
        }
    }
}
impl MapView {
    pub fn focus_refinery(&mut self, b: &Bookmark) {
        self.pending_refinery = Some(b.clone());
    }
    fn update_route(&mut self, target: Option<&Bookmark>, catalog: &Catalog) {
        let key = target.map(|b| (b.system.clone(), b.xyz_km));
        if key != self.route_key {
            self.route_points.clear();
            self.route_names.clear();
            if let Some(b) = target {
                // A moving/updated target must not reset the camera or undo a
                // manually selected system. Only a change of target system can fit.
                let target_system_changed = self
                    .route_key
                    .as_ref()
                    .is_none_or(|(system, _)| !system.eq_ignore_ascii_case(&b.system));
                if target_system_changed && !self.system.eq_ignore_ascii_case(&b.system) {
                    self.system = b.system.clone();
                    self.marker = None;
                    self.fit = true;
                }
                if let Ok(Some(route)) = routes::suggest(b, &catalog.markers) {
                    self.route_names = route.names().into_iter().map(str::to_owned).collect();
                    match route {
                        routes::Route::Celestial(m) => self.route_points.push(m.xyz_km),
                        routes::Route::Pair { a, b, .. } => {
                            self.route_points.push(a.xyz_km);
                            self.route_points.push(b.xyz_km);
                        }
                    }
                }
            }
            self.route_key = key;
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &Catalog,
        bookmarks: &[Bookmark],
        selected: Option<usize>,
    ) -> (Option<usize>, Option<usize>) {
        let target = selected.and_then(|i| bookmarks.get(i));
        self.update_route(target, catalog);
        if let Some(b) = self.pending_refinery.take() {
            self.system = b.system;
            self.overview = false;
            self.fit = false;
            self.camera.center = b.xyz_km;
            self.camera.scale = 0.00003;
            self.refineries = true;
            self.refinery_selected = Some(b.name);
        }
        crate::theme::eyebrow(ui, "STELLAR CARTOGRAPHY / ROOT COORDINATES");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.overview, true, "All systems");
            ui.selectable_value(&mut self.overview, false, "System detail");
        });
        if self.overview {
            return self.show_overview(ui, bookmarks, selected);
        }
        let catalog = &self.display;
        let old_system = self.system.clone();
        let mut fit_route = false;
        ui.horizontal_wrapped(|ui| {
            for system in ["Stanton", "Pyro", "Nyx"] {
                ui.selectable_value(&mut self.system, system.into(), system);
            }
            if ui.button("Fit system").clicked() {
                self.fit = true;
            }
            if ui
                .add_enabled(
                    target.is_some_and(|b| b.system == self.system),
                    egui::Button::new("Fit route"),
                )
                .clicked()
            {
                fit_route = true;
            }
            if ui.button("Top").clicked() {
                self.camera.tilt = std::f64::consts::FRAC_PI_2;
            }
            if ui.button("3D").clicked() {
                self.camera.tilt = 0.65;
            }
            if ui.button("Side").clicked() {
                self.camera.tilt = 0.001;
            }
            if ui.button("Focus star").clicked() {
                if let Some(star) = self.data.stars.get(&self.system) {
                    self.camera.center = star.xyz_km;
                }
            }
        });
        if old_system != self.system {
            self.marker = None;
            self.fit = true;
        }
        let mut focus_points = None;
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.refineries, "Refineries");
            ui.checkbox(&mut self.belts, "Asteroid belts");
            ui.checkbox(&mut self.stations, "Major stations");
            ui.checkbox(&mut self.orbits, "Planet rings");
            ui.checkbox(&mut self.halo, "Aaron Halo");
            egui::ComboBox::from_id_salt("halo_band")
                .selected_text(if self.band == 0 {
                    "All bands".into()
                } else {
                    format!("Band {}", self.band)
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.band, 0, "All bands");
                    for b in &self.data.halo.bands {
                        ui.selectable_value(&mut self.band, b.number, format!("Band {}", b.number));
                    }
                });
        });
        egui::ComboBox::from_id_salt("focus_landmark")
            .selected_text("Focus landmark...")
            .show_ui(ui, |ui| {
                for belt in self
                    .data
                    .features
                    .belts
                    .iter()
                    .filter(|b| b.system == self.system)
                {
                    if ui.selectable_label(false, &belt.name).clicked() {
                        if let Some(center) = map::parent_position(
                            catalog,
                            &belt.system,
                            &belt.parent,
                            belt.center_km,
                        ) {
                            focus_points = Some(
                                (0..32)
                                    .map(|i| {
                                        let a = i as f64 * std::f64::consts::TAU / 32.;
                                        [
                                            center[0] + belt.outer_km * a.cos(),
                                            center[1] + belt.outer_km * a.sin(),
                                            center[2],
                                        ]
                                    })
                                    .collect::<Vec<_>>(),
                            );
                            self.belts = true;
                        }
                    }
                }
                for (index, station) in self
                    .data
                    .features
                    .stations
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.system == self.system)
                {
                    if ui.selectable_label(false, &station.name).clicked() {
                        if let (Some(center), Some(point)) = (
                            map::parent_position(
                                catalog,
                                &station.system,
                                &station.parent,
                                [0.; 3],
                            ),
                            map::parent_position(
                                catalog,
                                &station.system,
                                &station.parent,
                                station.offset_km,
                            ),
                        ) {
                            focus_points = Some(vec![center, point]);
                            self.selected_station = Some(index);
                            self.stations = true;
                        }
                    }
                }
            });
        // Reserve a compact profile strip even without a route, preventing
        // layout changes when destination data becomes temporarily unavailable.
        let (profile, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 72.), egui::Sense::hover());
        if self.route_points.len() == 2 {
            if let Some(target) = target.filter(|b| b.system == self.system) {
                let painter = ui.painter_at(profile);
                let a = self.route_points[0];
                let b = self.route_points[1];
                let d: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
                let length: f64 = d.iter().map(|x| x * x).sum();
                let fraction = |p: [f64; 3]| {
                    ((0..3).map(|i| (p[i] - a[i]) * d[i]).sum::<f64>() / length).clamp(0., 1.)
                };
                let at = |f: f64, y: f32| {
                    egui::pos2(
                        profile.left() + 24. + (profile.width() - 48.) * f as f32,
                        profile.top() + y,
                    )
                };
                painter.text(
                    at(0., 4.),
                    egui::Align2::LEFT_TOP,
                    "ROUTE PROFILE / OFFSET MAGNIFIED",
                    egui::FontId::monospace(10.),
                    Color32::GRAY,
                );
                painter.line_segment([at(0., 47.), at(1., 47.)], Stroke::new(1_f32, CYAN));
                for (i, name) in self.route_names.iter().enumerate() {
                    painter.text(
                        at(i as f64, 63.),
                        if i == 0 {
                            egui::Align2::LEFT_CENTER
                        } else {
                            egui::Align2::RIGHT_CENTER
                        },
                        name,
                        egui::FontId::proportional(10.),
                        CYAN,
                    );
                }
                let f = fraction(target.xyz_km);
                painter.line_segment([at(f, 47.), at(f, 26.)], Stroke::new(1_f32, AMBER));
                painter.circle_stroke(at(f, 26.), 5., Stroke::new(1_f32, AMBER));
                painter.text(
                    at(f, 19.),
                    egui::Align2::CENTER_BOTTOM,
                    "DESTINATION",
                    egui::FontId::monospace(10.),
                    AMBER,
                );
                if let Some((system, p)) = &self.ship {
                    if system == &self.system {
                        painter.circle_filled(at(fraction(*p), 47.), 4., Color32::WHITE);
                        painter.text(
                            at(fraction(*p), 39.),
                            egui::Align2::CENTER_BOTTOM,
                            "YOU",
                            egui::FontId::monospace(10.),
                            Color32::WHITE,
                        );
                    }
                }
            }
        }
        ui.weak("Drag: orbit · Shift-drag: pan · Wheel: zoom · Click QT point: name · Double-click bookmark: edit");
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(
                ui.available_width().max(1.),
                ui.available_height().max(100.),
            ),
            egui::Sense::click_and_drag(),
        );
        let size = [rect.width() as f64, rect.height() as f64];
        if self.fit {
            let mut points: Vec<_> = catalog
                .markers
                .iter()
                .filter(|m| m.system == self.system)
                .map(|m| m.xyz_km)
                .chain(
                    bookmarks
                        .iter()
                        .filter(|b| b.system == self.system)
                        .map(|b| b.xyz_km),
                )
                .collect();
            if let Some(star) = self.data.stars.get(&self.system) {
                points.push(star.xyz_km);
            }
            for belt in self
                .data
                .features
                .belts
                .iter()
                .filter(|b| b.system == self.system && b.parent.is_empty())
            {
                for i in 0..32 {
                    let a = i as f64 * std::f64::consts::TAU / 32.;
                    points.push([
                        belt.center_km[0] + belt.outer_km * a.cos(),
                        belt.center_km[1] + belt.outer_km * a.sin(),
                        belt.center_km[2],
                    ]);
                }
            }
            self.camera.fit(&points, size);
            self.fit = false;
        }
        if fit_route {
            let mut points = self.route_points.clone();
            if let Some(b) = target {
                points.push(b.xyz_km);
            }
            self.camera.fit(&points, size);
        }
        if let Some(points) = focus_points {
            self.camera.fit(&points, size);
        }
        if response.dragged() {
            let d = ui.input(|i| i.pointer.delta());
            if ui.input(|i| i.modifiers.shift) {
                self.camera.pan([d.x as f64, d.y as f64]);
            } else {
                self.camera.yaw += d.x as f64 * 0.008;
                self.camera.tilt = (self.camera.tilt + d.y as f64 * 0.006)
                    .clamp(0.001, std::f64::consts::FRAC_PI_2);
            }
        }
        if response.hovered() {
            let delta = ui.input(|i| i.smooth_scroll_delta.y);
            self.camera.scale =
                (self.camera.scale * (delta as f64 * 0.002).exp()).clamp(1e-12, 1e4);
        }
        let painter = ui.painter_at(rect);
        let mut occupied_labels = Vec::new();
        painter.rect_filled(rect, 4., Color32::from_rgb(6, 15, 24));
        for s in &self.data.background {
            let p = rect.min + egui::vec2(s[0] * rect.width(), s[1] * rect.height());
            let value = 50 + (s[2] * 90.) as u8;
            painter.circle_filled(
                p,
                if s[2] > 0.95 { 1.1 } else { 0.6 },
                Color32::from_rgb(value, value, value.saturating_add(15)),
            );
        }
        let at = |p| {
            let [x, y] = self.camera.project(p);
            rect.center() + egui::vec2(x as f32, y as f32)
        };
        let ring = |center: [f64; 3], radius: f64, color: Color32, width: f32| {
            let points = (0..=256)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / 256.;
                    at([
                        center[0] + radius * a.cos(),
                        center[1] + radius * a.sin(),
                        center[2],
                    ])
                })
                .collect();
            painter.add(egui::Shape::line(points, Stroke::new(width, color)));
        };
        // Static world-plane grid, scaled to readable spacing at each zoom level.
        let step = 10_f64.powf((90. / self.camera.scale).log10().ceil());
        let gx = (self.camera.center[0] / step).floor() * step;
        let gy = (self.camera.center[1] / step).floor() * step;
        for i in -18..=18 {
            let offset = i as f64 * step;
            let color = Color32::from_rgba_unmultiplied(50, 100, 116, 45);
            painter.line_segment(
                [
                    at([gx + offset, gy - 18. * step, 0.]),
                    at([gx + offset, gy + 18. * step, 0.]),
                ],
                Stroke::new(0.6_f32, color),
            );
            painter.line_segment(
                [
                    at([gx - 18. * step, gy + offset, 0.]),
                    at([gx + 18. * step, gy + offset, 0.]),
                ],
                Stroke::new(0.6_f32, color),
            );
        }
        painter.rect_stroke(
            rect,
            2.,
            Stroke::new(1_f32, crate::theme::EDGE),
            egui::StrokeKind::Inside,
        );
        for (corner, sx, sy) in [
            (rect.left_top(), 1., 1.),
            (rect.right_top(), -1., 1.),
            (rect.left_bottom(), 1., -1.),
            (rect.right_bottom(), -1., -1.),
        ] {
            painter.line_segment(
                [corner, corner + egui::vec2(22. * sx, 0.)],
                Stroke::new(2_f32, CYAN),
            );
            painter.line_segment(
                [corner, corner + egui::vec2(0., 22. * sy)],
                Stroke::new(2_f32, CYAN),
            );
        }
        painter.text(
            rect.left_top() + egui::vec2(16., 16.),
            egui::Align2::LEFT_TOP,
            format!("{} / SYSTEM VIEW", self.system.to_uppercase()),
            egui::FontId::monospace(12.),
            CYAN,
        );
        painter.text(
            rect.left_bottom() + egui::vec2(16., -14.),
            egui::Align2::LEFT_BOTTOM,
            "◇ DESTINATION    □ QT POINT    R REFINERY APPROACH    ☼ STAR / NO QT",
            egui::FontId::monospace(10.),
            Color32::from_rgb(140, 166, 181),
        );
        if self.orbits {
            for (i, m) in catalog
                .markers
                .iter()
                .enumerate()
                .filter(|(_, m)| m.system == self.system && map::is_primary(&m.system, &m.name))
            {
                let center = map::orbit_center(catalog, m);
                ring(
                    center,
                    (m.xyz_km[0] - center[0]).hypot(m.xyz_km[1] - center[1]),
                    if self.marker == Some(i) {
                        CYAN
                    } else {
                        Color32::from_rgb(38, 80, 94)
                    },
                    if self.marker == Some(i) { 1.5 } else { 0.7 },
                );
            }
        }
        if self.halo && self.system == "Stanton" {
            for (band, p) in &self.data.dust {
                let point = at(*p);
                if rect.contains(point) {
                    painter.circle_filled(
                        point,
                        0.65,
                        if *band == self.band {
                            AMBER
                        } else {
                            Color32::from_rgb(88, 104, 94)
                        },
                    );
                }
            }
            for b in &self.data.halo.bands {
                let selected = self.band == b.number;
                if selected || (b.outer_km - b.inner_km) * self.camera.scale > 4. {
                    for r in [b.inner_km, b.outer_km] {
                        ring(
                            self.data.halo.origin_km,
                            r,
                            if selected {
                                AMBER
                            } else {
                                Color32::from_rgb(61, 75, 71)
                            },
                            0.6,
                        );
                    }
                }
                if selected || b.number == 5 {
                    let a = -self.camera.yaw - std::f64::consts::FRAC_PI_2;
                    let r = (b.inner_km + b.outer_km) / 2.;
                    let p = at([r * a.cos(), r * a.sin(), 0.]);
                    let text = if selected {
                        format!("AARON HALO · Band {}", b.number)
                    } else {
                        "AARON HALO".into()
                    };
                    label(&painter, &mut occupied_labels, p, &text, AMBER);
                }
            }
        }
        if self.belts {
            for belt in self
                .data
                .features
                .belts
                .iter()
                .filter(|b| b.system == self.system)
            {
                let Some(center) =
                    map::parent_position(catalog, &belt.system, &belt.parent, belt.center_km)
                else {
                    continue;
                };
                if belt.outer_km * self.camera.scale < 5. {
                    continue;
                }
                let color = Color32::from_rgb(127, 145, 132);
                for radius in [belt.inner_km, belt.outer_km] {
                    ring(center, radius, color.gamma_multiply(0.5), 0.7);
                }
                for p in &belt.particles {
                    let pos = at(std::array::from_fn(|i| center[i] + p[i]));
                    if rect.contains(pos) {
                        painter.circle_filled(pos, 0.75, color);
                    }
                }
                let a = -self.camera.yaw - std::f64::consts::FRAC_PI_2;
                let r = (belt.inner_km + belt.outer_km) / 2.;
                let pos = at([center[0] + r * a.cos(), center[1] + r * a.sin(), center[2]]);
                if rect.contains(pos) {
                    label(&painter, &mut occupied_labels, pos, &belt.name, AMBER);
                }
            }
        }
        let active = target.filter(|b| b.system == self.system);
        if active.is_some() {
            if self.route_points.len() == 2 {
                painter.line_segment(
                    [at(self.route_points[0]), at(self.route_points[1])],
                    Stroke::new(2_f32, CYAN),
                );
            }
        }
        let mut hits: Vec<(Pos2, bool, usize)> = Vec::new();
        for (i, m) in catalog
            .markers
            .iter()
            .enumerate()
            .filter(|(_, m)| m.system == self.system)
        {
            let p = at(m.xyz_km);
            if !rect.contains(p) {
                continue;
            }
            let important =
                self.marker == Some(i) || active.is_some() && self.route_names.contains(&m.name);
            let color = if important {
                CYAN
            } else {
                Color32::from_rgb(146, 170, 184)
            };
            if important {
                painter.circle_stroke(p, 12., Stroke::new(1_f32, CYAN));
                painter.circle_stroke(
                    p,
                    16.,
                    Stroke::new(0.7_f32, Color32::from_rgba_unmultiplied(104, 223, 235, 70)),
                );
            }
            if m.body_radius_km > 0. {
                painter.circle_filled(p, 7., Color32::from_rgb(29, 65, 82));
                painter.circle_stroke(p, 7., Stroke::new(1_f32, color));
                painter.circle_filled(p - egui::vec2(2., 2.), 3., color);
            } else {
                painter.rect_stroke(
                    egui::Rect::from_center_size(p, egui::vec2(7., 7.)),
                    0.,
                    Stroke::new(1_f32, color),
                    egui::StrokeKind::Inside,
                );
            }
            if important || map::is_primary(&m.system, &m.name) {
                label(&painter, &mut occupied_labels, p, &m.name, color);
            }
            hits.push((p, false, i));
        }
        if let Some(star) = self.data.stars.get(&self.system) {
            let p = at(star.xyz_km);
            if rect.contains(p) {
                for r in (1..=5).rev() {
                    painter.circle_filled(
                        p,
                        r as f32 * 4.,
                        Color32::from_rgba_unmultiplied(
                            255,
                            210,
                            125,
                            if r == 1 { 255 } else { 12 },
                        ),
                    );
                }
                label(
                    &painter,
                    &mut occupied_labels,
                    p,
                    &format!("{} star · No QT", self.system),
                    AMBER,
                );
            }
        }
        for (i, b) in bookmarks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.system == self.system)
        {
            let p = at(b.xyz_km);
            if !rect.contains(p) {
                continue;
            }
            let color = if selected == Some(i) {
                AMBER
            } else {
                Color32::from_rgb(119, 191, 176)
            };
            painter.circle_stroke(
                p,
                if selected == Some(i) { 8. } else { 4. },
                Stroke::new(2_f32, color),
            );
            if selected == Some(i) {
                label(&painter, &mut occupied_labels, p, &b.name, color);
            }
            hits.push((p, true, i));
        }
        let mut refinery_hits = Vec::new();
        if self.refineries {
            for (name, b) in self
                .refinery_points
                .iter()
                .filter(|(_, b)| b.system == self.system)
            {
                let p = at(b.xyz_km);
                if !rect.contains(p) {
                    continue;
                }
                painter.text(
                    p,
                    egui::Align2::CENTER_CENTER,
                    "R",
                    egui::FontId::monospace(12.),
                    AMBER,
                );
                refinery_hits.push((p, name.clone()));
                if let Some((_, badge)) = name.rsplit_once(" / ") {
                    if badge.ends_with(" READY") {
                        painter.text(
                            p + egui::vec2(10., -10.),
                            egui::Align2::LEFT_CENTER,
                            badge,
                            egui::FontId::monospace(10.),
                            AMBER,
                        );
                    }
                }
                if self
                    .refinery_selected
                    .as_ref()
                    .is_some_and(|n| n == name || n == &b.name)
                {
                    label(
                        &painter,
                        &mut occupied_labels,
                        p,
                        &format!("{name} (approach)"),
                        AMBER,
                    );
                }
            }
        }
        let mut station_hits = Vec::new();
        if self.stations {
            for (index, station) in self
                .data
                .features
                .stations
                .iter()
                .enumerate()
                .filter(|(_, s)| s.system == self.system)
            {
                let Some(xyz) = map::parent_position(
                    catalog,
                    &station.system,
                    &station.parent,
                    station.offset_km,
                ) else {
                    continue;
                };
                let p = at(xyz);
                if !rect.contains(p) {
                    continue;
                }
                let highlighted = self.selected_station == Some(index);
                let color = if highlighted {
                    AMBER
                } else {
                    Color32::from_rgb(135, 203, 184)
                };
                painter.rect_stroke(
                    egui::Rect::from_center_size(p, egui::vec2(10., 10.)),
                    2.,
                    Stroke::new(1.5_f32, color),
                    egui::StrokeKind::Inside,
                );
                station_hits.push((p, index));
                // Label references when their parent and station are separated at this zoom.
                let separation =
                    station.offset_km.iter().map(|x| x * x).sum::<f64>().sqrt() * self.camera.scale;
                if highlighted || separation > 18. {
                    label(&painter, &mut occupied_labels, p, &station.name, color);
                }
            }
        }
        if let Some((system, xyz)) = &self.ship {
            if system == &self.system {
                let p = at(*xyz);
                if rect.contains(p) {
                    painter.circle_filled(p, 5., Color32::WHITE);
                    painter.circle_stroke(p, 11., Stroke::new(1.5_f32, Color32::WHITE));
                    label(&painter, &mut occupied_labels, p, "YOU", Color32::WHITE);
                    if let Some(b) = active {
                        painter.line_segment([p, at(b.xyz_km)], Stroke::new(0.6_f32, AMBER));
                    }
                }
            }
        }
        let mut pick = None;
        let mut edit = None;
        if response.clicked() || response.double_clicked() {
            if let Some(cursor) = response.interact_pointer_pos() {
                self.refinery_selected = refinery_hits
                    .iter()
                    .filter(|(p, _)| p.distance(cursor) < 12.)
                    .min_by(|a, b| a.0.distance(cursor).total_cmp(&b.0.distance(cursor)))
                    .map(|(_, n)| n.clone());
                self.selected_station = station_hits
                    .iter()
                    .filter(|(p, _)| p.distance(cursor) < 12.)
                    .min_by(|a, b| a.0.distance(cursor).total_cmp(&b.0.distance(cursor)))
                    .map(|(_, i)| *i);
                let nearest = hits
                    .iter()
                    .filter(|(p, _, _)| p.distance(cursor) < 14.)
                    .min_by(|a, b| a.0.distance(cursor).total_cmp(&b.0.distance(cursor)));
                if let Some((_, bookmark, index)) = nearest {
                    if *bookmark {
                        pick = Some(*index);
                        if response.double_clicked() {
                            edit = Some(*index);
                        }
                    } else {
                        self.marker = Some(*index);
                    }
                } else {
                    self.marker = None;
                }
            }
        }
        (pick, edit)
    }
}
fn label(
    painter: &egui::Painter,
    occupied: &mut Vec<egui::Rect>,
    p: Pos2,
    text: &str,
    color: Color32,
) {
    let font = egui::FontId::proportional(12.);
    let galley = painter.layout_no_wrap(text.to_owned(), font, color);
    let clip = painter.clip_rect();
    let size = galley.size();
    let mut pos = egui::pos2(
        (p.x + 12.)
            .min(clip.right() - size.x - 8.)
            .max(clip.left() + 8.),
        (p.y - 20.).max(clip.top() + 36.),
    );
    let base = pos;
    for step in 0..24 {
        let offset = if step % 2 == 0 {
            -(step as f32 / 2.) * 20.
        } else {
            (step as f32 + 1.) / 2. * 20.
        };
        pos.y = (base.y + offset).clamp(
            clip.top() + 36.,
            (clip.bottom() - size.y - 20.).max(clip.top() + 36.),
        );
        let bounds = egui::Rect::from_min_size(pos - egui::vec2(4., 2.), size + egui::vec2(8., 4.));
        if !occupied.iter().any(|r| r.intersects(bounds)) {
            break;
        }
    }
    let bounds = egui::Rect::from_min_size(pos - egui::vec2(4., 2.), size + egui::vec2(8., 4.));
    occupied.push(bounds);
    if (pos - base).length() > 5. {
        painter.line_segment(
            [p, egui::pos2(pos.x, pos.y + size.y / 2.)],
            Stroke::new(0.6_f32, color.gamma_multiply(0.6)),
        );
    }
    painter.rect_filled(bounds, 1., Color32::from_rgba_unmultiplied(7, 20, 29, 225));
    painter.galley(pos, galley, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target(system: &str, xyz_km: [f64; 3]) -> Bookmark {
        Bookmark {
            name: "Moving target".into(),
            system: system.into(),
            xyz_km,
            metadata: Default::default(),
        }
    }
    #[test]
    fn display_contains_nyx_planets_and_all_gateway_pairs() {
        let map = MapView::default();
        assert_eq!(
            map.display
                .markers
                .iter()
                .filter(|m| m.system == "Nyx" && map::is_primary("Nyx", &m.name))
                .count(),
            3
        );
        for a in ["Stanton", "Pyro", "Nyx"] {
            for b in ["Stanton", "Pyro", "Nyx"] {
                if a != b {
                    assert!(map.display.markers.iter().any(|m| m.system == a
                        && m.name == format!("{b} Gateway")
                        && m.xyz_km.iter().all(|v| v.is_finite())));
                }
            }
        }
    }
    #[test]
    fn route_updates_preserve_camera_and_manual_system_selection() {
        let catalog = routes::catalog().unwrap();
        let mut map = MapView::default();
        map.fit = false;
        map.camera.scale = 0.123;
        map.camera.center = [12., 34., 56.];
        map.camera.yaw = 0.7;
        map.camera.tilt = 0.9;
        map.marker = Some(3);
        for i in 0..10 {
            map.update_route(Some(&target("Stanton", [i as f64, 2., 3.])), &catalog);
            assert!(!map.fit);
            assert_eq!(map.camera.scale, 0.123);
            assert_eq!(map.camera.center, [12., 34., 56.]);
            assert_eq!(map.camera.yaw, 0.7);
            assert_eq!(map.camera.tilt, 0.9);
            assert_eq!(map.marker, Some(3));
        }
        map.system = "Pyro".into(); // user browses another system
        map.update_route(Some(&target("Stanton", [20., 2., 3.])), &catalog);
        assert_eq!(map.system, "Pyro");
        assert!(!map.fit);
        map.update_route(None, &catalog);
        assert!(!map.fit);
        map.update_route(Some(&target("Nyx", [1., 2., 3.])), &catalog);
        assert_eq!(map.system, "Nyx");
        assert!(map.fit);
    }
}
