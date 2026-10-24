use image::imageops::{FilterType, crop_imm, resize};
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cap = sc_bookmarks::capture::Session::start();
    let mut r = sc_bookmarks::recognizer::Recognizer::load(std::path::Path::new("runtime"))?;
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < 10 {
        if let Some(p) = cap.take() {
            cap.stop();
            let im = image::DynamicImage::ImageRgba8(
                image::RgbaImage::from_raw(p.size[0], p.size[1], p.rgba).unwrap(),
            )
            .to_rgb8();
            for (name, x, y, w) in [
                ("camera", im.width() - 400, 4, 180),
                ("solar", im.width() - 400, 30, 400),
                ("root", im.width() - 400, 43, 400),
            ] {
                for h in [14, 16] {
                    for filter in [
                        FilterType::Nearest,
                        FilterType::Triangle,
                        FilterType::CatmullRom,
                    ] {
                        let crop = crop_imm(&im, x, y, w, h).to_image();
                        let out = resize(&crop, w * 4, h * 4, filter);
                        let (t, c) = r.read(&out)?;
                        println!("{name} h={h} {filter:?} {c:.3} {t}");
                    }
                }
            }
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err("No frame".into())
}
