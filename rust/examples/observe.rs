use sc_bookmarks::{
    capture::Pixels,
    ocr_worker::read_fixed,
    overlay::{Rect, debug_box},
    recognizer::Recognizer,
};
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 3 {
        return Err("Usage: observe RUNTIME_DIRECTORY GAME_SCREENSHOT".into());
    }
    let im = image::ImageReader::open(&a[2])?
        .with_guessed_format()?
        .decode()?
        .to_rgba8();
    let [x, y, r, b] = debug_box(im.width(), im.height());
    let crop = image::imageops::crop_imm(&im, x, y, r - x, b - y).to_image();
    let pixels = Pixels {
        desktop: Rect {
            left: 0,
            top: 0,
            width: im.width(),
            height: im.height(),
        },
        stamp: std::time::Instant::now(),
        source_size: [im.width(), im.height()],
        origin: [x, y],
        size: [r - x, b - y],
        rgba: crop.into_raw(),
    };
    let mut reader = Recognizer::load(std::path::Path::new(&a[1]))?;
    let start = std::time::Instant::now();
    let o = read_fixed(&mut reader, &pixels, true)?;
    println!(
        "Position {:?}\nCamera {:?}\nSystem {:?}\nShard {:?}\nError {:?}\nElapsed {}ms",
        o.position,
        o.camera,
        o.system,
        o.shard,
        o.error,
        start.elapsed().as_millis()
    );
    for (t, c) in o.evidence {
        println!("{c:.3}: {t}");
    }
    Ok(())
}
