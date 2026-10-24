use sc_bookmarks::{map::*, routes};
use serde::Deserialize;
#[derive(Deserialize)]
struct Case {
    point: [f64; 3],
    center: [f64; 3],
    yaw: f64,
    tilt: f64,
    scale: f64,
    expected: [f64; 2],
}
#[test]
fn projection_matches_reference_and_fit_keeps_points_visible() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("map_reference.json")).unwrap();
    for c in cases {
        let camera = Camera {
            center: c.center,
            yaw: c.yaw,
            tilt: c.tilt,
            scale: c.scale,
        };
        for (a, b) in camera.project(c.point).into_iter().zip(c.expected) {
            assert!((a - b).abs() < 1e-8 * b.abs().max(1.));
        }
    }
    let catalog = routes::catalog().unwrap();
    for system in ["Stanton", "Pyro", "Nyx"] {
        let points: Vec<_> = catalog
            .markers
            .iter()
            .filter(|m| m.system == system)
            .map(|m| m.xyz_km)
            .collect();
        for tilt in [0.001, 0.65, std::f64::consts::FRAC_PI_2] {
            let mut camera = Camera {
                tilt,
                ..Default::default()
            };
            camera.fit(&points, [1000., 600.]);
            for &p in &points {
                let [x, y] = camera.project(p);
                assert!(x.abs() <= 421. && y.abs() <= 236.);
            }
        }
    }
}
#[test]
fn halo_particles_are_bounded_and_primary_rings_are_correct() {
    let data = Data::load().unwrap();
    assert_eq!(data.stars.len(), 3);
    assert_eq!(data.halo.bands.len(), 10);
    assert_eq!(data.dust.len(), 2600);
    for (number, p) in &data.dust {
        let b = data
            .halo
            .bands
            .iter()
            .find(|b| b.number == *number)
            .unwrap();
        let r = (p[0] - data.halo.origin_km[0]).hypot(p[1] - data.halo.origin_km[1]);
        assert!(r >= b.inner_km && r <= b.outer_km);
    }
    let catalog = routes::catalog().unwrap();
    for (system, count) in [("Stanton", 4), ("Pyro", 6), ("Nyx", 0)] {
        assert_eq!(
            catalog
                .markers
                .iter()
                .filter(|m| m.system == system && is_primary(system, &m.name))
                .count(),
            count
        );
    }
    assert!(!is_primary("Nyx", "Delamar"));
    assert_ne!(data.stars["Stanton"].xyz_km, [0.; 3]);
}

#[test]
fn new_belts_are_bounded_and_station_offsets_resolve() {
    let data = Data::load().unwrap();
    let catalog = routes::catalog().unwrap();
    assert_eq!(data.features.belts.len(), 3);
    assert_eq!(data.features.stations.len(), 6);
    for belt in &data.features.belts {
        assert_eq!(belt.particles.len(), 1200);
        assert!(parent_position(&catalog, &belt.system, &belt.parent, belt.center_km).is_some());
        for p in &belt.particles {
            let r = p[0].hypot(p[1]);
            assert!(r >= belt.inner_km && r <= belt.outer_km);
        }
    }
    for s in &data.features.stations {
        let p = parent_position(&catalog, &s.system, &s.parent, s.offset_km).unwrap();
        assert!(p.iter().all(|x| x.is_finite()));
        assert!(s.offset_km.iter().map(|x| x * x).sum::<f64>().sqrt() < 10000.);
    }
    let iv = catalog
        .markers
        .iter()
        .find(|m| m.name == "Pyro IV")
        .unwrap();
    let v = catalog.markers.iter().find(|m| m.name == "Pyro V").unwrap();
    assert_eq!(orbit_center(&catalog, iv), v.xyz_km);
    assert_eq!(orbit_center(&catalog, v), [0.; 3]);
}
