//! Offline development probe; no game access or data writes.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("Usage: recognize RUNTIME_DIRECTORY ROW_IMAGE".into());
    }
    let mut reader = sc_bookmarks::recognizer::Recognizer::load(std::path::Path::new(&args[1]))?;
    let image = image::open(&args[2])?.to_rgb8();
    let started = std::time::Instant::now();
    let (text, score) = reader.read(&image)?;
    println!(
        "{text}\nconfidence={score:.5}; elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
