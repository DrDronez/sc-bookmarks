use crate::{capture::Pixels, ocr_text, recognizer::Recognizer};
use image::{
    RgbImage,
    imageops::{FilterType, crop_imm, resize},
};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
    time::Instant,
};
pub struct Observation {
    pub stamp: Instant,
    pub position: Option<[f64; 3]>,
    pub camera: Option<[f64; 3]>,
    pub shard: Option<String>,
    pub system: Option<String>,
    pub error: Option<String>,
    pub evidence: Vec<(String, f64)>,
    pub desktop: crate::overlay::Rect,
}
pub struct Worker {
    input: Option<SyncSender<Pixels>>,
    output: Receiver<Result<Observation, String>>,
}
impl Worker {
    pub fn start(runtime: PathBuf) -> Self {
        let (input, rx) = mpsc::sync_channel::<Pixels>(0);
        let (tx, output) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let mut reader = match Recognizer::load(&runtime) {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                    return;
                }
            };
            let mut metadata_at = None;
            let mut shard_at: Option<Instant> = None;
            let mut rotation: Option<Instant> = None;
            let mut completed_slot = 0_u64;
            let mut slot_attempts = 0;
            let mut position_at: Option<Instant> = None;
            let mut previous_shard = None;
            let mut metadata_row = 0;
            let mut shard_row = 0;
            let mut auxiliary_system = true;
            let mut coordinate_layout = CoordinateLayout::default();
            while let Ok(pixels) = rx.recv() {
                if pixels.stamp.elapsed().as_secs_f64() > 8. {
                    continue;
                }
                let slot = rotation.map(|t| t.elapsed().as_secs() / 60).unwrap_or(0);
                let scheduled = rotation.is_some() && slot > completed_slot;
                let read_metadata = if rotation.is_none() {
                    metadata_at.is_none()
                } else {
                    scheduled && slot % 2 == 0
                };
                let read_shard = if rotation.is_none() {
                    shard_at.is_none()
                } else {
                    scheduled && slot % 2 == 1
                };
                let read_position = position_at.is_none_or(|t| t.elapsed().as_millis() >= 900);
                if read_position {
                    position_at = Some(Instant::now());
                }
                coordinate_layout.set_size(pixels.size);
                let mut result = read_options_inner(
                    &mut reader,
                    &pixels,
                    false,
                    false,
                    read_position && !coordinate_layout.cached(),
                );
                if read_position {
                    if let Ok(o) = &mut result {
                        if o.position.is_none() {
                            let rgba = image::RgbaImage::from_raw(
                                pixels.size[0],
                                pixels.size[1],
                                pixels.rgba.clone(),
                            );
                            if let Some(rgba) = rgba {
                                let im = image::DynamicImage::ImageRgba8(rgba).to_rgb8();
                                match coordinate_layout.read(&mut reader, &runtime, &im) {
                                    Ok((p, evidence)) => {
                                        o.position = Some(p);
                                        o.error = None;
                                        o.evidence.extend(evidence);
                                    }
                                    Err(e) => {
                                        o.error = Some(format!("Coordinates: {e}"));
                                    }
                                }
                            }
                        }
                    }
                }
                if let Ok(o) = &mut result {
                    if o.camera.is_none() && coordinate_layout.camera_bounds.is_some() {
                        if let Some(rgba) = image::RgbaImage::from_raw(
                            pixels.size[0],
                            pixels.size[1],
                            pixels.rgba.clone(),
                        ) {
                            o.camera = coordinate_layout.camera(
                                &mut reader,
                                &image::DynamicImage::ImageRgba8(rgba).to_rgb8(),
                            );
                        }
                    }
                }
                // At most one auxiliary row per sample. Never put a 10-row
                // metadata search in front of publishing fresh navigation.
                if read_position && (read_metadata || read_shard) {
                    if let (Ok(o), Some(rgba)) = (
                        &mut result,
                        image::RgbaImage::from_raw(
                            pixels.size[0],
                            pixels.size[1],
                            pixels.rgba.clone(),
                        ),
                    ) {
                        let im = image::DynamicImage::ImageRgba8(rgba).to_rgb8();
                        let width = im.width();
                        if read_metadata && (!read_shard || auxiliary_system) {
                            auxiliary_system = false;
                            let ys = [236, 223, 249, 210, 262, 197, 275, 184, 288, 171];
                            if let Ok((text, score)) = row(
                                &mut reader,
                                &im,
                                width.saturating_sub(400),
                                ys[metadata_row],
                                width.min(400),
                                12,
                            ) {
                                o.system = ocr_text::system(&text, score).map(str::to_owned);
                                o.evidence.push((text, score));
                            }
                            if o.system.is_some() {
                                metadata_at = Some(Instant::now());
                            } else {
                                metadata_row = (metadata_row + 1) % ys.len();
                            }
                        } else {
                            auxiliary_system = true;
                            if let Ok((text, score)) = row(
                                &mut reader,
                                &im,
                                width.saturating_sub(340),
                                [68, 81, 94][shard_row],
                                width.min(340),
                                15,
                            ) {
                                o.shard = ocr_text::shard(&text, score);
                                o.evidence.push((text, score));
                            }
                            if o.shard.is_some() && o.shard == previous_shard {
                                shard_at = Some(Instant::now());
                            } else {
                                shard_row = (shard_row + 1) % 3;
                            }
                        }
                    }
                }
                if read_position {
                    if let Ok(o) = &result {
                        let done = (read_metadata && o.system.is_some())
                            || (read_shard && o.shard.is_some() && o.shard == previous_shard);
                        if o.shard.is_some() {
                            previous_shard = o.shard.clone();
                        }
                        if scheduled {
                            slot_attempts += 1;
                            if done || slot_attempts >= 12 {
                                completed_slot = slot;
                                slot_attempts = 0;
                            }
                        }
                    }
                    if rotation.is_none() && metadata_at.is_some() && shard_at.is_some() {
                        rotation = Some(Instant::now());
                    }
                }
                if tx.send(result).is_err() {
                    break;
                }
            }
        });
        Self {
            input: Some(input),
            output,
        }
    }
    pub fn submit(&self, pixels: Pixels) {
        if let Some(tx) = &self.input {
            let _ = tx.try_send(pixels);
        }
    }
    pub fn poll(&self) -> Option<Result<Observation, String>> {
        self.output.try_recv().ok()
    }
}
fn row(
    reader: &mut Recognizer,
    image: &RgbImage,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> Result<(String, f64), String> {
    if x + w > image.width() || y + h > image.height() || w == 0 || h == 0 {
        return Err("Debug row outside captured image".into());
    }
    let crop = crop_imm(image, x, y, w, h).to_image();
    let scale = if w <= 400 || h == 15 { 4 } else { 3 };
    let enlarged = resize(&crop, w * scale, h * scale, FilterType::CatmullRom);
    reader.read(&enlarged).map_err(|e| e.to_string())
}
fn strengthen_minus(crop: &mut RgbImage) {
    let w = crop.width();
    // The HUD minus is only 2 bright pixels wide. Strengthen isolated midline
    // strokes before recognition; do not reinterpret punctuation in OCR text.
    let mut runs = Vec::new();
    let mut begin = None;
    let mut last = 0;
    for x in 0..w {
        let ink = (0..12).any(|y| {
            let p = crop.get_pixel(x, y);
            let lo = *p.0.iter().min().unwrap();
            let hi = *p.0.iter().max().unwrap();
            lo > 170 && hi - lo < 65
        });
        if ink {
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
    for pair in runs.windows(2) {
        let (a, b) = pair[0];
        let (next, end) = pair[1];
        if a < 70 || b - a < 2 || b - a > 5 || next - b > 8 || end - next < 4 {
            continue;
        }
        let ys: Vec<_> = (0..12)
            .filter(|y| {
                (a..b).any(|x| {
                    let p = crop.get_pixel(x, *y);
                    *p.0.iter().min().unwrap() > 170
                })
            })
            .collect();
        if !ys.is_empty() && ys.len() <= 2 && ys.iter().all(|y| (5..=8).contains(y)) {
            for y in ys {
                for x in a.saturating_sub(2)..(b + 2).min(w) {
                    crop.put_pixel(x, y, image::Rgb([255; 3]));
                }
            }
        }
    }
}
fn camera_row(reader: &mut Recognizer, image: &RgbImage) -> Result<(String, f64), String> {
    let w = image.width().min(180);
    let x = image.width().saturating_sub(400);
    if image.height() < 18 {
        return Err("Camera row outside capture".into());
    }
    let crop = crop_imm(image, x, 4, w, 14).to_image();
    let enlarged = resize(&crop, w * 4, 56, FilterType::CatmullRom);
    reader.read(&enlarged).map_err(|e| e.to_string())
}
fn camera_fallback(reader: &mut Recognizer, image: &RgbImage) -> Result<(String, f64), String> {
    if image.height() < 16 {
        return Err("Camera row outside capture".into());
    }
    let w = image.width().min(260);
    let mut crop = crop_imm(image, image.width().saturating_sub(420), 4, w, 12).to_image();
    strengthen_minus(&mut crop);
    reader
        .read(&resize(&crop, w * 4, 48, FilterType::CatmullRom))
        .map_err(|e| e.to_string())
}
pub fn read_fixed(
    reader: &mut Recognizer,
    pixels: &Pixels,
    metadata: bool,
) -> Result<Observation, String> {
    read_options(reader, pixels, metadata, metadata)
}
fn read_options(
    reader: &mut Recognizer,
    pixels: &Pixels,
    metadata: bool,
    read_shard: bool,
) -> Result<Observation, String> {
    read_options_inner(reader, pixels, metadata, read_shard, true)
}
fn read_options_inner(
    reader: &mut Recognizer,
    pixels: &Pixels,
    metadata: bool,
    read_shard: bool,
    read_position: bool,
) -> Result<Observation, String> {
    let rgba = image::RgbaImage::from_raw(pixels.size[0], pixels.size[1], pixels.rgba.clone())
        .ok_or("Invalid capture pixels")?;
    let image = image::DynamicImage::ImageRgba8(rgba).to_rgb8();
    let width = image.width();
    let mut result = Observation {
        stamp: pixels.stamp,
        position: None,
        camera: None,
        shard: None,
        system: None,
        error: None,
        evidence: Vec::new(),
        desktop: pixels.desktop,
    };
    let camera = camera_row(reader, &image)
        .ok()
        .filter(|(s, c)| ocr_text::camera(s, *c).is_some())
        .or_else(|| camera_fallback(reader, &image).ok());
    let expanded = camera.as_ref().is_some_and(|(s, _)| {
        (s.to_ascii_lowercase().contains("cam") || s.to_ascii_lowercase().contains("can"))
            && s.to_ascii_lowercase().contains("ir")
    });
    if let Some((text, score)) = camera {
        result.camera = ocr_text::camera(&text, score);
        result.evidence.push((text, score));
    }
    if !read_position {
        return Ok(result);
    }
    let starts = if expanded { [30, 43] } else { [17, 30] };
    let solar = row(
        reader,
        &image,
        width.saturating_sub(400),
        starts[0],
        width.min(400),
        14,
    )
    .ok();
    let root = row(
        reader,
        &image,
        width.saturating_sub(400),
        starts[1],
        width.min(400),
        14,
    )
    .ok();
    let mut solar = solar.into_iter().collect::<Vec<_>>();
    let mut root = root.into_iter().collect::<Vec<_>>();
    let mut position = ocr_text::coordinate_consensus(&solar, &root);
    if position.is_err() {
        // One bounded second pass using the wider original row crop. Never accept a guessed digit or missing sign.
        for (rows, y, w) in [(&mut solar, starts[0], 700), (&mut root, starts[1], 500)] {
            if let Ok(v) = row(reader, &image, width.saturating_sub(w), y, width.min(w), 12) {
                rows.push(v);
            }
        }
        position = ocr_text::coordinate_consensus(&solar, &root);
    }
    match position {
        Ok(p) => result.position = Some(p),
        Err(e) => result.error = Some(e.into()),
    }
    result.evidence.extend(solar);
    result.evidence.extend(root);
    if read_shard {
        for y in [68, 81, 94] {
            if let Ok((text, score)) = row(
                reader,
                &image,
                width.saturating_sub(340),
                y,
                width.min(340),
                15,
            ) {
                if result.shard.is_none() {
                    result.shard = ocr_text::shard(&text, score);
                }
                result.evidence.push((text, score));
                if result.shard.is_some() {
                    break;
                }
            }
        }
    }
    if metadata {
        for y in (171..289).step_by(13) {
            if let Ok((text, score)) = row(
                reader,
                &image,
                width.saturating_sub(400),
                y,
                width.min(400),
                12,
            ) {
                if result.system.is_none() {
                    result.system = ocr_text::system(&text, score).map(str::to_owned);
                }
                result.evidence.push((text, score));
                if result.system.is_some() {
                    break;
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strengthens_midline_sign_but_not_decimal_or_digit() {
        let mut im = RgbImage::new(200, 12);
        for x in 87..89 {
            im.put_pixel(x, 7, image::Rgb([255; 3]));
        }
        for y in 3..11 {
            im.put_pixel(94, y, image::Rgb([255; 3]));
            im.put_pixel(98, y, image::Rgb([255; 3]));
        }
        // Baseline dot is not a minus.
        for x in 120..122 {
            im.put_pixel(x, 10, image::Rgb([255; 3]));
        }
        for y in 3..11 {
            im.put_pixel(127, y, image::Rgb([255; 3]));
            im.put_pixel(131, y, image::Rgb([255; 3]));
        }
        strengthen_minus(&mut im);
        assert_eq!(im.get_pixel(85, 7).0, [255; 3]);
        assert_eq!(im.get_pixel(118, 10).0, [0; 3]);
        assert_eq!(im.get_pixel(93, 3).0, [0; 3]);
    }
}

/// Coordinates recover independently of CamDir. Discovery is bounded to the
/// upper-right debug rows, and successful bounds are reused without detection.
#[derive(Default)]
pub struct CoordinateLayout {
    bounds: Option<[crate::detector::Bounds; 2]>,
    camera_bounds: Option<crate::detector::Bounds>,
    size: [u32; 2],
    detector: Option<crate::detector::Detector>,
    last_discovery: Option<Instant>,
    failures: u32,
}
fn coordinate_label(text: &str) -> Option<usize> {
    let label = text
        .split(':')
        .take(2)
        .collect::<Vec<_>>()
        .join(":")
        .to_lowercase()
        .replace('0', "o")
        .replace('5', "s");
    if label.contains("zone") && label.contains("solarsystem") {
        Some(0)
    } else if label.contains("zone") && label.contains("root") {
        Some(1)
    } else {
        None
    }
}
impl CoordinateLayout {
    pub fn set_size(&mut self, size: [u32; 2]) {
        if self.size != size {
            self.bounds = None;
            self.camera_bounds = None;
            self.last_discovery = None;
            self.failures = 0;
            self.size = size;
        }
    }
    pub fn camera(&self, reader: &mut Recognizer, im: &RgbImage) -> Option<[f64; 3]> {
        let (s, c) = crate::detector::read(reader, im, self.camera_bounds?).ok()?;
        ocr_text::camera(&s, c)
    }
    pub fn cached(&self) -> bool {
        self.bounds.is_some()
    }
    fn cached_read(
        &self,
        reader: &mut Recognizer,
        im: &RgbImage,
    ) -> Result<([f64; 3], Vec<(String, f64)>), String> {
        let b = self.bounds.ok_or("Layout not located")?;
        let a = crate::detector::read(reader, im, b[0])?;
        let c = crate::detector::read(reader, im, b[1])?;
        if coordinate_label(&a.0) != Some(0) || coordinate_label(&c.0) != Some(1) {
            return Err("Coordinate row labels moved or unreadable".into());
        }
        let p = ocr_text::coordinates(Some((&a.0, a.1)), Some((&c.0, c.1)), true)
            .map_err(str::to_owned)?;
        Ok((p, vec![a, c]))
    }
    pub fn read(
        &mut self,
        reader: &mut Recognizer,
        runtime: &std::path::Path,
        im: &RgbImage,
    ) -> Result<([f64; 3], Vec<(String, f64)>), String> {
        self.set_size([im.width(), im.height()]);
        if self.bounds.is_some() {
            match self.cached_read(reader, im) {
                Ok(v) => {
                    self.failures = 0;
                    return Ok(v);
                }
                Err(e) => {
                    self.failures += 1;
                    if self.failures < 3 {
                        return Err(e);
                    }
                }
            }
        }
        if self
            .last_discovery
            .is_some_and(|t| t.elapsed().as_secs() < 15)
        {
            return Err("Waiting for bounded layout retry; no confirmed coordinates".into());
        }
        self.last_discovery = Some(Instant::now());
        if self.detector.is_none() {
            self.detector = Some(crate::detector::Detector::load(runtime)?);
        }
        let w = im.width().min(1200);
        let h = im.height().min(180);
        let x = im.width() - w;
        let search = crop_imm(im, x, 0, w, h).to_image();
        let boxes = self.detector.as_mut().unwrap().detect(&search)?;
        let mut found = [None, None];
        let mut reads = [None, None];
        for b in crate::detector::bands(&boxes).into_iter().take(16) {
            let b = [b[0] + x, b[1], b[2] + x, b[3]];
            if let Ok(v) = crate::detector::read(reader, im, b) {
                if ocr_text::camera(&v.0, v.1).is_some() {
                    self.camera_bounds = Some(b);
                }
                if v.1 >= 0.82 {
                    if let Some(i) = coordinate_label(&v.0) {
                        found[i] = Some(b);
                        reads[i] = Some(v);
                    }
                }
            }
            if found.iter().all(Option::is_some) {
                break;
            }
        }
        let row = |i: usize| reads[i].as_ref().map(|(s, c)| (s.as_str(), *c));
        let p = ocr_text::coordinates(row(0), row(1), true).map_err(str::to_owned)?;
        self.bounds = Some([found[0].unwrap(), found[1].unwrap()]);
        self.failures = 0;
        Ok((p, reads.into_iter().flatten().collect()))
    }
}
#[cfg(test)]
mod layout_tests {
    use super::*;
    #[test]
    fn labels_require_system_rows_and_size_changes_clear_cache() {
        assert_eq!(coordinate_label("Zone: 5olar5ystem_123 Pos: 1"), Some(0));
        assert_eq!(coordinate_label("Zone: R00t Pos: 1"), Some(1));
        assert_eq!(coordinate_label("Zone: Ship Pos: 1"), None);
        let mut l = CoordinateLayout::default();
        l.size = [100, 100];
        l.bounds = Some([[0, 0, 10, 10], [0, 12, 10, 22]]);
        l.set_size([100, 100]);
        assert!(l.cached());
        l.set_size([200, 100]);
        assert!(!l.cached());
    }
}
