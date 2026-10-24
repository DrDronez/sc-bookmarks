//! PP-OCRv3 detector with bounded CPU inference and horizontal HUD text regions.
use image::{
    RgbImage,
    imageops::{FilterType, resize},
};
use ort::{session::Session, value::Tensor};
use std::path::Path;
pub type Bounds = [u32; 4];
pub struct Detector {
    session: Session,
}
impl Detector {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let env = crate::ocr_runtime::environment(dir)?;
        let session = Session::builder(env)
            .map_err(|e| e.to_string())?
            .with_execution_providers([ort::ep::CPU::default().with_arena_allocator(false).build()])
            .map_err(|e| e.to_string())?
            .with_memory_pattern(false)
            .map_err(|e| e.to_string())?
            .with_intra_threads(1)
            .map_err(|e| e.to_string())?
            .with_inter_threads(1)
            .map_err(|e| e.to_string())?
            .with_intra_op_spinning(false)
            .map_err(|e| e.to_string())?
            .with_inter_op_spinning(false)
            .map_err(|e| e.to_string())?
            .commit_from_file(dir.join("ch_PP-OCRv3_det_infer.onnx"))
            .map_err(|e| e.to_string())?;
        Ok(Self { session })
    }
    pub fn detect(&mut self, im: &RgbImage) -> Result<Vec<Bounds>, String> {
        if im.width() == 0 || im.height() == 0 {
            return Err("Empty detector input".into());
        }
        let scale = (736. / im.height().min(im.width()) as f64)
            .max(1.)
            .min(2048. / im.width().max(im.height()) as f64);
        let w = ((im.width() as f64 * scale / 32.).round() as u32 * 32).max(32);
        let h = ((im.height() as f64 * scale / 32.).round() as u32 * 32).max(32);
        let small = resize(im, w, h, FilterType::Triangle);
        let n = w as usize * h as usize;
        let mut data = vec![0_f32; 3 * n];
        for (i, p) in small.pixels().enumerate() {
            for c in 0..3 {
                data[c * n + i] =
                    (p[c] as f32 / 255. - [0.485, 0.456, 0.406][c]) / [0.229, 0.224, 0.225][c];
            }
        }
        let tensor = Tensor::from_array(([1_usize, 3, h as usize, w as usize], data))
            .map_err(|e| e.to_string())?;
        let out = self
            .session
            .run(ort::inputs![tensor])
            .map_err(|e| e.to_string())?;
        let (shape, values) = out[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| e.to_string())?;
        if shape.len() != 4 || shape[0] != 1 || shape[1] != 1 {
            return Err("Unexpected detector output".into());
        }
        let dw = shape[3] as usize;
        let dh = shape[2] as usize;
        if dw * dh != values.len() || dw * dh > 8_000_000 {
            return Err("Detector output outside bounds".into());
        }
        Ok(regions(values, dw, dh, im.width(), im.height()))
    }
}
fn regions(values: &[f32], w: usize, h: usize, ow: u32, oh: u32) -> Vec<Bounds> {
    let mut mask = vec![0_u8; w * h];
    for y in 0..h {
        for x in 0..w {
            if values[y * w + x] > 0.3 {
                for dy in 0..=1 {
                    for dx in 0..=1 {
                        if x + dx < w && y + dy < h {
                            mask[(y + dy) * w + x + dx] = 1;
                        }
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for i in 0..mask.len() {
        if mask[i] == 0 {
            continue;
        }
        mask[i] = 0;
        stack.push(i);
        let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
        let (mut score, mut count) = (0_f64, 0_usize);
        while let Some(k) = stack.pop() {
            let x = k % w;
            let y = k / w;
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
            score += values[k] as f64;
            count += 1;
            for ny in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    let j = ny * w + nx;
                    if mask[j] != 0 {
                        mask[j] = 0;
                        stack.push(j);
                    }
                }
            }
        }
        if count < 6 || score / (count as f64) < 0.5 || x1 - x0 < 3 || y1 - y0 < 2 {
            continue;
        }
        let bw = (x1 - x0) as f64;
        let bh = (y1 - y0) as f64;
        let pad = bw * bh * 1.6 / (2. * (bw + bh));
        let b = [
            ((x0 as f64 - pad).max(0.) / w as f64 * ow as f64) as u32,
            ((y0 as f64 - pad).max(0.) / h as f64 * oh as f64) as u32,
            ((x1 as f64 + pad).min(w as f64) / w as f64 * ow as f64) as u32,
            ((y1 as f64 + pad).min(h as f64) / h as f64 * oh as f64) as u32,
        ];
        if b[2] - b[0] > 3 && b[3] - b[1] > 3 {
            out.push(b);
        }
        if out.len() >= 1000 {
            break;
        }
    }
    out.sort_by_key(|b| b[1]);
    out
}
pub fn bands(boxes: &[Bounds]) -> Vec<Bounds> {
    let mut out: Vec<Bounds> = Vec::new();
    for b in boxes {
        if let Some(g) = out.iter_mut().find(|g| {
            ((g[1] + g[3]) as f64 - (b[1] + b[3]) as f64).abs()
                < ((g[3] - g[1]).min(b[3] - b[1])) as f64 * 1.1
        }) {
            g[0] = g[0].min(b[0]);
            g[1] = g[1].min(b[1]);
            g[2] = g[2].max(b[2]);
            g[3] = g[3].max(b[3]);
        } else {
            out.push(*b);
        }
    }
    out.sort_by_key(|b| b[1]);
    out
}
pub fn read(
    reader: &mut crate::recognizer::Recognizer,
    im: &RgbImage,
    b: Bounds,
) -> Result<(String, f64), String> {
    let [x, y, r, bottom] = b;
    if r <= x || bottom <= y || r > im.width() || bottom > im.height() {
        return Err("Text bounds outside image".into());
    }
    let crop = image::imageops::crop_imm(im, x, y, r - x, bottom - y).to_image();
    let f = (48. / (bottom - y) as f64).min(4.);
    let crop = resize(
        &crop,
        ((r - x) as f64 * f).max(1.) as u32,
        ((bottom - y) as f64 * f).max(1.) as u32,
        FilterType::CatmullRom,
    );
    reader.read(&crop).map_err(|e| e.to_string())
}
