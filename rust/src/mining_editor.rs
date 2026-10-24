use eframe::egui;
use sc_bookmarks::{Bookmark, mining};
use serde_json::{Value, json};
fn optional_number(ui: &mut egui::Ui, value: &mut Value, label: &str, max: f64) {
    let mut present = value.as_f64().is_some();
    ui.horizontal(|ui| {
        if ui.checkbox(&mut present, label).changed() {
            *value = if present { json!(0.) } else { Value::Null };
        }
        if present {
            let mut n = value.as_f64().unwrap_or(0.);
            if ui
                .add(egui::DragValue::new(&mut n).range(0.0..=max).speed(0.1))
                .changed()
            {
                *value = json!(n);
            }
        }
    });
}
pub fn edit(ui: &mut egui::Ui, b: &mut Bookmark) {
    let data = b.metadata.entry("mining".into()).or_insert_with(
        || json!({"mass":null,"resistance":null,"instability":null,"scu":null,"composition":[]}),
    );
    if !data.is_object() {
        ui.colored_label(
            egui::Color32::RED,
            "Invalid mining data; cancel and review the import.",
        );
        return;
    }
    ui.separator();
    ui.heading("Mining scan");
    ui.label("Unchecked numeric fields are blank.");
    for (key, label, max) in [
        ("mass", "Mass", 1e12),
        ("resistance", "Resistance %", 100.),
        ("instability", "Instability", 1e12),
        ("scu", "Total SCU", 1e12),
    ] {
        optional_number(ui, &mut data[key], label, max);
    }
    let scu = data["scu"].as_f64();
    if data.get("composition").is_none() {
        data["composition"] = json!([]);
    }
    let Some(rows) = data["composition"].as_array_mut() else {
        return;
    };
    ui.label("Composition");
    let mut remove = None;
    egui::ScrollArea::vertical()
        .id_salt("composition")
        .max_height(220.)
        .show(ui, |ui| {
            for (i, row) in rows.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    ui.group(|ui| {
                        let mut ore = row["ore"].as_str().unwrap_or("Iron").to_owned();
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("ore")
                                .selected_text(&ore)
                                .show_ui(ui, |ui| {
                                    let mut ores = mining::ORES.to_vec();
                                    ores.sort();
                                    for name in ores {
                                        ui.selectable_value(&mut ore, name.to_owned(), name);
                                    }
                                });
                            if ui.small_button("Remove").clicked() {
                                remove = Some(i);
                            }
                        });
                        row["ore"] = ore.into();
                        optional_number(ui, &mut row["percent"], "Composition %", 100.);
                        optional_number(ui, &mut row["quality"], "Quality", 1e12);
                        ui.label(match mining::estimated_scu(scu, row) {
                            Some(n) => format!("Estimated SCU: {n:.4}"),
                            None => "Estimated SCU: —".into(),
                        });
                    });
                });
            }
        });
    if let Some(i) = remove {
        rows.remove(i);
    }
    if ui
        .add_enabled(rows.len() < 100, egui::Button::new("Add ore entry"))
        .clicked()
    {
        rows.push(json!({"ore":"Iron","percent":null,"quality":null}));
    }
}
