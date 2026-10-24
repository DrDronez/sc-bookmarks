//! Parse recognized text; inference and pixel preprocessing are separate stages.
use regex::Regex;
use std::sync::LazyLock;
static UNIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)([-+]?\d+\.\d{2,4})\s*(km|m)").unwrap());
static CAMERA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)Ca[mn].{0,2}ir\s*:\s*(-?\d{1,3})\s+(-?\d{1,3})\s+(-?\d{1,3})").unwrap()
});
static SHARD_LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)Shard[Iil1]d\s*:\s*(.+?)\s*$").unwrap());
static SHARD_VALUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z]+[_-][A-Za-z0-9]+[_-]\d{5,}[_-]\d{3,}$").unwrap());
static SYSTEM_LABEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)Current\s+player\s+location|SolarSystem|AsteroidCluster").unwrap()
});
static SYSTEM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)Nyx|Stanton|Pyro").unwrap());
static DUPLICATE_MINUS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(^|\s|km|m)--(\d)").unwrap());
static DECIMAL_SPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(\d)\.\s+(\d{4}\s*km|\d{2}\s*m)").unwrap());
pub fn parse_units(text: &str) -> Result<[f64; 3], &'static str> {
    let text = text.replace(',', ".").replace('−', "-");
    // Preserve observed signs/digits; normalize only duplicated sign strokes
    // and spacing inside a number with the required printed unit precision.
    let text = DUPLICATE_MINUS.replace_all(&text, "${1}-${2}");
    let text = DECIMAL_SPACE.replace_all(&text, "${1}.${2}");
    let mut values = Vec::with_capacity(3);
    for cap in UNIT.captures_iter(&text) {
        let whole = cap.get(0).unwrap();
        let before = text[..whole.start()].chars().next_back();
        // In the reference, a unit directly followed by a number gains a space.
        let previous_unit = text[..whole.start()].to_ascii_lowercase().ends_with('m');
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || ".+-".contains(c))
            && !previous_unit
        {
            continue;
        }
        if text[whole.end()..]
            .chars()
            .next()
            .is_some_and(|c| !c.is_whitespace() && !c.is_ascii_digit() && c != '-' && c != '+')
        {
            continue;
        }
        let km = cap[2].eq_ignore_ascii_case("km");
        let expected = if km { 4 } else { 2 };
        if cap[1].split('.').nth(1).unwrap().len() != expected {
            return Err("Coordinate precision unclear");
        }
        let n =
            cap[1].parse::<f64>().map_err(|_| "Invalid coordinate")? / if km { 1. } else { 1000. };
        if !n.is_finite() {
            return Err("Invalid coordinate");
        }
        values.push(n);
    }
    values
        .try_into()
        .map_err(|_| "Need three coordinate values with units")
}
pub fn camera(text: &str, confidence: f64) -> Option<[f64; 3]> {
    if !confidence.is_finite() || confidence < 0.75 {
        return None;
    }
    let c = CAMERA.captures(text)?;
    let end = c.get(0)?.end();
    if text[end..]
        .chars()
        .next()
        .is_some_and(|v| !v.is_whitespace() && v != 'F')
    {
        return None;
    }
    let values = [
        c[1].parse::<f64>().ok()?,
        c[2].parse::<f64>().ok()?,
        c[3].parse::<f64>().ok()?,
    ];
    values
        .iter()
        .all(|v| (-180. ..=180.).contains(v))
        .then_some(values)
}
// OCR can turn underscore strokes into l/I/| or omit them. Only recover
// boundaries around a fully read region token and two separated numeric fields.
// Never substitute a digit/letter within the region, build or instance value.
static SHARD_BOUNDARIES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
    r"(?i)^(pub|ptu|eptu)[_\s|lI-]*([a-z]{2,4}[0-9][a-z])[_\s|lI-]*([0-9]{5,})[_\s-]+([0-9]{3,})$"
).unwrap()
});
pub fn shard(text: &str, confidence: f64) -> Option<String> {
    if !confidence.is_finite() || confidence < 0.85 {
        return None;
    }
    let captures = SHARD_LABEL.captures(text)?;
    let value = captures[1].split_whitespace().collect::<Vec<_>>().join("_");
    if SHARD_VALUE.is_match(&value) {
        return Some(value);
    }
    let parts = SHARD_BOUNDARIES.captures(captures[1].trim())?;
    Some(format!(
        "{}_{}_{}_{}",
        parts[1].to_ascii_lowercase(),
        parts[2].to_ascii_lowercase(),
        &parts[3],
        &parts[4]
    ))
}
pub fn system(text: &str, confidence: f64) -> Option<&'static str> {
    if !confidence.is_finite() || confidence < 0.8 || !SYSTEM_LABEL.is_match(text) {
        return None;
    }
    for found in SYSTEM.find_iter(text) {
        if text[..found.start()]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_alphabetic())
        {
            continue;
        }
        let rest = &text[found.end()..];
        if rest.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && !rest.to_ascii_lowercase().starts_with("solarsystem")
        {
            continue;
        }
        return Some(match found.as_str().to_ascii_lowercase().as_str() {
            "nyx" => "Nyx",
            "pyro" => "Pyro",
            _ => "Stanton",
        });
    }
    None
}
/// Adaptive layout requires both independent printed coordinate rows; the fixed
/// layout preserves the reference's allowance for a single high-confidence row.
pub fn coordinates(
    solar: Option<(&str, f64)>,
    root: Option<(&str, f64)>,
    adaptive: bool,
) -> Result<[f64; 3], &'static str> {
    let positions: Vec<_> = [solar, root]
        .into_iter()
        .flatten()
        .filter(|(_, score)| score.is_finite() && *score >= 0.82)
        .filter_map(|(text, _)| parse_units(text).ok())
        .collect();
    match positions.as_slice() {
        [a, b] => {
            if (0..3).any(|i| (a[i] - b[i]).abs() > 0.002) {
                Err("Printed coordinate rows disagree")
            } else {
                Ok(std::array::from_fn(|i| (a[i] + b[i]) / 2.))
            }
        }
        [p] if !adaptive => Ok(*p),
        _ => Err("Coordinate rows unreadable"),
    }
}
/// Cached shard values require three complete corroborating OCR reads, not a
/// fallback solely because the current shard read failed.
pub fn corroborated_shard<'a>(known: &'a [String], reads: &[(&str, f64)]) -> Option<&'a str> {
    fn normalized(s: &str) -> String {
        s.to_ascii_lowercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| if c == 's' { '5' } else { c })
            .collect()
    }
    let winners: Vec<_> = known
        .iter()
        .filter(|s| SHARD_VALUE.is_match(s))
        .filter(|s| {
            let expected = normalized(s);
            reads
                .iter()
                .filter(|(text, score)| {
                    score.is_finite() && *score >= 0.8 && normalized(text) == expected
                })
                .count()
                >= 3
        })
        .collect();
    if winners.len() == 1 {
        Some(winners[0])
    } else {
        None
    }
}

/// Require corroboration from the two separately printed coordinate rows.
/// Multiple renderings may rescue a row, but competing agreements are rejected.
pub fn coordinate_consensus(
    solar: &[(String, f64)],
    root: &[(String, f64)],
) -> Result<[f64; 3], &'static str> {
    let candidates = |rows: &[(String, f64)]| {
        rows.iter()
            .filter(|(_, c)| c.is_finite() && *c >= 0.82)
            .filter_map(|(s, _)| parse_units(s).ok())
            .collect::<Vec<_>>()
    };
    let mut winner: Option<[f64; 3]> = None;
    for a in candidates(solar) {
        for b in candidates(root) {
            if (0..3).all(|i| (a[i] - b[i]).abs() <= 0.002) {
                let p = std::array::from_fn(|i| (a[i] + b[i]) / 2.);
                if winner.is_some_and(|old| (0..3).any(|i| (old[i] - p[i]).abs() > 0.002)) {
                    return Err("Ambiguous coordinate readings");
                }
                winner = Some(p);
            }
        }
    }
    winner.ok_or("Printed coordinate rows could not be corroborated")
}
#[cfg(test)]
mod consensus_tests {
    use super::*;
    fn r(x: &str) -> (String, f64) {
        (format!("{x}km -19064547.2928km 493.0179km"), 0.9)
    }
    #[test]
    fn normalizes_duplicate_sign_and_decimal_spacing_without_guessing_digits() {
        assert_eq!(
            parse_units("7159952. 7384km--19064547.2928km 493.0179km").unwrap(),
            [7159952.7384, -19064547.2928, 493.0179]
        );
        assert!(parse_units("7159952./384km -19064547.2928km 493.0179km").is_err());
        assert!(parse_units("7159952.7384km +-19064547.2928km 493.0179km").is_err());
    }
    #[test]
    fn rescues_digit_only_when_other_printed_row_agrees() {
        assert!(coordinate_consensus(&[r("7159952.1384")], &[r("7159952.7384")]).is_err());
        assert_eq!(
            coordinate_consensus(
                &[r("7159952.1384"), r("7159952.7384")],
                &[r("7159952.7384")]
            )
            .unwrap()[0],
            7159952.7384
        );
        assert!(
            coordinate_consensus(
                &[r("7159952.1384"), r("7159952.7384")],
                &[r("7159952.1384"), r("7159952.7384")]
            )
            .is_err()
        );
        assert!(coordinate_consensus(&[], &[r("7159952.7384")]).is_err());
    }
}
