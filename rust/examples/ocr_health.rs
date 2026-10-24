fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let capture = sc_bookmarks::capture::Session::start();
    let mut reader = sc_bookmarks::recognizer::Recognizer::load(std::path::Path::new("runtime"))?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while std::time::Instant::now() < deadline {
        if let Some(p) = capture.take() {
            let start = std::time::Instant::now();
            let o = sc_bookmarks::ocr_worker::read_fixed(&mut reader, &p, false)?;
            println!(
                "ms={} age={:.2} pos={:?} cam={:?} err={:?}",
                start.elapsed().as_millis(),
                o.stamp.elapsed().as_secs_f64(),
                o.position,
                o.camera,
                o.error
            );
            for (s, c) in o.evidence {
                println!("{c:.3} {s}");
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    capture.stop();
    Ok(())
}
