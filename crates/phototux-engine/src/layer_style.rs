//! Nondestructive layer styles — shadow, stroke, outer glow, color overlay.
//!
//! GPU application lands with the composite planner; this module owns the
//! serializable stack and CPU reference for styles on RGBA8.

use serde::{Deserialize, Serialize};

/// One style effect on a layer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayerStyle {
    DropShadow {
        enabled: bool,
        offset_x: f32,
        offset_y: f32,
        blur: f32,
        opacity: f32,
        color_rgba: [f32; 4],
    },
    Stroke {
        enabled: bool,
        width: f32,
        opacity: f32,
        color_rgba: [f32; 4],
        /// Where the outline sits relative to the layer's own alpha.
        #[serde(default)]
        position: StrokePosition,
    },
    OuterGlow {
        enabled: bool,
        radius: f32,
        opacity: f32,
        color_rgba: [f32; 4],
    },
    ColorOverlay {
        enabled: bool,
        opacity: f32,
        color_rgba: [f32; 4],
    },
    /// Shadow cast inward from the layer's edges.
    InnerShadow {
        enabled: bool,
        offset_x: f32,
        offset_y: f32,
        blur: f32,
        opacity: f32,
        color_rgba: [f32; 4],
    },
    /// Glow radiating inward from the layer's edges.
    InnerGlow {
        enabled: bool,
        radius: f32,
        opacity: f32,
        color_rgba: [f32; 4],
    },
    /// Linear gradient clipped to the layer's coverage.
    GradientOverlay {
        enabled: bool,
        opacity: f32,
        angle_deg: f32,
        start_rgba: [f32; 4],
        end_rgba: [f32; 4],
    },
    /// Lit edge relief from the layer's own alpha slope.
    Bevel {
        enabled: bool,
        size: f32,
        depth: f32,
        angle_deg: f32,
        opacity: f32,
    },
}

/// Where a stroke sits relative to the layer's alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrokePosition {
    /// Entirely outside the coverage — the shipped behaviour, and the default
    /// so documents written before the field keep the outline they had.
    #[default]
    Outside,
    /// Entirely inside the coverage.
    Inside,
    /// Straddling the edge.
    Center,
}

impl StrokePosition {
    pub const ALL: &'static [Self] = &[Self::Outside, Self::Inside, Self::Center];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Outside => "outside",
            Self::Inside => "inside",
            Self::Center => "center",
        }
    }

    /// Parse a chrome label; `None` names no position.
    #[must_use]
    pub fn parse(label: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|p| p.as_str() == label)
    }

    /// Integer code for the GPU stroke pass.
    #[must_use]
    pub fn as_u32(self) -> u32 {
        match self {
            Self::Outside => 0,
            Self::Inside => 1,
            Self::Center => 2,
        }
    }
}

impl LayerStyle {
    pub fn drop_shadow_default() -> Self {
        Self::DropShadow {
            enabled: true,
            offset_x: 4.0,
            offset_y: 4.0,
            blur: 4.0,
            opacity: 0.55,
            color_rgba: [0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn stroke_default() -> Self {
        Self::Stroke {
            enabled: true,
            width: 2.0,
            opacity: 1.0,
            color_rgba: [0.0, 0.0, 0.0, 1.0],
            position: StrokePosition::Outside,
        }
    }

    pub fn outer_glow_default() -> Self {
        Self::OuterGlow {
            enabled: true,
            radius: 6.0,
            opacity: 0.65,
            color_rgba: [1.0, 0.85, 0.2, 1.0],
        }
    }

    pub fn color_overlay_default() -> Self {
        Self::ColorOverlay {
            enabled: true,
            opacity: 0.45,
            color_rgba: [0.2, 0.45, 0.9, 1.0],
        }
    }

    /// Short kind key for the registry, the chrome and `.ptx`.
    ///
    /// Chosen so `action.layer.{kind}` reproduces the action ids that shipped
    /// before these entries were generated — a renamed action id would drop a
    /// user's custom shortcut without saying so.
    #[must_use]
    pub fn kind_key(&self) -> &'static str {
        match self {
            Self::DropShadow { .. } => "drop-shadow",
            Self::Stroke { .. } => "stroke-style",
            Self::OuterGlow { .. } => "outer-glow",
            Self::ColorOverlay { .. } => "color-overlay",
            Self::InnerShadow { .. } => "inner-shadow",
            Self::InnerGlow { .. } => "inner-glow",
            Self::GradientOverlay { .. } => "gradient-overlay",
            Self::Bevel { .. } => "bevel",
        }
    }

    /// Display name for menus and the effect list.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::DropShadow { .. } => "Drop Shadow",
            Self::Stroke { .. } => "Stroke",
            Self::OuterGlow { .. } => "Outer Glow",
            Self::ColorOverlay { .. } => "Color Overlay",
            Self::InnerShadow { .. } => "Inner Shadow",
            Self::InnerGlow { .. } => "Inner Glow",
            Self::GradientOverlay { .. } => "Gradient Overlay",
            Self::Bevel { .. } => "Bevel",
        }
    }

    /// Whether this style contributes to the render.
    #[must_use]
    pub fn enabled(&self) -> bool {
        match *self {
            Self::DropShadow { enabled, .. }
            | Self::Stroke { enabled, .. }
            | Self::OuterGlow { enabled, .. }
            | Self::ColorOverlay { enabled, .. }
            | Self::InnerShadow { enabled, .. }
            | Self::InnerGlow { enabled, .. }
            | Self::GradientOverlay { enabled, .. }
            | Self::Bevel { enabled, .. } => enabled,
        }
    }

    /// One default instance of every style, in the order they render.
    pub const ALL_KINDS: &'static [Self] = &[
        Self::DropShadow {
            enabled: true,
            offset_x: 4.0,
            offset_y: 4.0,
            blur: 4.0,
            opacity: 0.55,
            color_rgba: [0.0, 0.0, 0.0, 1.0],
        },
        Self::OuterGlow {
            enabled: true,
            radius: 6.0,
            opacity: 0.65,
            color_rgba: [1.0, 0.85, 0.2, 1.0],
        },
        Self::InnerShadow {
            enabled: true,
            offset_x: 3.0,
            offset_y: 3.0,
            blur: 4.0,
            opacity: 0.5,
            color_rgba: [0.0, 0.0, 0.0, 1.0],
        },
        Self::InnerGlow {
            enabled: true,
            radius: 5.0,
            opacity: 0.5,
            color_rgba: [1.0, 0.95, 0.7, 1.0],
        },
        Self::Bevel {
            enabled: true,
            size: 4.0,
            depth: 1.0,
            angle_deg: 135.0,
            opacity: 0.7,
        },
        Self::ColorOverlay {
            enabled: true,
            opacity: 0.45,
            color_rgba: [0.2, 0.45, 0.9, 1.0],
        },
        Self::GradientOverlay {
            enabled: true,
            opacity: 0.8,
            angle_deg: 90.0,
            start_rgba: [0.95, 0.35, 0.2, 1.0],
            end_rgba: [0.2, 0.35, 0.95, 1.0],
        },
        Self::Stroke {
            enabled: true,
            width: 2.0,
            opacity: 1.0,
            color_rgba: [0.0, 0.0, 0.0, 1.0],
            position: StrokePosition::Outside,
        },
    ];

    /// The default style for a kind key; `None` for an unknown key.
    #[must_use]
    pub fn default_for_kind(kind: &str) -> Option<Self> {
        Self::ALL_KINDS
            .iter()
            .find(|s| s.kind_key() == kind)
            .copied()
    }
}

/// Apply enabled styles under/over `src` into a new buffer (CPU reference).
///
/// # Errors
/// Returns an error when dimensions are zero or `src` length mismatches.
pub fn apply_styles_rgba8(
    width: u32,
    height: u32,
    src: &[u8],
    styles: &[LayerStyle],
) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 {
        return Err("zero dimensions".into());
    }
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "dimensions overflow".to_owned())?;
    if src.len() != pixels {
        return Err(format!("src length {} != expected {pixels}", src.len()));
    }

    let mut base = vec![0_u8; pixels];
    // Glow + shadows first (under).
    for style in styles {
        match style {
            LayerStyle::DropShadow {
                enabled: true,
                offset_x,
                offset_y,
                blur: _,
                opacity,
                color_rgba,
            } => {
                stamp_offset_alpha(
                    &mut base, src, width, height, *offset_x, *offset_y, color_rgba, *opacity,
                );
            }
            LayerStyle::OuterGlow {
                enabled: true,
                radius,
                opacity,
                color_rgba,
            } => {
                // Approximate glow as several soft rings around alpha.
                let r = radius.max(1.0);
                for k in 1..=3 {
                    let o = r * (k as f32) / 3.0;
                    let falloff = *opacity * (1.0 - (k as f32 - 1.0) / 3.0) * 0.35;
                    stamp_offset_alpha(&mut base, src, width, height, o, 0.0, color_rgba, falloff);
                    stamp_offset_alpha(&mut base, src, width, height, -o, 0.0, color_rgba, falloff);
                    stamp_offset_alpha(&mut base, src, width, height, 0.0, o, color_rgba, falloff);
                    stamp_offset_alpha(&mut base, src, width, height, 0.0, -o, color_rgba, falloff);
                }
            }
            _ => {}
        }
    }
    // Source over under-styles.
    over_straight(&mut base, src);
    // Color overlay on source coverage.
    for style in styles {
        if let LayerStyle::ColorOverlay {
            enabled: true,
            opacity,
            color_rgba,
        } = style
        {
            color_overlay(&mut base, src, color_rgba, *opacity);
        }
    }
    // Strokes over source.
    for style in styles {
        if let LayerStyle::Stroke {
            enabled: true,
            width: sw,
            opacity,
            color_rgba,
            ..
        } = style
        {
            stroke_outline(&mut base, src, width, height, *sw, color_rgba, *opacity);
        }
    }
    Ok(base)
}

fn color_overlay(dst: &mut [u8], src: &[u8], color: &[f32; 4], opacity: f32) {
    let opacity = opacity.clamp(0.0, 1.0);
    let cr = (color[0].clamp(0.0, 1.0) * 255.0).round() as u8;
    let cg = (color[1].clamp(0.0, 1.0) * 255.0).round() as u8;
    let cb = (color[2].clamp(0.0, 1.0) * 255.0).round() as u8;
    for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
        let sa = s[3] as f32 / 255.0;
        if sa <= 0.0 {
            continue;
        }
        let cover = opacity * sa;
        d[0] = mix_u8(d[0], cr, cover);
        d[1] = mix_u8(d[1], cg, cover);
        d[2] = mix_u8(d[2], cb, cover);
    }
}

fn stamp_offset_alpha(
    dst: &mut [u8],
    src: &[u8],
    width: u32,
    height: u32,
    ox: f32,
    oy: f32,
    color: &[f32; 4],
    opacity: f32,
) {
    let w = width as i32;
    let h = height as i32;
    let dx = ox.round() as i32;
    let dy = oy.round() as i32;
    let opacity = opacity.clamp(0.0, 1.0);
    let cr = (color[0].clamp(0.0, 1.0) * 255.0).round() as u8;
    let cg = (color[1].clamp(0.0, 1.0) * 255.0).round() as u8;
    let cb = (color[2].clamp(0.0, 1.0) * 255.0).round() as u8;
    for y in 0..h {
        for x in 0..w {
            let sx = x - dx;
            let sy = y - dy;
            if sx < 0 || sy < 0 || sx >= w || sy >= h {
                continue;
            }
            let si = (sy as usize * width as usize + sx as usize) * 4;
            let a = src[si + 3] as f32 / 255.0 * opacity;
            if a <= 0.0 {
                continue;
            }
            let di = (y as usize * width as usize + x as usize) * 4;
            let cover = a;
            dst[di] = mix_u8(dst[di], cr, cover);
            dst[di + 1] = mix_u8(dst[di + 1], cg, cover);
            dst[di + 2] = mix_u8(dst[di + 2], cb, cover);
            dst[di + 3] = mix_u8(dst[di + 3], 255, cover);
        }
    }
}

fn stroke_outline(
    dst: &mut [u8],
    src: &[u8],
    width: u32,
    height: u32,
    stroke_w: f32,
    color: &[f32; 4],
    opacity: f32,
) {
    let r = stroke_w.ceil().max(1.0) as i32;
    let w = width as i32;
    let h = height as i32;
    let opacity = opacity.clamp(0.0, 1.0);
    let cr = (color[0].clamp(0.0, 1.0) * 255.0).round() as u8;
    let cg = (color[1].clamp(0.0, 1.0) * 255.0).round() as u8;
    let cb = (color[2].clamp(0.0, 1.0) * 255.0).round() as u8;
    for y in 0..h {
        for x in 0..w {
            let i = (y as usize * width as usize + x as usize) * 4;
            if src[i + 3] == 0 || !is_alpha_edge(src, width, w, h, x, y) {
                continue;
            }
            stamp_stroke_disk(
                dst,
                width,
                w,
                h,
                StrokeDisk {
                    x,
                    y,
                    r,
                    cr,
                    cg,
                    cb,
                    opacity,
                },
            );
        }
    }
}

fn is_alpha_edge(src: &[u8], width: u32, w: i32, h: i32, x: i32, y: i32) -> bool {
    for oy in -1..=1 {
        for ox in -1..=1 {
            if ox == 0 && oy == 0 {
                continue;
            }
            let nx = x + ox;
            let ny = y + oy;
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                return true;
            }
            let ni = (ny as usize * width as usize + nx as usize) * 4;
            if src[ni + 3] == 0 {
                return true;
            }
        }
    }
    false
}

fn stamp_stroke_disk(dst: &mut [u8], width: u32, w: i32, h: i32, disk: StrokeDisk) {
    for oy in -disk.r..=disk.r {
        for ox in -disk.r..=disk.r {
            if ox * ox + oy * oy > disk.r * disk.r {
                continue;
            }
            let px = disk.x + ox;
            let py = disk.y + oy;
            if px < 0 || py < 0 || px >= w || py >= h {
                continue;
            }
            let di = (py as usize * width as usize + px as usize) * 4;
            dst[di] = mix_u8(dst[di], disk.cr, disk.opacity);
            dst[di + 1] = mix_u8(dst[di + 1], disk.cg, disk.opacity);
            dst[di + 2] = mix_u8(dst[di + 2], disk.cb, disk.opacity);
            dst[di + 3] = mix_u8(dst[di + 3], 255, disk.opacity);
        }
    }
}

struct StrokeDisk {
    x: i32,
    y: i32,
    r: i32,
    cr: u8,
    cg: u8,
    cb: u8,
    opacity: f32,
}

fn over_straight(dst: &mut [u8], src: &[u8]) {
    for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
        let sa = s[3] as f32 / 255.0;
        if sa <= 0.0 {
            continue;
        }
        let da = d[3] as f32 / 255.0;
        let out_a = sa + da * (1.0 - sa);
        if out_a <= 0.0 {
            continue;
        }
        for c in 0..3 {
            let sc = s[c] as f32 / 255.0;
            let dc = d[c] as f32 / 255.0;
            let oc = (sc * sa + dc * da * (1.0 - sa)) / out_a;
            d[c] = (oc * 255.0).round().clamp(0.0, 255.0) as u8;
        }
        d[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

fn mix_u8(dst: u8, src: u8, cover: f32) -> u8 {
    let t = cover.clamp(0.0, 1.0);
    ((dst as f32) * (1.0 - t) + (src as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    /// A style kind that cannot be looked up by its key is a style nothing can
    /// create, and a duplicate key silently shadows another style.
    #[test]
    fn every_style_kind_round_trips_by_key() {
        let mut seen: Vec<&str> = Vec::new();
        for style in LayerStyle::ALL_KINDS {
            let key = style.kind_key();
            assert!(!seen.contains(&key), "{key} is used twice");
            seen.push(key);
            assert_eq!(LayerStyle::default_for_kind(key).as_ref(), Some(style));
            assert!(!style.label().is_empty());
            assert!(style.enabled(), "{key} defaults to disabled");
        }
        assert_eq!(LayerStyle::default_for_kind("nonsense"), None);
    }

    /// The GPU pass switches on these codes.
    #[test]
    fn stroke_positions_round_trip_and_have_distinct_codes() {
        let mut codes = Vec::new();
        for &p in StrokePosition::ALL {
            assert_eq!(StrokePosition::parse(p.as_str()), Some(p));
            assert!(!codes.contains(&p.as_u32()), "{p:?} reuses a code");
            codes.push(p.as_u32());
        }
        assert_eq!(StrokePosition::parse("nonsense"), None);
        // Documents written before the field have no position; they must keep
        // the outline they were drawn with.
        assert_eq!(StrokePosition::default(), StrokePosition::Outside);
    }

    use super::*;

    #[test]
    fn shadow_extends_bounds() {
        let w = 16_u32;
        let h = 16_u32;
        let mut src = vec![0_u8; (w * h * 4) as usize];
        for y in 2..4 {
            for x in 2..4 {
                let o = ((y * w + x) * 4) as usize;
                src[o] = 255;
                src[o + 1] = 0;
                src[o + 2] = 0;
                src[o + 3] = 255;
            }
        }
        let out =
            apply_styles_rgba8(w, h, &src, &[LayerStyle::drop_shadow_default()]).expect("styles");
        let o = ((6 * w + 6) * 4) as usize;
        assert!(out[o + 3] > 0, "expected shadow alpha");
    }

    #[test]
    fn outer_glow_and_overlay_apply() {
        let w = 8_u32;
        let h = 8_u32;
        let mut src = vec![0_u8; (w * h * 4) as usize];
        let o = ((3 * w + 3) * 4) as usize;
        src[o] = 255;
        src[o + 1] = 255;
        src[o + 2] = 255;
        src[o + 3] = 255;
        let out = apply_styles_rgba8(
            w,
            h,
            &src,
            &[
                LayerStyle::outer_glow_default(),
                LayerStyle::color_overlay_default(),
            ],
        )
        .expect("styles");
        assert_eq!(out.len(), src.len());
    }
}
