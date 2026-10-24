use crate::{Bookmark, mining};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    Category,
    Name,
    System,
    Newest,
    OreType,
    OreQuality,
}
impl Sort {
    pub const ALL: [Self; 6] = [
        Self::Category,
        Self::Name,
        Self::System,
        Self::Newest,
        Self::OreType,
        Self::OreQuality,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Category => "Category",
            Self::Name => "Name",
            Self::System => "System",
            Self::Newest => "Newest",
            Self::OreType => "Ore type",
            Self::OreQuality => "Ore quality",
        }
    }
}
pub fn category(b: &Bookmark) -> &str {
    b.metadata
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("Personal")
}
pub fn categories(bookmarks: &[Bookmark]) -> Vec<String> {
    let mut result = vec![
        "Mining".into(),
        "Salvage".into(),
        "Personal".into(),
        "Corp".into(),
    ];
    for b in bookmarks {
        let cat = category(b);
        if !result.iter().any(|v| v == cat) {
            result.push(cat.into());
        }
    }
    result
}
pub fn best_quality(b: &Bookmark, ore: &str) -> f64 {
    b.metadata
        .get("mining")
        .and_then(|v| v["composition"].as_array())
        .into_iter()
        .flatten()
        .filter(|r| ore.is_empty() || r["ore"].as_str() == Some(ore))
        .filter_map(|r| r["quality"].as_f64())
        .fold(-1., f64::max)
}
fn first_ore<'a>(b: &'a Bookmark, ore: &str) -> &'a str {
    b.metadata
        .get("mining")
        .and_then(|v| v["composition"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|r| r["ore"].as_str())
        .filter(|s| ore.is_empty() || *s == ore)
        .min()
        .unwrap_or("~")
}
#[derive(Default)]
pub struct Filter {
    pub query: String,
    pub category: String,
    pub system: String,
    pub ore: String,
    pub minimum: Option<f64>,
    pub sort: Sort,
}
impl Filter {
    /// Returns source indices, so sorting/filtering never changes edit/delete identity.
    pub fn apply(&self, bookmarks: &[Bookmark], categories: &[String]) -> Vec<usize> {
        let query = self.query.trim().to_lowercase();
        let system = self.system.trim().to_lowercase();
        let mut result = bookmarks
            .iter()
            .enumerate()
            .filter_map(|(i, b)| {
                ((self.category.is_empty() || category(b) == self.category)
                    && (system.is_empty() || b.system.to_lowercase() == system)
                    && format!("{} {}", b.name, b.system)
                        .to_lowercase()
                        .contains(&query)
                    && mining::matches(b, &self.ore, self.minimum))
                .then_some(i)
            })
            .collect::<Vec<_>>();
        result.sort_by(|&a, &b| {
            let a = &bookmarks[a];
            let b = &bookmarks[b];
            match self.sort {
                Sort::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                Sort::System => (a.system.to_lowercase(), a.name.to_lowercase())
                    .cmp(&(b.system.to_lowercase(), b.name.to_lowercase())),
                Sort::Newest => b
                    .metadata
                    .get("utc")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .cmp(a.metadata.get("utc").and_then(|v| v.as_str()).unwrap_or("")),
                _ => {
                    let rank = |b: &Bookmark| {
                        categories
                            .iter()
                            .position(|c| c == category(b))
                            .unwrap_or(categories.len())
                    };
                    (rank(a), a.name.to_lowercase()).cmp(&(rank(b), b.name.to_lowercase()))
                }
            }
        });
        match self.sort {
            Sort::OreQuality => result.sort_by(|&a, &b| {
                best_quality(&bookmarks[b], &self.ore)
                    .total_cmp(&best_quality(&bookmarks[a], &self.ore))
            }),
            Sort::OreType => result.sort_by(|&a, &b| {
                let a = &bookmarks[a];
                let b = &bookmarks[b];
                first_ore(a, &self.ore)
                    .cmp(first_ore(b, &self.ore))
                    .then_with(|| best_quality(b, &self.ore).total_cmp(&best_quality(a, &self.ore)))
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            }),
            _ => {}
        }
        result
    }
}
