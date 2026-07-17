//! Display ICC profile discovery (colord / filesystem / sRGB tag).

use std::path::PathBuf;
use std::process::Command;

/// Result of probing the host display color profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayProfileInfo {
    /// Human-readable name or path tag for soft-proof UI.
    pub name: String,
    /// Absolute path when a profile file was found; empty for tagged sRGB.
    pub path: String,
    /// Discovery source: `colord` | `env` | `xdg` | `srgb`.
    pub source: String,
}

impl DisplayProfileInfo {
    pub fn srgb() -> Self {
        Self {
            name: "sRGB".into(),
            path: String::new(),
            source: "srgb".into(),
        }
    }

    pub fn soft_proof_tag(&self) -> String {
        if !self.path.is_empty() {
            format!("display:{}", self.path)
        } else {
            format!("display:{}", self.name)
        }
    }
}

/// Probe display ICC: colord D-Bus → `COLOUR_PROFILE` / `ICC_PROFILE` env →
/// `~/.local/share/icc` → tagged sRGB.
pub fn discover_display_profile() -> DisplayProfileInfo {
    if let Some(info) = probe_colord() {
        return info;
    }
    if let Some(info) = probe_env() {
        return info;
    }
    if let Some(info) = probe_xdg_icc() {
        return info;
    }
    DisplayProfileInfo::srgb()
}

fn probe_colord() -> Option<DisplayProfileInfo> {
    // Best-effort: colord D-Bus via busctl. Bound the wait so a stuck session bus
    // cannot hang AppSession construction (and therefore QML window creation).
    let output = Command::new("timeout")
        .args([
            "1s",
            "busctl",
            "--user",
            "call",
            "org.freedesktop.ColorManager",
            "/org/freedesktop/ColorManager",
            "org.freedesktop.ColorManager",
            "GetDevices",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    // Look for an absolute .icc/.icm path in the reply blob.
    for token in text.split(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
        if (token.ends_with(".icc") || token.ends_with(".icm")) && token.starts_with('/') {
            let path = PathBuf::from(token);
            if path.is_file() {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Display")
                    .to_owned();
                return Some(DisplayProfileInfo {
                    name,
                    path: token.to_owned(),
                    source: "colord".into(),
                });
            }
        }
    }
    // Colord present but no path parsed — still tag as colord-managed display.
    if text.contains("ColorManager") || text.contains("/org/freedesktop/ColorManager/devices") {
        return Some(DisplayProfileInfo {
            name: "Display (colord)".into(),
            path: String::new(),
            source: "colord".into(),
        });
    }
    None
}

fn probe_env() -> Option<DisplayProfileInfo> {
    for key in ["COLOUR_PROFILE", "ICC_PROFILE", "GDK_ICC_PROFILE"] {
        if let Ok(val) = std::env::var(key) {
            let path = PathBuf::from(val.trim());
            if path.is_file() {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Display")
                    .to_owned();
                return Some(DisplayProfileInfo {
                    name,
                    path: path.display().to_string(),
                    source: "env".into(),
                });
            }
        }
    }
    None
}

fn probe_xdg_icc() -> Option<DisplayProfileInfo> {
    let home = std::env::var_os("HOME")?;
    let dir = PathBuf::from(home).join(".local/share/icc");
    let entries = std::fs::read_dir(&dir).ok()?;
    let mut best: Option<PathBuf> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "icc" || ext == "icm" {
            best = Some(path);
            break;
        }
    }
    let path = best?;
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Display")
        .to_owned();
    Some(DisplayProfileInfo {
        name,
        path: path.display().to_string(),
        source: "xdg".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_always_returns_name() {
        let info = discover_display_profile();
        assert!(!info.name.is_empty());
        assert!(!info.source.is_empty());
        assert!(info.soft_proof_tag().starts_with("display:"));
    }
}
