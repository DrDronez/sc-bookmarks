use crate::{Bookmark, mining};
use serde_json::{Value, json};
use std::collections::BTreeMap;
pub fn normalize(mut b: Bookmark) -> Result<Bookmark, String> {
    b.name = b.name.trim().into();
    b.system = b.system.trim().into();
    b.validate()?;
    let m = &mut b.metadata;
    let shard = m.entry("shard_id".into()).or_insert(Value::Null);
    if shard.as_str() == Some("") {
        *shard = Value::Null
    }
    if !shard.is_null() {
        let s = shard.as_str().ok_or("Invalid shard")?;
        if s.chars().count() > 128 || s.chars().any(|c| (c as u32) < 32) {
            return Err("Invalid shard".into());
        }
    }
    let category = m
        .get("category")
        .and_then(Value::as_str)
        .unwrap_or("Personal")
        .trim();
    if category.is_empty() || category.chars().count() > 80 {
        return Err("Invalid category".into());
    }
    let category = if category.to_lowercase() == "mining" {
        "Mining".into()
    } else {
        category.to_owned()
    };
    m.insert("category".into(), json!(category));
    let notes = m.entry("notes".into()).or_insert(json!(""));
    if !notes.is_string() || notes.as_str().unwrap().chars().count() > 10000 {
        return Err("Invalid notes".into());
    }
    if let Some(v) = m.get("mining") {
        let v = mining::clean(v)?;
        m.insert("mining".into(), v);
    }
    m.insert("frame".into(), json!("root"));
    m.insert("units".into(), json!("km"));
    Ok(b)
}
pub fn encode(b: Bookmark) -> Result<String, String> {
    let b = normalize(b)?;
    let m = &b.metadata;
    let extended = m.contains_key("mining") || m["notes"].as_str() != Some("");
    let mut fields = vec![
        ("v", if extended { "2".into() } else { "1".into() }),
        ("x", b.xyz_km[0].to_string()),
        ("y", b.xyz_km[1].to_string()),
        ("z", b.xyz_km[2].to_string()),
        ("units", "km".into()),
        ("frame", "root".into()),
        ("name", b.name),
        ("system", b.system),
        ("category", m["category"].as_str().unwrap().into()),
    ];
    for (key, field) in [
        ("shard_id", "shard"),
        ("utc", "created"),
        ("notes", "notes"),
    ] {
        if let Some(s) = m.get(key).and_then(Value::as_str).filter(|s| !s.is_empty()) {
            fields.push((field, s.into()));
        }
    }
    if let Some(v) = m.get("mining") {
        fields.push(("mining", v.to_string()));
    }
    Ok(format!(
        "scbm://bookmark?{}",
        form_urlencoded::Serializer::new(String::new())
            .extend_pairs(fields)
            .finish()
    ))
}
pub fn decode(link: &str) -> Result<Bookmark, String> {
    if link.chars().count() > 200000 {
        return Err("Link too long".into());
    }
    let link = link.trim();
    let (head, query) = link.split_once('?').ok_or("Missing query")?;
    let (scheme, authority) = head.split_once("://").ok_or("Invalid scheme")?;
    if !scheme.eq_ignore_ascii_case("scbm")
        || !matches!(authority, "bookmark" | "bookmark/")
        || query.contains('#')
    {
        return Err("Expected scbm bookmark link".into());
    }
    let mut q = BTreeMap::new();
    let mut count = 0;
    for (k, v) in form_urlencoded::parse(query.as_bytes()) {
        count += 1;
        if count > 20 || q.insert(k.into_owned(), v.into_owned()).is_some() {
            return Err("Too many or duplicate fields".into());
        }
    }
    let get = |key: &str| {
        q.get(key)
            .map(String::as_str)
            .ok_or_else(|| format!("Missing {key}"))
    };
    if !matches!(get("v")?, "1" | "2") || get("units")? != "km" || get("frame")? != "root" {
        return Err("Unsupported version, units or frame".into());
    }
    let mut b = Bookmark {
        name: get("name")?.into(),
        system: get("system")?.into(),
        xyz_km: [0.; 3],
        metadata: BTreeMap::new(),
    };
    for (i, k) in ["x", "y", "z"].iter().enumerate() {
        b.xyz_km[i] = get(k)?.parse().map_err(|_| "Invalid coordinate")?;
    }
    for (field, key) in [
        ("shard", "shard_id"),
        ("created", "utc"),
        ("category", "category"),
        ("notes", "notes"),
    ] {
        if let Some(v) = q.get(field) {
            b.metadata.insert(key.into(), json!(v));
        }
    }
    if let Some(v) = q.get("created").filter(|s| !s.is_empty()) {
        if chrono::DateTime::parse_from_rfc3339(v).is_err()
            && chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S%.f").is_err()
            && chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_err()
        {
            return Err("Invalid creation date".into());
        }
    }
    if let Some(v) = q.get("mining") {
        b.metadata.insert(
            "mining".into(),
            serde_json::from_str(v).map_err(|_| "Invalid mining JSON")?,
        );
    }
    b.metadata.insert("kind".into(), json!("imported bookmark"));
    normalize(b)
}
pub fn decode_many(text: &str) -> Result<Vec<Bookmark>, String> {
    if text.chars().count() > 4000000 {
        return Err("Batch too large".into());
    }
    let lines: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if lines.is_empty() || lines.len() > 500 {
        return Err("Use 1–500 links".into());
    }
    lines.into_iter().map(decode).collect()
}
