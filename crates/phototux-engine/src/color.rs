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

    pub fn set_background(&mut self, rgba: [f32; 4]) {
        self.background = clamp_rgba(rgba);
    }

    /// Pipe-joined `#RRGGBB` recent colors for QML.
    pub fn recent_hex_joined(&self) -> String {
        self.recent
            .iter()
            .copied()
            .map(Self::to_hex)
            .collect::<Vec<_>>()
            .join("|")
    }

    fn push_recent(&mut self, rgba: [f32; 4]) {
        self.recent.retain(|c| !rgba_nearly_equal(*c, rgba));
        self.recent.insert(0, rgba);
        self.recent.truncate(16);
    }

    pub fn to_hex(rgba: [f32; 4]) -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            channel_to_u8(rgba[0]),
            channel_to_u8(rgba[1]),
            channel_to_u8(rgba[2])
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

fn channel_to_u8(channel: f32) -> u8 {
    let scaled = (channel.clamp(0.0, 1.0) * 255.0).round();
    // Channel is clamped to [0, 1]; scaled fits in 0..=255.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "channel clamped to unit interval before u8 conversion"
    )]
    {
        scaled as u8
    }
}

fn rgba_nearly_equal(a: [f32; 4], b: [f32; 4]) -> bool {
    const EPS: f32 = 1e-5;
    a.iter()
        .zip(b.iter())
        .all(|(lhs, rhs)| (lhs - rhs).abs() <= EPS)
}

/// RGB → HSL, all components in 0..1 (hue wraps).
///
/// The pivot the Hue/Saturation adjustment turns on, and the reference the
/// WGSL in `phototux_gpu::composite` mirrors — the parity fixture sweeps every
/// adjustment kind against this on a real device. It lived in `layer.rs`, two
/// of that file's twenty concepts and the only two with no layer in them, so a
/// shader comment saying "mirrors phototux_engine::rgb_to_hsl" sent a reader
/// to the layer module. `pub(crate)`, not `pub`: it was never re-exported from
/// the crate root, so the wider visibility advertised an API nobody had.
#[must_use]
pub(crate) fn rgb_to_hsl(rgb: [f32; 3]) -> [f32; 3] {
    let max = rgb[0].max(rgb[1]).max(rgb[2]);
    let min = rgb[0].min(rgb[1]).min(rgb[2]);
    let l = (max + min) * 0.5;
    let span = max - min;
    if span <= f32::EPSILON {
        return [0.0, 0.0, l];
    }
    let s = if l > 0.5 {
        span / (2.0 - max - min)
    } else {
        span / (max + min)
    };
    let h = if max == rgb[0] {
        ((rgb[1] - rgb[2]) / span).rem_euclid(6.0)
    } else if max == rgb[1] {
        (rgb[2] - rgb[0]) / span + 2.0
    } else {
        (rgb[0] - rgb[1]) / span + 4.0
    };
    [(h / 6.0).rem_euclid(1.0), s, l]
}

/// HSL → RGB, inverse of [`rgb_to_hsl`].
#[must_use]
pub(crate) fn hsl_to_rgb(hsl: [f32; 3]) -> [f32; 3] {
    let [h, s, l] = hsl;
    if s <= f32::EPSILON {
        return [l; 3];
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let channel = |mut t: f32| {
        t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    [channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0)]
}

#[cfg(test)]
mod tests {

    /// HSL is the pivot the Hue/Saturation adjustment turns on, so a colour
    /// must survive the round trip it makes on every pixel.
    #[test]
    fn hsl_round_trips() {
        for rgb in [
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.5, 0.5, 0.5],
            [0.9, 0.2, 0.35],
            [0.1, 0.7, 0.4],
            [0.25, 0.3, 0.95],
        ] {
            let back = hsl_to_rgb(rgb_to_hsl(rgb));
            for (a, b) in back.iter().zip(rgb) {
                assert!((a - b).abs() < 1e-4, "{rgb:?} -> {back:?}");
            }
        }
    }
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let c = [0.2, 0.4, 0.6, 1.0];
        let hex = ColorState::to_hex(c);
        let back = ColorState::from_hex(&hex).expect("hex");
        assert!((back[0] - c[0]).abs() < 0.01);
    }

    #[test]
    fn recent_hex_joined_lists_foreground_picks() {
        let mut colors = ColorState::default();
        colors.set_foreground([1.0, 0.0, 0.0, 1.0]);
        colors.set_foreground([0.0, 1.0, 0.0, 1.0]);
        let joined = colors.recent_hex_joined();
        assert!(joined.contains("#00FF00"));
        assert!(joined.contains("#FF0000"));
    }
}
