//! Document color profile metadata (handbook 16 / DR-012).
//!
//! Assign changes interpretation only. Convert rewrites pixels (separate command).

use serde::{Deserialize, Serialize};

/// Working / document profile identity (not ICC bytes yet).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentColorState {
    /// Profile tag shown in UI (e.g. `sRGB`, `Display-P3`).
    pub assigned_profile: String,
    /// True when pixels were last written assuming `assigned_profile`.
    pub pixels_match_assigned: bool,
}

impl Default for DocumentColorState {
    fn default() -> Self {
        Self {
            assigned_profile: "sRGB".into(),
            pixels_match_assigned: true,
        }
    }
}

impl DocumentColorState {
    /// Assign a profile without rewriting pixels (DR-012).
    pub fn assign_profile(&mut self, profile: impl Into<String>) {
        let next = profile.into();
        if next != self.assigned_profile {
            self.assigned_profile = next;
            self.pixels_match_assigned = false;
        }
    }

    /// Record that a convert operation rewrote pixels to match the assigned profile.
    pub fn mark_converted(&mut self) {
        self.pixels_match_assigned = true;
    }

    /// Prepare convert: assign target profile and return whether a pixel rewrite is needed.
    pub fn begin_convert(&mut self, target: impl Into<String>) -> ConvertPlan {
        let target = target.into();
        let from = self.assigned_profile.clone();
        if from == target && self.pixels_match_assigned {
            return ConvertPlan {
                from,
                to: target,
                rewrite_pixels: false,
            };
        }
        let rewrite_pixels = from != target;
        self.assigned_profile = target.clone();
        ConvertPlan {
            from,
            to: target,
            rewrite_pixels,
        }
    }
}

/// Result of planning a profile convert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertPlan {
    pub from: String,
    pub to: String,
    pub rewrite_pixels: bool,
}

/// Approximate sRGB ↔ Display-P3 linear matrix convert on straight RGBA8.
///
/// Unknown profile pairs leave pixels unchanged (caller still marks converted).
pub fn convert_rgba8_profile(pixels: &mut [u8], from: &str, to: &str) {
    if from == to || pixels.len() < 4 {
        return;
    }
    let matrix = match (from, to) {
        ("sRGB", "Display-P3") => SRGB_TO_P3,
        ("Display-P3", "sRGB") => P3_TO_SRGB,
        _ => return,
    };
    for px in pixels.chunks_exact_mut(4) {
        let r = srgb_eotf(px[0] as f32 / 255.0);
        let g = srgb_eotf(px[1] as f32 / 255.0);
        let b = srgb_eotf(px[2] as f32 / 255.0);
        let nr = matrix[0][0] * r + matrix[0][1] * g + matrix[0][2] * b;
        let ng = matrix[1][0] * r + matrix[1][1] * g + matrix[1][2] * b;
        let nb = matrix[2][0] * r + matrix[2][1] * g + matrix[2][2] * b;
        px[0] = (srgb_oetf(nr.clamp(0.0, 1.0)) * 255.0).round() as u8;
        px[1] = (srgb_oetf(ng.clamp(0.0, 1.0)) * 255.0).round() as u8;
        px[2] = (srgb_oetf(nb.clamp(0.0, 1.0)) * 255.0).round() as u8;
    }
}

// Approx Bradford-adapted matrices (linear light).
const SRGB_TO_P3: [[f32; 3]; 3] = [
    [0.8225, 0.1774, 0.0000],
    [0.0332, 0.9669, 0.0000],
    [0.0171, 0.0724, 0.9108],
];
const P3_TO_SRGB: [[f32; 3]; 3] = [
    [1.2249, -0.2247, 0.0000],
    [-0.0420, 1.0419, 0.0000],
    [-0.0197, -0.0786, 1.0979],
];

fn srgb_eotf(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb_oetf(c: f32) -> f32 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assign_marks_mismatch() {
        let mut c = DocumentColorState::default();
        assert!(c.pixels_match_assigned);
        c.assign_profile("Display-P3");
        assert_eq!(c.assigned_profile, "Display-P3");
        assert!(!c.pixels_match_assigned);
        c.mark_converted();
        assert!(c.pixels_match_assigned);
    }

    #[test]
    fn convert_srgb_to_p3_changes_red() {
        let mut px = [255_u8, 0, 0, 255];
        convert_rgba8_profile(&mut px, "sRGB", "Display-P3");
        assert!(px[0] < 255 || px[1] > 0);
    }
}
