use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Serialize, Deserialize)]
pub struct Organization {
    #[serde(default = "defaults")]
    pub categories: Vec<String>,
    #[serde(default)]
    pub hide_other_systems: bool,
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub sort: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
fn defaults() -> Vec<String> {
    ["Mining", "Salvage", "Personal", "Corp"]
        .map(str::to_owned)
        .to_vec()
}
impl Default for Organization {
    fn default() -> Self {
        Self {
            categories: defaults(),
            hide_other_systems: false,
            system: String::new(),
            sort: "Category".into(),
            extra: BTreeMap::new(),
        }
    }
}
impl Organization {
    pub fn load(p: &std::path::Path) -> Result<Self, String> {
        match std::fs::read(p) {
            Ok(b) => {
                let mut s: Self = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
                if !s.categories.iter().any(|c| c == "Mining") {
                    s.categories.insert(0, "Mining".into());
                }
                validate(&s.categories)?;
                Ok(s)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn save(&self, p: &std::path::Path) -> Result<(), String> {
        validate(&self.categories)?;
        crate::storage::write_atomic(
            p,
            &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
    }
}
pub fn validate(names: &[String]) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for n in names {
        if n.trim().is_empty()
            || n.chars().count() > 80
            || n.eq_ignore_ascii_case("All categories")
            || !seen.insert(n.to_lowercase())
        {
            return Err("Use unique category names, 1–80 characters".into());
        }
    }
    if !names.iter().any(|n| n == "Mining") {
        return Err("Mining is a permanent category".into());
    }
    Ok(())
}
