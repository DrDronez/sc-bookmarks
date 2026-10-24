use sc_bookmarks::overlay::*;
use serde::Deserialize;
#[derive(Deserialize)]
struct Case {
    rect: [i32; 4],
    position: [f64; 2],
    expected: [i32; 4],
    debug: [u32; 4],
}
#[test]
fn placement_matches_python_across_windows_and_resolutions() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("overlay_reference.json")).unwrap();
    for c in cases {
        let [left, top, w, h] = c.rect;
        let rect = Rect {
            left,
            top,
            width: w as u32,
            height: h as u32,
        };
        let result = placement(rect, c.position).unwrap();
        assert_eq!(
            [
                result.left,
                result.top,
                result.width as i32,
                result.height as i32
            ],
            c.expected
        );
        assert_eq!(debug_box(rect.width, rect.height), c.debug);
        assert!(
            result.left >= left
                && result.top >= top
                && result.left + result.width as i32 <= left + w
                && result.top + result.height as i32 <= top + h
        );
    }
    assert!(
        placement(
            Rect {
                left: 0,
                top: 0,
                width: 1920,
                height: 1080
            },
            [f64::NAN, 50.]
        )
        .is_err()
    );
    assert_eq!(drag_position([1920, 1080], [960., 540.]), [50., 50.]);
    assert_eq!(drag_position([1920, 1080], [-100., 2000.]), [0., 100.]);
}
#[test]
fn compatible_settings_preserve_unknown_fields_and_corrupt_file() {
    let dir = std::env::temp_dir().join(format!("scbm-overlay-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("overlay-settings.json");
    let original=br#"{"position":[50.0,12.5],"presets":{"Prospector":[25,75],"Wide":[100,0]},"future":true}"#;
    std::fs::write(&path, original).unwrap();
    let mut settings = Settings::load(&path).unwrap();
    settings.position = [75., 50.];
    settings.save(&path).unwrap();
    assert_eq!(Settings::load(&path).unwrap(), settings);
    assert_eq!(settings.extra["future"], true);
    assert_eq!(
        std::fs::read(path.with_extension("json.bak")).unwrap(),
        original
    );
    std::fs::write(&path, b"{bad").unwrap();
    assert!(Settings::load(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"{bad");
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(path.with_extension("json.bak")).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn projected_marker_handles_aspect_roll_strafe_and_range() {
    let center = project([0., 1., 0.], [0.; 3], [1920, 1080], 60.).unwrap();
    assert_eq!(center, [960., 540.]);
    // Lateral translation changes the target bearing without any turn.
    assert!(project([0.1, 1., 0.], [0.; 3], [1920, 1080], 60.).unwrap()[0] > 960.);
    assert!(project([0., 1., 0.1], [0.; 3], [1920, 1080], 60.).unwrap()[1] < 540.);
    for size in [
        [1280, 720],
        [1920, 1080],
        [3440, 1440],
        [3840, 2160],
        [5120, 1440],
    ] {
        let p = project([0., 1., 0.], [0.; 3], size, 60.).unwrap();
        assert_eq!(p, [size[0] as f64 / 2., size[1] as f64 / 2.]);
    }
    // Camera basis includes roll; world right becomes camera up at +90° Y rotation.
    let p = project([0.1, 1., 0.], [0., 90., 0.], [1920, 1080], 60.).unwrap();
    assert!((p[0] - 960.).abs() < 0.001 && p[1] < 540.);
    assert!(project([0., -1., 0.], [0.; 3], [1920, 1080], 60.).is_none());
    assert!(project([0., 5.001, 0.], [0.; 3], [1920, 1080], 60.).is_none());
    assert!(project([5., 0.1, 0.], [0.; 3], [1920, 1080], 60.).is_none());
    assert!(project([f64::NAN, 1., 0.], [0.; 3], [1920, 1080], 60.).is_none());
}
#[test]
fn new_overlay_settings_default_off_and_validate_hotkey() {
    let s: Settings = serde_json::from_str(r#"{"position":[50,50]}"#).unwrap();
    assert!(!s.bookmark_3d);
    assert_eq!(s.theme, Theme::Classic);
    assert_eq!(s.hotkey, 0x79);
    let mut s = s;
    s.hotkey = 0xffff;
    assert!(s.validate().is_err());
    s.hotkey = 0;
    s.vertical_fov = f64::NAN;
    assert!(s.validate().is_err());
}
