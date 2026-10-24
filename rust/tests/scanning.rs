use sc_bookmarks::scanning::*;
use serde::Deserialize;
#[derive(Deserialize)]
struct Case {
    rs: u64,
    description: String,
    matches: Vec<Vec<usize>>,
}
#[test]
fn all_reference_signatures_match_python() {
    let charts = charts().unwrap();
    let cases: Vec<Case> = serde_json::from_str(include_str!("scanning_reference.json")).unwrap();
    for c in cases {
        assert_eq!(description(&charts, c.rs), c.description, "RS {}", c.rs);
        for (chart, expected) in charts.iter().zip(c.matches) {
            assert_eq!(chart.filtered("", Some(c.rs), None), expected);
        }
    }
}
#[test]
fn parses_filters_and_sorts_numeric_values() {
    assert_eq!(parse_rs(" 18,000 "), Ok(Some(18000)));
    assert_eq!(parse_rs(""), Ok(None));
    for text in ["-1", "0", "1.5", "1k", "18446744073709551616"] {
        assert!(parse_rs(text).is_err());
    }
    let charts = charts().unwrap();
    let salvage = &charts[2];
    let ascending = salvage.filtered("", None, Some((0, false)));
    assert_eq!(salvage.rows[ascending[0]][0], Cell::Number(1700));
    let descending = salvage.filtered("", None, Some((0, true)));
    assert_eq!(salvage.rows[descending[0]][0], Cell::Number(30000));
    assert_eq!(charts[1].filtered("ROC", Some(8000), None).len(), 1);
    assert!(charts[0].filtered("ROC", Some(8000), None).is_empty());
    assert_eq!(
        description(&charts, 8000),
        "Scan Results for 8,000 Indicate:\nSurface: 2 ROC Deposits\nSpace  : 4 Salvage Scrap Pieces"
    );
}
