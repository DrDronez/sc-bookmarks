//! Append-only local coordinate history, never game files.
use std::{fs::OpenOptions, path::Path};
pub fn append(path: &Path, p: [f64; 3]) -> Result<(), String> {
    let exists = path.exists();
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let mut w = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(file);
    if !exists {
        w.write_record(["utc", "monotonic_seconds", "x_km", "y_km", "z_km"])
            .map_err(|e| e.to_string())?;
    }
    static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    w.write_record([
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        EPOCH
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_secs_f64()
            .to_string(),
        p[0].to_string(),
        p[1].to_string(),
        p[2].to_string(),
    ])
    .map_err(|e| e.to_string())?;
    w.flush().map_err(|e| e.to_string())
}
