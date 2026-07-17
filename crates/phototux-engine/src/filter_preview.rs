//! Ephemeral filter gallery preview session (handbook 15).
//!
//! Preview does not mutate document authority until [`crate::command_id::FILTER_COMMIT`].
//! Commit rejects cancelled tokens and stale document generations.

use crate::cancel::CancelToken;
use crate::layer::{FilterEffect, LayerId, MAX_BLUR_RADIUS};

/// Proposed gallery filter before commit.
#[derive(Debug, Clone)]
pub struct FilterPreviewSession {
    pub layer_id: LayerId,
    /// Effect kind: `gaussian` | `motion` | `emboss` | `sharpen`.
    pub kind: String,
    pub p0: f32,
    pub p1: f32,
    pub p2: f32,
    /// Document generation when preview started (stale if graph moves).
    pub started_generation: u64,
    pub cancel: CancelToken,
}

impl FilterPreviewSession {
    pub fn new(layer_id: LayerId, kind: impl Into<String>, generation: u64) -> Self {
        let kind = kind.into();
        let (p0, p1, p2) = default_params(&kind);
        Self {
            layer_id,
            kind,
            p0,
            p1,
            p2,
            started_generation: generation,
            cancel: CancelToken::new(),
        }
    }

    pub fn is_stale(&self, current_generation: u64) -> bool {
        self.started_generation != current_generation
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    pub fn set_params(&mut self, p0: f32, p1: f32, p2: f32) {
        self.p0 = p0;
        self.p1 = p1;
        self.p2 = p2;
    }

    /// Build a temporary effect for GPU pack overlay (id `0` is preview-only).
    pub fn to_effect(&self) -> Option<FilterEffect> {
        match self.kind.as_str() {
            "gaussian" => Some(FilterEffect::gaussian_blur(
                0,
                self.p0.clamp(0.0, MAX_BLUR_RADIUS),
            )),
            "motion" => Some(FilterEffect::motion_blur(0, self.p0, self.p1)),
            "emboss" => Some(FilterEffect::emboss(0, self.p0, self.p1)),
            "sharpen" => Some(FilterEffect::sharpen(0, self.p0)),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self.kind.as_str() {
            "gaussian" => "Gaussian Blur",
            "motion" => "Motion Blur",
            "emboss" => "Emboss",
            "sharpen" => "Sharpen",
            _ => "Filter",
        }
    }
}

/// Known gallery effect kinds (v1 shipped set).
pub const GALLERY_EFFECT_KINDS: &[&str] = &["gaussian", "motion", "emboss", "sharpen"];

pub fn kind_is_supported(kind: &str) -> bool {
    GALLERY_EFFECT_KINDS.contains(&kind)
}

fn default_params(kind: &str) -> (f32, f32, f32) {
    match kind {
        "gaussian" => (4.0, 0.0, 0.0),
        "motion" => (8.0, 0.0, 0.0),
        "emboss" => (1.0, 135.0, 0.0),
        "sharpen" => (1.0, 0.0, 0.0),
        _ => (0.0, 0.0, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gaussian_effect_from_preview() {
        let preview = FilterPreviewSession::new(LayerId(1), "gaussian", 3);
        let effect = preview.to_effect().expect("effect");
        assert_eq!(effect.name, "Gaussian Blur");
    }
}
