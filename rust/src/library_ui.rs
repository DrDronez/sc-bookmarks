use eframe::egui;
use sc_bookmarks::{
    Bookmark,
    library::{self, Filter, Sort},
    mining,
};
#[derive(Default)]
pub struct LibraryView {
    pub filter: Filter,
    pub order: Vec<String>,
    pub current_only: bool,
    pub current: String,
    quality: String,
    visible: Vec<usize>,
    categories: Vec<String>,
    systems: Vec<String>,
    dirty: bool,
    error: String,
}
impl LibraryView {
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        bookmarks: &[Bookmark],
        selected: Option<usize>,
    ) -> (Option<usize>, Option<usize>) {
        if self.categories.is_empty() || self.dirty {
            self.categories = self.order.clone();
            for c in bookmarks.iter().map(|b| library::category(b).to_owned()) {
                if !self.categories.contains(&c) {
                    self.categories.push(c);
                }
            }
            self.systems = bookmarks.iter().map(|b| b.system.clone()).collect();
            self.systems.sort();
            self.systems.dedup();
            self.dirty = true;
        }
        if ui
            .checkbox(&mut self.current_only, "Only current system")
            .changed()
        {
            self.dirty = true;
        }
        self.dirty |= ui
            .add(
                crate::clear_text::ClearText::singleline(&mut self.filter.query)
                    .hint_text("Search bookmarks"),
            )
            .changed();
        egui::CollapsingHeader::new("Filter & sort")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("Category");
                egui::ComboBox::from_id_salt("library_category")
                    .selected_text(if self.filter.category.is_empty() {
                        "All categories"
                    } else {
                        &self.filter.category
                    })
                    .show_ui(ui, |ui| {
                        self.dirty |= ui
                            .selectable_value(
                                &mut self.filter.category,
                                String::new(),
                                "All categories",
                            )
                            .changed();
                        for c in &self.categories {
                            self.dirty |= ui
                                .selectable_value(&mut self.filter.category, c.clone(), c)
                                .changed();
                        }
                    });
                ui.label("System");
                egui::ComboBox::from_id_salt("library_system")
                    .selected_text(if self.filter.system.is_empty() {
                        "All systems"
                    } else {
                        &self.filter.system
                    })
                    .show_ui(ui, |ui| {
                        self.dirty |= ui
                            .selectable_value(&mut self.filter.system, String::new(), "All systems")
                            .changed();
                        for s in &self.systems {
                            self.dirty |= ui
                                .selectable_value(&mut self.filter.system, s.clone(), s)
                                .changed();
                        }
                    });
                ui.label("Sort by");
                egui::ComboBox::from_id_salt("library_sort")
                    .selected_text(self.filter.sort.label())
                    .show_ui(ui, |ui| {
                        for s in Sort::ALL {
                            self.dirty |= ui
                                .selectable_value(&mut self.filter.sort, s, s.label())
                                .changed();
                        }
                    });
                ui.label("Ore type");
                egui::ComboBox::from_id_salt("library_ore")
                    .selected_text(if self.filter.ore.is_empty() {
                        "Any ore"
                    } else {
                        &self.filter.ore
                    })
                    .show_ui(ui, |ui| {
                        self.dirty |= ui
                            .selectable_value(&mut self.filter.ore, String::new(), "Any ore")
                            .changed();
                        let mut ores = mining::ORES.to_vec();
                        ores.sort();
                        for ore in ores {
                            self.dirty |= ui
                                .selectable_value(&mut self.filter.ore, ore.to_owned(), ore)
                                .changed();
                        }
                    });
                ui.horizontal(|ui| {
                    ui.label("Quality ≥");
                    if ui
                        .add(
                            crate::clear_text::ClearText::singleline(&mut self.quality)
                                .desired_width(85.),
                        )
                        .changed()
                    {
                        self.error.clear();
                        if self.quality.trim().is_empty() {
                            self.filter.minimum = None;
                            self.dirty = true;
                        } else {
                            match self.quality.trim().parse::<f64>() {
                                Ok(n) if n.is_finite() && (0. ..=1e12).contains(&n) => {
                                    self.filter.minimum = Some(n);
                                    self.dirty = true;
                                }
                                _ => self.error = "Quality must be a nonnegative number.".into(),
                            }
                        }
                    }
                });
                if ui.button("Clear filters").clicked() {
                    self.filter = Filter::default();
                    self.quality.clear();
                    self.error.clear();
                    self.dirty = true;
                }
            });
        if self.dirty {
            self.visible = self.filter.apply(bookmarks, &self.categories);
            if self.current_only && !self.current.is_empty() {
                self.visible
                    .retain(|i| bookmarks[*i].system.eq_ignore_ascii_case(&self.current));
            }
            self.dirty = false;
        }
        if !self.error.is_empty() {
            ui.colored_label(egui::Color32::LIGHT_RED, &self.error);
            return (None, None);
        }
        ui.weak(format!("{} SAVED LOCATIONS", self.visible.len()));
        ui.separator();
        let mut picked = None;
        let mut edit = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for &i in &self.visible {
                let b = &bookmarks[i];
                let quality = library::best_quality(b, &self.filter.ore);
                let suffix = if library::category(b) == "Mining" && quality >= 0. {
                    format!(" · Q{quality}")
                } else {
                    String::new()
                };
                let r = ui.selectable_label(
                    selected == Some(i),
                    format!(
                        "{}\n{} / {}{}",
                        b.name,
                        b.system,
                        library::category(b),
                        suffix
                    ),
                );
                if r.clicked() {
                    picked = Some(i);
                }
                if r.double_clicked() {
                    edit = Some(i);
                }
            }
        });
        (picked, edit)
    }
}
