use sc_bookmarks::ocr_text::*;
use serde::Deserialize;
#[derive(Deserialize)]
struct Case {
    text: String,
    expected: Option<[f64; 3]>,
}
#[test]
fn printed_units_match_python() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("ocr_text_reference.json")).unwrap();
    for c in cases {
        assert_eq!(parse_units(&c.text).ok(), c.expected, "{}", c.text);
    }
}
#[test]
fn independent_rows_must_agree() {
    let a = "1.0000km 2.0000km 3.0000km";
    let b = "1.0000km 2.0000km 9.0000km";
    assert_eq!(
        coordinates(Some((a, 0.9)), None, false).unwrap(),
        [1., 2., 3.]
    );
    assert!(coordinates(Some((a, 0.9)), None, true).is_err());
    assert!(coordinates(Some((a, 0.9)), Some((b, 0.9)), false).is_err());
    assert!(coordinates(Some((a, 0.81)), None, false).is_err());
}
#[test]
fn camera_and_metadata_confidence_gates() {
    assert_eq!(
        camera("CamDir: -179 17 -154 FOV: 60", 0.8),
        Some([-179., 17., -154.])
    );
    assert!(camera("CamDir: 181 0 0", 0.99).is_none());
    assert!(camera("CamDir: 1 2 30.5", 0.99).is_none());
    assert!(camera("CamDir: 1 2 3", 0.74).is_none());
    assert_eq!(
        shard("ShardId: pub use1b 12881860 020", 0.9).as_deref(),
        Some("pub_use1b_12881860_020")
    );
    assert!(shard("ShardId: pub_use1b_1288_020", 0.9).is_none());
    assert_eq!(
        system("Current player location: Stanton", 0.85),
        Some("Stanton")
    );
    assert_eq!(system("Zone: NyxSolarSystem", 0.85), Some("Nyx"));
    assert!(system("chat: fly to Stanton", 0.99).is_none());
    let known = vec!["pub_use1b_12881860_020".into()];
    let read = ("pubuse1b12881860020", 0.9);
    assert!(corroborated_shard(&known, &[read, read]).is_none());
    assert_eq!(
        corroborated_shard(&known, &[read, read, read]),
        Some(known[0].as_str())
    );
}
