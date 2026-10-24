//! Public refinery data and local-only cargo comparisons. No game integration.
use crate::{Bookmark, routes::Catalog};
use serde::{Deserialize, Serialize};
#[derive(Clone, Deserialize, Serialize)]
pub struct Bonus {
    pub id_terminal: u64,
    pub terminal_name: String,
    pub star_system_name: String,
    pub commodity_name: String,
    pub value: f64,
    pub date_modified: i64,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Method {
    pub name: String,
    pub rating_yield: u8,
    pub rating_cost: u8,
    pub rating_speed: u8,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Data {
    pub refineries_yields: Vec<Bonus>,
    pub refineries_methods: Vec<Method>,
}
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Load {
    pub ore: String,
    pub scu: f64,
    pub quality: Option<f64>,
}
pub fn ore_key(s: &str) -> String {
    s.to_lowercase()
        .replace(" (ore)", "")
        .replace("aluminium", "aluminum")
        .replace("quantainium", "quantanium")
        .trim()
        .to_owned()
}
impl Data {
    pub fn bundled() -> Self {
        serde_json::from_str(include_str!("../../route-data/refineries-uex.json"))
            .expect("Refinery snapshot")
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.refineries_methods.is_empty()
            || self.refineries_yields.is_empty()
            || self.refineries_yields.len() > 5000
            || self.refineries_methods.len() > 30
        {
            return Err("Invalid dataset size".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for b in &self.refineries_yields {
            if !b.value.is_finite()
                || b.value.abs() > 100.
                || b.terminal_name.len() > 200
                || b.commodity_name.len() > 100
                || !["Stanton", "Pyro", "Nyx"].contains(&b.star_system_name.as_str())
                || !seen.insert((b.id_terminal, ore_key(&b.commodity_name)))
            {
                return Err("Invalid or duplicate refinery bonus".into());
            }
        }
        if self.refineries_methods.iter().any(|m| {
            m.name.len() > 100
                || ![m.rating_yield, m.rating_cost, m.rating_speed]
                    .iter()
                    .all(|v| (1..=3).contains(v))
        }) {
            return Err("Invalid method ratings".into());
        }
        Ok(())
    }
    pub fn terminals(&self) -> Vec<&Bonus> {
        let mut seen = std::collections::BTreeSet::new();
        self.refineries_yields
            .iter()
            .filter(|b| seen.insert(b.id_terminal))
            .collect()
    }
    pub fn bonus(&self, id: u64, ore: &str) -> Option<&Bonus> {
        self.refineries_yields
            .iter()
            .find(|b| b.id_terminal == id && ore_key(&b.commodity_name) == ore_key(ore))
    }
    /// A missing record never becomes a zero bonus or a winning partial score.
    pub fn score(&self, id: u64, load: &[Load]) -> Option<f64> {
        if load.iter().any(|r| !r.scu.is_finite() || r.scu < 0.) {
            return None;
        }
        let mut total = 0.;
        let mut weighted = 0.;
        for r in load
            .iter()
            .filter(|r| r.scu > 0. && r.scu.is_finite() && !ore_key(&r.ore).contains("inert"))
        {
            let bonus = self.bonus(id, &r.ore)?;
            total += r.scu;
            weighted += r.scu * bonus.value;
        }
        (total > 0.).then(|| weighted / total)
    }
}
pub fn from_bookmark(b: &Bookmark) -> Vec<Load> {
    let Some(m) = b.metadata.get("mining") else {
        return vec![];
    };
    m["composition"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            let ore = r["ore"].as_str()?;
            if ore_key(ore).contains("inert") {
                return None;
            }
            Some(Load {
                ore: ore.into(),
                scu: crate::mining::estimated_scu(m["scu"].as_f64(), r).unwrap_or(0.),
                quality: r["quality"].as_f64(),
            })
        })
        .collect()
}
/// Terminal approaches use known catalog points; local terminals use their celestial anchor.
pub fn approach(b: &Bonus, catalog: &Catalog) -> Option<Bookmark> {
    let name = b.terminal_name.split(" - ").nth(1)?.split(" (").next()?;
    let anchor = match name {
        "Levski" => "Delamar",
        "Ruin Station" => "Terminus",
        "Orbituary" => "Bloom",
        "Checkmate" => "Checkmate",
        other => other,
    };
    static EXTRA: std::sync::OnceLock<Catalog> = std::sync::OnceLock::new();
    let extra = EXTRA.get_or_init(|| {
        serde_json::from_str(include_str!("../../route-data/map-reference-points.json"))
            .expect("Reference points")
    });
    let m = catalog
        .markers
        .iter()
        .chain(extra.markers.iter())
        .find(|m| {
            m.system == b.star_system_name
                && (m.name == anchor
                    || (anchor.contains("-L") && m.name.starts_with(&format!("{anchor} "))))
        })?;
    Some(Bookmark {
        name: m.name.clone(),
        system: m.system.clone(),
        xyz_km: m.xyz_km,
        metadata: Default::default(),
    })
}
pub fn distance(origin: &Bookmark, target: &Bookmark) -> Option<f64> {
    (origin.system.eq_ignore_ascii_case(&target.system)).then(|| {
        (0..3)
            .map(|i| (origin.xyz_km[i] - target.xyz_km[i]).powi(2))
            .sum::<f64>()
            .sqrt()
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weighted_missing_and_inert() {
        let mut d = Data::bundled();
        d.refineries_yields = vec![
            Bonus {
                id_terminal: 1,
                terminal_name: "Test".into(),
                star_system_name: "Stanton".into(),
                commodity_name: "Iron (Ore)".into(),
                value: 10.,
                date_modified: 0,
            },
            Bonus {
                id_terminal: 1,
                terminal_name: "Test".into(),
                star_system_name: "Stanton".into(),
                commodity_name: "Gold (Ore)".into(),
                value: -5.,
                date_modified: 0,
            },
        ];
        let mut l = vec![
            Load {
                ore: "Iron".into(),
                scu: 3.,
                quality: Some(600.),
            },
            Load {
                ore: "Gold".into(),
                scu: 1.,
                quality: None,
            },
        ];
        assert_eq!(d.score(1, &l), Some(6.25));
        l.push(Load {
            ore: "Unknown".into(),
            scu: 1.,
            quality: None,
        });
        assert_eq!(d.score(1, &l), None);
        l.last_mut().unwrap().ore = "Inert Materials".into();
        assert_eq!(d.score(1, &l), Some(6.25));
    }
    #[test]
    fn prefill_preserves_quality_batches_and_ignores_inert() {
        let b:Bookmark=serde_json::from_value(serde_json::json!({"name":"Rock","system":"Nyx","xyz_km":[1,2,3],"mining":{"scu":100,"composition":[{"ore":"Iron","percent":10,"quality":664},{"ore":"Iron","percent":20,"quality":325},{"ore":"Inert Materials","percent":70,"quality":0}]}})).unwrap();
        let rows = from_bookmark(&b);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].scu, 10.);
        assert_eq!(rows[1].quality, Some(325.));
        assert_eq!(ore_key("Quantainium"), ore_key("Quantanium (Ore)"));
        let mut d = Data::bundled();
        d.refineries_yields.push(d.refineries_yields[0].clone());
        assert!(d.validate().is_err());
    }
    #[test]
    fn snapshot_and_approaches() {
        let d = Data::bundled();
        d.validate().unwrap();
        let c = crate::routes::catalog().unwrap();
        for t in d.terminals() {
            assert!(approach(t, &c).is_some(), "{}", t.terminal_name);
        }
        let a = approach(d.terminals()[0], &c).unwrap();
        let mut b = a.clone();
        b.system = "Different".into();
        assert_eq!(distance(&a, &b), None);
    }
}
