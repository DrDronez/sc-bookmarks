use sc_bookmarks::{
    Bookmark,
    guidance::{ExitAlert, TravelMotion, approach, camera_steering},
    ocr_worker::Observation,
    position_filter::{PositionFilter, Reading},
};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
fn heading_change(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (a, _, _) = sc_bookmarks::guidance::camera_basis(a);
    let (b, _, _) = sc_bookmarks::guidance::camera_basis(b);
    (0..3)
        .map(|i| a[i] * b[i])
        .sum::<f64>()
        .clamp(-1., 1.)
        .acos()
        .to_degrees()
}
pub struct Live {
    epoch: Instant,
    last: Option<Instant>,
    filter: PositionFilter,
    pub motion: TravelMotion,
    pub system: String,
    pub shard: String,
    shard_candidate: Option<(Instant, String)>,
    shard_time: Option<Instant>,
    camera: Option<(Instant, [f64; 3])>,
    pending_camera: Option<(Instant, [f64; 3])>,
    position: Option<(Instant, [f64; 3])>,
    average_stamp: f64,
    pub alert: ExitAlert,
    pub lead: f64,
    pub show_overlay: bool,
}
impl Default for Live {
    fn default() -> Self {
        Self {
            epoch: Instant::now(),
            last: None,
            filter: Default::default(),
            motion: Default::default(),
            system: String::new(),
            shard: String::new(),
            shard_candidate: None,
            shard_time: None,
            camera: None,
            pending_camera: None,
            position: None,
            average_stamp: 0.,
            alert: Default::default(),
            lead: 3.,
            show_overlay: true,
        }
    }
}
impl Live {
    pub fn ingest(&mut self, o: &Observation) {
        if self.last == Some(o.stamp) {
            return;
        }
        self.last = Some(o.stamp);
        if let Some(s) = &o.system {
            if !self.system.is_empty() && self.system != *s {
                self.filter = Default::default();
                self.motion = Default::default();
                self.position = None;
                self.camera = None;
                self.pending_camera = None;
                self.shard.clear();
                self.shard_candidate = None;
                self.shard_time = None;
                self.alert.arm(false);
            }
            self.system = s.clone();
        }
        if let Some(s) = &o.shard {
            if self.shard == *s
                || self.shard_candidate.as_ref().is_some_and(|(t, v)| {
                    v == s && o.stamp.saturating_duration_since(*t).as_secs() <= 45
                })
            {
                self.shard = s.clone();
                self.shard_time = Some(o.stamp);
            } else {
                self.shard.clear();
            }
            self.shard_candidate = Some((o.stamp, s.clone()));
        }
        if let Some(c) = o.camera {
            let jump = self.camera.is_some_and(|(t, old)| {
                o.stamp.saturating_duration_since(t).as_secs_f64() < 2.5
                    && heading_change(old, c) > 35.
            });
            let confirmed = self.pending_camera.is_some_and(|(t, old)| {
                o.stamp.saturating_duration_since(t).as_secs_f64() < 1.5
                    && heading_change(old, c) < 12.
            });
            if !jump || confirmed {
                self.camera = Some((o.stamp, c));
                self.pending_camera = None;
            } else {
                self.pending_camera = Some((o.stamp, c));
            }
        }
        if let Some(p) = o.position {
            let stamp = o.stamp.saturating_duration_since(self.epoch).as_secs_f64();
            if let Ok(average) = self.filter.add(Reading { stamp, xyz: p }) {
                if average.as_ref().is_none_or(|a| a.newest == stamp) {
                    self.motion.add(stamp, p);
                }
                if let Some(a) = average {
                    self.average_stamp = a.stamp;

                    self.position = Some((self.epoch + Duration::from_secs_f64(a.newest), a.xyz));
                }
            }
        }
    }
    pub fn sample_status(&self) -> String {
        format!(
            "Position samples: {}/8 · fresh: {} · QT: {}",
            self.filter.count(),
            self.fresh(),
            self.motion.quantum
        )
    }
    pub fn fresh(&self) -> bool {
        self.position
            .is_some_and(|(t, _)| t.elapsed().as_secs_f64() <= 2.5)
    }
    pub fn ship(&self) -> Option<[f64; 3]> {
        if self.motion.quantum {
            self.motion
                .position(self.epoch.elapsed().as_secs_f64())
                .map(|(p, _)| p)
        } else {
            self.position.map(|(_, p)| {
                sc_bookmarks::guidance::estimate(
                    p,
                    self.average_stamp,
                    self.motion.velocity,
                    self.epoch.elapsed().as_secs_f64(),
                )
                .0
            })
        }
    }
    pub fn bookmark(&self) -> Result<Bookmark, &'static str> {
        if !self.fresh() {
            return Err("Waiting for fresh coordinates (eight valid samples required)");
        }
        if self.system.is_empty() {
            return Err("Current system has not been identified yet");
        }
        let p = self.ship().ok_or("Waiting for position estimate")?;
        let mut metadata = BTreeMap::new();
        metadata.insert("category".into(), "Personal".into());
        metadata.insert(
            "utc".into(),
            chrono::Utc::now()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                .into(),
        );
        if !self.shard.is_empty()
            && self
                .shard_time
                .is_some_and(|t| t.elapsed().as_secs() <= 150)
        {
            metadata.insert("shard_id".into(), self.shard.clone().into());
        }
        Ok(Bookmark {
            name: "New space bookmark".into(),
            system: self.system.clone(),
            xyz_km: p,
            metadata,
        })
    }
    // Brief OCR gaps should not replace the entire normal-flight marker.
    // Capture/bookmark accuracy and QT alerts retain the strict 2.5s freshness gate.
    fn display_fresh(&self) -> bool {
        self.position.is_some_and(|(t, _)| {
            t.elapsed().as_secs_f64() <= if self.motion.quantum { 2.5 } else { 6.0 }
        })
    }
    pub fn projected(
        &self,
        target: &Bookmark,
        size: [u32; 2],
        fov: f64,
        distant: bool,
    ) -> Option<[f64; 2]> {
        if !self.fresh()
            || self.motion.quantum
            || self.system.trim().is_empty()
            || !self.system.eq_ignore_ascii_case(&target.system)
        {
            return None;
        }
        let (_, camera) = self
            .camera
            .filter(|(t, _)| t.elapsed().as_secs_f64() <= 2.5)?;
        let p = self.ship()?;
        sc_bookmarks::overlay::project_with_range(
            std::array::from_fn(|i| target.xyz_km[i] - p[i]),
            camera,
            size,
            fov,
            distant,
        )
    }
    pub fn marker_angles(&self, target: &Bookmark) -> Option<(f64, f64)> {
        if !self.display_fresh()
            || self.motion.quantum
            || !self.system.eq_ignore_ascii_case(&target.system)
        {
            return None;
        }
        let p = self.ship()?;
        if approach(p, target.xyz_km, self.motion.velocity).distance <= 0.02 {
            return None;
        }
        let (_, c) = self
            .camera
            .filter(|(t, _)| t.elapsed().as_secs_f64() <= 6.)?;
        Some(camera_steering(
            c,
            std::array::from_fn(|i| target.xyz_km[i] - p[i]),
        ))
    }
    pub fn guidance(&self, target: &Bookmark) -> (String, bool) {
        if !self.system.is_empty() && !self.system.eq_ignore_ascii_case(&target.system) {
            return ("Destination is in another system".into(), true);
        }
        let Some(p) = self.ship() else {
            return ("Waiting for live coordinates".into(), true);
        };
        let a = approach(p, target.xyz_km, self.motion.velocity);
        let mut text = format!("{:.2} km", a.distance);
        if !self.display_fresh() {
            return (
                format!("{text} · held estimate\nCoordinates unreadable — guidance paused"),
                true,
            );
        }
        if self.motion.quantum {
            if let Some(t) = a.seconds {
                text.push_str(&if t >= 0. {
                    format!("\nQT closest approach in {t:.1}s")
                } else {
                    "\nPassed closest approach".into()
                });
            }
            text.push_str(if self.alert.fired {
                "\nDROP OUT OF QT"
            } else if self.alert.armed {
                "\nExit alert armed"
            } else {
                "\nExit alert not armed"
            });
        } else if a.distance <= 0.02 {
            text.push_str("\nAT BOOKMARK (within 20 m)");
        } else {
            if let Some((_, c)) = self.camera.filter(|(t, _)| t.elapsed().as_secs_f64() <= 6.) {
                let (yaw, pitch) =
                    camera_steering(c, std::array::from_fn(|i| target.xyz_km[i] - p[i]));
                text.push_str(&if yaw.abs() < 2. && pitch.abs() < 2. {
                    "\nON COURSE".into()
                } else {
                    format!(
                        "\n{} {:.1}°    {} {:.1}°",
                        if yaw >= 0. { "→ RIGHT" } else { "← LEFT" },
                        yaw.abs(),
                        if pitch >= 0. { "↑ UP" } else { "↓ DOWN" },
                        pitch.abs()
                    )
                });
            } else {
                text.push_str("\nWaiting for CamDir");
            }
            if self
                .camera
                .is_some_and(|(t, _)| t.elapsed().as_secs_f64() > 2.5)
            {
                text.push_str("\nHolding last heading");
            } else if !self.fresh() {
                text.push_str("\nPosition estimated");
            }

            if a.closing > 0.0001 {
                let seconds = a.distance / a.closing;
                text.push_str(&format!(
                    "\nETA {} at closing speed",
                    sc_bookmarks::guidance::arrival_eta(seconds)
                ));
            } else {
                text.push_str("\nNot closing on destination");
            }
        }
        (text, false)
    }
    pub fn tick(&mut self, target: Option<&Bookmark>) {
        if let Some(b) = target {
            if b.system.eq_ignore_ascii_case(&self.system) {
                if let Some(p) = self.ship() {
                    let a = approach(p, b.xyz_km, self.motion.velocity);
                    if self
                        .alert
                        .update(self.motion.quantum, self.fresh(), a.seconds, self.lead)
                    {
                        unsafe {
                            let _ = windows::Win32::System::Diagnostics::Debug::MessageBeep(
                                windows::Win32::UI::WindowsAndMessaging::MB_ICONEXCLAMATION,
                            );
                        }
                    }
                }
            }
        }
    }
    pub fn show(&mut self, ui: &mut eframe::egui::Ui, target: &Bookmark, running: bool) {
        let (text, warn) = self.guidance(target);
        if warn {
            ui.weak(text);
        } else {
            ui.label(text);
        }
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_overlay, "Overlay");
            ui.label("Exit lead (s)");
            ui.add(eframe::egui::DragValue::new(&mut self.lead).range(0. ..=120.));
            if ui
                .add_enabled(
                    self.alert.armed || (running && self.fresh()),
                    eframe::egui::Button::new(if self.alert.armed {
                        "Disarm exit alert"
                    } else {
                        "Arm exit alert"
                    }),
                )
                .clicked()
            {
                self.alert.arm(!self.alert.armed);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observation(stamp: Instant, p: [f64; 3]) -> Observation {
        Observation {
            stamp,
            position: Some(p),
            camera: Some([0., 0., 0.]),
            system: Some("Stanton".into()),
            shard: None,
            error: None,
            evidence: Vec::new(),
            desktop: sc_bookmarks::overlay::Rect {
                left: 0,
                top: 0,
                width: 1920,
                height: 1080,
            },
        }
    }
    #[test]
    fn isolated_heading_flip_requires_confirmation_but_real_turn_recovers() {
        let mut live = Live::default();
        let now = Instant::now();
        let mut o = observation(now, [0.; 3]);
        o.camera = Some([-38., 12., 94.]);
        live.ingest(&o);
        o.stamp += Duration::from_millis(250);
        o.camera = Some([38., 12., 94.]);
        live.ingest(&o);
        assert_eq!(live.camera.unwrap().1, [-38., 12., 94.]);
        o.stamp += Duration::from_millis(250);
        o.camera = Some([-38., 12., 94.]);
        live.ingest(&o);
        assert!(live.pending_camera.is_none());
        o.stamp += Duration::from_millis(250);
        o.camera = Some([38., 12., 94.]);
        live.ingest(&o);
        o.stamp += Duration::from_millis(250);
        live.ingest(&o);
        assert_eq!(live.camera.unwrap().1, [38., 12., 94.]);
        assert!(heading_change([0., 0., 179.], [0., 0., -179.]) < 3.);
    }
    #[test]
    fn heading_only_read_keeps_position_age_and_sample_count() {
        let mut live = Live::default();
        let old = Instant::now() - Duration::from_secs(4);
        live.position = Some((old, [1., 2., 3.]));
        let mut o = observation(Instant::now(), [99.; 3]);
        o.position = None;
        o.camera = Some([12., 3., -45.]);
        live.ingest(&o);
        assert_eq!(live.position, Some((old, [1., 2., 3.])));
        assert_eq!(live.camera.unwrap().1, [12., 3., -45.]);
        assert_eq!(live.filter.count(), 0);
        assert!(!live.fresh());
    }
    #[test]
    fn display_grace_does_not_relax_capture_or_qt() {
        let mut live = Live::default();
        live.position = Some((Instant::now() - Duration::from_secs(4), [0.; 3]));
        assert!(live.display_fresh());
        assert!(!live.fresh());
        assert!(live.bookmark().is_err());
        live.motion.quantum = true;
        assert!(!live.display_fresh());
        live.motion.quantum = false;
        live.position = Some((Instant::now() - Duration::from_secs(7), [0.; 3]));
        assert!(!live.display_fresh());
    }
    #[test]
    fn qt_uses_recent_motion_and_stale_input_cannot_alert() {
        let mut live = Live::default();
        live.epoch = Instant::now() - Duration::from_secs(20);
        for i in 0..8 {
            live.ingest(&observation(
                live.epoch + Duration::from_secs(12 + i),
                [i as f64 * 100., 0., 0.],
            ));
        }
        assert!(live.motion.quantum);
        assert!(live.fresh());
        assert!((live.motion.velocity[0] - 100.).abs() < 1e-9);
        assert!(live.ship().unwrap()[0] > 790.);
        let target = Bookmark {
            name: "Target".into(),
            system: "Stanton".into(),
            xyz_km: [1000., 0., 0.],
            metadata: BTreeMap::new(),
        };
        live.alert.arm(true);
        live.position = Some((Instant::now() - Duration::from_secs(10), [700., 0., 0.]));
        live.tick(Some(&target));
        assert!(!live.alert.fired);
        assert!(live.guidance(&target).0.contains("paused"));
    }
    #[test]
    fn scheduled_shard_refresh_preserves_confirmation_but_changed_id_needs_two() {
        let mut live = Live::default();
        let now = Instant::now();
        let mut o = observation(now, [1., 2., 3.]);
        o.shard = Some("pub_use1b_12881860_130".into());
        live.ingest(&o);
        o.stamp += Duration::from_secs(1);
        live.ingest(&o);
        o.stamp += Duration::from_secs(120);
        live.ingest(&o);
        assert_eq!(live.shard, "pub_use1b_12881860_130");
        o.shard = Some("pub_use1b_12881860_131".into());
        o.stamp += Duration::from_secs(120);
        live.ingest(&o);
        assert!(live.shard.is_empty());
        o.stamp += Duration::from_secs(1);
        live.ingest(&o);
        assert_eq!(live.shard, "pub_use1b_12881860_131");
    }
    #[test]
    fn shard_needs_two_matching_reads() {
        let mut live = Live::default();
        let mut o = observation(Instant::now(), [0.; 3]);
        o.shard = Some("pub_use1b_12345678_290".into());
        live.ingest(&o);
        assert!(live.shard.is_empty());
        o.stamp += Duration::from_millis(1);
        live.ingest(&o);
        assert_eq!(live.shard, o.shard.unwrap());
    }

    #[test]
    fn distant_marker_requires_matching_known_system_and_fresh_readings() {
        let mut live = Live::default();
        live.system = "Stanton".into();
        live.position = Some((Instant::now(), [0.; 3]));
        live.camera = Some((Instant::now(), [0.; 3]));
        let b = Bookmark {
            name: "Far".into(),
            system: "Stanton".into(),
            xyz_km: [0., 1_000_000., 0.],
            metadata: BTreeMap::new(),
        };
        assert!(live.projected(&b, [1920, 1080], 60., true).is_some());
        assert!(live.projected(&b, [1920, 1080], 60., false).is_none());
        live.system = "Pyro".into();
        assert!(live.projected(&b, [1920, 1080], 60., true).is_none());
        live.system.clear();
        assert!(live.projected(&b, [1920, 1080], 60., true).is_none());
        live.system = "Stanton".into();
        live.motion.quantum = true;
        assert!(live.projected(&b, [1920, 1080], 60., true).is_none());
        live.motion.quantum = false;
        live.camera = Some((Instant::now() - Duration::from_secs(3), [0.; 3]));
        assert!(live.projected(&b, [1920, 1080], 60., true).is_none());
        live.camera = Some((Instant::now(), [0.; 3]));
        live.position = Some((Instant::now() - Duration::from_secs(3), [0.; 3]));
        assert!(live.projected(&b, [1920, 1080], 60., true).is_none());
    }
}
