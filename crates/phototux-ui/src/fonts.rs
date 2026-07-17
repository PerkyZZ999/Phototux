//! Host font family discovery (fontconfig / `fc-list`).

use std::collections::BTreeSet;
use std::process::Command;

/// Preferred fallbacks when discovery yields nothing usable.
pub const FALLBACK_FONTS: &[&str] = &["Noto Sans", "DejaVu Sans", "Noto Sans Mono"];

/// Discover installed font family names via `fc-list`.
///
/// Returns a JSON array of unique family strings. On failure, returns the
/// fallback list so the Character panel always has usable entries.
pub fn discover_font_families_json() -> String {
    let families = discover_font_families();
    serde_json::to_string(&families)
        .unwrap_or_else(|_| serde_json::to_string(&FALLBACK_FONTS).unwrap_or_else(|_| "[]".into()))
}

fn discover_font_families() -> Vec<String> {
    let output = Command::new("fc-list").args([":", "family"]).output();
    let Ok(output) = output else {
        return FALLBACK_FONTS.iter().map(|s| (*s).to_owned()).collect();
    };
    if !output.status.success() {
        return FALLBACK_FONTS.iter().map(|s| (*s).to_owned()).collect();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        // fc-list may emit comma-separated aliases per line.
        for part in line.split(',') {
            let name = part.trim();
            if name.is_empty() || name.len() > 80 {
                continue;
            }
            // Skip stylistic face suffixes that confuse QML font.family.
            if name.contains(':') {
                continue;
            }
            set.insert(name.to_owned());
        }
    }
    if set.is_empty() {
        return FALLBACK_FONTS.iter().map(|s| (*s).to_owned()).collect();
    }
    let mut list: Vec<String> = set.into_iter().collect();
    // Cap for ComboBox responsiveness; keep fallbacks pinned at the front.
    if list.len() > 400 {
        list.truncate(400);
    }
    for fb in FALLBACK_FONTS.iter().rev() {
        list.retain(|f| f != *fb);
        list.insert(0, (*fb).to_owned());
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_returns_json_array() {
        let json = discover_font_families_json();
        let parsed: Vec<String> = serde_json::from_str(&json).expect("json array");
        assert!(!parsed.is_empty());
        assert!(
            parsed
                .iter()
                .any(|f| f == "Noto Sans" || f == "DejaVu Sans")
        );
    }
}
