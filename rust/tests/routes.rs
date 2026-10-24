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
