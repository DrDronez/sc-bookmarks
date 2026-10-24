use sc_bookmarks::{
    Bookmark,
    routes::{Route, catalog, suggest},
};
#[test]
fn python_route_parity() {
    let catalog = catalog().unwrap();
    assert_eq!(catalog.markers.len(), 70);
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("route_reference.json")).unwrap();
    for case in cases {
        let bookmark: Bookmark = serde_json::from_value(case["bookmark"].clone()).unwrap();
        let actual = suggest(&bookmark, &catalog.markers).unwrap();
        let expected = &case["result"];
        if expected.is_null() {
            assert!(actual.is_none());
            continue;
        }
        let actual = actual.unwrap();
        let names: Vec<String> = serde_json::from_value(expected["names"].clone()).unwrap();
        assert_eq!(actual.names(), names, "bookmark: {:?}", bookmark);
        if let Route::Pair {
            distance_km,
            fraction,
            ..
        } = actual
        {
            assert!((distance_km - expected["distance"].as_f64().unwrap()).abs() < 1e-6);
            assert!((fraction - expected["fraction"].as_f64().unwrap()).abs() < 1e-12);
        }
    }
}

#[test]
fn current_position_leg_is_finite_improving_and_unobstructed() {
    use sc_bookmarks::routes::{Marker, suggest_from_position};
    let mut b = Bookmark {
        name: "Target".into(),
        system: "Stanton".into(),
        xyz_km: [50., 10., 0.],
        metadata: Default::default(),
    };
    let marker = Marker {
        name: "Forward".into(),
        system: "Stanton".into(),
        xyz_km: [100., 0., 0.],
        arrival_km: 10.,
        body_radius_km: 0.,
        roi_radius_km: 0.,
        position_error_km: 0.,
    };
    let mut markers = vec![marker];
    let r = suggest_from_position(&b, "Stanton", [0.; 3], &markers).unwrap();
    assert!((r.distance_km - 10.).abs() < 1e-9);
    assert_eq!(r.position, [50., 0., 0.]);
    assert_eq!(r.travel_km, 50.);
    assert!(suggest_from_position(&b, "Pyro", [0.; 3], &markers).is_none());
    assert!(suggest_from_position(&b, "Stanton", [f64::NAN, 0., 0.], &markers).is_none());
    b.xyz_km = [-50., 10., 0.];
    assert!(suggest_from_position(&b, "Stanton", [0.; 3], &markers).is_none());
    b.xyz_km = [110., 0., 0.];
    let r = suggest_from_position(&b, "Stanton", [0.; 3], &markers).unwrap();
    assert_eq!(r.position, [90., 0., 0.]); // cannot project past arrival zone
    assert_eq!(r.distance_km, 20.);
    b.xyz_km = [50., 10., 0.];
    let mut obstacle = markers[0].clone();
    obstacle.name = "Blocking body".into();
    obstacle.xyz_km = [25., 0., 0.];
    obstacle.body_radius_km = 5.;
    obstacle.arrival_km = 0.;
    markers.push(obstacle);
    assert!(suggest_from_position(&b, "Stanton", [0.; 3], &markers).is_none());
    markers.pop();
    b.metadata.insert("frame".into(), "local".into());
    assert!(suggest_from_position(&b, "Stanton", [0.; 3], &markers).is_none());
}
