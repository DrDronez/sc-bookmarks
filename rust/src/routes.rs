//! Offline routing port. No capture, game input, or network access.
use crate::{Bookmark, closest};
use serde::Deserialize;
#[derive(Debug, Clone, Deserialize)]
pub struct Marker {
    pub name: String,
    pub system: String,
    pub xyz_km: [f64; 3],
    #[serde(default)]
    pub arrival_km: f64,
    #[serde(default)]
    pub body_radius_km: f64,
    #[serde(default)]
    pub roi_radius_km: f64,
    #[serde(default)]
    pub position_error_km: f64,
}
#[derive(Debug, Deserialize)]
pub struct Catalog {
    pub markers: Vec<Marker>,
}
#[derive(Debug)]
pub enum Route<'a> {
    Celestial(&'a Marker),
    Pair {
        a: &'a Marker,
        b: &'a Marker,
        distance_km: f64,
        fraction: f64,
        position: [f64; 3],
        allowance_km: f64,
    },
}
impl Route<'_> {
    pub fn names(&self) -> Vec<&str> {
        match self {
            Self::Celestial(m) => vec![&m.name],
            Self::Pair { a, b, .. } => vec![&a.name, &b.name],
        }
    }
    pub fn label(&self) -> String {
        self.names().join(" ↔ ")
    }
}
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}
fn lerp(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    std::array::from_fn(|i| a[i] + t * (b[i] - a[i]))
}
pub fn catalog() -> Result<Catalog, serde_json::Error> {
    serde_json::from_str(include_str!("../../route-data/quantum-markers.json"))
}
pub fn suggest<'a>(
    bookmark: &Bookmark,
    markers: &'a [Marker],
) -> Result<Option<Route<'a>>, &'static str> {
    bookmark.validate()?;
    if bookmark
        .metadata
        .get("frame")
        .and_then(|v| v.as_str())
        .unwrap_or("root")
        .to_lowercase()
        != "root"
    {
        return Err("Root coordinates required");
    }
    let target = bookmark.xyz_km;
    let candidates: Vec<_> = markers
        .iter()
        .filter(|m| m.system.to_lowercase() == bookmark.system.trim().to_lowercase())
        .collect();
    let body = candidates
        .iter()
        .copied()
        .filter(|m| {
            m.body_radius_km > 0.
                && m.roi_radius_km > 0.
                && distance(target, m.xyz_km) <= m.roi_radius_km
        })
        .min_by(|a, b| {
            a.roi_radius_km
                .total_cmp(&b.roi_radius_km)
                .then_with(|| distance(target, a.xyz_km).total_cmp(&distance(target, b.xyz_km)))
                .then_with(|| a.name.cmp(&b.name))
        });
    if let Some(m) = body {
        return Ok(Some(Route::Celestial(m)));
    }
    let mut best: Option<Route> = None;
    for (i, a) in candidates.iter().enumerate() {
        for b in candidates.iter().skip(i + 1) {
            let length = distance(a.xyz_km, b.xyz_km);
            if length <= a.arrival_km + b.arrival_km {
                continue;
            }
            let low = a.arrival_km / length;
            let high = 1. - b.arrival_km / length;
            let Some((dist, fraction, position)) = closest(target, a.xyz_km, b.xyz_km, low, high)
            else {
                continue;
            };
            let start = lerp(a.xyz_km, b.xyz_km, low);
            let end = lerp(a.xyz_km, b.xyz_km, high);
            if candidates.iter().any(|m| {
                m.body_radius_km > 0.
                    && closest(m.xyz_km, start, end, 0., 1.).is_some_and(|r| r.0 < m.body_radius_km)
            }) {
                continue;
            }
            let allowance = a.position_error_km.max(b.position_error_km);
            let better = match &best {
                None => true,
                Some(Route::Pair {
                    a: old_a,
                    b: old_b,
                    distance_km,
                    allowance_km,
                    ..
                }) => {
                    (dist, allowance, &a.name, &b.name)
                        < (*distance_km, *allowance_km, &old_a.name, &old_b.name)
                }
                _ => false,
            };
            if better {
                best = Some(Route::Pair {
                    a,
                    b,
                    distance_km: dist,
                    fraction,
                    position,
                    allowance_km: allowance,
                });
            }
        }
    }
    Ok(best)
}
