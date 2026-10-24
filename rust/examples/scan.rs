fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let a: Vec<_> = std::env::args().collect();
    let dir = std::path::Path::new(&a[1]);
    let im = image::ImageReader::open(&a[2])?
        .with_guessed_format()?
        .decode()?
        .to_rgb8();
    let mut d = sc_bookmarks::detector::Detector::load(dir)?;
    let mut r = sc_bookmarks::recognizer::Recognizer::load(dir)?;
    let start = std::time::Instant::now();
    match sc_bookmarks::scan_ocr::asteroid(&mut d, &mut r, &im) {
        Ok(v) => {
            println!("{v:?}");
            for t in sc_bookmarks::scan_ocr::texts(&mut d, &mut r, &im)?
                .into_iter()
                .filter(|t| t.bounds[0] > 1300 && t.bounds[1] > 590 && t.bounds[1] < 720)
            {
                println!("{t:?}");
            }
        }
        Err(e) => {
            println!("{e}");
            for t in sc_bookmarks::scan_ocr::texts(&mut d, &mut r, &im)? {
                println!("{:?} {:.2}: {}", t.bounds, t.score, t.text);
            }
        }
    }
    println!("elapsed {:?}", start.elapsed());
    Ok(())
}
