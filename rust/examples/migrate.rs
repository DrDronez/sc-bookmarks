fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("migrate SOURCE TARGET".into());
    }
    sc_bookmarks::protocol::migrate(
        std::path::Path::new(&args[1]),
        std::path::Path::new(&args[2]),
    )
}
