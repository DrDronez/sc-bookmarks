#[allow(dead_code)]
#[path = "../src/live_ui.rs"]
mod live_ui;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("image path")?;
    let im = image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .to_rgba8();
    let [x, y, r, b] = sc_bookmarks::overlay::debug_box(im.width(), im.height());
    let crop = image::imageops::crop_imm(&im, x, y, r - x, b - y).to_image();
    let worker = sc_bookmarks::ocr_worker::Worker::start("runtime".into());
    let mut live = live_ui::Live::default();
    let end = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let mut last = std::time::Instant::now() - std::time::Duration::from_secs(1);
    while std::time::Instant::now() < end {
        if last.elapsed().as_millis() >= 900 {
            last = std::time::Instant::now();
            worker.submit(sc_bookmarks::capture::Pixels {
                stamp: last,
                source_size: [im.width(), im.height()],
                size: [r - x, b - y],
                origin: [x, y],
                desktop: sc_bookmarks::overlay::Rect {
                    left: 0,
                    top: 0,
                    width: im.width(),
                    height: im.height(),
                },
                rgba: crop.as_raw().clone(),
            });
        }
        if let Some(Ok(o)) = worker.poll() {
            live.ingest(&o);
            println!(
                "{} latency={:.2}s",
                live.sample_status(),
                o.stamp.elapsed().as_secs_f64()
            );
            if live.fresh() {
                let b = live.bookmark()?;
                assert_eq!(b.system, "Nyx");
                assert!(live.guidance(&b).0.contains("AT BOOKMARK"));
                println!("PASS: fresh bookmark and arrival guidance through native OCR");
                return Ok(());
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    Err("Replay never reached fresh navigation".into())
}
