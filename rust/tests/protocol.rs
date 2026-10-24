use sc_bookmarks::{Bookmark, organization::Organization, protocol, storage};
#[test]
fn inbox_instance_and_migration_preserve_source() {
    let root = std::env::temp_dir().join(format!(
        "scbm-parity-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let source = root.join("old");
    let target = root.join("new");
    std::fs::create_dir_all(&source).unwrap();
    let b = Bookmark {
        name: "Unicode α".into(),
        system: "Nyx".into(),
        xyz_km: [1., -2., 3.],
        metadata: std::collections::BTreeMap::from([(
            "future_field".into(),
            serde_json::json!([1, 2, 3]),
        )]),
    };
    storage::save(&source.join("bookmarks.json"), &[b.clone()]).unwrap();
    let before = std::fs::read(source.join("bookmarks.json")).unwrap();
    let mut org = Organization::default();
    org.categories = vec!["Mining".into(), "Fleet".into()];
    org.save(&source.join("organization.json")).unwrap();
    protocol::migrate(&source, &target).unwrap();
    assert_eq!(
        std::fs::read(source.join("bookmarks.json")).unwrap(),
        before
    );
    assert_eq!(
        storage::load(&target.join("bookmarks.json")).unwrap()[0].metadata,
        b.metadata
    );
    assert!(protocol::migrate(&source, &target).is_err());
    let lock = protocol::acquire(&target).unwrap().unwrap();
    assert!(protocol::acquire(&target).unwrap().is_none());
    protocol::deliver(&target, &sc_bookmarks::links::encode(b).unwrap()).unwrap();
    let inbox = protocol::poll(&target);
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].1.as_ref().unwrap().name, "Unicode α");
    drop(lock);
    assert!(protocol::acquire(&target).unwrap().is_some());
    std::fs::remove_dir_all(root).unwrap();
}
