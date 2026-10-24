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
    let found = sc_bookmarks::scan_ocr::contacts(&mut d, &mut r, &im, &[], true)?;
    println!("{found:?}\nelapsed {:?}", start.elapsed());
    let t = std::time::Instant::now();
    println!(
        "Quick {:?}",
        sc_bookmarks::scan_ocr::contacts(&mut d, &mut r, &im, &found, false)?
    );
    println!("Quick elapsed {:?}", t.elapsed());
    Ok(())
}
