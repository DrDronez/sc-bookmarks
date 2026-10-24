//! Pure navigation math. Units are kilometres and monotonic seconds.
//! This module neither captures pixels nor sends game input.
use std::collections::VecDeque;
pub type Vec3 = [f64; 3];
pub const FRESH_SECONDS: f64 = 2.5;
fn dot(a: Vec3, b: Vec3) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn norm(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}
fn sub(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| a[i] - b[i])
}
#[derive(Debug)]
pub struct Approach {
    pub distance: f64,
    pub closing: f64,
    pub seconds: Option<f64>,
    pub miss: Option<f64>,
}
pub fn approach(position: Vec3, target: Vec3, velocity: Vec3) -> Approach {
    let d = sub(target, position);
    let distance = norm(d);
    if norm(velocity) < 1e-8 {
        return Approach {
            distance,
            closing: 0.,
            seconds: None,
            miss: None,
        };
    }
    let seconds = dot(d, velocity) / dot(velocity, velocity);
    Approach {
        distance,
        closing: dot(d, velocity) / distance.max(1e-12),
        seconds: Some(seconds),
        miss: Some(norm(std::array::from_fn(|i| d[i] - seconds * velocity[i]))),
    }
}
pub fn estimate(position: Vec3, stamp: f64, velocity: Vec3, now: f64) -> (Vec3, bool) {
    let age = (now - stamp).max(0.);
    (
        std::array::from_fn(|i| position[i] + velocity[i] * age.min(10.)),
        age > 10.,
    )
}
/// Camera Euler convention validated by the Python reference: Rz Ry Rx, +Y forward.
pub fn camera_basis(angles: Vec3) -> (Vec3, Vec3, Vec3) {
    let [a, b, c] = angles.map(f64::to_radians);
    let (sa, ca) = a.sin_cos();
    let (sb, cb) = b.sin_cos();
    let (sc, cc) = c.sin_cos();
    (
        [cc * sb * sa - sc * ca, sc * sb * sa + cc * ca, cb * sa],
        [cc * cb, sc * cb, -sb],
        [cc * sb * ca + sc * sa, sc * sb * ca - cc * sa, cb * ca],
    )
}
/// Positive angles mean right and up respectively.
pub fn camera_steering(angles: Vec3, delta: Vec3) -> (f64, f64) {
    let (f, r, u) = camera_basis(angles);
    let x = dot(delta, r);
    let z = dot(delta, f);
    (
        x.atan2(z).to_degrees(),
        dot(delta, u).atan2(x.hypot(z)).to_degrees(),
    )
}
#[derive(Default)]
pub struct TravelMotion {
    samples: VecDeque<(f64, Vec3)>,
    pub quantum: bool,
    pub velocity: Vec3,
}
impl TravelMotion {
    pub fn add(&mut self, t: f64, p: Vec3) {
        if !t.is_finite() || p.iter().any(|n| !n.is_finite()) {
            return;
        }
        if let Some((last, _)) = self.samples.back() {
            if t <= *last {
                return;
            }
            if t - *last > FRESH_SECONDS {
                self.samples.clear();
                self.velocity = [0.; 3];
            }
        }
        if self.samples.len() == 4 {
            self.samples.pop_front();
        }
        self.samples.push_back((t, p));
        if self.samples.len() < 3 {
            return;
        }
        let segments: Vec<Vec3> = self
            .samples
            .iter()
            .zip(self.samples.iter().skip(1))
            .map(|((ta, pa), (tb, pb))| std::array::from_fn(|i| (pb[i] - pa[i]) / (tb - ta)))
            .collect();
        self.velocity = std::array::from_fn(|i| {
            let mut values: Vec<f64> = segments.iter().map(|v| v[i]).collect();
            values.sort_by(f64::total_cmp);
            let n = values.len();
            if n % 2 == 0 {
                (values[n / 2 - 1] + values[n / 2]) / 2.
            } else {
                values[n / 2]
            }
        });
        let a = segments[segments.len() - 2];
        let b = segments[segments.len() - 1];
        if norm(a) > 5. && norm(b) > 5. && dot(a, b) > 0. {
            self.quantum = true;
        } else if norm(a) < 2. && norm(b) < 2. {
            self.quantum = false;
        }
    }
    pub fn fresh(&self, now: f64) -> bool {
        self.samples.len() >= 3
            && self
                .samples
                .back()
                .is_some_and(|(t, _)| now >= *t && now - *t <= FRESH_SECONDS)
    }
    pub fn position(&self, now: f64) -> Option<(Vec3, bool)> {
        self.samples
            .back()
            .map(|(t, p)| estimate(*p, *t, self.velocity, now))
    }
}
/// One cue per arming; a stale sample can never trigger an alert.
#[derive(Default)]
pub struct ExitAlert {
    pub armed: bool,
    pub fired: bool,
    seen_ahead: bool,
}
impl ExitAlert {
    pub fn arm(&mut self, armed: bool) {
        *self = Self {
            armed,
            ..Self::default()
        };
    }
    pub fn update(&mut self, quantum: bool, fresh: bool, seconds: Option<f64>, lead: f64) -> bool {
        if !quantum || !fresh {
            return false;
        }
        let Some(t) = seconds.filter(|t| t.is_finite()) else {
            return false;
        };
        let lead = if lead.is_finite() {
            lead.clamp(0., 120.)
        } else {
            3.
        };
        if t > lead {
            self.seen_ahead = true;
        }
        if self.armed && !self.fired && (self.seen_ahead || t >= 0.) && t <= lead {
            self.fired = true;
            true
        } else {
            false
        }
    }
}
