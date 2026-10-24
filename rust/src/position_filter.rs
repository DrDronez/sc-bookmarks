//! Eight-reading robust position average, matching the existing OCR pipeline.
//! Stores coordinates only, never a history of full screenshots.
use std::collections::VecDeque;
#[derive(Clone, Copy, Debug)]
pub struct Reading {
    pub stamp: f64,
    pub xyz: [f64; 3],
}
#[derive(Debug)]
pub struct Average {
    pub stamp: f64,
    pub xyz: [f64; 3],
    pub newest: f64,
}
#[derive(Default)]
pub struct PositionFilter {
    frames: VecDeque<Reading>,
}
fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n % 2 == 0 {
        (values[n / 2 - 1] + values[n / 2]) / 2.
    } else {
        values[n / 2]
    }
}
// Compensated summation keeps precision when averaging large system coordinates.
fn sum(values: impl Iterator<Item = f64>) -> f64 {
    let mut s = 0_f64;
    let mut correction = 0.;
    for x in values {
        let next = s + x;
        correction += if s.abs() >= x.abs() {
            (s - next) + x
        } else {
            (x - next) + s
        };
        s = next;
    }
    s + correction
}
impl PositionFilter {
    pub fn count(&self) -> usize {
        self.frames.len()
    }
    pub fn add(&mut self, reading: Reading) -> Result<Option<Average>, &'static str> {
        if !reading.stamp.is_finite() || reading.xyz.iter().any(|x| !x.is_finite()) {
            return Err("Invalid coordinate reading");
        }
        if let Some(last) = self.frames.back() {
            if reading.stamp <= last.stamp {
                return Ok(None);
            }
            if reading.stamp - last.stamp > 10. {
                self.frames.clear();
            }
        }
        if let Some(last) = self.frames.back() {
            let distance = (0..3)
                .map(|i| (reading.xyz[i] - last.xyz[i]).powi(2))
                .sum::<f64>()
                .sqrt();
            if distance > 400000. * (reading.stamp - last.stamp) + 0.1 {
                return Err("Implausible coordinate jump rejected");
            }
        }
        if self.frames.len() == 8 {
            self.frames.pop_front();
        }
        self.frames.push_back(reading);
        if self.frames.len() < 8 {
            return Ok(None);
        }
        let center: [f64; 3] =
            std::array::from_fn(|i| median(self.frames.iter().map(|r| r.xyz[i]).collect()));
        let limits: [f64; 3] = std::array::from_fn(|i| {
            (6. * median(
                self.frames
                    .iter()
                    .map(|r| (r.xyz[i] - center[i]).abs())
                    .collect(),
            ))
            .max(0.02)
        });
        let good: Vec<_> = self
            .frames
            .iter()
            .filter(|r| (0..3).all(|i| (r.xyz[i] - center[i]).abs() <= limits[i]))
            .collect();
        if good.len() < 5 {
            return Ok(None);
        }
        Ok(Some(Average {
            stamp: sum(good.iter().map(|r| r.stamp)) / good.len() as f64,
            xyz: std::array::from_fn(|i| sum(good.iter().map(|r| r.xyz[i])) / good.len() as f64),
            newest: good.last().unwrap().stamp,
        }))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_bad_digit_without_contaminating_average() {
        let mut filter = PositionFilter::default();
        for i in 0..7 {
            assert!(
                filter
                    .add(Reading {
                        stamp: i as f64,
                        xyz: [10000000., 10., 20.]
                    })
                    .unwrap()
                    .is_none()
            );
        }
        let result = filter
            .add(Reading {
                stamp: 7.,
                xyz: [10001000., 10., 20.],
            })
            .unwrap()
            .unwrap();
        assert_eq!(result.xyz, [10000000., 10., 20.]);
        assert_eq!(result.newest, 6.);
        assert_eq!(result.stamp, 3.);
    }
    #[test]
    fn preserves_movement_and_resets_after_gap() {
        let mut filter = PositionFilter::default();
        for i in 0..8 {
            let result = filter
                .add(Reading {
                    stamp: i as f64,
                    xyz: [i as f64 * 100000., 0., 0.],
                })
                .unwrap();
            if i == 7 {
                assert_eq!(result.unwrap().xyz, [350000., 0., 0.]);
            }
        }
        assert!(
            filter
                .add(Reading {
                    stamp: 8.,
                    xyz: [1e10, 0., 0.]
                })
                .is_err()
        );
        assert_eq!(filter.count(), 8);
        assert!(
            filter
                .add(Reading {
                    stamp: 7.,
                    xyz: [0.; 3]
                })
                .unwrap()
                .is_none()
        );
        assert!(
            filter
                .add(Reading {
                    stamp: 20.,
                    xyz: [1e10, 0., 0.]
                })
                .unwrap()
                .is_none()
        );
        assert_eq!(filter.count(), 1);
    }
}
