fn main() {
    let o = sc_bookmarks::native_overlay::Overlays::default();
    o.update(vec![sc_bookmarks::native_overlay::Panel {
        rect: sc_bookmarks::overlay::Rect {
            left: 40,
            top: 40,
            width: 380,
            height: 160,
        },
        title: "SC BOOKMARKS / OVERLAY CHECK".into(),
        lines: "Native click-through panel\nNo game input".into(),
        warning: false,
        floating: true,
        angles: Some((17.8, 12.4)),
        theme: Default::default(),
        projected: false,
    }]);
    std::thread::sleep(std::time::Duration::from_secs(1));
    println!("{:?}", o.health());
    assert_eq!(o.health(), Ok(1));
    o.update(Vec::new());
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert_eq!(o.health(), Ok(0));
}
