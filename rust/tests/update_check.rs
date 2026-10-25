use sc_bookmarks::update_check::{available, summarize};

#[test]
fn detects_alpha_releases_and_compares_numbers() {
    let json = br#"[{"tag_name":"v0.0.9-alpha"},{"tag_name":"0.0.13-alpha","body":"- Faster scans\n- New maps"}]"#;
    let update = available(json, "0.0.12-alpha").unwrap().unwrap();
    assert_eq!(update.version, "0.0.13-alpha");
    assert_eq!(update.changes, ["Faster scans", "New maps"]);
}

#[test]
fn ignores_drafts_invalid_tags_and_old_versions() {
    let json = br#"[{"tag_name":"0.1.0","draft":true},{"tag_name":"nightly"},{"tag_name":"0.0.11-alpha"},{"tag_name":"0.0.12-alpha"}]"#;
    assert!(available(json, "0.0.12-alpha").unwrap().is_none());
    assert!(available(b"[]", "0.0.12-alpha").unwrap().is_none());
}

#[test]
fn release_order_does_not_determine_newest_version() {
    let json =
        br#"[{"tag_name":"0.0.13-alpha"},{"tag_name":"0.0.14-alpha"},{"tag_name":"0.0.13"}]"#;
    assert_eq!(
        available(json, "0.0.12-alpha").unwrap().unwrap().version,
        "0.0.14-alpha"
    );
}

#[test]
fn stable_beats_same_version_alpha_and_build_metadata_is_ignored() {
    assert!(
        available(br#"[{"tag_name":"0.0.12"}]"#, "0.0.12-alpha")
            .unwrap()
            .is_some()
    );
    assert!(
        available(br#"[{"tag_name":"0.0.12-alpha+new"}]"#, "0.0.12-alpha")
            .unwrap()
            .is_none()
    );
    assert!(
        available(br#"[{"tag_name":"0.0.12-alpha"}]"#, "0.0.12")
            .unwrap()
            .is_none()
    );
}

#[test]
fn malformed_and_error_responses_are_failures() {
    for json in [
        b"not json".as_slice(),
        br#"{"message":"rate limited"}"#,
        br#"[{}]"#,
    ] {
        assert!(available(json, "0.0.12-alpha").is_err());
    }
    assert!(available(b"[]", "invalid").is_err());
}

#[test]
fn summary_is_three_short_plain_text_bullets() {
    let notes = format!(
        "## Changes\n- **Maps** and `routes`\n- {}\n* Faster scans\n- Fourth\n## Download\n- Install instructions",
        "星 ".repeat(100)
    );
    let summary = summarize(&notes);
    assert_eq!(summary.len(), 3);
    assert_eq!(summary[0], "Maps and routes");
    assert!(summary[1].chars().count() <= 120);
    assert!(summary[1].ends_with('…'));
    assert_eq!(summary[2], "Faster scans");
}

#[test]
fn summary_stops_before_installation_and_skips_code() {
    assert_eq!(
        summarize("```\n- Not a change\n```\n## Changes\n- One change\n## Download\n- Install it"),
        ["One change"]
    );
    assert_eq!(summarize(""), ["See the repository for release details."]);
    let update = available(
        br#"[{"tag_name":"0.0.13-alpha","body":null}]"#,
        "0.0.12-alpha",
    )
    .unwrap()
    .unwrap();
    assert_eq!(update.changes, ["See the repository for release details."]);
}
