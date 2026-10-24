//! CPU-only PP-OCRv3 recognition using the same model and dictionary as Python.
//! Preprocessing/CTC conventions derived from RapidOCR/PaddleOCR (Apache-2.0).
use image::RgbImage;
use ort::{session::Session, value::Tensor};
use std::path::Path;
type Error = Box<dyn std::error::Error + Send + Sync>;
pub struct Recognizer {
    session: Session,
    characters: Vec<String>,
}
impl Recognizer {
    pub fn load(directory: &Path) -> Result<Self, Error> {
        let env = crate::ocr_runtime::environment(directory)?;
        let session = Session::builder(env)?
            .with_execution_providers([ort::ep::CPU::default().with_arena_allocator(true).build()])
            .map_err(|e| e.to_string())?
            .with_memory_pattern(false)
            .map_err(|e| e.to_string())?
            .with_intra_threads(1)
            .map_err(|e| e.to_string())?
            .with_inter_threads(1)
            .map_err(|e| e.to_string())?
            .with_parallel_execution(false)
            .map_err(|e| e.to_string())?
            .with_intra_op_spinning(false)
            .map_err(|e| e.to_string())?
            .with_inter_op_spinning(false)
            .map_err(|e| e.to_string())?
            .commit_from_file(directory.join("ch_PP-OCRv3_rec_infer.onnx"))?;
        let dictionary = session
            .metadata()?
            .custom("character")
            .ok_or("Recognition model has no character dictionary")?;
        let mut characters = vec![String::new()];
        characters.extend(dictionary.lines().map(str::to_owned));
        characters.push(" ".into());
        Ok(Self {
            session,
            characters,
        })
    }
    pub fn read(&mut self, image: &RgbImage) -> Result<(String, f64), Error> {
        let (width, data) = normalize(image)?;
        let tensor = Tensor::from_array(([1_usize, 3, 48, width], data))?;
        let outputs = self.session.run(ort::inputs![tensor])?;
        let (shape, values) = outputs[0].try_extract_tensor::<f32>()?;
        if shape.len() != 3 || shape[0] != 1 || shape[2] as usize != self.characters.len() {
            return Err("Unexpected recognition output shape".into());
        }
        decode(values, shape[1] as usize, &self.characters)
    }
}
pub fn normalize(image: &RgbImage) -> Result<(usize, Vec<f32>), Error> {
    if image.width() == 0 || image.height() == 0 {
        return Err("Empty OCR image".into());
    }
    let width = (48. * image.width() as f64 / image.height() as f64).floor() as usize;
    if width == 0 || width > 8192 {
        return Err("OCR text row exceeds bounded width".into());
    }
    // OpenCV's half-pixel bilinear sampling, interleaved RGB to normalized CHW.
    let mut values = vec![0.; 3 * 48 * width];
    for y in 0..48 {
        let fy = ((y as f64 + 0.5) * image.height() as f64 / 48. - 0.5).max(0.);
        let y0 = (fy.floor() as u32).min(image.height() - 1);
        let y1 = (y0 + 1).min(image.height() - 1);
        let wy = fy - y0 as f64;
        for x in 0..width {
            let fx = ((x as f64 + 0.5) * image.width() as f64 / width as f64 - 0.5).max(0.);
            let x0 = (fx.floor() as u32).min(image.width() - 1);
            let x1 = (x0 + 1).min(image.width() - 1);
            let wx = fx - x0 as f64;
            for c in 0..3 {
                let a = image.get_pixel(x0, y0)[c] as f64 * (1. - wx)
                    + image.get_pixel(x1, y0)[c] as f64 * wx;
                let b = image.get_pixel(x0, y1)[c] as f64 * (1. - wx)
                    + image.get_pixel(x1, y1)[c] as f64 * wx;
                values[c * 48 * width + y * width + x] =
                    ((a * (1. - wy) + b * wy).round() as f32 / 255. - 0.5) / 0.5;
            }
        }
    }
    Ok((width, values))
}
pub fn decode(values: &[f32], steps: usize, characters: &[String]) -> Result<(String, f64), Error> {
    let classes = characters.len();
    if classes == 0 || steps.checked_mul(classes) != Some(values.len()) {
        return Err("Invalid CTC output length".into());
    }
    let mut result = String::new();
    let mut confidence = 0.;
    let mut count = 0;
    let mut previous = usize::MAX;
    for row in values.chunks_exact(classes) {
        let mut best = 0;
        for i in 0..classes {
            if !row[i].is_finite() {
                return Err("Nonfinite OCR output".into());
            }
            if row[i] > row[best] {
                best = i;
            }
        }
        if best != 0 && best != previous {
            result.push_str(&characters[best]);
            confidence += row[best] as f64;
            count += 1;
        }
        previous = best;
    }
    // Reference appends one near-zero confidence before computing the mean.
    Ok((result, (confidence + 1e-50) / (count + 1) as f64))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ctc_repeat_blank_and_confidence_match_reference() {
        let chars = vec!["".into(), "A".into(), "B".into()];
        let (text, score) = decode(
            &[
                0., 0.9, 0.1, 0., 0.8, 0.2, 1., 0., 0., 0., 0.9, 0.1, 0., 0.1, 0.9,
            ],
            5,
            &chars,
        )
        .unwrap();
        assert_eq!(text, "AAB");
        assert!((score - 0.675).abs() < 1e-6);
    }
    #[test]
    fn normalizes_channels_and_bounds() {
        let im = RgbImage::from_pixel(20, 10, image::Rgb([255, 0, 128]));
        let (w, data) = normalize(&im).unwrap();
        assert_eq!(w, 96);
        assert_eq!(data[0], 1.);
        assert_eq!(data[48 * w], -1.);
        assert!(normalize(&RgbImage::new(0, 0)).is_err());
    }
}
