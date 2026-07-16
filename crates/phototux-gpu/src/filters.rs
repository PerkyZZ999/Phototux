//! Nondestructive GPU filter helpers (Phase 10).

use phototux_engine::{AdjustmentParams, FilterParams};

/// Describe a filter/adjustment pass for the composite planner.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterPass {
    pub label: &'static str,
    pub shader_key: &'static str,
    pub params: [f32; 4],
}

/// Map engine adjustment params to a GPU pass descriptor.
pub fn adjustment_pass(params: &AdjustmentParams) -> FilterPass {
    match *params {
        AdjustmentParams::BrightnessContrast {
            brightness,
            contrast,
        } => FilterPass {
            label: "Brightness/Contrast",
            shader_key: "adjust.brightness_contrast",
            params: [brightness, contrast, 0.0, 0.0],
        },
        AdjustmentParams::Levels {
            black,
            white,
            gamma,
        } => FilterPass {
            label: "Levels",
            shader_key: "adjust.levels",
            params: [black, white, gamma, 0.0],
        },
        AdjustmentParams::HueSaturation {
            hue,
            saturation,
            lightness,
        } => FilterPass {
            label: "Hue/Saturation",
            shader_key: "adjust.hue_sat",
            params: [hue, saturation, lightness, 0.0],
        },
        AdjustmentParams::Invert => FilterPass {
            label: "Invert",
            shader_key: "adjust.invert",
            params: [0.0; 4],
        },
        AdjustmentParams::Threshold { level } => FilterPass {
            label: "Threshold",
            shader_key: "adjust.threshold",
            params: [level, 0.0, 0.0, 0.0],
        },
        AdjustmentParams::Posterize { levels } => {
            #[expect(
                clippy::cast_precision_loss,
                reason = "posterize level counts fit f32 mantissa for UI params"
            )]
            let levels_f = levels as f32;
            FilterPass {
                label: "Posterize",
                shader_key: "adjust.posterize",
                params: [levels_f, 0.0, 0.0, 0.0],
            }
        }
    }
}

/// Map engine filter params to a GPU pass descriptor.
pub fn filter_pass(params: &FilterParams) -> FilterPass {
    match *params {
        FilterParams::GaussianBlur { radius } => FilterPass {
            label: "Gaussian Blur",
            shader_key: "filter.gaussian_blur",
            params: [radius, 0.0, 0.0, 0.0],
        },
        FilterParams::BoxBlur { radius } => FilterPass {
            label: "Box Blur",
            shader_key: "filter.box_blur",
            params: [radius, 0.0, 0.0, 0.0],
        },
        FilterParams::Sharpen { amount } => FilterPass {
            label: "Sharpen",
            shader_key: "filter.sharpen",
            params: [amount, 0.0, 0.0, 0.0],
        },
        FilterParams::Invert => FilterPass {
            label: "Invert",
            shader_key: "filter.invert",
            params: [0.0; 4],
        },
        FilterParams::Offset { x, y } => FilterPass {
            label: "Offset",
            shader_key: "filter.offset",
            params: [x as f32, y as f32, 0.0, 0.0],
        },
    }
}

/// CPU reference invert for golden tests / preview without a full pipeline.
pub fn cpu_invert_rgba(pixels: &mut [u8]) {
    for px in pixels.chunks_exact_mut(4) {
        let [r, g, b, _] = px else { continue };
        *r = 255 - *r;
        *g = 255 - *g;
        *b = 255 - *b;
    }
}

/// CPU reference brightness (additive in 0..1 space).
pub fn cpu_brightness_rgba(pixels: &mut [u8], brightness: f32) {
    let delta = (brightness.clamp(-1.0, 1.0) * 255.0).round();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "brightness clamped to [-1, 1]; delta fits i32"
    )]
    let delta = delta as i32;
    for px in pixels.chunks_exact_mut(4) {
        for c in &mut px[..3] {
            let next = i32::from(*c) + delta;
            *c = u8::try_from(next.clamp(0, 255)).unwrap_or(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invert_flips_rgb() {
        let mut px = [10_u8, 20, 30, 255];
        cpu_invert_rgba(&mut px);
        assert_eq!(&px[..3], &[245, 235, 225]);
    }

    #[test]
    fn adjustment_maps_invert() {
        let pass = adjustment_pass(&AdjustmentParams::Invert);
        assert_eq!(pass.shader_key, "adjust.invert");
    }
}
