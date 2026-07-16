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

/// CPU reference levels remap (matches GPU `apply_levels` intent).
pub fn cpu_levels_rgba(pixels: &mut [u8], black: f32, white: f32, gamma: f32) {
    let black = black.clamp(0.0, 1.0);
    let white = white.clamp(0.0, 1.0).max(black + 1e-4);
    let gamma = gamma.clamp(0.01, 10.0);
    let span = white - black;
    for px in pixels.chunks_exact_mut(4) {
        for c in &mut px[..3] {
            let mut t = (f32::from(*c) / 255.0 - black) / span;
            t = t.clamp(0.0, 1.0).powf(1.0 / gamma);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "levels output clamped to 0..255 before cast"
            )]
            let v = (t * 255.0).round().clamp(0.0, 255.0) as u8;
            *c = v;
        }
    }
}

/// Separable box-ish Gaussian approximation for unit tests (horizontal then vertical).
pub fn cpu_gaussian_rgba(pixels: &mut [u8], width: u32, height: u32, radius: f32) {
    let radius = radius.clamp(0.0, 64.0);
    if radius < 0.01 || width == 0 || height == 0 {
        return;
    }
    let w = width as usize;
    let h = height as usize;
    let support = radius.ceil() as usize;
    let sigma = (radius * 0.5).max(0.5);
    let two_sigma2 = 2.0 * sigma * sigma;
    let mut weights = Vec::with_capacity(support * 2 + 1);
    let mut wsum = 0.0_f32;
    for i in -(support as i32)..=support as i32 {
        let weight = (-(i * i) as f32 / two_sigma2).exp();
        weights.push(weight);
        wsum += weight;
    }
    for weight in &mut weights {
        *weight /= wsum;
    }

    let mut temp = pixels.to_vec();
    // Horizontal
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0_f32; 4];
            for (k, &weight) in weights.iter().enumerate() {
                let ox = (x as i32 + k as i32 - support as i32).clamp(0, w as i32 - 1) as usize;
                let idx = (y * w + ox) * 4;
                for c in 0..4 {
                    acc[c] += f32::from(pixels[idx + c]) * weight;
                }
            }
            let out = (y * w + x) * 4;
            for c in 0..4 {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "blur accumulator clamped to byte"
                )]
                {
                    temp[out + c] = acc[c].round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
    // Vertical
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0_f32; 4];
            for (k, &weight) in weights.iter().enumerate() {
                let oy = (y as i32 + k as i32 - support as i32).clamp(0, h as i32 - 1) as usize;
                let idx = (oy * w + x) * 4;
                for c in 0..4 {
                    acc[c] += f32::from(temp[idx + c]) * weight;
                }
            }
            let out = (y * w + x) * 4;
            for c in 0..4 {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "blur accumulator clamped to byte"
                )]
                {
                    pixels[out + c] = acc[c].round().clamp(0.0, 255.0) as u8;
                }
            }
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

    #[test]
    fn levels_pulls_midtones() {
        let mut px = [128_u8, 128, 128, 255];
        cpu_levels_rgba(&mut px, 0.0, 1.0, 2.0);
        assert!(px[0] > 128);
    }

    #[test]
    fn gaussian_softens_edge() {
        let mut px = vec![0_u8; 9 * 4];
        px[4 * 4] = 255;
        px[4 * 4 + 1] = 255;
        px[4 * 4 + 2] = 255;
        px[4 * 4 + 3] = 255;
        cpu_gaussian_rgba(&mut px, 3, 3, 1.0);
        assert!(px[0] > 0);
        assert!(px[4 * 4] < 255);
    }
}
