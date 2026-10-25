use crate::{
    detector::{self, Bounds, Detector},
    mining,
    recognizer::Recognizer,
    scanning,
};
use image::RgbImage;
use regex::Regex;
use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Instant};
#[derive(Clone, Debug)]
pub struct Text {
    pub bounds: Bounds,
    pub text: String,
    pub score: f64,
}
#[derive(Clone, Debug)]
pub struct Asteroid {
    pub mining: Value,
    pub range_m: f64,
}
#[derive(Clone, Debug)]
pub struct Contact {
    pub bounds: Bounds,
    pub rs: u64,
    pub text: String,
}
pub fn parse(lines: &[String]) -> Result<Value, String> {
    let text = lines.join("\n");
    let mut data = json!({});
    for (key, label) in [
        ("mass", "MASS"),
        ("resistance", "RESISTANCE"),
        ("instability", "[IJ]NSTABILITY"),
    ] {
        let re = Regex::new(&format!(r"(?i){}\s*:?[^\d\n]{{0,4}}(\d+(?:\.\d+)?)", label)).unwrap();
        data[key] = re
            .captures(&text)
            .and_then(|c| c[1].parse::<f64>().ok())
            .map_or(Value::Null, |n| json!(n));
    }
    let re = Regex::new(r"(?i)(\d+(?:\.\d+)?)\s*SCU").unwrap();
    data["scu"] = re
        .captures(&text)
        .and_then(|c| c[1].parse::<f64>().ok())
        .map_or(Value::Null, |n| json!(n));
    let row = Regex::new(r"(?i)(\d+(?:\.\d+)?)\s*%\s*(.*?)\s*(\d+(?:\.\d+)?)\s*$").unwrap();
    let inert = Regex::new(r"(?i)(\d+(?:\.\d+)?)\s*%\s*INERT").unwrap();
    let mut rows = Vec::new();
    for line in lines {
        if let Some(c) = inert.captures(line) {
            rows.push(
                json!({"ore":"Inert Materials","percent":c[1].parse::<f64>().ok(),"quality":0.}),
            );
            continue;
        }
        if let Some(c) = row.captures(line) {
            let name = c[2]
                .to_lowercase()
                .replace("ore", "")
                .chars()
                .filter(char::is_ascii_alphabetic)
                .collect::<String>();
            if let Some(ore) = mining::ORES
                .iter()
                .find(|o| o.to_lowercase().replace(' ', "") == name)
            {
                rows.push(json!({"ore":ore,"percent":c[1].parse::<f64>().ok(),"quality":c[3].parse::<f64>().ok()}));
            }
        }
    }
    data["composition"] = json!(rows);
    mining::clean(&data)
}
pub fn texts(
    det: &mut Detector,
    reader: &mut Recognizer,
    im: &RgbImage,
) -> Result<Vec<Text>, String> {
    let boxes = det.detect(im)?;
    let mut result = Vec::new();
    for b in boxes.into_iter().take(400) {
        let mut best = None;
        for (top, bottom) in [(1, 2), (4, 0)] {
            let bounds = [
                b[0].saturating_sub(2),
                b[1].saturating_sub(top),
                (b[2] + 3).min(im.width()),
                (b[3] + bottom).min(im.height()),
            ];
            if let Ok((text, score)) = detector::read(reader, im, bounds) {
                if best.as_ref().is_none_or(|t: &Text| score > t.score) {
                    best = Some(Text {
                        bounds: b,
                        text,
                        score,
                    });
                }
            }
        }
        if let Some(t) = best {
            result.push(t);
        }
    }
    Ok(result)
}
pub fn asteroid(
    det: &mut Detector,
    reader: &mut Recognizer,
    im: &RgbImage,
) -> Result<Asteroid, String> {
    let raw = texts(det, reader, im)?;
    let mut headings: Vec<Bounds> = raw
        .iter()
        .filter(|t| {
            let s = t
                .text
                .to_uppercase()
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .collect::<String>();
            s.contains("SCANRESULT")
        })
        .map(|t| t.bounds)
        .collect();
    if headings.is_empty() {
        for t in &raw {
            if t.text.trim().eq_ignore_ascii_case("SCAN") {
                let b = t.bounds;
                let h = b[3] - b[1];
                for r in &raw {
                    if r.text.to_uppercase().starts_with("RESULT")
                        && r.bounds[0] >= b[2].saturating_sub(h)
                        && r.bounds[0] <= b[2] + 3 * h
                        && r.bounds[1].abs_diff(b[1]) < h
                    {
                        headings.push([
                            b[0],
                            b[1].min(r.bounds[1]),
                            r.bounds[2],
                            b[3].max(r.bounds[3]),
                        ]);
                    }
                }
            }
        }
    }
    let mut mining = None;
    for b in headings {
        let w = b[2] - b[0];
        let h = b[3] - b[1];
        let panel: Vec<_> = raw
            .iter()
            .filter(|t| {
                t.bounds[0] >= b[0].saturating_sub(w / 6)
                    && t.bounds[2] <= b[0] + w * 22 / 10
                    && t.bounds[1] >= b[3]
                    && t.bounds[1] <= b[3] + h * 24
            })
            .collect();
        let bands = detector::bands(&panel.iter().map(|t| t.bounds).collect::<Vec<_>>());
        let mut lines = Vec::new();
        for band in bands {
            let mut fragments: Vec<_> = panel
                .iter()
                .filter(|t| {
                    t.score >= 0.65
                        && t.bounds[0] >= band[0]
                        && t.bounds[2] <= band[2]
                        && t.bounds[1] >= band[1]
                        && t.bounds[3] <= band[3]
                })
                .collect();
            fragments.sort_by_key(|t| t.bounds[0]);
            let assembled = fragments
                .iter()
                .map(|t| t.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let parsed = parse(&[assembled.clone()])?;
            let valid_composition = parsed["composition"]
                .as_array()
                .is_some_and(|a| !a.is_empty());
            if !assembled.is_empty() && (!assembled.contains('%') || valid_composition) {
                lines.push(assembled);
            } else if let Ok((text, score)) = detector::read(reader, im, band) {
                if score >= 0.65 {
                    lines.push(text);
                }
            }
        }
        let candidate = parse(&lines)?;
        if candidate["scu"].is_number()
            && candidate["composition"]
                .as_array()
                .is_some_and(|a| !a.is_empty())
        {
            mining = Some(candidate);
            break;
        }
    }
    let mining =
        mining.ok_or("Cannot read SCAN RESULTS and composition; keep the results panel visible")?;
    let range = Regex::new(r"(?i)^\s*(\d+(?:\.\d+)?)\s*(km|m)\s*$").unwrap();
    let mut ranges = Vec::new();
    for t in raw
        .iter()
        .filter(|t| t.text.to_uppercase().contains("ORE") && !t.text.contains('%'))
    {
        let b = t.bounds;
        let h = b[3] - b[1];
        let cx = (b[0] + b[2]) as f64 / 2.;
        if cx < im.width() as f64 * 0.15
            || cx > im.width() as f64 * 0.70
            || b[1] < im.height() / 4
            || b[1] > im.height() * 78 / 100
        {
            continue;
        }
        for r in &raw {
            if r.score < 0.65
                || r.bounds[1] < b[3]
                || r.bounds[1] > b[3] + h * 2
                || (((r.bounds[0] + r.bounds[2]) as f64 / 2.) - cx).abs()
                    > (b[2] - b[0]).max(h) as f64 * 0.7
            {
                continue;
            }
            if let Some(c) = range.captures(&r.text) {
                if let Ok(n) = c[1].parse::<f64>() {
                    let distance = n * if c[2].eq_ignore_ascii_case("km") {
                        1000.
                    } else {
                        1.
                    };
                    ranges.push((
                        (cx / im.width() as f64 - 0.5).powi(2)
                            + ((b[3] + h) as f64 / im.height() as f64 - 0.5).powi(2),
                        distance,
                    ));
                }
            }
        }
    }
    ranges.sort_by(|a, b| a.0.total_cmp(&b.0));
    let range_m = ranges
        .first()
        .ok_or("Cannot read the selected asteroid range; center its ore label")?
        .1;
    Ok(Asteroid { mining, range_m })
}
fn icon(im: &RgbImage, b: Bounds) -> bool {
    let h = b[3] - b[1];
    let mut count = 0;
    for y in b[1].saturating_sub(h * 4 / 10)..(b[3] + h * 4 / 10).min(im.height()) {
        for x in b[0].saturating_sub(h * 18 / 10)..b[0] {
            let p = im.get_pixel(x, y);
            let [r, g, bl] = p.0.map(|v| v as f64 / 255.);
            let max = r.max(g).max(bl);
            let min = r.min(g).min(bl);
            let d = max - min;
            if d == 0. || max <= 140. / 255. || d / max <= 100. / 255. {
                continue;
            }
            let hue = if max == r {
                60. * ((g - bl) / d).rem_euclid(6.)
            } else if max == g {
                60. * ((bl - r) / d + 2.)
            } else {
                60. * ((r - g) / d + 4.)
            };
            if hue > 20. && hue < 210. {
                count += 1;
            }
        }
    }
    count as f64 > (5_f64).max((h * h) as f64 * 0.035)
}
pub fn contacts(
    det: &mut Detector,
    reader: &mut Recognizer,
    im: &RgbImage,
    known: &[Contact],
    search: bool,
) -> Result<Vec<Contact>, String> {
    let boxes = if search {
        det.detect(im)?
    } else {
        known.iter().map(|c| c.bounds).collect()
    };
    let re = Regex::new(r"^\s*(\d{1,3},\d{3})\s*$").unwrap();
    let charts = scanning::charts()?;
    let candidate_pattern = Regex::new(r"\d[,.]\d{2}").unwrap();
    let mut found = Vec::new();
    let start = Instant::now();
    for b in boxes {
        if found.len() >= 8 || start.elapsed().as_secs_f32() >= 8.0 {
            break;
        }
        let h = b[3] - b[1];
        let w = b[2] - b[0];
        if h < 5 || h > 60.max(im.height() / 20) || !(1.3..=7.).contains(&(w as f64 / h as f64)) {
            continue;
        }
        let b = if search {
            [
                b[0].saturating_sub(2),
                b[1].saturating_sub(2),
                (b[2] + 2).min(im.width()),
                (b[3] + 2).min(im.height()),
            ]
        } else {
            b
        };
        let h = b[3] - b[1];
        if !icon(im, b) && !icon(im, [b[0] + h.min(b[2] - b[0]), b[1], b[2], b[3]]) {
            continue;
        }
        let Ok((candidate, _)) = detector::read(reader, im, b) else {
            continue;
        };

        if !candidate_pattern.is_match(&candidate) {
            continue;
        }
        let mut votes: BTreeMap<u64, (usize, Bounds)> = BTreeMap::new();
        let mut confident = None;
        'variants: for trim in [0., 0.5, 0.75, 1.] {
            for inset in [0., 0.16] {
                let bb = [
                    b[0] + (h as f64 * trim) as u32,
                    b[1] + (h as f64 * inset) as u32,
                    b[2],
                    b[3] - (h as f64 * inset) as u32,
                ];
                if bb[0] >= bb[2] || bb[1] >= bb[3] {
                    continue;
                }
                for gray in [false, true] {
                    if start.elapsed().as_secs_f32() >= 8.0 {
                        break 'variants;
                    }
                    let crop =
                        image::imageops::crop_imm(im, bb[0], bb[1], bb[2] - bb[0], bb[3] - bb[1])
                            .to_image();
                    let crop = image::imageops::resize(
                        &crop,
                        crop.width() * 4,
                        crop.height() * 4,
                        if gray {
                            image::imageops::FilterType::Triangle
                        } else {
                            image::imageops::FilterType::CatmullRom
                        },
                    );
                    let crop = if gray {
                        image::DynamicImage::ImageLuma8(
                            image::DynamicImage::ImageRgb8(crop).to_luma8(),
                        )
                        .to_rgb8()
                    } else {
                        crop
                    };
                    if let Ok((t, s)) = reader.read(&crop) {
                        if s >= 0.65 {
                            if let Some(c) = re.captures(&t) {
                                if let Ok(value) = c[1].replace(',', "").parse::<u64>() {
                                    let v = votes.entry(value).or_insert((0, bb));
                                    v.0 += 1;
                                    // Two independent preprocessing reads must agree; never
                                    // fast-accept a conflict or drop the badge icon check.
                                    if s >= 0.90 {
                                        if fast_agreement(confident, value, s, votes.len())
                                            && icon(im, bb)
                                        {
                                            break 'variants;
                                        }
                                        confident = Some(value);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut ordered: Vec<_> = votes.into_iter().collect();
        ordered.sort_by_key(|(_, v)| std::cmp::Reverse(v.0));
        if let Some((rs, (n, read_bounds))) = ordered.first() {
            if *n >= 2 && ordered.get(1).is_none_or(|(_, v)| v.0 < *n) && icon(im, *read_bounds) {
                found.push(Contact {
                    bounds: b,
                    rs: *rs,
                    text: scanning::overlay_description(&charts, *rs),
                });
            }
        }
    }
    Ok(found)
}

fn fast_agreement(previous: Option<u64>, value: u64, confidence: f64, alternatives: usize) -> bool {
    confidence >= 0.90 && previous == Some(value) && alternatives == 1
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quick_rs_requires_confident_uncontested_agreement() {
        assert!(fast_agreement(Some(8000), 8000, 0.95, 1));
        assert!(!fast_agreement(None, 8000, 0.95, 1));
        assert!(!fast_agreement(Some(8000), 9000, 0.95, 1));
        assert!(!fast_agreement(Some(8000), 8000, 0.8, 1));
        assert!(!fast_agreement(Some(8000), 8000, 0.95, 2));
    }
    #[test]
    fn rock_repeated_ores_and_inert() {
        let lines = [
            "MASS: 26283",
            "RESISTANCE: 0%",
            "INSTABILITY: 31.43",
            "COMPOSITION 29.02 SCU",
            "9.75% IRON (ORE) 664",
            "67.91% IRON (ORE) 325",
            "22.32% INERT MATERIALS 0",
        ]
        .map(str::to_owned);
        let m = parse(&lines).unwrap();
        assert_eq!(m["mass"], 26283.);
        assert_eq!(m["composition"].as_array().unwrap().len(), 3);
        assert_eq!(m["composition"][2]["ore"], "Inert Materials");
    }
}
