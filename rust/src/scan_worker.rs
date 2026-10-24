use crate::{
    capture::Pixels,
    detector::Detector,
    ocr_worker::{self, Observation},
    recognizer::Recognizer,
    scan_ocr::{self, Asteroid, Contact},
};
use std::{path::PathBuf, sync::mpsc, thread, time::Instant};
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Asteroid,
    Contacts,
}
pub struct ResultFrame {
    pub stamp: Instant,
    pub desktop: crate::overlay::Rect,
    pub size: [u32; 2],
    pub observation: Option<Observation>,
    pub asteroid: Option<Result<Asteroid, String>>,
    pub contacts: Option<Result<Vec<Contact>, String>>,
}
pub struct Worker {
    tx: mpsc::SyncSender<(Kind, Pixels)>,
    rx: mpsc::Receiver<Result<ResultFrame, String>>,
}
impl Worker {
    pub fn start(runtime: PathBuf) -> Self {
        let (tx, input) = mpsc::sync_channel::<(Kind, Pixels)>(1);
        let (output, rx) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let initialized = (|| {
                Ok::<_, String>((
                    Detector::load(&runtime)?,
                    Recognizer::load(&runtime).map_err(|e| e.to_string())?,
                ))
            })();
            let (mut detector, mut reader) = match initialized {
                Ok(v) => v,
                Err(e) => {
                    let _ = output.send(Err(e));
                    return;
                }
            };
            let mut known = Vec::new();
            let mut last_search: Option<Instant> = None;
            let mut size = [0, 0];
            while let Ok((kind, pixels)) = input.recv() {
                let result = (|| {
                    let rgba =
                        image::RgbaImage::from_raw(pixels.size[0], pixels.size[1], pixels.rgba)
                            .ok_or("Invalid scan image")?;
                    let im = image::DynamicImage::ImageRgba8(rgba).to_rgb8();
                    let mut result = ResultFrame {
                        stamp: pixels.stamp,
                        desktop: pixels.desktop,
                        size: pixels.size,
                        observation: None,
                        asteroid: None,
                        contacts: None,
                    };
                    if kind == Kind::Asteroid {
                        let [x, y, r, b] = crate::overlay::debug_box(im.width(), im.height());
                        let crop = image::imageops::crop_imm(&im, x, y, r - x, b - y).to_image();
                        let rgba = image::DynamicImage::ImageRgb8(crop).to_rgba8();
                        let debug = Pixels {
                            desktop: pixels.desktop,
                            stamp: pixels.stamp,
                            source_size: pixels.source_size,
                            origin: [x, y],
                            size: [r - x, b - y],
                            rgba: rgba.into_raw(),
                        };
                        result.observation =
                            Some(ocr_worker::read_fixed(&mut reader, &debug, true)?);
                        result.asteroid = Some(scan_ocr::asteroid(&mut detector, &mut reader, &im));
                    } else {
                        if size != pixels.size {
                            known.clear();
                            last_search = None;
                            size = pixels.size;
                        }
                        let search = last_search.is_none_or(|t| {
                            t.elapsed().as_secs() >= if known.is_empty() { 5 } else { 10 }
                        });
                        if search {
                            last_search = Some(Instant::now());
                        }
                        let found =
                            scan_ocr::contacts(&mut detector, &mut reader, &im, &known, search);
                        if let Ok(v) = &found {
                            if !v.is_empty() {
                                known = v.clone();
                            }
                        }
                        result.contacts = Some(found);
                    }
                    Ok::<_, String>(result)
                })();
                if output.send(result).is_err() {
                    break;
                }
            }
        });
        Self { tx, rx }
    }
    pub fn submit(&self, kind: Kind, pixels: Pixels) -> bool {
        self.tx.try_send((kind, pixels)).is_ok()
    }
    pub fn poll(&self) -> Option<Result<ResultFrame, String>> {
        self.rx.try_recv().ok()
    }
}
