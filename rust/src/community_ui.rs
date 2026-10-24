use eframe::egui;
use sc_bookmarks::{Bookmark, routes::Catalog};
#[derive(serde::Deserialize)]
struct Entry {
    name: String,
    system: String,
    parent: String,
    kind: String,
    frame: String,
    qt: String,
    notes: String,
    source: String,
    coordinate_source: String,
    x: String,
    y: String,
    z: String,
}
pub struct Community {
    entries: Vec<Entry>,
    query: String,
    system: String,
    kind: String,
    parent: String,
    qt_only: bool,
    nav_only: bool,
    sort: usize,
    selected: Option<usize>,
    cache_key: String,
    visible: Vec<usize>,
}
impl Default for Community {
    fn default() -> Self {
        Self {
            entries: serde_json::from_str(include_str!("../../route-data/community-pois.json"))
                .expect("Bundled community catalog"),
            query: String::new(),
            system: "All".into(),
            kind: "All".into(),
            parent: "All".into(),
            qt_only: false,
            nav_only: false,
            sort: 0,
            selected: None,
            cache_key: String::new(),
            visible: Vec::new(),
        }
    }
}
fn target(e: &Entry, catalog: &Catalog) -> Option<Bookmark> {
    let matches: Vec<_> = catalog
        .markers
        .iter()
        .filter(|m| {
            m.system.eq_ignore_ascii_case(&e.system) && m.name.eq_ignore_ascii_case(&e.name)
        })
        .collect();
    let mut local = None;
    let xyz = if matches.len() == 1 {
        matches[0].xyz_km
    } else {
        let xyz = [
            e.x.parse::<f64>().ok()?,
            e.y.parse::<f64>().ok()?,
            e.z.parse::<f64>().ok()?,
        ];
        if xyz.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let frame = e.frame.strip_prefix("Local to ")?;
        if frame.eq_ignore_ascii_case(&e.system) {
            xyz
        } else {
            let v = sc_bookmarks::community_coordinates::Local {
                container: frame.into(),
                xyz_km: xyz,
            };
            let root = sc_bookmarks::community_coordinates::position(
                &e.system,
                &v,
                chrono::Utc::now().timestamp_millis() as f64 / 1000.,
            )
            .ok()?;
            local = Some(v);
            root
        }
    };
    let mut metadata = std::collections::BTreeMap::new();
    metadata.insert(
        "notes".into(),
        format!("Community reference (2026-10-21). {}", e.notes).into(),
    );
    metadata.insert("category".into(), "Personal".into());
    if let Some(local) = local {
        metadata.insert("community_local".into(), serde_json::to_value(local).ok()?);
    }
    Some(Bookmark {
        name: e.name.clone(),
        system: e.system.clone(),
        xyz_km: xyz,
        metadata,
    })
}
impl Community {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &Catalog,
    ) -> (Option<Bookmark>, Option<Bookmark>) {
        let mut navigate = None;
        let mut copy = None;
        ui.heading("COMMUNITY BOOKMARKS");
        ui.weak("Researched POI catalog · 2026-10-21 · separate from your personal library");
        ui.horizontal_wrapped(|ui| {
            ui.label("Search");
            ui.add(crate::clear_text::ClearText::singleline(&mut self.query));
            egui::ComboBox::from_id_salt("community_system")
                .selected_text(format!("System: {}", if self.system == "All" { "All systems" } else { &self.system }))
                .show_ui(ui, |ui| {
                    for s in ["All", "Stanton", "Pyro", "Nyx"] {
                        ui.selectable_value(&mut self.system, s.into(), if s == "All" { "All systems" } else { s });
                    }
                });
            let parents: std::collections::BTreeSet<_> = self
                .entries
                .iter()
                .filter(|e| self.system == "All" || e.system == self.system)
                .map(|e| e.parent.clone())
                .filter(|p| !p.is_empty())
                .collect();
            if self.parent != "All" && !parents.contains(&self.parent) {
                self.parent = "All".into();
            }
            egui::ComboBox::from_id_salt("community_parent")
                .selected_text(format!("Parent: {}", if self.parent == "All" { "All containers" } else { &self.parent }))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.parent, "All".into(), "All containers");
                    for p in parents {
                        ui.selectable_value(&mut self.parent, p.clone(), p);
                    }
                });
            let kinds: std::collections::BTreeSet<_> =
                self.entries.iter().map(|e| e.kind.clone()).collect();
            egui::ComboBox::from_id_salt("community_kind")
                .selected_text(format!("Type: {}", if self.kind == "All" { "All types" } else if self.kind.is_empty() { "Unspecified" } else { &self.kind }))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.kind, "All".into(), "All types");
                    for k in kinds {
                        ui.selectable_value(&mut self.kind, k.clone(), if k.is_empty() { "Unspecified" } else { &k });
                    }
                });
            egui::ComboBox::from_id_salt("community_sort")
                .selected_text(format!("Sort by: {}", ["Name", "System", "Type", "QT status", "Parent container"][self.sort]))
                .show_ui(ui, |ui| {
                    for (i, s) in ["Name", "System", "Type", "QT status", "Parent container"]
                        .iter()
                        .enumerate()
                    {
                        ui.selectable_value(&mut self.sort, i, *s);
                    }
                });
            ui.checkbox(&mut self.qt_only, "Verified QT markers");
            ui.checkbox(&mut self.nav_only, "Has navigation coordinates").on_hover_text("Show entries that can be selected as destinations, even before capture starts.");
        });
        if let Some(i) = self.selected {
            let e = &self.entries[i];
            ui.separator();
            ui.label(format!("{} · {} · {}", e.name, e.system, e.kind));
            ui.label(format!("{} · {}", e.qt, e.frame));
            ui.label(format!(
                "Parent: {} · Listed XYZ km: {}, {}, {}",
                e.parent, e.x, e.y, e.z
            ));
            ui.add(egui::Label::new(&e.notes).truncate())
                .on_hover_text(&e.notes);
            let t = target(e, catalog);
            if t.as_ref()
                .is_some_and(|b| b.metadata.contains_key("community_local"))
            {
                ui.weak("Planet-relative position uses a rotation estimate. Verify in game; system clock and source data affect accuracy.");
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(t.is_some(), egui::Button::new("Copy link"))
                    .clicked()
                {
                    if let Some(b) = t.clone() {
                        if let Ok(link) = sc_bookmarks::links::encode(b) {
                            ui.ctx().copy_text(link);
                        }
                    }
                }
                if ui
                    .add_enabled(t.is_some(), egui::Button::new("Navigate"))
                    .clicked()
                {
                    navigate = t.clone();
                }
                if ui
                    .add_enabled(t.is_some(), egui::Button::new("Copy to personal library"))
                    .clicked()
                {
                    copy = t.clone();
                }
                if t.is_none() {
                    ui.weak(if e.x.is_empty() || e.y.is_empty() || e.z.is_empty() {
                        "Coordinates missing from source; navigation and bookmark links need XYZ."
                    } else { "Coordinate container unavailable or unsupported; cannot safely navigate this entry yet." });
                }
            });
            ui.collapsing("Sources", |ui| {
                for url in [&e.source, &e.coordinate_source] {
                    if url.starts_with("https://") {
                        ui.hyperlink(url);
                    }
                }
            });
        }
        let key = format!(
            "{:?}",
            (
                &self.query,
                &self.system,
                &self.kind,
                &self.parent,
                self.qt_only,
                self.nav_only,
                self.sort
            )
        );
        if key != self.cache_key {
            let q = self.query.trim().to_lowercase();
            let mut ids: Vec<_> = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    (self.system == "All" || self.system == e.system)
                        && (self.kind == "All" || self.kind == e.kind)
                        && (self.parent == "All" || self.parent == e.parent)
                        && (!self.qt_only || e.qt.starts_with("Yes"))
                        && (!self.nav_only || target(e, catalog).is_some())
                        && (q.is_empty()
                            || format!("{} {} {} {}", e.name, e.parent, e.kind, e.notes)
                                .to_lowercase()
                                .contains(&q))
                })
                .map(|(i, _)| i)
                .collect();
            ids.sort_by_cached_key(|i| {
                let e = &self.entries[*i];
                format!(
                    "{} {}",
                    match self.sort {
                        1 => &e.system,
                        2 => &e.kind,
                        3 => &e.qt,
                        4 => &e.parent,
                        _ => &e.name,
                    },
                    e.name
                )
                .to_lowercase()
            });
            self.visible = ids;
            self.cache_key = key;
        }
        let ids = &self.visible;
        ui.label(format!("{} of {} entries", ids.len(), self.entries.len()));
        egui::ScrollArea::vertical().show_rows(ui, 28., ids.len(), |ui, range| {
            for row in range {
                let i = ids[row];
                let e = &self.entries[i];
                if ui
                    .selectable_label(
                        self.selected == Some(i),
                        format!(
                            "{}  |  {}  |  {}  |  {}  |  {}",
                            e.name,
                            e.system,
                            if e.parent.is_empty() {
                                "Unspecified parent"
                            } else {
                                &e.parent
                            },
                            e.kind,
                            e.qt
                        ),
                    )
                    .clicked()
                {
                    self.selected = Some(i);
                }
            }
        });
        (navigate, copy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn community_is_separate_and_local_positions_are_not_used_as_root() {
        let c = Community::default();
        let catalog = sc_bookmarks::routes::catalog().unwrap();
        assert_eq!(c.entries.len(), 2901);
        let navigable = c
            .entries
            .iter()
            .filter(|e| target(e, &catalog).is_some())
            .count();
        println!(
            "Community destinations usable: {navigable} / {}",
            c.entries.len()
        );
        assert!(navigable > 1500);
        let benny = c.entries.iter().find(|e| e.name == "Bennyhenge").unwrap();
        let b = target(benny, &catalog).expect("Bennyhenge resolves through Yela");
        assert!(b.metadata.contains_key("community_local"));
        let link = sc_bookmarks::links::encode(b).unwrap();
        assert!(
            sc_bookmarks::links::decode(&link)
                .unwrap()
                .metadata
                .contains_key("community_local")
        );
        assert!(
            c.entries
                .iter()
                .any(|e| e.x.is_empty() && target(e, &catalog).is_none())
        );
        for e in &c.entries {
            if let Some(b) = target(e, &catalog) {
                assert!(b.validate().is_ok());
                assert!(!b.metadata.contains_key("utc"));
            }
        }
    }
}
