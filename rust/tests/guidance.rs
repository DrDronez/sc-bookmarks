use sc_bookmarks::guidance::*;
use serde::Deserialize;
#[derive(Deserialize)]
struct Case {
    position: Vec3,
    target: Vec3,
    velocity: Vec3,
    angles: Vec3,
    approach: [f64; 4],
    steering: [f64; 2],
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-8 * b.abs().max(1.), "{a} != {b}");
}
#[test]
fn matches_python_camera_and_approach() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("guidance_reference.json")).unwrap();
    for c in cases {
        let a = approach(c.position, c.target, c.velocity);
        for (x, y) in [a.distance, a.closing, a.seconds.unwrap(), a.miss.unwrap()]
            .into_iter()
            .zip(c.approach)
        {
            near(x, y);
        }
        let (yaw, pitch) = camera_steering(
            c.angles,
            std::array::from_fn(|i| c.target[i] - c.position[i]),
        );
        near(yaw, c.steering[0]);
        near(pitch, c.steering[1]);
    }
}
#[test]
fn stopped_and_bounded_prediction() {
    let a = approach([0.; 3], [1., 0., 0.], [0.; 3]);
    assert!(a.seconds.is_none());
    assert_eq!(a.closing, 0.);
    assert_eq!(estimate([1.; 3], 1., [2.; 3], 100.), ([21.; 3], true));
    assert_eq!(camera_steering([0.; 3], [1., 1., 1.]).0, 45.);
    assert!(camera_steering([0.; 3], [0., 1., 1.]).1 > 0.);
}
#[test]
fn qt_hysteresis_gaps_and_one_shot_alert() {
    let mut m = TravelMotion::default();
    for i in 0..4 {
        m.add(i as f64, [i as f64 * 10., 0., 0.]);
    }
    assert!(m.quantum && m.fresh(3.));
    assert_eq!(m.velocity, [10., 0., 0.]);
    m.add(3., [999.; 3]);
    assert_eq!(m.velocity, [10., 0., 0.]);
    m.add(4., [31., 0., 0.]);
    assert!(m.quantum);
    m.add(5., [32., 0., 0.]);
    assert!(!m.quantum);
    m.add(10., [33., 0., 0.]);
    assert!(!m.fresh(10.));
    assert_eq!(m.velocity, [0.; 3]);
    let mut a = ExitAlert::default();
    a.arm(true);
    assert!(!a.update(true, true, Some(-1.), 3.));
    assert!(!a.update(true, true, Some(10.), 3.));
    assert!(!a.update(true, false, Some(2.), 3.));
    assert!(!a.update(false, true, Some(2.), 3.));
    assert!(a.update(true, true, Some(2.), 3.));
    assert!(!a.update(true, true, Some(1.), 3.));
}

#[test]
fn arrival_eta_formats_hours_without_day_wrapping() {
    use sc_bookmarks::guidance::arrival_eta;
    for (seconds, expected) in [
        (0., "00:00:00"),
        (59.9, "00:00:59"),
        (60., "00:01:00"),
        (3599., "00:59:59"),
        (3600., "01:00:00"),
        (90061., "25:01:01"),
    ] {
        assert_eq!(arrival_eta(seconds), expected);
    }
    assert_eq!(arrival_eta(f64::INFINITY), "—");
    assert_eq!(arrival_eta(-1.), "—");
}
