//! Per-user URI registration and bounded file inbox. No game access.
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
pub fn data_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .with_file_name("navigator-data-rust")
}
pub fn acquire(dir: &Path) -> Result<Option<File>, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("instance.lock"))
        .map_err(|e| e.to_string())?;
    match f.try_lock() {
        Ok(()) => Ok(Some(f)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
pub fn deliver(dir: &Path, uri: &str) -> Result<(), String> {
    let canonical = crate::links::encode(crate::links::decode(uri)?)?;
    let inbox = dir.join("incoming-links");
    fs::create_dir_all(&inbox).map_err(|e| e.to_string())?;
    let name = format!(
        "{}-{}.json",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos(),
        std::process::id()
    );
    crate::storage::write_atomic(
        &inbox.join(name),
        &serde_json::to_vec(&serde_json::json!({"uri":canonical})).map_err(|e| e.to_string())?,
    )
}
pub fn poll(dir: &Path) -> Vec<(PathBuf, Result<crate::Bookmark, String>)> {
    let Ok(entries) = fs::read_dir(dir.join("incoming-links")) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|s| s == "json"))
        .take(5)
        .map(|p| {
            let result = (|| {
                if fs::metadata(&p).map_err(|e| e.to_string())?.len() > 800000 {
                    return Err("Incoming link too large".into());
                }
                let v: serde_json::Value =
                    serde_json::from_slice(&fs::read(&p).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                crate::links::decode(v["uri"].as_str().ok_or("Missing URI")?)
            })();
            (p, result)
        })
        .collect()
}
pub fn register() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let command = format!("\"{}\" \"%1\"", exe.display());
    let base = r"HKCU\Software\Classes\scbm";
    let existing = std::process::Command::new("reg.exe")
        .args(["query", &format!(r"{base}\shell\open\command"), "/ve"])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| e.to_string())?;
    if existing.status.success() {
        let old = String::from_utf8_lossy(&existing.stdout).to_lowercase();
        if !old.contains("scbm_protocol.py")
            && !old.contains("sc-bookmarks.exe")
            && !old.contains("sc bookmarks.exe")
        {
            return Err(
                "scbm is registered to another application; its handler was left unchanged".into(),
            );
        }
    }
    for (key, name, value) in [
        (base.to_owned(), None, "URL:SC Bookmarks".to_owned()),
        (base.to_owned(), Some("URL Protocol"), String::new()),
        (format!(r"{base}\shell\open\command"), None, command),
    ] {
        let mut cmd = std::process::Command::new("reg.exe");
        cmd.args(["add", &key]);
        if let Some(name) = name {
            cmd.args(["/v", name]);
        } else {
            cmd.arg("/ve");
        }
        let output = cmd
            .args(["/t", "REG_SZ", "/d", &value, "/f"])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
    }
    Ok(())
}
/// Copies compatible user documents into an empty new library; originals stay intact.
pub fn migrate(source: &Path, target: &Path) -> Result<(), String> {
    if target.join("bookmarks.json").exists() {
        return Err("Import folder is available only before this Rust library has saved bookmarks. Use link import to merge libraries.".into());
    }
    let bookmarks = crate::storage::load(&source.join("bookmarks.json"))?;
    if !source.join("bookmarks.json").is_file() {
        return Err("Choose the navigator-data folder containing bookmarks.json".into());
    }
    for b in &bookmarks {
        b.validate()?;
    }
    let organization = crate::organization::Organization::load(&source.join("organization.json"))?;
    let settings = crate::overlay::Settings::load(&source.join("overlay-settings.json"))?;
    fs::create_dir_all(target).map_err(|e| e.to_string())?;
    organization.save(&target.join("organization.json"))?;
    settings.save(&target.join("overlay-settings.json"))?;
    crate::storage::save(&target.join("bookmarks.json"), &bookmarks)
}
