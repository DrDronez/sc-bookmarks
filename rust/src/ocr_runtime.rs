//! Exactly one ORT environment per process, shared across bounded OCR workers.
use std::{path::Path, sync::OnceLock};
static ENV: OnceLock<Result<ort::environment::Environment, String>> = OnceLock::new();
pub fn environment(dir: &Path) -> Result<&'static ort::environment::Environment, String> {
    ENV.get_or_init(|| {
        ort::init_from(dir.join("onnxruntime.dll"))
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| e.to_string())
    })
    .as_ref()
    .map_err(Clone::clone)
}
