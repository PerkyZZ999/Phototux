//! Nondestructive layer styles — shadow, stroke, outer glow, color overlay.
//!
//! GPU application lands with the composite planner; this module owns the
//! serializable stack and CPU reference for styles on RGBA8.

use serde::{Deserialize, Serialize};

use crate::layer::MAX_ADJUSTMENT_SLOTS;

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

    /// Turn this style on or off, leaving its parameters alone.
    pub fn set_enabled(&mut self, on: bool) {
        match self {
            Self::DropShadow { enabled, .. }
            | Self::Stroke { enabled, .. }
            | Self::OuterGlow { enabled, .. }
            | Self::ColorOverlay { enabled, .. }
            | Self::InnerShadow { enabled, .. }
            | Self::InnerGlow { enabled, .. }
            | Self::GradientOverlay { enabled, .. }
            | Self::Bevel { enabled, .. } => *enabled = on,
        }
    }

    /// Scalar editor slots, as `(label, min, max)`; position is the slot index.
    ///
    /// Same contract as the adjustment and filter vocabularies: the chrome
    /// builds one control per entry rather than naming the styles it knows.
    #[must_use]
    pub fn editor_slots(&self) -> &'static [(&'static str, f32, f32)] {
        match self {
            Self::DropShadow { .. } | Self::InnerShadow { .. } => &[
                ("Offset X", -64.0, 64.0),
                ("Offset Y", -64.0, 64.0),
                ("Blur", 0.0, 64.0),
                ("Opacity", 0.0, 1.0),
            ],
            Self::OuterGlow { .. } | Self::InnerGlow { .. } => {
                &[("Radius", 0.0, 64.0), ("Opacity", 0.0, 1.0)]
            }
            Self::Bevel { .. } => &[
                ("Size", 1.0, 32.0),
                ("Depth", 0.0, 4.0),
                ("Angle", 0.0, 360.0),
                ("Opacity", 0.0, 1.0),
            ],
            Self::ColorOverlay { .. } => &[("Opacity", 0.0, 1.0)],
            Self::GradientOverlay { .. } => &[("Opacity", 0.0, 1.0), ("Angle", 0.0, 360.0)],
            Self::Stroke { .. } => &[
                ("Width", 0.0, 32.0),
                ("Opacity", 0.0, 1.0),
                ("Position", 0.0, 2.0),
            ],
        }
    }

    /// Parameters projected onto the scalar slots.
    #[must_use]
    pub fn slots(&self) -> [f32; MAX_ADJUSTMENT_SLOTS] {
        let mut out = [0.0; MAX_ADJUSTMENT_SLOTS];
        let values: &[f32] = &match *self {
            Self::DropShadow {
                offset_x,
                offset_y,
                blur,
                opacity,
                ..
            }
            | Self::InnerShadow {
                offset_x,
                offset_y,
                blur,
                opacity,
                ..
            } => vec![offset_x, offset_y, blur, opacity],
            Self::OuterGlow {
                radius, opacity, ..
            }
            | Self::InnerGlow {
                radius, opacity, ..
            } => vec![radius, opacity],
            Self::Bevel {
                size,
                depth,
                angle_deg,
                opacity,
                ..
            } => vec![size, depth, angle_deg, opacity],
            Self::ColorOverlay { opacity, .. } => vec![opacity],
            Self::GradientOverlay {
                opacity, angle_deg, ..
            } => vec![opacity, angle_deg],
            Self::Stroke {
                width,
                opacity,
                position,
                ..
            } => {
                #[expect(clippy::cast_precision_loss, reason = "three position codes")]
                let code = position.as_u32() as f32;
                vec![width, opacity, code]
            }
        };
        out[..values.len()].copy_from_slice(values);
        out
    }

    /// Rebuild this style from scalar slot values, keeping its colours.
    #[must_use]
    pub fn with_slots(&self, p: [f32; MAX_ADJUSTMENT_SLOTS]) -> Self {
        match *self {
            Self::DropShadow {
                enabled,
                color_rgba,
                ..
            } => Self::DropShadow {
                enabled,
                offset_x: p[0],
                offset_y: p[1],
                blur: p[2],
                opacity: p[3],
                color_rgba,
            },
            Self::InnerShadow {
                enabled,
                color_rgba,
                ..
            } => Self::InnerShadow {
                enabled,
                offset_x: p[0],
                offset_y: p[1],
                blur: p[2],
                opacity: p[3],
                color_rgba,
            },
            Self::OuterGlow {
                enabled,
                color_rgba,
                ..
            } => Self::OuterGlow {
                enabled,
                radius: p[0],
                opacity: p[1],
                color_rgba,
            },
            Self::InnerGlow {
                enabled,
                color_rgba,
                ..
            } => Self::InnerGlow {
                enabled,
                radius: p[0],
                opacity: p[1],
                color_rgba,
            },
            Self::Bevel { enabled, .. } => Self::Bevel {
                enabled,
                size: p[0],
                depth: p[1],
                angle_deg: p[2],
                opacity: p[3],
            },
            Self::ColorOverlay {
                enabled,
                color_rgba,
                ..
            } => Self::ColorOverlay {
                enabled,
                opacity: p[0],
                color_rgba,
            },
            Self::GradientOverlay {
                enabled,
                start_rgba,
                end_rgba,
                ..
            } => Self::GradientOverlay {
                enabled,
                opacity: p[0],
                angle_deg: p[1],
                start_rgba,
                end_rgba,
            },
            Self::Stroke {
                enabled,
                color_rgba,
                ..
            } => Self::Stroke {
                enabled,
                width: p[0],
                opacity: p[1],
                color_rgba,
                // A slider carries a float; anything between codes rounds to
                // the nearest position rather than silently reverting.
                position: match p[2].round() {
                    v if v <= 0.0 => StrokePosition::Outside,
                    v if v <= 1.0 => StrokePosition::Inside,
                    _ => StrokePosition::Center,
                },
            },
        }
    }

    /// Labels of the colours this style carries; position is the colour index.
    #[must_use]
    pub fn color_labels(&self) -> &'static [&'static str] {
        match self {
            Self::Bevel { .. } => &[],
            Self::GradientOverlay { .. } => &["Start", "End"],
            _ => &["Color"],
        }
    }

    /// The colours this style carries, index-aligned with [`Self::color_labels`].
    #[must_use]
    pub fn colors(&self) -> Vec<[f32; 4]> {
        match *self {
            Self::DropShadow { color_rgba, .. }
            | Self::InnerShadow { color_rgba, .. }
            | Self::OuterGlow { color_rgba, .. }
            | Self::InnerGlow { color_rgba, .. }
            | Self::ColorOverlay { color_rgba, .. }
            | Self::Stroke { color_rgba, .. } => vec![color_rgba],
            Self::GradientOverlay {
                start_rgba,
                end_rgba,
                ..
            } => vec![start_rgba, end_rgba],
            Self::Bevel { .. } => Vec::new(),
        }
    }

    /// Replace one colour, leaving everything else alone.
    ///
    /// An index past [`Self::color_labels`] is ignored rather than clamped
    /// onto colour zero: the caller named a colour this style does not have,
    /// and repainting a different one is worse than doing nothing.
    #[must_use]
    pub fn with_color(&self, index: usize, rgba: [f32; 4]) -> Self {
        let mut out = *self;
        if index >= self.color_labels().len() {
            return out;
        }
        match &mut out {
            Self::DropShadow { color_rgba, .. }
            | Self::InnerShadow { color_rgba, .. }
            | Self::OuterGlow { color_rgba, .. }
            | Self::InnerGlow { color_rgba, .. }
            | Self::ColorOverlay { color_rgba, .. }
            | Self::Stroke { color_rgba, .. } => *color_rgba = rgba,
            Self::GradientOverlay {
                start_rgba,
                end_rgba,
                ..
            } => {
                if index == 0 {
                    *start_rgba = rgba;
                } else {
                    *end_rgba = rgba;
                }
            }
            Self::Bevel { .. } => {}
        }
        out
    }
}

/// `[{index, kind, label, enabled, slots, colors, editor}]` for the chrome.
///
/// Everything the Properties panel needs to draw an editor for a layer's
/// styles without naming a single style kind.
#[must_use]
pub fn layer_styles_json(styles: &[LayerStyle]) -> String {
    let entries: Vec<serde_json::Value> = styles
        .iter()
        .enumerate()
        .map(|(index, style)| {
            let slot_count = style.editor_slots().len();
            serde_json::json!({
                "index": index,
                "kind": style.kind_key(),
                "label": style.label(),
                "enabled": style.enabled(),
                "slots": &style.slots()[..slot_count],
                "colors": style.colors(),
                "editor": {
                    "slots": style
                        .editor_slots()
                        .iter()
                        .map(|&(label, min, max)| {
                            serde_json::json!({ "label": label, "min": min, "max": max })
                        })
                        .collect::<Vec<_>>(),
                    "colors": style.color_labels(),
                },
            })
        })
        .collect();
    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into())
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
    /// The chrome edits a style only through its slots and colours, so a
    /// parameter the projection drops is a control that silently does nothing.
    #[test]
    fn style_slots_and_colors_round_trip() {
        for style in LayerStyle::ALL_KINDS {
            let slots = style.slots();
            assert_eq!(
                &style.with_slots(slots),
                style,
                "{} does not survive its own slot projection",
                style.kind_key()
            );
            assert!(
                style.editor_slots().len() <= crate::MAX_ADJUSTMENT_SLOTS,
                "{} declares more slots than the projection carries",
                style.kind_key()
            );
            for (index, (label, min, max)) in style.editor_slots().iter().enumerate() {
                assert!(!label.is_empty());
                assert!(min < max, "{} slot {index}: {min}..{max}", style.kind_key());
            }

            let colors = style.colors();
            assert_eq!(
                colors.len(),
                style.color_labels().len(),
                "{} describes a different number of colours than it carries",
                style.kind_key()
            );
            for index in 0..colors.len() {
                let painted = style.with_color(index, [0.1, 0.2, 0.3, 1.0]);
                assert_eq!(
                    painted.colors()[index],
                    [0.1, 0.2, 0.3, 1.0],
                    "{} colour {index} did not take",
                    style.kind_key()
                );
                // Every other colour must be untouched.
                for other in (0..colors.len()).filter(|o| *o != index) {
                    assert_eq!(painted.colors()[other], colors[other]);
                }
            }
        }
    }

    /// A colour index the style does not have must change nothing, rather than
    /// repainting colour zero — the caller named something that is not there.
    #[test]
    fn an_out_of_range_color_index_changes_nothing() {
        for style in LayerStyle::ALL_KINDS {
            let past_end = style.color_labels().len();
            assert_eq!(&style.with_color(past_end, [1.0; 4]), style);
        }
    }

    /// Every style must be switchable without disturbing its parameters.
    #[test]
    fn enabling_a_style_leaves_its_parameters_alone() {
        for style in LayerStyle::ALL_KINDS {
            let mut off = *style;
            off.set_enabled(false);
            assert!(!off.enabled(), "{} ignored set_enabled", style.kind_key());
            assert_eq!(off.slots(), style.slots());
            assert_eq!(off.colors(), style.colors());
            off.set_enabled(true);
            assert_eq!(&off, style);
        }
    }

    /// The chrome sends a float for a discrete position; it must land on a
    /// real one rather than silently reverting to the default.
    #[test]
    fn a_fractional_stroke_position_rounds_to_a_real_one() {
        let stroke = LayerStyle::default_for_kind("stroke-style").expect("stroke");
        for (value, expected) in [
            (0.0, StrokePosition::Outside),
            (0.4, StrokePosition::Outside),
            (0.6, StrokePosition::Inside),
            (1.4, StrokePosition::Inside),
            (1.6, StrokePosition::Center),
            (2.0, StrokePosition::Center),
        ] {
            let mut slots = stroke.slots();
            slots[2] = value;
            let LayerStyle::Stroke { position, .. } = stroke.with_slots(slots) else {
                panic!("with_slots changed the kind");
            };
            assert_eq!(position, expected, "at {value}");
        }
    }

    /// The panel reads this JSON instead of naming style kinds, so a style
    /// missing from it has no editor.
    #[test]
    fn styles_json_describes_every_style_it_is_given() {
        let json = layer_styles_json(LayerStyle::ALL_KINDS);
        for style in LayerStyle::ALL_KINDS {
            assert!(
                json.contains(style.kind_key()),
                "{} missing from the chrome JSON",
                style.kind_key()
            );
            for (label, _, _) in style.editor_slots() {
                assert!(json.contains(label), "{label} missing from chrome JSON");
            }
        }
        assert_eq!(layer_styles_json(&[]), "[]");
    }

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
