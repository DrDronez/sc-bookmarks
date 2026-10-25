//! Read-only startup release check. No credentials or user data are sent.
use semver::Version;
use serde::Deserialize;

pub const REPOSITORY: &str = "https://github.com/DrDronez/sc-bookmarks";
const RELEASES_API: &str =
    "https://api.github.com/repos/DrDronez/sc-bookmarks/releases?per_page=100";

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    body: Option<String>,
}

#[derive(Debug)]
pub struct Update {
    pub version: String,
    pub changes: Vec<String>,
}

// Tags may use GitHub's conventional v prefix. Compare semantic versions, not text.
fn version(tag: &str) -> Option<Version> {
    Version::parse(tag.trim().trim_start_matches('v')).ok()
}

pub fn available(json: &[u8], installed: &str) -> Result<Option<Update>, String> {
    let installed = version(installed).ok_or("Invalid installed version")?;
    let releases: Vec<Release> = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    // Alpha builds are published as prereleases, so /releases/latest misses them.
    let newest = releases
        .into_iter()
        .filter(|r| !r.draft)
        .filter_map(|r| {
            let v = version(&r.tag_name)?;
            (v.cmp_precedence(&installed).is_gt()).then_some((v, r))
        })
        .max_by(|(a, _), (b, _)| a.cmp_precedence(b));
    Ok(newest.map(|(_, release)| Update {
        version: release.tag_name,
        changes: summarize(release.body.as_deref().unwrap_or_default()),
    }))
}

fn concise(line: &str) -> String {
    let text = line.replace(['*', '`'], "");
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= 120 {
        return text;
    }
    let prefix: String = text.chars().take(117).collect();
    let end = prefix.rfind(' ').unwrap_or(prefix.len());
    format!("{}…", &prefix[..end])
}

pub fn summarize(body: &str) -> Vec<String> {
    let mut bullets = Vec::new();
    let mut fenced = false;
    for line in body.lines().map(str::trim) {
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        // Stop at the next section once change bullets have been collected.
        if !bullets.is_empty() && line.starts_with('#') {
            break;
        }
        if let Some(text) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            let text = concise(text);
            if !text.is_empty() {
                bullets.push(text);
            }
            if bullets.len() == 3 {
                break;
            }
        }
    }
    if bullets.is_empty() {
        bullets.push("See the repository for release details.".into());
    }
    bullets
}

pub fn check() -> Result<Option<Update>, String> {
    // Use the same Windows HTTPS client as the optional refinery-data refresh.
    let mut cmd = std::process::Command::new("curl.exe");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let output = cmd
        .args([
            "--disable",
            "--fail",
            "--silent",
            "--show-error",
            "--connect-timeout",
            "5",
            "--max-time",
            "15",
            "--max-filesize",
            "2000000",
            "--proto",
            "=https",
            "--user-agent",
            concat!("SC-Bookmarks/", env!("CARGO_PKG_VERSION")),
            "--header",
            "Accept: application/vnd.github+json",
            "--url",
            RELEASES_API,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() || output.stdout.len() > 2_000_000 {
        return Err("Release check unavailable".into());
    }
    available(&output.stdout, env!("CARGO_PKG_VERSION"))
}
