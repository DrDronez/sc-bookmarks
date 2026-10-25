//! Offline signature reference. Loaded once; shared by tables and contact labels.
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    Text(String),
    Number(u64),
}
impl Cell {
    pub fn display(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Number(n) => grouped(*n),
        }
    }
    fn compare(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Number(a), Self::Number(b)) => a.cmp(b),
            (Self::Text(a), Self::Text(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
            (Self::Number(_), _) => Ordering::Less,
            _ => Ordering::Greater,
        }
    }
}
pub fn grouped(n: u64) -> String {
    let text = n.to_string();
    let mut out = String::with_capacity(text.len() + text.len() / 3);
    for (i, c) in text.chars().enumerate() {
        if i > 0 && (text.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}
#[derive(Clone, Debug)]
pub struct Chart {
    pub name: &'static str,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
    pub salvage: bool,
    pub surface: bool,
}
impl Chart {
    fn load(name: &'static str, text: &str, salvage: bool, surface: bool) -> Result<Self, String> {
        let mut reader = csv::Reader::from_reader(text.trim_start_matches('\u{feff}').as_bytes());
        let headers = reader
            .headers()
            .map_err(|e| e.to_string())?
            .iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if headers.len() != if salvage { 2 } else { 7 } {
            return Err(format!("Unexpected columns in {name}"));
        }
        let mut rows = Vec::new();
        for record in reader.records() {
            let record = record.map_err(|e| e.to_string())?;
            rows.push(
                record
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        if i == if salvage { 1 } else { 0 } {
                            Ok(Cell::Text(s.into()))
                        } else {
                            s.parse::<u64>()
                                .map(Cell::Number)
                                .map_err(|e| e.to_string())
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        Ok(Self {
            name,
            headers,
            rows,
            salvage,
            surface,
        })
    }
    pub fn matching_columns(&self, row: usize, rs: u64) -> Vec<usize> {
        self.rows[row]
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match c {
                Cell::Number(v) if *v == rs => Some(i),
                _ => None,
            })
            .collect()
    }
    pub fn filtered(
        &self,
        query: &str,
        rs: Option<u64>,
        sort: Option<(usize, bool)>,
    ) -> Vec<usize> {
        let query = query.trim().to_lowercase();
        let text_column = if self.salvage { 1 } else { 0 };
        let mut result = (0..self.rows.len())
            .filter(|&i| {
                self.rows[i][text_column]
                    .display()
                    .to_lowercase()
                    .contains(&query)
                    && rs.is_none_or(|rs| !self.matching_columns(i, rs).is_empty())
            })
            .collect::<Vec<_>>();
        if let Some((column, descending)) = sort.filter(|(col, _)| *col < self.headers.len()) {
            result.sort_by(|&a, &b| {
                let order = self.rows[a][column].compare(&self.rows[b][column]);
                if descending { order.reverse() } else { order }
            });
        }
        result
    }
    pub fn note(&self) -> &'static str {
        if self.salvage {
            "Listed salvage possibilities; scrap count does not indicate panel size or yield. Wreck values refer to spawned salvage contacts. Other contact types can share an RS reading."
        } else if self.surface {
            "RS = base signature × contributing rocks of the same primary ore (1–6). Totals can include overlapping clusters. Mixed contacts may produce other totals; RS does not reveal quality or yield. Hand/ROC rows identify deposit count, not mineral type."
        } else {
            "RS = base signature × contributing rocks of the same primary ore (1–6). Totals can include overlapping clusters. Mixed contacts may produce other totals; RS does not reveal quality or yield."
        }
    }
}
pub fn charts() -> Result<Vec<Chart>, String> {
    Ok(vec![
        Chart::load(
            "Mining — Space",
            include_str!("../../scanning-data/mining-space.csv"),
            false,
            false,
        )?,
        Chart::load(
            "Mining — Planetary Surface",
            include_str!("../../scanning-data/mining-planetary-surface.csv"),
            false,
            true,
        )?,
        Chart::load(
            "Salvage",
            include_str!("../../scanning-data/salvage.csv"),
            true,
            false,
        )?,
    ])
}
pub fn parse_rs(text: &str) -> Result<Option<u64>, &'static str> {
    let text = text.trim().replace([',', ' '], "");
    if text.is_empty() {
        return Ok(None);
    }
    if !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err("Enter a positive whole RS reading, such as 18,000.");
    }
    text.parse::<u64>()
        .ok()
        .filter(|n| *n > 0)
        .map(Some)
        .ok_or("Enter a positive whole RS reading, such as 18,000.")
}
pub fn description(charts: &[Chart], rs: u64) -> String {
    let mut groups: [Vec<String>; 2] = [Vec::new(), Vec::new()];
    for chart in charts {
        let group = &mut groups[if chart.surface { 0 } else { 1 }];
        for (i, row) in chart.rows.iter().enumerate() {
            for count in chart.matching_columns(i, rs) {
                let texts = if chart.salvage {
                    row[1]
                        .display()
                        .split(" or ")
                        .map(|part| {
                            let part = part.trim();
                            let mut words = part.split_whitespace();
                            match (words.next(), words.next(), words.next(), words.next()) {
                                (Some(n), Some("scrap"), Some(p @ ("piece" | "pieces")), None)
                                    if n.parse::<u64>().is_ok() =>
                                {
                                    format!(
                                        "{n} Salvage Scrap Piece{}",
                                        if p == "pieces" { "s" } else { "" }
                                    )
                                }
                                _ => part.into(),
                            }
                        })
                        .collect::<Vec<_>>()
                } else {
                    let name = row[0].display();
                    vec![if name == "ROC mining" || name == "Hand mining" {
                        format!(
                            "{count} {} Deposit{}",
                            name.trim_end_matches(" mining"),
                            if count == 1 { "" } else { "s" }
                        )
                    } else {
                        format!("{count} {name} Rock{}", if count == 1 { "" } else { "s" })
                    }]
                };
                for text in texts {
                    if !group.iter().any(|s| s.eq_ignore_ascii_case(&text)) {
                        group.push(text);
                    }
                }
            }
        }
    }
    format!(
        "Scan Results for {} Indicate:\nSurface: {}\nSpace  : {}",
        grouped(rs),
        if groups[0].is_empty() {
            "No exact reference match".into()
        } else {
            groups[0].join(" / ")
        },
        if groups[1].is_empty() {
            "No exact reference match".into()
        } else {
            groups[1].join(" / ")
        }
    )
}

/// Compact overlay copy omits contexts without a catalog match.
/// Keep the full reference description for chart/parity consumers.
pub fn overlay_description(charts: &[Chart], rs: u64) -> String {
    let full = description(charts, rs);
    let rows: Vec<_> = full
        .lines()
        .skip(1)
        .filter(|line| !line.ends_with("No exact reference match"))
        .collect();
    if rows.is_empty() {
        String::new()
    } else {
        format!("Scan results · {} RS\n{}", grouped(rs), rows.join("\n"))
    }
}
