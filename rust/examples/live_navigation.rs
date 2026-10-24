#[path = "../src/live_ui.rs"]
mod live_ui;
fn main() {
    let capture = sc_bookmarks::capture::Session::start();
    let worker = sc_bookmarks::ocr_worker::Worker::start("runtime".into());
    let mut live = live_ui::Live::default();
    let end = std::time::Instant::now() + std::time::Duration::from_secs(25);
    while std::time::Instant::now() < end {
        if let Some(p) = capture.take() {
            worker.submit(p);
        }
        if let Some(Ok(o)) = worker.poll() {
            live.ingest(&o);
            println!(
                "{} p={:?} system={} age={:.1} camera={:?} error={:?}",
                live.sample_status(),
                o.position,
                live.system,
                o.stamp.elapsed().as_secs_f64(),
                o.camera,
                o.error
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    capture.stop();
    assert!(live.ship().is_some(), "No filtered position");
}
