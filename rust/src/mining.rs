use serde_json::{Value, json};
pub const ORES: &[&str] = &[
    "Quantainium",
    "Stileron",
    "Riccite",
    "Gold",
    "Borase",
    "Lindinium",
    "Taranite",
    "Bexalite",
    "Agricium",
    "Tungsten",
    "Laranite",
    "Titanium",
    "Torite",
    "Ice",
    "Beryl",
    "Hephaestanite",
    "Tin",
    "Aluminium",
    "Copper",
    "Corundum",
    "Iron",
    "Silicon",
    "Quartz",
    "Aslarite",
    "Carinite",
    "Ouratite",
    "Savrilium",
    "Inert Materials",
];
fn number(v: &Value, max: f64) -> Result<Value, String> {
    if v.is_null() || v.as_str().is_some_and(|s| s.trim().is_empty()) {
        return Ok(Value::Null);
    }
    let n = if let Some(s) = v.as_str() {
        s.trim().parse::<f64>().ok()
    } else {
        v.as_f64()
    }
    .ok_or("Expected number or blank")?;
    if !n.is_finite() || n < 0. || n > max {
        return Err("Number outside allowed range".into());
    }
    Ok(json!(n))
}
pub fn clean(v: &Value) -> Result<Value, String> {
    if !v.is_object() {
        return Err("Invalid mining data".into());
    }
    let mut out = json!({});
    for key in ["mass", "resistance", "instability", "scu"] {
        out[key] = number(&v[key], if key == "resistance" { 100. } else { 1e12 })?;
    }
    let empty = vec![];
    let rows = match v.get("composition") {
        None => &empty,
        Some(x) => x.as_array().ok_or("Invalid composition")?,
    };
    if rows.len() > 100 {
        return Err("At most 100 composition entries".into());
    }
    let mut result = vec![];
    for row in rows {
        let ore = row["ore"].as_str().ok_or("Missing ore")?;
        if !ORES.contains(&ore) {
            return Err("Unknown ore".into());
        }
        result.push(json!({"ore":ore,"percent":number(&row["percent"],100.)?,"quality":number(&row["quality"],1e12)?}));
    }
    out["composition"] = json!(result);
    Ok(out)
}
pub fn estimated_scu(scu: Option<f64>, row: &Value) -> Option<f64> {
    row["quality"].as_f64()?;
    Some(scu? * row["percent"].as_f64()? / 100.)
}
pub fn matches(bookmark: &crate::Bookmark, ore: &str, minimum: Option<f64>) -> bool {
    if ore.is_empty() && minimum.is_none() {
        return true;
    }
    if bookmark.metadata.get("category").and_then(Value::as_str) != Some("Mining") {
        return false;
    }
    bookmark
        .metadata
        .get("mining")
        .and_then(|m| m["composition"].as_array())
        .is_some_and(|rows| {
            rows.iter().any(|r| {
                (ore.is_empty() || r["ore"].as_str() == Some(ore))
                    && minimum.is_none_or(|n| r["quality"].as_f64().is_some_and(|q| q >= n))
            })
        })
}
