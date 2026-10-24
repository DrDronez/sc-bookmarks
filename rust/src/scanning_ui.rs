use eframe::egui;
use sc_bookmarks::scanning::{self, Chart};
pub struct ScanningView {
    charts: Vec<Chart>,
    mode: usize,
    rs: String,
    query: String,
    sort: Option<(usize, bool)>,
    visible: Vec<usize>,
    summary: String,
    selection: Option<usize>,
    parsed: Option<u64>,
}
impl Default for ScanningView {
    fn default() -> Self {
        let mut view = Self {
            charts: scanning::charts().expect("Bundled scanning tables"),
            mode: 0,
            rs: String::new(),
            query: String::new(),
            sort: None,
            visible: Vec::new(),
            summary: String::new(),
            selection: None,
            parsed: None,
        };
        view.refresh();
        view
    }
}
impl ScanningView {
    fn refresh(&mut self) {
        self.selection = None;
        self.visible.clear();
        self.parsed = None;
        match scanning::parse_rs(&self.rs) {
            Err(e) => self.summary = e.into(),
            Ok(rs) => {
                self.parsed = rs;
                self.visible = self.charts[self.mode].filtered(&self.query, rs, self.sort);
                self.summary = if let Some(rs) = rs {
                    self.selection = self.visible.first().copied();
                    let others = self
                        .charts
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != self.mode)
                        .filter_map(|(_, c)| {
                            let count = c.filtered("", Some(rs), None).len();
                            (count > 0).then(|| format!("{}: {count}", c.name))
                        })
                        .collect::<Vec<_>>();
                    format!(
                        "{} RS · {} matching row(s). {}{}",
                        scanning::grouped(rs),
                        self.visible.len(),
                        if others.is_empty() {
                            String::new()
                        } else {
                            format!("Also matches {}. ", others.join("; "))
                        },
                        if self.visible.is_empty() {
                            "No exact match here; mixed contacts may give other totals."
                        } else {
                            ""
                        }
                    )
                } else {
                    format!(
                        "{} entries · Enter an RS reading to find exact matches, or browse the full chart.",
                        self.visible.len()
                    )
                };
            }
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading("SCAN SIGNATURE REFERENCE");
        let old_mode = self.mode;
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.label("Reference chart");
            egui::ComboBox::from_id_salt("scan_chart")
                .selected_text(self.charts[self.mode].name)
                .show_ui(ui, |ui| {
                    for (i, c) in self.charts.iter().enumerate() {
                        ui.selectable_value(&mut self.mode, i, c.name);
                    }
                });
            ui.label("RS reading");
            changed |= ui
                .add(crate::clear_text::ClearText::singleline(&mut self.rs).desired_width(95.))
                .changed();
            ui.label("Ore / contact");
            changed |= ui
                .add(crate::clear_text::ClearText::singleline(&mut self.query).desired_width(150.))
                .changed();
            if ui.button("Clear").clicked() {
                self.rs.clear();
                self.query.clear();
                changed = true;
            }
        });
        if self.mode != old_mode {
            self.sort = None;
            changed = true;
        }
        if changed {
            self.refresh();
        }
        ui.colored_label(egui::Color32::from_rgb(104, 223, 235), &self.summary);
        ui.separator();
        let mut sort_clicked = None;
        let chart = &self.charts[self.mode];
        egui::ScrollArea::both()
            .max_height((ui.available_height() - 140.).max(100.))
            .show(ui, |ui| {
                egui::Grid::new("scan_table")
                    .striped(true)
                    .min_col_width(70.)
                    .show(ui, |ui| {
                        for (i, h) in chart.headers.iter().enumerate() {
                            let marker = match self.sort {
                                Some((c, d)) if c == i => {
                                    if d {
                                        " ▼"
                                    } else {
                                        " ▲"
                                    }
                                }
                                _ => "",
                            };
                            if ui.button(format!("{h}{marker}")).clicked() {
                                sort_clicked = Some(i);
                            }
                        }
                        ui.end_row();
                        for &index in &self.visible {
                            let matches = self
                                .parsed
                                .map(|rs| chart.matching_columns(index, rs))
                                .unwrap_or_default();
                            for (col, value) in chart.rows[index].iter().enumerate() {
                                let text = egui::RichText::new(value.display());
                                let text = if matches.contains(&col) {
                                    text.color(egui::Color32::from_rgb(239, 189, 112))
                                } else {
                                    text
                                };
                                if ui
                                    .selectable_label(self.selection == Some(index), text)
                                    .clicked()
                                {
                                    self.selection = Some(index);
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
        ui.separator();
        if let Some(index) = self.selection {
            let row = &chart.rows[index];
            if chart.salvage {
                ui.label(format!("{} RS → {}", row[0].display(), row[1].display()));
            } else if let Some(rs) = self.parsed {
                let counts = chart
                    .matching_columns(index, rs)
                    .iter()
                    .map(|n| format!("{n} {} / deposits", if *n == 1 { "rock" } else { "rocks" }))
                    .collect::<Vec<_>>();
                ui.label(format!(
                    "{} RS → {} · {}",
                    scanning::grouped(rs),
                    counts.join(", "),
                    row[0].display()
                ));
            } else {
                ui.label(format!(
                    "{} · Base RS: {}. Read across for the number of contributing rocks.",
                    row[0].display(),
                    row[1].display()
                ));
            }
        }
        ui.label(chart.note());
        ui.weak("Alpha 4.10.2 · Offline scanning reference");
        if let Some(column) = sort_clicked {
            self.sort = Some((column, self.sort.is_some_and(|(c, d)| c == column && !d)));
            self.refresh();
        }
    }
}
