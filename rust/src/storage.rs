use crate::Bookmark;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};
/// Loading never writes: malformed libraries must not be replaced with an empty list.
pub fn load(path: &Path) -> Result<Vec<Bookmark>, String> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(e.to_string()),
    }
}
pub fn save(path: &Path, bookmarks: &[Bookmark]) -> Result<(), String> {
    for b in bookmarks {
        b.validate().map_err(str::to_owned)?;
    }
    let bytes = serde_json::to_vec_pretty(bookmarks).map_err(|e| e.to_string())?;
    write_atomic(path, &bytes)
}
/// Write a complete JSON document without losing the previous version on failure.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing data directory")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    let mut created = false;
    let result = (|| -> io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        created = true;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        if path.exists() {
            fs::copy(path, path.with_extension("json.bak"))?;
        }
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    if created && result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.map_err(|e| e.to_string())
}
