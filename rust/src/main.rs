#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod clear_text;
mod community_ui;
use eframe::egui;
mod capture_ui;
mod library_ui;
mod live_ui;
mod map_ui;
mod mining_editor;
mod overlay_ui;
mod scanning_ui;
mod theme;
use sc_bookmarks::{Bookmark, links, routes, storage};
use std::{collections::BTreeMap, path::PathBuf};
struct Desktop {
    organization: sc_bookmarks::organization::Organization,
    category_editor: Option<Vec<(String, String)>>,
    organization_blocked: bool,
    migration: String,
    show_migration: bool,
    last_inbox: std::time::Instant,
    last_logged: Option<std::time::Instant>,
    initial_size: bool,
    overlay_diagnostic_at: std::time::Instant,
    bookmarks: Vec<Bookmark>,
    path: PathBuf,
    selected: Option<usize>,
    catalog: routes::Catalog,
    status: String,
    editor: Option<Bookmark>,
    editing: Option<usize>,
    paste: String,
    show_import: bool,
    delete_pending: bool,
    route: String,
    current_route: Option<String>,
    community_refresh: std::time::Instant,
    route_checked: std::time::Instant,
    load_failed: bool,
    scanning: scanning_ui::ScanningView,
    show_scanning: bool,
    library: library_ui::LibraryView,
    overlay_settings: overlay_ui::OverlaySettingsView,
    map: map_ui::MapView,
    show_map: bool,
    capture: capture_ui::CaptureView,
    show_capture: bool,
    show_community: bool,
    community: community_ui::Community,
    community_target: Option<Bookmark>,
    auto_capture_pending: bool,
    auto_capture_check: std::time::Instant,
    live: live_ui::Live,
    capture_generation: u64,
    overlays: sc_bookmarks::native_overlay::Overlays,
    timer: Option<(std::time::Instant, String, String)>,
    capture_name: String,
    capture_category: String,
    contacts: Option<(
        std::time::Instant,
        sc_bookmarks::overlay::Rect,
        Vec<sc_bookmarks::scan_ocr::Contact>,
    )>,
}
impl Desktop {
    fn new() -> Self {
        // Development port uses its own directory; original Python data is never opened implicitly.
        let path = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("."))
            .with_file_name("navigator-data-rust")
            .join("bookmarks.json");
        let (bookmarks, status, load_failed) = match storage::load(&path) {
            Ok(b) => (
                b,
                "Ready · enable r_displayinfo 3 and start capture.".into(),
                false,
            ),
            Err(e) => (
                vec![],
                format!("Library could not load: {e}. Saving disabled."),
                true,
            ),
        };
        let overlay_settings =
            overlay_ui::OverlaySettingsView::new(path.with_file_name("overlay-settings.json"));
        let (organization, organization_blocked) =
            match sc_bookmarks::organization::Organization::load(
                &path.with_file_name("organization.json"),
            ) {
                Ok(s) => (s, false),
                Err(_) => (Default::default(), true),
            };
        let mut library = library_ui::LibraryView::default();
        library.order = organization.categories.clone();
        library.current_only = organization.hide_other_systems;
        library.filter.sort = sc_bookmarks::library::Sort::ALL
            .into_iter()
            .find(|s| s.label() == organization.sort)
            .unwrap_or_default();
        Self {
            organization,
            category_editor: None,
            organization_blocked,
            migration: String::new(),
            show_migration: false,
            last_inbox: std::time::Instant::now(),
            last_logged: None,
            initial_size: true,
            overlay_diagnostic_at: std::time::Instant::now(),
            bookmarks,
            path,
            selected: None,
            catalog: routes::catalog().expect("Embedded catalog"),
            status,
            editor: None,
            editing: None,
            paste: String::new(),
            show_import: false,
            delete_pending: false,
            route: String::new(),
            current_route: None,
            community_refresh: std::time::Instant::now() - std::time::Duration::from_secs(2),
            route_checked: std::time::Instant::now() - std::time::Duration::from_secs(2),
            load_failed,
            scanning: Default::default(),
            show_scanning: false,
            library,
            overlay_settings,
            map: Default::default(),
            show_map: true,
            capture: Default::default(),
            show_capture: false,
            show_community: false,
            community: Default::default(),
            community_target: None,
            auto_capture_pending: true,
            auto_capture_check: std::time::Instant::now() - std::time::Duration::from_secs(5),
            live: Default::default(),
            capture_generation: 0,
            overlays: Default::default(),
            timer: None,
            capture_name: "New space bookmark".into(),
            capture_category: "Personal".into(),
            contacts: None,
        }
    }
    fn persist(&mut self, candidate: Vec<Bookmark>) -> bool {
        if self.load_failed {
            self.status = "Saving disabled after library load failure.".into();
            return false;
        }
        match storage::save(&self.path, &candidate) {
            Ok(()) => {
                self.bookmarks = candidate;
                self.library.invalidate();
                self.status = "Saved locally.".into();
                true
            }
            Err(e) => {
                self.status = format!("Save failed: {e}");
                false
            }
        }
    }
    fn route_label(&self) -> String {
        match &self.current_route {
            Some(v) => format!(
                "Closest approach via {v} (from here) · Catalog: {}",
                self.route
            ),
            None => self.route.clone(),
        }
    }
    fn select(&mut self, i: usize) {
        self.current_route = None;
        self.route_checked = std::time::Instant::now() - std::time::Duration::from_secs(2);
        self.community_target = None;
        self.live.alert.arm(false);
        self.selected = Some(i);
        if !self.capture.running() {
            self.status = "Destination queued; guidance starts when capture is running.".into();
        }
        self.route = match routes::suggest(&self.bookmarks[i], &self.catalog.markers) {
            Ok(Some(r)) => r.label(),
            Ok(None) => "No usable QT pair".into(),
            Err(e) => e.into(),
        };
    }
}
impl eframe::App for Desktop {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.initial_size {
            if let Some(monitor) = ctx.input(|i| i.viewport().monitor_size) {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    1440_f32.min(monitor.x * 0.92),
                    940_f32.min(monitor.y * 0.88),
                )));
                self.initial_size = false;
            }
        }
        self.capture.hide_border = self.overlay_settings.saved.hide_capture_border;
        if self.auto_capture_pending && self.auto_capture_check.elapsed().as_secs() >= 5 {
            self.auto_capture_check = std::time::Instant::now();
            if windows_capture::window::Window::from_name("Star Citizen ")
                .or_else(|_| windows_capture::window::Window::from_name("Star Citizen"))
                .is_ok()
            {
                self.capture.start();
                self.auto_capture_pending = false;
            }
        }
        if self.capture_generation != self.capture.generation {
            self.live = Default::default();
            self.capture_generation = self.capture.generation;
        }
        if self.last_inbox.elapsed().as_millis() >= 500 {
            self.last_inbox = std::time::Instant::now();
            for (path, incoming) in sc_bookmarks::protocol::poll(self.path.parent().unwrap()) {
                match incoming {
                    Ok(b) => {
                        let canonical = links::encode(b.clone()).ok();
                        let duplicate = self.bookmarks.iter().position(|old| {
                            canonical.is_some() && links::encode(old.clone()).ok() == canonical
                        });
                        let index = if let Some(i) = duplicate {
                            Some(i)
                        } else {
                            let i = self.bookmarks.len();
                            let mut all = self.bookmarks.clone();
                            all.push(b);
                            self.persist(all).then_some(i)
                        };
                        if let Some(i) = index {
                            self.select(i);
                            self.library.current_only = false;
                            self.library.filter = Default::default();
                            self.library.invalidate();
                            let _ = std::fs::remove_file(path);
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        }
                    }
                    Err(e) => {
                        self.status = format!("Cannot import shared link: {e}");
                        let _ = std::fs::rename(&path, path.with_extension("failed"));
                    }
                }
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        self.capture.poll(ctx, self.show_capture);
        if let Some(result) = self.capture.scan_result.take() {
            if let Some(contacts) = result.contacts {
                match contacts {
                    Ok(v) if !v.is_empty() => {
                        self.contacts = Some((std::time::Instant::now(), result.desktop, v))
                    }
                    Err(e) => self.status = e,
                    _ => {}
                }
            }
            if let Some(rock) = result.asteroid {
                match rock {
                    Err(e) => self.status = e,
                    Ok(rock) => {
                        if let Some(o) = result.observation {
                            if let (Some(p), Some(camera)) = (o.position, o.camera) {
                                let forward = sc_bookmarks::guidance::camera_basis(camera).0;
                                let mut metadata = BTreeMap::new();
                                metadata.insert("category".into(), "Mining".into());
                                metadata.insert("mining".into(), rock.mining);
                                metadata.insert(
                                    "utc".into(),
                                    chrono::Utc::now()
                                        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                                        .into(),
                                );
                                if let Some(shard) = o.shard.or_else(|| {
                                    self.live.bookmark().ok().and_then(|b| {
                                        b.metadata
                                            .get("shard_id")
                                            .and_then(|v| v.as_str())
                                            .map(str::to_owned)
                                    })
                                }) {
                                    metadata.insert("shard_id".into(), shard.into());
                                } else {
                                    self.status="Asteroid read, but shard is unreadable; verify server before saving.".into();
                                }
                                let system = o.system.unwrap_or_else(|| self.live.system.clone());
                                self.editor = Some(Bookmark {
                                    name: "Scanned asteroid".into(),
                                    system,
                                    xyz_km: std::array::from_fn(|i| {
                                        p[i] + forward[i] * rock.range_m / 1000.
                                    }),
                                    metadata,
                                });
                                self.editing = None;
                            } else {
                                self.status="Scan read, but frame coordinates or CamDir are unreadable. Try again with r_displayinfo 3 visible.".into();
                            }
                        }
                    }
                }
            }
        }
        if let Some(o) = &self.capture.observation {
            self.live.ingest(o);
            if !self
                .organization
                .extra
                .get("auto_system")
                .and_then(|v| v.as_bool())
                .unwrap_or(true)
            {
                self.live.system = self.organization.system.clone();
            }
            if self.last_logged != Some(o.stamp) && self.live.fresh() {
                self.last_logged = Some(o.stamp);
                if let Some(p) = self.live.ship() {
                    let path = self.path.with_file_name("travels.csv");
                    if let Err(e) = sc_bookmarks::travel_log::append(&path, p) {
                        self.status = format!("Travel log: {e}");
                    }
                }
            }
        }
        if self.library.current != self.live.system {
            self.library.current = self.live.system.clone();
            self.library.invalidate();
        }
        self.map.ship = self.live.ship().map(|p| (self.live.system.clone(), p));
        if self.community_refresh.elapsed().as_secs_f64() >= 1. {
            self.community_refresh = std::time::Instant::now();
            for b in self
                .bookmarks
                .iter_mut()
                .chain(self.community_target.iter_mut())
            {
                if let Err(e) = sc_bookmarks::community_coordinates::refresh(b) {
                    self.status = e;
                }
            }
        }
        if !self.capture.running() || !self.live.fresh() {
            self.current_route = None;
        } else if self.route_checked.elapsed().as_secs_f64() >= 1. {
            self.route_checked = std::time::Instant::now();
            self.current_route = self
                .selected
                .and_then(|i| self.bookmarks.get(i))
                .or(self.community_target.as_ref())
                .and_then(|b| {
                    self.live.ship().and_then(|p| {
                        routes::suggest_from_position(
                            b,
                            &self.live.system,
                            p,
                            &self.catalog.markers,
                        )
                    })
                })
                .map(|r| r.marker.name.clone());
        }
        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(18))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        theme::eyebrow(ui, "S C B M   /   NAVIGATION CONSOLE");
                        ui.label(egui::RichText::new("SC BOOKMARKS").size(28.).strong());
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("SETTINGS").clicked() {
                            self.overlay_settings.open();
                        }
                        ui.label(
                            egui::RichText::new("OBSERVE + OVERLAY")
                                .size(11.)
                                .color(theme::AMBER),
                        );
                    });
                });
                ui.add_space(8.);
                ui.horizontal_wrapped(|ui| {
                    if theme::tab(ui, self.show_map, "01  /  SYSTEM MAP") {
                        self.show_map = true;
                        self.show_scanning = false;
                        self.show_capture = false;
                        self.show_community = false;
                    }
                    if theme::tab(
                        ui,
                        !self.show_scanning
                            && !self.show_map
                            && !self.show_capture
                            && !self.show_community,
                        "02  /  BOOKMARKS",
                    ) {
                        self.show_scanning = false;
                        self.show_map = false;
                        self.show_capture = false;
                        self.show_community = false;
                    }
                    if theme::tab(ui, self.show_scanning, "03  /  SCANNING") {
                        self.show_scanning = true;
                        self.show_map = false;
                        self.show_capture = false;
                        self.show_community = false;
                    }
                    if theme::tab(ui, self.show_capture, "04  /  CAPTURE") {
                        self.show_capture = true;
                        self.show_community = false;
                        self.show_map = false;
                        self.show_scanning = false;
                    }
                    if theme::tab(ui, self.show_community, "05 / COMMUNITY") {
                        self.show_community = true;
                        self.show_map = false;
                        self.show_scanning = false;
                        self.show_capture = false;
                    }
                });
            });
        egui::TopBottomPanel::bottom("status")
            .exact_height(30.)
            .resizable(false)
            .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(5))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("●  LOCAL").color(theme::CYAN).size(11.));
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&self.status)
                                .size(11.)
                                .color(theme::MUTED),
                        )
                        .truncate(),
                    )
                    .on_hover_text(&self.status);
                });
            });
        egui::SidePanel::left("library")
            .default_width(255.)
            .width_range(220. ..=400.)
            .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(16))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("library_rail")
                    .show(ui, |ui| {
                        theme::eyebrow(ui, "LOCATIONS / PERSONAL LIBRARY");
                        ui.add_space(5.);
                        if ui
                            .add_sized(
                                [ui.available_width(), 36.],
                                egui::Button::new(
                                    egui::RichText::new("+  ENTER COORDINATES").color(theme::CYAN),
                                ),
                            )
                            .clicked()
                        {
                            self.editing = None;
                            self.editor = Some(Bookmark {
                                name: "New space bookmark".into(),
                                system: "Stanton".into(),
                                xyz_km: [0.; 3],
                                metadata: BTreeMap::new(),
                            });
                        }
                        ui.add(crate::clear_text::ClearText::singleline(
                            &mut self.capture_name,
                        ));
                        egui::ComboBox::from_id_salt("capture_category")
                            .selected_text(&self.capture_category)
                            .show_ui(ui, |ui| {
                                for category in &self.organization.categories {
                                    ui.selectable_value(
                                        &mut self.capture_category,
                                        category.clone(),
                                        category,
                                    );
                                }
                            });
                        if ui
                            .add_enabled(
                                !self.capture.running(),
                                egui::Button::new("Start / resume capture"),
                            )
                            .clicked()
                        {
                            self.capture.start();
                        }
                        if ui
                            .add_enabled(
                                self.capture.running() && self.live.fresh(),
                                egui::Button::new("Bookmark here"),
                            )
                            .clicked()
                        {
                            match self.live.bookmark() {
                                Ok(mut b) => {
                                    b.name = self.capture_name.clone();
                                    b.metadata.insert(
                                        "category".into(),
                                        self.capture_category.clone().into(),
                                    );
                                    let mut all = self.bookmarks.clone();
                                    all.push(b);
                                    let i = all.len() - 1;
                                    if self.persist(all) {
                                        self.select(i);
                                    }
                                }
                                Err(e) => self.status = e.into(),
                            }
                        }
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    self.capture.running(),
                                    egui::Button::new("Capture in 8s"),
                                )
                                .clicked()
                            {
                                self.timer = Some((
                                    std::time::Instant::now() + std::time::Duration::from_secs(8),
                                    self.capture_name.clone(),
                                    self.capture_category.clone(),
                                ));
                            }
                            if self.timer.is_some() && ui.button("Cancel timer").clicked() {
                                self.timer = None;
                            }
                        });
                        if ui
                            .add_enabled(
                                self.capture.running(),
                                egui::Button::new("Bookmark asteroid"),
                            )
                            .clicked()
                        {
                            self.capture
                                .request_scan(sc_bookmarks::scan_worker::Kind::Asteroid);
                            self.status = "Reading asteroid scan…".into();
                        }
                        if (self.selected.is_some() || self.community_target.is_some())
                            && ui.button("Clear target / hide overlay").clicked()
                        {
                            self.selected = None;
                            self.community_target = None;
                            self.live.alert.arm(false);
                        }
                        ui.collapsing("Library tools", |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("Import links").clicked() {
                                    self.show_import = true;
                                }
                                if ui.button("Copy all").clicked() {
                                    match self
                                        .bookmarks
                                        .iter()
                                        .cloned()
                                        .map(links::encode)
                                        .collect::<Result<Vec<_>, _>>()
                                    {
                                        Ok(v) => ctx.copy_text(v.join("\n")),
                                        Err(e) => self.status = e,
                                    }
                                }
                            });
                            if ui.button("Enable scbm links").clicked() {
                                self.status = match sc_bookmarks::protocol::register() {
                            Ok(()) => {
                                "scbm links now open this portable copy. Re-enable after moving it."
                                    .into()
                            }
                            Err(e) => e,
                        };
                            }
                            if ui.button("Import previous data folder").clicked() {
                                self.show_migration = true;
                            }
                            if ui.button("Organize categories").clicked() {
                                let mut names = self.organization.categories.clone();
                                for c in self
                                    .bookmarks
                                    .iter()
                                    .map(|b| sc_bookmarks::library::category(b).to_owned())
                                {
                                    if !names.contains(&c) {
                                        names.push(c);
                                    }
                                }
                                self.category_editor =
                                    Some(names.into_iter().map(|n| (n.clone(), n)).collect());
                            }
                        });
                        ui.separator();
                        let (picked, edit) = self.library.show(ui, &self.bookmarks, self.selected);
                        if let Some(i) = picked {
                            self.select(i)
                        }
                        if let Some(i) = edit {
                            self.editing = Some(i);
                            self.editor = Some(self.bookmarks[i].clone())
                        }
                    });
            });
        if self.show_map {
            egui::TopBottomPanel::bottom("route_dock")
                .exact_height(124.)
                .resizable(false)
                .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(10))
                .show(ctx, |ui| {
                    // Every state occupies the same three rows. OCR text may change,
                    // but it must never resize the map above this dock.
                    if let Some(i) = self.selected.filter(|i| *i < self.bookmarks.len()) {
                        let b = &self.bookmarks[i];
                        ui.horizontal(|ui| {
                            if ui.button("Edit / notes").clicked() {
                                self.editing = Some(i);
                                self.editor = Some(b.clone());
                            }
                            if ui.button("Copy link").clicked() {
                                match links::encode(b.clone()) {
                                    Ok(s) => ctx.copy_text(s),
                                    Err(e) => self.status = e,
                                }
                            }
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!("{}  /  {}", b.name, b.system))
                                        .size(16.)
                                        .color(theme::AMBER),
                                )
                                .truncate(),
                            )
                            .on_hover_text(&b.name);
                        });
                        let (guidance, warn) = self.live.guidance(b);
                        let compact = guidance
                            .lines()
                            .filter(|line| !line.contains("strafe"))
                            .collect::<Vec<_>>()
                            .join("   ·   ");
                        ui.add_sized(
                            [ui.available_width(), 24.],
                            egui::Label::new(egui::RichText::new(compact).color(if warn {
                                theme::MUTED
                            } else {
                                theme::CYAN
                            }))
                            .truncate(),
                        )
                        .on_hover_text(guidance);
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.live.show_overlay, "Overlay");
                            ui.label("Exit lead (s)");
                            ui.add(egui::DragValue::new(&mut self.live.lead).range(0. ..=120.));
                            if ui
                                .add_enabled(
                                    self.live.alert.armed
                                        || (self.capture.running() && self.live.fresh()),
                                    egui::Button::new(if self.live.alert.armed {
                                        "Disarm alert"
                                    } else {
                                        "Arm QT alert"
                                    }),
                                )
                                .clicked()
                            {
                                self.live.alert.arm(!self.live.alert.armed);
                            }
                            ui.separator();
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!("QT: {}", self.route_label()))
                                        .color(theme::CYAN),
                                )
                                .truncate(),
                            )
                            .on_hover_text(format!("{}\nGeometric suggestions only; game QT availability and minimum jump distance are not known.", self.route_label()));
                        });
                    } else if let Some(b) = &self.community_target {
                        ui.label(format!("Community destination: {} / {}", b.name, b.system));
                        ui.add(
                            egui::Label::new(self.live.guidance(b).0.replace('\n', " · "))
                                .truncate(),
                        );
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.live.show_overlay, "Overlay");
                            ui.label(format!("QT: {}", self.route_label()));
                        });
                    } else {
                        theme::eyebrow(ui, "DESTINATION / QUANTUM APPROACH");
                        ui.add_sized(
                            [ui.available_width(), 24.],
                            egui::Label::new(
                                "Select a bookmark to see guidance and suggested QT points.",
                            )
                            .truncate(),
                        );
                        ui.checkbox(&mut self.live.show_overlay, "Overlay");
                    }
                });
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.show_capture {
                let mut automatic = self
                    .organization
                    .extra
                    .get("auto_system")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let mut changed = ui
                    .checkbox(&mut automatic, "Detect current system automatically")
                    .changed();
                if !automatic {
                    ui.horizontal(|ui| {
                        ui.label("Current system");
                        egui::ComboBox::from_id_salt("manual_system")
                            .selected_text(&self.organization.system)
                            .show_ui(ui, |ui| {
                                for system in ["Stanton", "Pyro", "Nyx"] {
                                    changed |= ui
                                        .selectable_value(
                                            &mut self.organization.system,
                                            system.into(),
                                            system,
                                        )
                                        .changed();
                                }
                            });
                    });
                }
                if changed && !self.organization_blocked {
                    self.organization
                        .extra
                        .insert("auto_system".into(), automatic.into());
                    if let Err(e) = self
                        .organization
                        .save(&self.path.with_file_name("organization.json"))
                    {
                        self.status = e;
                    }
                }
                ui.weak(self.live.sample_status());
                self.capture.show(ui, &self.live.system, &self.live.shard);
                return;
            }
            if self.show_community {
                if let Some(b) = &self.community_target {
                    ui.label(format!("Navigating: {}", b.name));
                    ui.label(self.live.guidance(b).0.replace('\n', " · "));
                }
                let (navigate, copy) = self.community.show(ui, &self.catalog);
                if let Some(b) = navigate {
                    self.selected = None;
                    self.live.alert.arm(false);
                    self.route = routes::suggest(&b, &self.catalog.markers)
                        .ok()
                        .flatten()
                        .map(|r| r.label())
                        .unwrap_or_else(|| "No usable QT pair".into());
                    self.current_route = None;
                    self.route_checked = std::time::Instant::now() - std::time::Duration::from_secs(2);
                    self.community_target = Some(b);
                    if !self.capture.running() {
                        self.status = "Community destination queued; guidance starts when capture is running.".into();
                    }
                }
                if let Some(b) = copy {
                    let mut all = self.bookmarks.clone();
                    all.push(b);
                    self.persist(all);
                }
                return;
            }
            if self.show_scanning {
                self.scanning.show(ui);
                return;
            }
            if self.show_map {
                let mut displayed = self.bookmarks.clone();
                let map_selected = if let Some(b) = &self.community_target {
                    displayed.push(b.clone());
                    Some(displayed.len() - 1)
                } else {
                    self.selected
                };
                let (pick, edit) = self.map.show(ui, &self.catalog, &displayed, map_selected);
                if let Some(i) = pick.filter(|i| *i < self.bookmarks.len()) {
                    self.select(i);
                }
                if let Some(i) = edit.filter(|i| *i < self.bookmarks.len()) {
                    self.editing = Some(i);
                    self.editor = Some(self.bookmarks[i].clone());
                }
                return;
            }
            if let Some(i) = self.selected.filter(|i| *i < self.bookmarks.len()) {
                let b = &self.bookmarks[i];
                ui.heading(&b.name);
                if !self.capture.running() {
                    ui.weak("Destination queued · start capture when ready. Copying is available now.");
                }
                self.live.show(ui, b, self.capture.running());
                ui.label(format!("System: {}", b.system));
                ui.label(format!(
                    "Root XYZ (km): {:.4}, {:.4}, {:.4}",
                    b.xyz_km[0], b.xyz_km[1], b.xyz_km[2]
                ));
                ui.label(format!("Suggested QT: {}", self.route_label()));
                ui.label(format!(
                    "Created on shard: {}",
                    b.metadata
                        .get("shard_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                ));
                if let Some(t) = b
                    .metadata
                    .get("utc")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                {
                    ui.label(format!(
                        "Saved: {} (local)",
                        t.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S")
                    ));
                }
                ui.horizontal(|ui| {
                    if ui.button("Edit / notes").clicked() {
                        self.editing = Some(i);
                        self.editor = Some(b.clone())
                    }
                    if ui.button("Copy link").clicked() {
                        match links::encode(b.clone()) {
                            Ok(s) => ctx.copy_text(s),
                            Err(e) => self.status = e,
                        }
                    }
                    if ui.button("Delete").clicked() {
                        self.delete_pending = true
                    }
                });
                ui.separator();
                ui.label(
                    b.metadata
                        .get("notes")
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                );
            } else {
                ui.heading("Choose a bookmark");
                ui.label("Import scbm links, capture your position, or enter coordinates.");
            }
        });
        if self.show_import {
            let mut apply = false;
            let mut cancel = false;
            egui::Window::new("Import bookmark links").show(ctx, |ui| {
                ui.add(
                    crate::clear_text::ClearText::multiline(&mut self.paste)
                        .desired_rows(10)
                        .desired_width(550.),
                );
                ui.horizontal(|ui| {
                    apply = ui.button("Import").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
            if apply {
                match links::decode_many(&self.paste) {
                    Ok(mut b) => {
                        let mut all = self.bookmarks.clone();
                        all.append(&mut b);
                        if self.persist(all) {
                            self.show_import = false;
                            self.paste.clear();
                        }
                    }
                    Err(e) => self.status = e,
                }
            }
            if cancel {
                self.show_import = false;
            }
        }
        if self.delete_pending {
            let mut yes = false;
            let mut no = false;
            egui::Window::new("Delete selected bookmark?").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    yes = ui.button("Delete").clicked();
                    no = ui.button("Cancel").clicked();
                });
            });
            if yes {
                if let Some(i) = self.selected {
                    let mut all = self.bookmarks.clone();
                    all.remove(i);
                    if self.persist(all) {
                        self.selected = None;
                    }
                }
                self.delete_pending = false
            }
            if no {
                self.delete_pending = false
            }
        }
        let mut save = false;
        let mut cancel = false;
        if let Some(b) = &mut self.editor {
            egui::Window::new("Bookmark details")
                .default_width(550.)
                .vscroll(true)
                .show(ctx, |ui| {
                    ui.label("Name");
                    ui.add(crate::clear_text::ClearText::singleline(&mut b.name));
                    ui.label("System");
                    ui.add(crate::clear_text::ClearText::singleline(&mut b.system));
                    if b.metadata.contains_key("community_local") {
                        ui.weak("Coordinates track the community container. Editing XYZ converts this to a fixed system position.");
                    }
                    let before_xyz=b.xyz_km;
                    for (axis, v) in ["X", "Y", "Z"].into_iter().zip(b.xyz_km.iter_mut()) {
                        ui.horizontal(|ui| {
                            ui.label(axis);
                            ui.add(egui::DragValue::new(v).speed(0.001).max_decimals(8));
                        });
                    }
                    if b.xyz_km != before_xyz {b.metadata.remove("community_local");}
                    let mut category = b
                        .metadata
                        .get("category")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Personal")
                        .to_owned();
                    ui.label("Category");
                    ui.add(crate::clear_text::ClearText::singleline(&mut category));
                    b.metadata.insert("category".into(), category.into());
                    let mut notes = b
                        .metadata
                        .get("notes")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_owned();
                    ui.label("Notes");
                    ui.add(crate::clear_text::ClearText::multiline(&mut notes).desired_rows(5));
                    b.metadata.insert("notes".into(), notes.into());
                    if b.metadata
                        .get("category")
                        .and_then(|v| v.as_str())
                        .is_some_and(|s| s.eq_ignore_ascii_case("mining"))
                    {
                        mining_editor::edit(ui, b);
                    }
                    ui.horizontal(|ui| {
                        save = ui.button("Save").clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                });
        }
        if save {
            match links::normalize(self.editor.clone().unwrap()) {
                Ok(mut b) => {
                    if self.editing.is_none() {
                        b.metadata.insert(
                            "utc".into(),
                            chrono::Utc::now()
                                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                                .into(),
                        );
                    }
                    let mut all = self.bookmarks.clone();
                    if let Some(i) = self.editing {
                        all[i] = b
                    } else {
                        all.push(b)
                    }
                    if self.persist(all) {
                        self.editor = None;
                        if let Some(i) = self.selected {
                            self.select(i)
                        }
                    }
                }
                Err(e) => self.status = e,
            }
        }
        if cancel {
            self.editor = None
        }
        if self.show_migration {
            let mut apply = false;
            let mut cancel = false;
            egui::Window::new("Import previous SC Bookmarks data").show(ctx, |ui| {
                ui.label("Path to your previous navigator-data folder. Originals are preserved.");
                ui.add(crate::clear_text::ClearText::singleline(
                    &mut self.migration,
                ));
                ui.horizontal(|ui| {
                    apply = ui.button("Import folder").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
            if cancel {
                self.show_migration = false;
            }
            if apply {
                match sc_bookmarks::protocol::migrate(
                    std::path::Path::new(self.migration.trim()),
                    self.path.parent().unwrap(),
                ) {
                    Ok(()) => {
                        *self = Self::new();
                        self.status = "Previous bookmarks and settings imported.".into();
                    }
                    Err(e) => self.status = e,
                }
            }
        }
        let mut save_categories = false;
        let mut cancel_categories = false;
        if let Some(rows) = &mut self.category_editor {
            egui::Window::new("Organize categories").show(ctx, |ui| {
                ui.label("Mining is fixed. Rename and reorder the other categories.");
                let mut swap = None;
                for i in 0..rows.len() {
                    ui.horizontal(|ui| {
                        if rows[i].0 == "Mining" {
                            ui.label("Mining");
                        } else {
                            ui.add(crate::clear_text::ClearText::singleline(&mut rows[i].1));
                        }
                        if i > 0 && ui.small_button("Up").clicked() {
                            swap = Some((i, i - 1));
                        }
                        if i + 1 < rows.len() && ui.small_button("Down").clicked() {
                            swap = Some((i, i + 1));
                        }
                    });
                }
                if let Some((a, b)) = swap {
                    rows.swap(a, b);
                }
                if ui.button("Add category").clicked() {
                    rows.push((String::new(), "New category".into()));
                }
                ui.horizontal(|ui| {
                    save_categories = ui
                        .add_enabled(!self.organization_blocked, egui::Button::new("Save"))
                        .clicked();
                    cancel_categories = ui.button("Cancel").clicked();
                });
            });
        }
        if save_categories {
            let rows = self.category_editor.clone().unwrap();
            let names: Vec<_> = rows.iter().map(|(_, n)| n.trim().to_owned()).collect();
            match sc_bookmarks::organization::validate(&names) {
                Err(e) => self.status = e,
                Ok(()) => {
                    let mut all = self.bookmarks.clone();
                    for b in &mut all {
                        if let Some((_, name)) = rows
                            .iter()
                            .find(|(old, _)| old == sc_bookmarks::library::category(b))
                        {
                            b.metadata.insert("category".into(), name.trim().into());
                        }
                    }
                    if self.persist(all) {
                        self.organization.categories = names.clone();
                        self.library.order = names;
                        self.library.invalidate();
                        match self
                            .organization
                            .save(&self.path.with_file_name("organization.json"))
                        {
                            Ok(()) => self.category_editor = None,
                            Err(e) => self.status = e,
                        }
                    }
                }
            }
        }
        if cancel_categories {
            self.category_editor = None;
        }
        if !self.organization_blocked
            && (self.organization.hide_other_systems != self.library.current_only
                || self.organization.sort != self.library.filter.sort.label())
        {
            self.organization.hide_other_systems = self.library.current_only;
            self.organization.sort = self.library.filter.sort.label().into();
            if let Err(e) = self
                .organization
                .save(&self.path.with_file_name("organization.json"))
            {
                self.status = e;
            }
        }
        self.overlay_settings.runtime_status = if self.overlays.hidden() {
            "All overlay visuals hidden by hotkey".into()
        } else {
            "Overlay visuals enabled".into()
        };
        self.overlay_settings.show(ctx);
        self.overlays
            .configure_hotkey(self.overlay_settings.saved.hotkey);
        let hotkey_error = self.overlays.hotkey_status();
        if !hotkey_error.is_empty() {
            self.status = hotkey_error;
        }

        self.live.tick(
            self.selected
                .and_then(|i| self.bookmarks.get(i))
                .or(self.community_target.as_ref()),
        );
        let mut panels = Vec::new();
        let game_rect = windows_capture::window::Window::from_name("Star Citizen ")
            .or_else(|_| windows_capture::window::Window::from_name("Star Citizen"))
            .ok()
            .and_then(|w| sc_bookmarks::window_geometry::query(w.as_raw_hwnd() as usize))
            .map(|g| g.client);
        // The selected target owns overlay visibility. OCR freshness only controls
        // guidance/alerts, never whether the user can see the panel.
        if self.live.show_overlay {
            if let (Some(b), Some(game)) = (
                self.selected
                    .and_then(|i| self.bookmarks.get(i))
                    .or(self.community_target.as_ref()),
                game_rect,
            ) {
                match sc_bookmarks::overlay::placement(game, self.overlay_settings.position()) {
                    Ok(rect) => {
                        let (lines, warning) = self.live.guidance(b);
                        panels.push(sc_bookmarks::native_overlay::Panel {
                            rect,
                            title: b.name.clone(),
                            lines,
                            warning,
                            floating: true,
                            angles: self.live.marker_angles(b),
                            theme: self.overlay_settings.saved.theme,
                            projected: false,
                        });
                    }
                    Err(e) => self.status = format!("Overlay placement: {e}"),
                }
            }
        }
        if self.live.show_overlay && self.overlay_settings.saved.bookmark_3d {
            if let (Some(b), Some(game)) = (
                self.selected
                    .and_then(|i| self.bookmarks.get(i))
                    .or(self.community_target.as_ref()),
                game_rect,
            ) {
                if let Some([x, y]) = self.live.projected(
                    b,
                    [game.width, game.height],
                    self.overlay_settings.saved.vertical_fov,
                    self.overlay_settings.saved.bookmark_3d_distant,
                ) {
                    let distance = self
                        .live
                        .ship()
                        .map(|p| {
                            (0..3)
                                .map(|i| (b.xyz_km[i] - p[i]).powi(2))
                                .sum::<f64>()
                                .sqrt()
                        })
                        .unwrap_or(0.);
                    // Small transparent surface: no full-screen bitmap, unaffected by normal overlay offset.
                    panels.push(sc_bookmarks::native_overlay::Panel {
                        rect: sc_bookmarks::overlay::Rect {
                            left: game.left + x.round() as i32 - 70,
                            top: game.top + y.round() as i32 - 23,
                            width: 140,
                            height: 66,
                        },
                        title: "3D bookmark test".into(),
                        lines: if distance <= 5. {
                            format!("BM · {:.0} m", distance * 1000.)
                        } else if distance >= 1_000_000. {
                            format!("BM · {:.2} M km", distance / 1_000_000.)
                        } else {
                            format!("BM · {:.1} km", distance)
                        },
                        warning: false,
                        floating: true,
                        angles: None,
                        theme: self.overlay_settings.saved.theme,
                        projected: true,
                    });
                }
            }
        }
        if self.capture.scans_enabled {
            if let Some((stamp, desktop, contacts)) = &self.contacts {
                if stamp.elapsed().as_secs() < 10
                    && game_rect
                        .is_some_and(|r| r.width == desktop.width && r.height == desktop.height)
                {
                    let desktop = game_rect.as_ref().unwrap();
                    for c in contacts {
                        let width = 380.min(desktop.width);
                        let height = (65
                            + c.text
                                .lines()
                                .skip(1)
                                .map(|l| 20 * (1 + l.chars().count() as u32 / 44))
                                .sum::<u32>())
                        .min(desktop.height);
                        let x = ((c.bounds[0] + c.bounds[2]) / 2)
                            .saturating_sub(width / 2)
                            .min(desktop.width - width);
                        let y = (c.bounds[3] + 5).min(desktop.height - height);
                        let (title, lines) = c.text.split_once('\n').unwrap_or((&c.text, ""));
                        panels.push(sc_bookmarks::native_overlay::Panel {
                            rect: sc_bookmarks::overlay::Rect {
                                left: desktop.left + x as i32,
                                top: desktop.top + y as i32,
                                width,
                                height,
                            },
                            title: title.into(),
                            lines: lines.into(),
                            warning: false,
                            floating: false,
                            angles: None,
                            theme: self.overlay_settings.saved.theme,
                            projected: false,
                        });
                    }
                }
            }
        }
        let requested = panels.len();
        self.overlays.update(panels);
        if self.overlay_diagnostic_at.elapsed().as_secs() >= 5 {
            self.overlay_diagnostic_at = std::time::Instant::now();
            let diagnostic = serde_json::json!({"enabled":self.live.show_overlay,"selected":self.selected,"game_rect":game_rect.map(|r|[r.left as i64,r.top as i64,r.width as i64,r.height as i64]),"requested_panels":requested,"native":format!("{:?}",self.overlays.health())});
            let _ = std::fs::write(
                self.path.with_file_name("overlay-status.json"),
                diagnostic.to_string(),
            );
        }
        if let Err(e) = self.overlays.health() {
            self.status = e;
        }
        if let Some((end, name, category)) = self.timer.clone() {
            if std::time::Instant::now() >= end {
                match self.live.bookmark() {
                    Ok(mut b) => {
                        self.timer = None;
                        b.name = name;
                        b.metadata.insert("category".into(), category.into());
                        let mut all = self.bookmarks.clone();
                        all.push(b);
                        let i = all.len() - 1;
                        if self.persist(all) {
                            self.select(i);
                        }
                    }
                    Err(e) => {
                        self.status = format!("Waiting for a fresh position: {e}");
                        if end.elapsed().as_secs() >= 15 {
                            self.timer = None;
                            self.status = format!("Bookmark not saved: {e}");
                        }
                    }
                }
            } else {
                self.status = format!(
                    "Bookmark in {} seconds",
                    end.saturating_duration_since(std::time::Instant::now())
                        .as_secs()
                        + 1
                );
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
        }
    }
}
fn main() -> eframe::Result {
    if std::env::args().nth(1).as_deref() == Some("--verify-runtime") {
        let report = (|| -> Result<serde_json::Value, String> {
            let runtime = std::env::current_exe()
                .map_err(|e| e.to_string())?
                .with_file_name("runtime");
            let mut reader =
                sc_bookmarks::recognizer::Recognizer::load(&runtime).map_err(|e| e.to_string())?;
            let mut detector = sc_bookmarks::detector::Detector::load(&runtime)?;
            let image = image::load_from_memory(include_bytes!("../tests/ocr-row.png"))
                .map_err(|e| e.to_string())?
                .to_rgb8();
            let (text, confidence) = reader.read(&image).map_err(|e| e.to_string())?;
            if !text.contains("Root") || confidence < 0.8 {
                return Err(format!("Recognition self-check failed: {text}"));
            }
            let regions = detector.detect(&image)?.len();
            if regions == 0 {
                return Err("Detector self-check found no text".into());
            }
            Ok(
                serde_json::json!({"ok":true,"recognition":text,"confidence":confidence,"detected_regions":regions}),
            )
        })();
        let passed = report.is_ok();
        let json = report.unwrap_or_else(|e| serde_json::json!({"ok":false,"error":e}));
        if let Some(path) = std::env::args().nth(2) {
            if std::fs::write(path, serde_json::to_vec_pretty(&json).unwrap()).is_err() {
                std::process::exit(2);
            }
        }
        std::process::exit(if passed { 0 } else { 1 });
    }
    let dir = sc_bookmarks::protocol::data_dir();
    if let Some(arg) = std::env::args().nth(1) {
        if arg.starts_with("scbm:") {
            if let Err(e) = sc_bookmarks::protocol::deliver(&dir, &arg) {
                eprintln!("{e}");
                return Ok(());
            }
        }
    }
    let _instance = match sc_bookmarks::protocol::acquire(&dir) {
        Ok(Some(f)) => f,
        Ok(None) => return Ok(()),
        Err(e) => {
            eprintln!("{e}");
            return Ok(());
        }
    };
    let icon_image = image::load_from_memory(include_bytes!("../assets/sc-bookmarks.png"))
        .expect("Bundled application icon")
        .into_rgba8();
    let icon = egui::IconData {
        width: icon_image.width(),
        height: icon_image.height(),
        rgba: icon_image.into_raw(),
    };
    eframe::run_native(
        concat!("SC Bookmarks — ", env!("CARGO_PKG_VERSION")),
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_icon(icon)
                .with_inner_size([1440., 940.])
                .with_min_inner_size([800., 540.]),
            ..Default::default()
        },
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(Desktop::new()))
        }),
    )
}
