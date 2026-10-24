use sc_bookmarks::{Bookmark, storage};
use std::fs;
#[test]
fn storage_preserves_previous_version_and_rejects_corruption() {
    let dir = std::env::temp_dir().join(format!("scbm-store-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bookmarks.json");
    let mut b: Bookmark = serde_json::from_str(
        r#"{"name":"Original","system":"Nyx","xyz_km":[1,2,3],"extra":{"keep":true}}"#,
    )
    .unwrap();
    storage::save(&path, &[b.clone()]).unwrap();
    b.name = "Renamed".into();
    storage::save(&path, &[b]).unwrap();
    assert_eq!(storage::load(&path).unwrap()[0].name, "Renamed");
    assert_eq!(
        storage::load(&path.with_extension("json.bak")).unwrap()[0].name,
        "Original"
    );
    assert_eq!(
        storage::load(&path).unwrap()[0].metadata["extra"]["keep"],
        true
    );
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&tmp, b"another writer").unwrap();
    assert!(storage::save(&path, &[]).is_err());
    assert_eq!(fs::read(&tmp).unwrap(), b"another writer");
    assert_eq!(storage::load(&path).unwrap()[0].name, "Renamed");
    fs::remove_file(tmp).unwrap();
    fs::write(&path, b"{broken").unwrap();
    assert!(storage::load(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"{broken");
    fs::remove_file(&path).unwrap();
    fs::remove_file(path.with_extension("json.bak")).unwrap();
    fs::remove_dir(dir).unwrap();
}
