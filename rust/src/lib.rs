use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub system: String,
    pub xyz_km: [f64; 3],
    #[serde(flatten)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}
impl Bookmark {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.name.trim().is_empty() || self.name.chars().count() > 200 {
            return Err("Invalid name");
        }
        if self.system.trim().is_empty() || self.system.chars().count() > 80 {
            return Err("Invalid system");
        }
        if self.xyz_km.iter().any(|v| !v.is_finite() || v.abs() > 1e12) {
            return Err("Invalid coordinates");
        }
        Ok(())
    }
}

pub fn closest(
    target: [f64; 3],
    start: [f64; 3],
    end: [f64; 3],
    lower: f64,
    upper: f64,
) -> Option<(f64, f64, [f64; 3])> {
    let d = std::array::from_fn::<_, 3, _>(|i| end[i] - start[i]);
    let length2: f64 = d.iter().map(|v| v * v).sum();
    if length2 <= 1e-12 {
        return None;
    }
    let t =
        ((0..3).map(|i| (target[i] - start[i]) * d[i]).sum::<f64>() / length2).clamp(lower, upper);
    let p = std::array::from_fn(|i| start[i] + t * d[i]);
    Some((
        (0..3)
            .map(|i| (target[i] - p[i]).powi(2))
            .sum::<f64>()
            .sqrt(),
        t,
        p,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_segment_not_extension() {
        assert_eq!(
            closest([12., 4., 0.], [0.; 3], [10., 0., 0.], 0., 1.),
            Some((20_f64.sqrt(), 1., [10., 0., 0.]))
        );
    }
    #[test]
    fn preserve_unknown_and_mining_fields() {
        let source = r#"{"name":"Rock","system":"Stanton","xyz_km":[1,2,3],"notes":"Keep","mining":{"composition":[{"ore":"Iron","quality":664}]},"future_field":true}"#;
        let b: Bookmark = serde_json::from_str(source).unwrap();
        b.validate().unwrap();
        let output = serde_json::to_value(b).unwrap();
        assert_eq!(output["future_field"], true);
        assert_eq!(output["mining"]["composition"][0]["quality"], 664);
    }
}
pub mod capture;
pub mod detector;
pub mod guidance;
pub mod library;
pub mod links;
pub mod map;
pub mod mining;
pub mod native_overlay;
pub mod ocr_runtime;
pub mod ocr_text;
pub mod ocr_worker;
pub mod organization;
pub mod overlay;
pub mod position_filter;
pub mod protocol;
pub mod recognizer;
pub mod routes;
pub mod scan_ocr;
pub mod scan_worker;
pub mod scanning;
pub mod storage;
pub mod window_geometry;

pub mod travel_log;
