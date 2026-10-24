fn main() {
    println!("cargo:rerun-if-changed=assets/sc-bookmarks.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/sc-bookmarks.ico")
            .set("ProductName", "SC Bookmarks")
            .set("FileDescription", "SC Bookmarks — navigation and bookmarks")
            .compile()
            .expect("Compile Windows application icon");
    }
}
