fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    let im = image::open(&args[1])?.to_rgb8();
    if args.len() != 6 {
        return Err("Usage: shard_probe IMAGE X Y WIDTH HEIGHT".into());
    }
    let n = |i: usize| args[i].parse::<u32>().unwrap();
    let game = image::imageops::crop_imm(&im, n(2), n(3), n(4), n(5)).to_image();
    let mut r = sc_bookmarks::recognizer::Recognizer::load(std::path::Path::new("runtime"))?;
    for (y, h, w) in [
        (80, 16, 340),
        (81, 14, 300),
        (82, 12, 240),
        (80, 18, 260),
        (79, 19, 260),
        (81, 16, 260),
        (82, 14, 260),
    ] {
        let crop = image::imageops::crop_imm(&game, game.width() - w, y, w, h).to_image();
        let out =
            image::imageops::resize(&crop, w * 4, h * 4, image::imageops::FilterType::CatmullRom);
        let (s, c) = r.read(&out)?;
        println!(
            "{y} {h} {w}: {:?} accepted={:?}",
            (&s, c),
            sc_bookmarks::ocr_text::shard(&s, c)
        );
    }
    let rgba = image::DynamicImage::ImageRgb8(game.clone()).to_rgba8();
    let template = sc_bookmarks::capture::Pixels {
        desktop: sc_bookmarks::overlay::Rect {
            left: 0,
            top: 0,
            width: game.width(),
            height: game.height(),
        },
        stamp: std::time::Instant::now(),
        source_size: [game.width(), game.height()],
        origin: [0, 0],
        size: [game.width(), game.height()],
        rgba: rgba.into_raw(),
    }
    .navigation_crop();
    let worker = sc_bookmarks::ocr_worker::Worker::start("runtime".into());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut previous = None;
    let mut count = 0;
    while std::time::Instant::now() < deadline {
        worker.submit(sc_bookmarks::capture::Pixels {
            desktop: template.desktop,
            stamp: std::time::Instant::now(),
            source_size: template.source_size,
            origin: template.origin,
            size: template.size,
            rgba: template.rgba.clone(),
        });
        if let Some(o) = worker.poll() {
            if let Some(s) = o?.shard {
                println!("Worker shard: {s}");
                count = if previous.as_ref() == Some(&s) {
                    count + 1
                } else {
                    1
                };
                previous = Some(s);
                if count >= 2 {
                    break;
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(count >= 2, "Two matching shard readings required");
    Ok(())
}
