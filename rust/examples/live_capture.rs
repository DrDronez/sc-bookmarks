fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let runtime = std::env::args().nth(1).ok_or("runtime path")?;
    let capture = sc_bookmarks::capture::Session::start();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(12);
    let mut reader = sc_bookmarks::recognizer::Recognizer::load(std::path::Path::new(&runtime))?;
    while std::time::Instant::now() < deadline {
        if let Some(p) = capture.take() {
            let o = sc_bookmarks::ocr_worker::read_fixed(&mut reader, &p, true)?;
            println!(
                "position={:?} cam={:?} system={:?} shard={:?}",
                o.position, o.camera, o.system, o.shard
            );
            for (t, c) in o.evidence {
                if t.contains("location") || t.contains("Shard") {
                    println!("{c:.3} {t}");
                }
            }
            capture.stop();
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Err(format!("No frame: {:?}", capture.status()).into())
}
