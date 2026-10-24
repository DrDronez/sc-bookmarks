//! Placement uses physical game-client pixels, independent of desktop UI DPI.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub position: [f64; 2],
    #[serde(default)]
    pub bookmark_3d: bool,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default = "default_fov")]
    pub vertical_fov: f64,
    #[serde(default = "default_hotkey")]
    pub hotkey: u32,
    #[serde(default)]
    pub presets: BTreeMap<String, [f64; 2]>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            position: [50.; 2],
            bookmark_3d: false,
            theme: Theme::default(),
            vertical_fov: default_fov(),
            hotkey: default_hotkey(),
            presets: BTreeMap::new(),
            extra: BTreeMap::new(),
        }
    }
}
pub fn validate_position(p: [f64; 2]) -> Result<(), String> {
    if p.iter().all(|v| v.is_finite() && (0. ..=100.).contains(v)) {
        Ok(())
    } else {
        Err("Positions must be between 0 and 100.".into())
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        validate_position(self.position)?;
        if !self.vertical_fov.is_finite() || !(30. ..=120.).contains(&self.vertical_fov) {
            return Err("Vertical FOV must be 30–120 degrees.".into());
        }
        if self.hotkey != 0
            && (!(0x75..=0x7b).contains(&(self.hotkey & 0xffff)) || self.hotkey >> 16 > 7)
        {
            return Err("Invalid overlay hotkey.".into());
        }
        for p in self.presets.values() {
            validate_position(*p)?;
        }
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read(path) {
            Ok(bytes) => {
                let s: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                s.validate()?;
                Ok(s)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        crate::storage::write_atomic(
            path,
            &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
}
pub fn placement(rect: Rect, position: [f64; 2]) -> Result<Rect, String> {
    validate_position(position)?;
    let width = rect.width.min(460);
    let height = rect.height.min(265);
    let left =
        (rect.left as f64 + (rect.width - width) as f64 * position[0] / 100.).round_ties_even();
    let top =
        (rect.top as f64 + (rect.height - height) as f64 * position[1] / 100.).round_ties_even();
    if left > i32::MAX as f64 || top > i32::MAX as f64 {
        return Err("Game window coordinates exceed desktop bounds.".into());
    }
    Ok(Rect {
        left: left as i32,
        top: top as i32,
        width,
        height,
    })
}
/// Mouse location is relative to the game rectangle in physical pixels.
pub fn drag_position(size: [u32; 2], cursor: [f64; 2]) -> [f64; 2] {
    std::array::from_fn(|i| {
        let total = size[i] as f64;
        let panel = total.min([460., 265.][i]);
        ((cursor[i] - panel / 2.) / (total - panel).max(1.) * 100.).clamp(0., 100.)
    })
}
/// Debug OCR search region, matching the Python reference's resolution scaling.
pub fn debug_box(width: u32, height: u32) -> [u32; 4] {
    [
        width.saturating_sub(700.max((width as f64 * 0.55).round_ties_even() as u32)),
        0,
        width,
        height.min(310.max((height as f64 * 0.35).round_ties_even() as u32)),
    ]
}

fn default_fov() -> f64 {
    60.
}
fn default_hotkey() -> u32 {
    0x79
} // F10; no modifier keys that can trigger ship movement.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum Theme {
    #[default]
    Classic,
    Misc,
    Anvil,
    Drake,
    Aegis,
    Argo,
}
impl Theme {
    pub const ALL: [Self; 6] = [
        Self::Classic,
        Self::Misc,
        Self::Anvil,
        Self::Drake,
        Self::Aegis,
        Self::Argo,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Misc => "MISC",
            Self::Anvil => "Anvil",
            Self::Drake => "Drake",
            Self::Aegis => "Aegis",
            Self::Argo => "Argo",
        }
    }
    // GDI BGR colors. Original treatments inspired by cockpit references; no game assets.
    pub fn colors(self) -> (u32, u32) {
        match self {
            Self::Classic => (0xffec63, 0x70cfff),
            Self::Misc => (0xe8dc75, 0x70ceef),
            Self::Anvil => (0xb9eca0, 0xe2f0d2),
            Self::Drake => (0x63b9f0, 0x97d8ed),
            Self::Aegis => (0x91e974, 0xbaf6b2),
            Self::Argo => (0x389aff, 0xc9e7ff),
        }
    }
}
/// Perspective projection with +Y forward and camera roll included. Returns client pixels.
/// Behind-camera/offscreen targets remain the responsibility of the normal guidance arrow.
pub fn project(delta: [f64; 3], angles: [f64; 3], size: [u32; 2], vfov: f64) -> Option<[f64; 2]> {
    if size.contains(&0)
        || !vfov.is_finite()
        || !(30. ..=120.).contains(&vfov)
        || delta.iter().chain(angles.iter()).any(|v| !v.is_finite())
    {
        return None;
    }
    let distance = delta.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !(0.001..=5.).contains(&distance) {
        return None;
    }
    let (f, r, u) = crate::guidance::camera_basis(angles);
    let dot = |v: [f64; 3]| (0..3).map(|i| delta[i] * v[i]).sum::<f64>();
    let z = dot(f);
    if z <= 0.000001 {
        return None;
    }
    let focal = size[1] as f64 / (2. * (vfov.to_radians() / 2.).tan());
    let x = size[0] as f64 / 2. + dot(r) / z * focal;
    let y = size[1] as f64 / 2. - dot(u) / z * focal;
    if x < 32. || y < 32. || x > size[0] as f64 - 32. || y > size[1] as f64 - 32. {
        None
    } else {
        Some([x, y])
    }
}
