//! Foreground / background color state (Phase 9).

use serde::{Deserialize, Serialize};

/// Sample source for the eyedropper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleSource {
    #[default]
    CurrentLayer,
    AllLayers,
}

/// Session color pair + recent colors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorState {
    pub foreground: [f32; 4],
    pub background: [f32; 4],
    pub recent: Vec<[f32; 4]>,
    pub sample_source: SampleSource,
}

impl Default for ColorState {
    fn default() -> Self {
        Self {
            foreground: [0.0, 0.0, 0.0, 1.0],
            background: [1.0, 1.0, 1.0, 1.0],
            recent: Vec::new(),
            sample_source: SampleSource::CurrentLayer,
        }
    }
}

impl ColorState {
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.foreground, &mut self.background);
    }

    pub fn reset_default(&mut self) {
        self.foreground = [0.0, 0.0, 0.0, 1.0];
        self.background = [1.0, 1.0, 1.0, 1.0];
    }

    pub fn set_foreground(&mut self, rgba: [f32; 4]) {
        let c = clamp_rgba(rgba);
        self.foreground = c;
        self.push_recent(c);
    }

    fn push_recent(&mut self, rgba: [f32; 4]) {
        self.recent.retain(|c| c != &rgba);
        self.recent.insert(0, rgba);
        self.recent.truncate(16);
    }

    pub fn to_hex(rgba: [f32; 4]) -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            (rgba[0] * 255.0).round() as u8,
            (rgba[1] * 255.0).round() as u8,
            (rgba[2] * 255.0).round() as u8
        )
    }

    pub fn from_hex(hex: &str) -> Option<[f32; 4]> {
        let hex = hex.trim().trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some([
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            1.0,
        ])
    }
}

fn clamp_rgba(rgba: [f32; 4]) -> [f32; 4] {
    [
        rgba[0].clamp(0.0, 1.0),
        rgba[1].clamp(0.0, 1.0),
        rgba[2].clamp(0.0, 1.0),
        rgba[3].clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let c = [0.2, 0.4, 0.6, 1.0];
        let hex = ColorState::to_hex(c);
        let back = ColorState::from_hex(&hex).expect("hex");
        assert!((back[0] - c[0]).abs() < 0.01);
    }
}
