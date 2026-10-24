use sc_bookmarks::{links, mining};
use serde_json::{Value, json};
#[test]
fn python_link_parity() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("link_reference.json")).unwrap();
    for c in cases {
        let b = links::decode(c["link"].as_str().unwrap()).unwrap();
        let actual = serde_json::to_value(&b).unwrap();
        for (k, v) in c["expected"].as_object().unwrap() {
            assert_eq!(&actual[k], v, "{k}");
        }
        let again = links::decode(&links::encode(b).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(again).unwrap(), actual);
    }
}
#[test]
fn rejects_invalid_links_and_atomic_batch() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("link_reference.json")).unwrap();
    let link = cases[1]["link"].as_str().unwrap();
    for bad in [
        format!("{link}&x=2"),
        link.replace("v=1", "v=99"),
        link.replace("units=km", "units=m"),
        link.replace("frame=root", "frame=ship"),
        link.replace("x=1.0", "x=nan"),
        format!("{link}&created=bad"),
    ] {
        assert!(links::decode(&bad).is_err(), "{bad}");
    }
    assert!(links::decode_many(&format!("{link}\ninvalid")).is_err());
    assert_eq!(
        links::decode_many(&format!("{link}\n\n{link}"))
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn mining_rows_and_blanks() {
    let row = json!({"ore":"Iron","percent":10.63,"quality":521});
    assert!((mining::estimated_scu(Some(58.58), &row).unwrap() - 6.227054).abs() < 1e-9);
    assert_eq!(
        mining::estimated_scu(Some(58.), &json!({"percent":10})),
        None
    );
    assert_eq!(
        mining::estimated_scu(Some(58.), &json!({"percent":0,"quality":0})),
        Some(0.)
    );
    for invalid in [
        json!({"resistance":101}),
        json!({"scu":true}),
        json!({"composition":[{"ore":"Iron","percent":-1}]}),
        json!({"composition":[{"ore":"Unknown"}]}),
    ] {
        assert!(mining::clean(&invalid).is_err());
    }
    let b=links::normalize(serde_json::from_value(json!({"name":"Rock","system":"Nyx","xyz_km":[1,2,3],"category":"Mining","mining":{"composition":[{"ore":"Iron","quality":325},{"ore":"Gold","quality":900}]}})).unwrap()).unwrap();
    assert!(!mining::matches(&b, "Iron", Some(500.)));
    assert!(mining::matches(&b, "Gold", Some(500.)));
}
