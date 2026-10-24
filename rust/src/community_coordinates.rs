//! Community snapshot frame conversion. Rotation is an estimate using the
//! source period (hours), phase (degrees) and 2020-01-01 UTC reference epoch.
//! Geometry is implemented here independently; no game data is read at runtime.
use crate::Bookmark;
use serde::{Deserialize, Serialize};
#[derive(Deserialize)]
struct Container {
    name: String,
    system: String,
    center: [f64; 3],
    period: f64,
    phase: f64,
    supported: bool,
}
#[derive(Deserialize, Serialize)]
pub struct Local {
    pub container: String,
    pub xyz_km: [f64; 3],
}
fn containers() -> &'static Vec<Container> {
    static C: std::sync::OnceLock<Vec<Container>> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        serde_json::from_str(include_str!("../../route-data/community-containers.json"))
            .expect("Container catalog")
    })
}
pub fn position(system: &str, local: &Local, unix_seconds: f64) -> Result<[f64; 3], String> {
    if !unix_seconds.is_finite()
        || local
            .xyz_km
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 1e12)
    {
        return Err("Invalid local coordinates".into());
    }
    let matches: Vec<_> = containers()
        .iter()
        .filter(|c| {
            c.system.eq_ignore_ascii_case(system) && c.name.eq_ignore_ascii_case(&local.container)
        })
        .collect();
    if matches.len() != 1 {
        return Err(format!(
            "No unique coordinate container for {}",
            local.container
        ));
    }
    let c = matches[0];
    if !c.supported {
        return Err("This container's tilted coordinate frame is not supported yet".into());
    }
    let angle = (c.phase
        + if c.period == 0. {
            0.
        } else {
            (unix_seconds - 1577836800.) * 0.1 / c.period
        })
    .rem_euclid(360.)
    .to_radians();
    let (sin, cos) = angle.sin_cos();
    let [x, y, z] = local.xyz_km;
    Ok([
        c.center[0] + x * cos - y * sin,
        c.center[1] + x * sin + y * cos,
        c.center[2] + z,
    ])
}
pub fn refresh(b: &mut Bookmark) -> Result<(), String> {
    if let Some(v) = b.metadata.get("community_local") {
        let local: Local = serde_json::from_value(v.clone())
            .map_err(|_| "Invalid community coordinate metadata")?;
        b.xyz_km = position(
            &b.system,
            &local,
            chrono::Utc::now().timestamp_millis() as f64 / 1000.,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotation_preserves_offset_and_tracks_quarter_turn() {
        let c = containers()
            .iter()
            .find(|c| c.system == "Stanton" && c.name == "Yela")
            .unwrap();
        let local = Local {
            container: "Yela".into(),
            xyz_km: [100., 0., 10.],
        };
        let a = position("Stanton", &local, 1577836800.).unwrap();
        let b = position("Stanton", &local, 1577836800. + c.period * 3600. / 4.).unwrap();
        let x = a[0] - c.center[0];
        let y = a[1] - c.center[1];
        assert!(((b[0] - c.center[0]) + y).abs() < 1e-5);
        assert!(((b[1] - c.center[1]) - x).abs() < 1e-5);
        assert_eq!(b[2] - c.center[2], 10.);
        assert!(position("Pyro", &local, 1577836800.).is_err());
        assert!(
            position(
                "Stanton",
                &Local {
                    container: "Missing".into(),
                    xyz_km: [0.; 3]
                },
                1577836800.
            )
            .is_err()
        );
    }
    #[test]
    fn local_links_preserve_frame_and_old_links_are_unchanged() {
        let mut b = Bookmark {
            name: "Bennyhenge".into(),
            system: "Stanton".into(),
            xyz_km: [0.; 3],
            metadata: Default::default(),
        };
        b.metadata.insert("community_local".into(),serde_json::json!({"container":"Yela","xyz_km":[-626.6486875,135.682515625,-1.818643433]}));
        let link = crate::links::encode(b).unwrap();
        assert!(link.contains("v=3"));
        let decoded = crate::links::decode(&link).unwrap();
        assert_eq!(decoded.metadata["community_local"]["container"], "Yela");
        assert!(decoded.xyz_km[0].abs() > 1_000_000.);
        assert!(crate::links::decode(&link.replace("v=3", "v=2")).is_err());
    }
}
