use sc_bookmarks::{
    Bookmark,
    library::{Filter, Sort},
};
use serde::Deserialize;
#[derive(Deserialize)]
struct Fixture {
    bookmarks: Vec<Bookmark>,
    categories: Vec<String>,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    sort: String,
    category: String,
    system: String,
    ore: String,
    minimum: Option<f64>,
    indices: Vec<usize>,
}
#[test]
fn sorting_and_filtering_match_python() {
    let data: Fixture = serde_json::from_str(include_str!("library_reference.json")).unwrap();
    for c in data.cases {
        let sort = Sort::ALL.into_iter().find(|s| s.label() == c.sort).unwrap();
        let filter = Filter {
            sort,
            category: c.category,
            system: c.system,
            ore: c.ore,
            minimum: c.minimum,
            ..Default::default()
        };
        assert_eq!(
            filter.apply(&data.bookmarks, &data.categories),
            c.indices,
            "{}",
            c.sort
        );
    }
    let filter = Filter {
        query: "  aLpHa  ".into(),
        ..Default::default()
    };
    assert_eq!(filter.apply(&data.bookmarks, &data.categories), vec![0]);
}
