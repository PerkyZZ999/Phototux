//! Brush presets and dynamics (Phase 9).

use serde::{Deserialize, Serialize};

/// Saved brush configuration for the preset panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrushPreset {
    pub name: String,
    pub size: f32,
    pub hardness: f32,
    pub opacity: f32,
    pub flow: f32,
    pub spacing: f32,
    pub smoothing: f32,
    pub size_pressure: bool,
    pub opacity_pressure: bool,
    /// Scatter amount 0..1 (handbook brush dynamics subset).
    #[serde(default)]
    pub scatter: f32,
    /// Tip texture key: `none` | `noise`.
    #[serde(default = "default_texture_kind")]
    pub texture: String,
    /// Texture mix 0..1.
    #[serde(default)]
    pub texture_strength: f32,
    pub color: [f32; 4],
}

fn default_texture_kind() -> String {
    "none".into()
}

impl Default for BrushPreset {
    fn default() -> Self {
        Self {
            name: "Default".into(),
            size: 24.0,
            hardness: 0.85,
            opacity: 1.0,
            flow: 1.0,
            spacing: 0.15,
            smoothing: 0.0,
            size_pressure: true,
            opacity_pressure: false,
            scatter: 0.0,
            texture: default_texture_kind(),
            texture_strength: 0.0,
            color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// In-memory preset library (import/export as JSON).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BrushPresetLibrary {
    pub presets: Vec<BrushPreset>,
}

impl BrushPresetLibrary {
    pub fn with_defaults() -> Self {
        Self {
            presets: vec![
                BrushPreset::default(),
                BrushPreset {
                    name: "Soft Round".into(),
                    hardness: 0.2,
                    ..BrushPreset::default()
                },
                BrushPreset {
                    name: "Hard Round".into(),
                    hardness: 1.0,
                    ..BrushPreset::default()
                },
                BrushPreset {
                    name: "Noise Tip".into(),
                    texture: "noise".into(),
                    texture_strength: 0.55,
                    hardness: 0.7,
                    ..BrushPreset::default()
                },
            ],
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    pub fn apply_index(&self, index: usize) -> Option<&BrushPreset> {
        self.presets.get(index)
    }

    pub fn upsert(&mut self, preset: BrushPreset) {
        if let Some(existing) = self.presets.iter_mut().find(|p| p.name == preset.name) {
            *existing = preset;
        } else {
            self.presets.push(preset);
        }
    }

    pub fn names_joined(&self) -> String {
        self.presets
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join("|")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_json_roundtrip() {
        let lib = BrushPresetLibrary::with_defaults();
        let json = lib.to_json().expect("json");
        let back = BrushPresetLibrary::from_json(&json).expect("parse");
        assert_eq!(back.presets.len(), 3);
    }
}
