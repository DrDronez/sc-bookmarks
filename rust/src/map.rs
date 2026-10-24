//! Orthographic map geometry. Static decoration is generated once, never simulated.
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Deserialize)]
pub struct Star {
    pub xyz_km: [f64; 3],
    pub approximate: bool,
}
#[derive(Deserialize)]
pub struct Band {
    pub number: u32,
    pub inner_km: f64,
    pub outer_km: f64,
}
#[derive(Deserialize)]
pub struct Halo {
    pub origin_km: [f64; 3],
    pub plane_z_km: f64,
    pub bands: Vec<Band>,
    pub survey_version: String,
}
pub struct Data {
    pub stars: BTreeMap<String, Star>,
    pub halo: Halo,
    pub dust: Vec<(u32, [f64; 3])>,
    pub background: Vec<[f32; 3]>,
}
fn random(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 11) as f64) / ((1u64 << 53) as f64)
}
impl Data {
    pub fn load() -> Result<Self, serde_json::Error> {
        let stars = serde_json::from_str(include_str!("../../route-data/stars.json"))?;
        let halo: Halo = serde_json::from_str(include_str!("../../route-data/aaron-halo.json"))?;
        let mut seed = 31415;
        let mut dust = Vec::new();
        for b in &halo.bands {
            for _ in 0..if b.number == 5 { 440 } else { 240 } {
                let r = (b.inner_km.powi(2)
                    + random(&mut seed) * (b.outer_km.powi(2) - b.inner_km.powi(2)))
                .sqrt();
                let a = random(&mut seed) * std::f64::consts::TAU;
                dust.push((
                    b.number,
                    [
                        halo.origin_km[0] + r * a.cos(),
                        halo.origin_km[1] + r * a.sin(),
                        halo.plane_z_km + (random(&mut seed) - 0.5) * 4000.,
                    ],
                ));
            }
        }
        let background = (0..420)
            .map(|_| {
                [
                    random(&mut seed) as f32,
                    random(&mut seed) as f32,
                    random(&mut seed) as f32,
                ]
            })
            .collect();
        Ok(Self {
            stars,
            halo,
            dust,
            background,
        })
    }
}
pub fn is_primary(system: &str, name: &str) -> bool {
    match system {
        "Stanton" => ["Hurston", "Crusader", "ArcCorp", "microTech"].contains(&name),
        "Pyro" => ["Pyro I", "Monox", "Bloom", "Pyro IV", "Pyro V", "Terminus"].contains(&name),
        _ => false,
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub center: [f64; 3],
    pub yaw: f64,
    pub tilt: f64,
    pub scale: f64,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            center: [0.; 3],
            yaw: 0.,
            tilt: 0.65,
            scale: 1e-5,
        }
    }
}
impl Camera {
    pub fn project(&self, p: [f64; 3]) -> [f64; 2] {
        let [x, y, z] = std::array::from_fn(|i| p[i] - self.center[i]);
        let (s, c) = self.yaw.sin_cos();
        [
            (x * c - y * s) * self.scale,
            ((x * s + y * c) * self.tilt.sin() - z * self.tilt.cos()) * self.scale,
        ]
    }
    pub fn fit(&mut self, points: &[[f64; 3]], size: [f64; 2]) {
        if points.is_empty() {
            return;
        }
        self.center = std::array::from_fn(|i| {
            let low = points.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min);
            let high = points
                .iter()
                .map(|p| p[i])
                .fold(f64::NEG_INFINITY, f64::max);
            (low + high) / 2.
        });
        self.scale = 1.;
        let projected: Vec<_> = points.iter().map(|&p| self.project(p)).collect();
        let low: [f64; 2] =
            std::array::from_fn(|i| projected.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min));
        let high: [f64; 2] = std::array::from_fn(|i| {
            projected
                .iter()
                .map(|p| p[i])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        let u = (low[0] + high[0]) / 2.;
        let v = (low[1] + high[1]) / 2.;
        let (s, c) = self.yaw.sin_cos();
        let (st, ct) = self.tilt.sin_cos();
        self.center = std::array::from_fn(|i| {
            self.center[i] + u * [c, -s, 0.][i] + v * [s * st, c * st, -ct][i]
        });
        self.scale = ((size[0] - 160.).max(1.) / (high[0] - low[0]).max(1.))
            .min((size[1] - 130.).max(1.) / (high[1] - low[1]).max(1.));
    }
    pub fn pan(&mut self, delta: [f64; 2]) {
        let [dx, dy] = delta;
        let (s, c) = self.yaw.sin_cos();
        let v = dy / self.tilt.sin().max(0.15);
        self.center[0] -= (dx * c + v * s) / self.scale;
        self.center[1] -= (-dx * s + v * c) / self.scale;
    }
}
