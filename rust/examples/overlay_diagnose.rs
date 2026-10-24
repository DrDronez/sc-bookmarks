fn main() {
    let w = windows_capture::window::Window::from_name("Star Citizen ")
        .or_else(|_| windows_capture::window::Window::from_name("Star Citizen"));
    match w {
        Ok(w) => {
            println!("hwnd {}", w.as_raw_hwnd() as usize);
            println!(
                "geometry {:?}",
                sc_bookmarks::window_geometry::query(w.as_raw_hwnd() as usize).map(|g| g.client)
            );
        }
        Err(e) => println!("{e}"),
    };
    let o = sc_bookmarks::native_overlay::Overlays::default();
    o.update(vec![sc_bookmarks::native_overlay::Panel {
        rect: sc_bookmarks::overlay::Rect {
            left: 730,
            top: 400,
            width: 460,
            height: 265,
        },
        title: "SC Bookmarks overlay verification".into(),
        lines: "Observe-only overlay".into(),
        warning: false,
        floating: true,
        angles: Some((17.8, 12.4)),
        theme: Default::default(),
        projected: false,
    }]);
    std::thread::sleep(std::time::Duration::from_secs(2));
    println!("health {:?}", o.health());
}
