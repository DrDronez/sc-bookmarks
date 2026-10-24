use sc_bookmarks::{capture::Pixels, overlay::Rect};
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 6 {
        return Err("Usage: layout_probe IMAGE X Y WIDTH HEIGHT".into());
    }
    let n = |i: usize| args[i].parse::<u32>().unwrap();
    let im = image::open(&args[1])?.to_rgba8();
    let (x, y, w, h) = (n(2), n(3), n(4), n(5));
    let game = image::imageops::crop_imm(&im, x, y, w, h).to_image();
    let p = Pixels {
        desktop: Rect {
            left: x as i32,
            top: y as i32,
            width: w,
            height: h,
        },
        stamp: std::time::Instant::now(),
        source_size: [w, h],
        origin: [0, 0],
        size: [w, h],
        rgba: game.into_raw(),
    }
    .navigation_crop();
    let im = image::DynamicImage::ImageRgba8(
        image::RgbaImage::from_raw(p.size[0], p.size[1], p.rgba.clone()).unwrap(),
    )
    .to_rgb8();
    let mut r = sc_bookmarks::recognizer::Recognizer::load(std::path::Path::new("runtime"))?;
    for variant in [0, 1, 2] {
        let mut view = im.clone();
        if variant == 1 {
            // Preserve readable CamDir; insert an extra zone row before coordinate rows.
            let lower =
                image::imageops::crop_imm(&im, 0, 18, im.width(), im.height() - 31).to_image();
            image::imageops::replace(&mut view, &lower, 0, 31);
        }
        if variant == 2 {
            view = image::imageops::resize(
                &view,
                view.width() * 3 / 2,
                view.height() * 3 / 2,
                image::imageops::FilterType::CatmullRom,
            );
        }
        let rgba = image::DynamicImage::ImageRgb8(view.clone()).to_rgba8();
        let pixels = Pixels {
            desktop: p.desktop,
            stamp: std::time::Instant::now(),
            source_size: p.source_size,
            origin: p.origin,
            size: [view.width(), view.height()],
            rgba: rgba.into_raw(),
        };
        let fixed = sc_bookmarks::ocr_worker::read_fixed(&mut r, &pixels, false)?;
        println!(
            "variant={variant} fixed={:?} camera={:?}",
            fixed.position, fixed.camera
        );
        let mut layout = sc_bookmarks::ocr_worker::CoordinateLayout::default();
        for pass in 0..2 {
            let t = std::time::Instant::now();
            let (pos, _) = layout.read(&mut r, std::path::Path::new("runtime"), &view)?;
            println!(
                "variant={variant} pass={pass} pos={pos:?} ms={}",
                t.elapsed().as_millis()
            );
        }
        assert!(layout_camera_check(&mut r, &view)?);
        let worker = sc_bookmarks::ocr_worker::Worker::start("runtime".into());
        let mut pixels = pixels;
        pixels.stamp = std::time::Instant::now();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        loop {
            worker.submit(Pixels {
                desktop: pixels.desktop,
                stamp: std::time::Instant::now(),
                source_size: pixels.source_size,
                origin: pixels.origin,
                size: pixels.size,
                rgba: pixels.rgba.clone(),
            });
            if let Some(o) = worker.poll() {
                let o = o?;
                println!(
                    "WORKER variant={variant} pos={:?} cam={:?}",
                    o.position, o.camera
                );
                assert!(o.position.is_some() && o.camera.is_some());
                break;
            }
            if std::time::Instant::now() > deadline {
                return Err("Worker replay timed out".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }
    Ok(())
}

fn layout_camera_check(
    r: &mut sc_bookmarks::recognizer::Recognizer,
    view: &image::RgbImage,
) -> Result<bool, String> {
    let mut layout = sc_bookmarks::ocr_worker::CoordinateLayout::default();
    layout.read(r, std::path::Path::new("runtime"), view)?;
    Ok(layout.camera(r, view).is_some())
}
