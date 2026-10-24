fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cap = sc_bookmarks::capture::Session::start();
    let mut r = sc_bookmarks::recognizer::Recognizer::load(std::path::Path::new("runtime"))?;
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < 10 {
        if let Some(p) = cap.take() {
            let im = image::DynamicImage::ImageRgba8(
                image::RgbaImage::from_raw(p.size[0], p.size[1], p.rgba).unwrap(),
            )
            .to_rgb8();
            cap.stop();
            let x0 = im.width() - 420;
            let mut runs = Vec::new();
            let mut begin = None;
            let mut last = 0;
            for x in 0..260 {
                let ink = (4..16)
                    .filter(|y| {
                        let p = im.get_pixel(x0 + x, *y);
                        let lo = *p.0.iter().min().unwrap();
                        let hi = *p.0.iter().max().unwrap();
                        lo > 170 && hi - lo < 65
                    })
                    .count();
                if ink >= 1 {
                    if begin.is_none() {
                        begin = Some(x);
                    }
                    last = x;
                }
                if x - last >= 4 {
                    if let Some(b) = begin.take() {
                        runs.push((b, last + 1));
                    }
                }
            }
            println!("runs={runs:?}");
            for (a, b) in runs {
                println!(
                    "bounds {a}-{b}: {:?}",
                    (4..16)
                        .map(|y| (a..b)
                            .filter(|x| {
                                let p = im.get_pixel(x0 + *x, y);
                                p.0.iter().min().unwrap() > &170
                            })
                            .count())
                        .collect::<Vec<_>>()
                );
                let crop = image::imageops::crop_imm(&im, x0 + a, 4, b - a, 12).to_image();
                let enlarged = image::imageops::resize(
                    &crop,
                    (b - a) * 4,
                    48,
                    image::imageops::FilterType::CatmullRom,
                );
                let mut out = image::RgbImage::new((b - a) * 4 + 24, 48);
                image::imageops::replace(&mut out, &enlarged, 12, 0);
                let (t, c) = r.read(&out)?;
                println!("{a}-{b}: {c:.3} {t}");
            }
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err("No frame".into())
}
