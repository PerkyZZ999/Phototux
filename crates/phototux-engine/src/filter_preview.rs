//! Ephemeral filter gallery preview session (handbook 15).
//!
//! Preview does not mutate document authority until [`crate::command_id::FILTER_COMMIT`].
//! Commit rejects cancelled tokens and stale document generations.

use crate::cancel::CancelToken;
use crate::layer::{FilterEffect, FilterParams, LayerId};

/// Proposed gallery filter before commit.
#[derive(Debug, Clone)]
pub struct FilterPreviewSession {
    pub layer_id: LayerId,
    /// Effect kind: `gaussian` | `motion` | `emboss` | `sharpen` | `noise`.
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
    #[must_use]
    pub fn to_effect(&self) -> Option<FilterEffect> {
        let params = FilterParams::default_for_kind(&self.kind)?;
        let mut slots = params.slots();
        slots[0] = self.p0;
        slots[1] = self.p1;
        slots[2] = self.p2;
        Some(FilterEffect {
            id: 0,
            name: params.label().to_owned(),
            enabled: true,
            opacity: 1.0,
            blend: crate::BlendMode::Normal,
            params: params.with_slots(slots).clamped(),
        })
    }

    #[must_use]
    pub fn label(&self) -> &'static str {
        FilterParams::default_for_kind(&self.kind).map_or("Filter", |p| p.label())
    }
}

/// Kinds the filter gallery offers.
///
/// The whole vocabulary. This was a hand-written list of five against a
/// thirteen-kind enum, and it was also the gate `kind_is_supported` used — so
/// Box Blur, Invert and Offset were refused by the preview as well as being
/// unrunnable in the plan.
#[must_use]
pub fn gallery_effect_kinds() -> Vec<&'static str> {
    FilterParams::ALL_KINDS
        .iter()
        .map(FilterParams::kind_key)
        .collect()
}

/// Whether the gallery and the preview know this kind.
#[must_use]
pub fn kind_is_supported(kind: &str) -> bool {
    FilterParams::default_for_kind(kind).is_some()
}

/// `{kind: {label, slots: [{label, min, max}]}}` for the gallery chrome.
#[must_use]
pub fn filter_catalog_json() -> String {
    let entries: Vec<serde_json::Value> = FilterParams::ALL_KINDS
        .iter()
        .map(|params| {
            let slots: Vec<serde_json::Value> = params
                .editor_slots()
                .iter()
                .map(|&(label, min, max)| {
                    serde_json::json!({ "label": label, "min": min, "max": max })
                })
                .collect();
            serde_json::json!({
                "id": params.kind_key(),
                "label": params.label(),
                "slots": slots,
                "defaults": params.slots()[..params.editor_slots().len()],
            })
        })
        .collect();
    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into())
}

fn default_params(kind: &str) -> (f32, f32, f32) {
    FilterParams::default_for_kind(kind).map_or((0.0, 0.0, 0.0), |p| {
        let s = p.slots();
        (s[0], s[1], s[2])
    })
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
