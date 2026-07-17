//! Layer types for the document stack (ADR-011, ADR-017).

use serde::{Deserialize, Serialize};

/// Stable id for a layer within a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LayerId(pub u64);

/// Typed node kinds for graph v2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    #[default]
    Raster,
    Group,
    Text,
    Adjustment,
    /// Vector shape layer (DR-027); rasterized for composite.
    Shape,
    /// Procedural solid fill (handbook 11); not a paintable raster buffer.
    Fill,
}

impl LayerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Raster => "raster",
            Self::Group => "group",
            Self::Text => "text",
            Self::Adjustment => "adjustment",
            Self::Shape => "shape",
            Self::Fill => "fill",
        }
    }
}

/// Solid fill definition for [`LayerKind::Fill`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FillContent {
    pub color_rgba: [f32; 4],
}

impl Default for FillContent {
    fn default() -> Self {
        Self {
            color_rgba: [0.45, 0.55, 0.75, 1.0],
        }
    }
}

/// Linear gradient fill for shape layers (document space).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeGradient {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub c0_rgba: [f32; 4],
    pub c1_rgba: [f32; 4],
}

/// Editable shape payload on a [`LayerKind::Shape`] layer (DR-027 / DR-028).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeContent {
    pub path: crate::paths::VectorPath,
    pub fill_rgba: [f32; 4],
    pub stroke_rgba: [f32; 4],
    pub stroke_width: f32,
    pub filled: bool,
    pub stroked: bool,
    /// Kind key: `rect` | `ellipse` | `polygon` | `line`.
    #[serde(default = "default_shape_kind")]
    pub kind: String,
    /// When true, host skips destructive bake and re-rasters from path each sync (v1 live).
    #[serde(default)]
    pub live_vector: bool,
    /// Optional linear gradient fill (overrides flat `fill_rgba` when present).
    #[serde(default)]
    pub gradient: Option<ShapeGradient>,
    /// Vector-preserving boolean partner (second operand path + op).
    #[serde(default)]
    pub boolean_partner: Option<ShapeBooleanPartner>,
}

/// Second operand for a vector-preserving boolean on a shape layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeBooleanPartner {
    pub op: String,
    pub path: crate::paths::VectorPath,
    pub fill_rgba: [f32; 4],
}

fn default_shape_kind() -> String {
    "rect".into()
}

impl Default for ShapeContent {
    fn default() -> Self {
        Self {
            path: crate::paths::VectorPath::polyline("Shape", Vec::new(), true),
            fill_rgba: [0.2, 0.45, 0.9, 1.0],
            stroke_rgba: [0.0, 0.0, 0.0, 1.0],
            stroke_width: 2.0,
            filled: true,
            stroked: true,
            kind: default_shape_kind(),
            live_vector: false,
            gradient: None,
            boolean_partner: None,
        }
    }
}

/// Affine transform in document space (identity default).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LayerTransform {
    pub translate_x: f32,
    pub translate_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation_deg: f32,
}

impl Default for LayerTransform {
    fn default() -> Self {
        Self {
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation_deg: 0.0,
        }
    }
}

/// Layer mask metadata (pixels live on GPU when present).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerMask {
    pub enabled: bool,
    pub linked: bool,
    pub density: f32,
    pub feather: f32,
    pub inverted: bool,
    /// Contrast refine (−1..1); applied at mask sample time.
    #[serde(default)]
    pub contrast: f32,
    /// Level shift refine (−1..1); applied at mask sample time.
    #[serde(default)]
    pub shift: f32,
}

impl Default for LayerMask {
    fn default() -> Self {
        Self {
            enabled: true,
            linked: true,
            density: 1.0,
            feather: 0.0,
            inverted: false,
            contrast: 0.0,
            shift: 0.0,
        }
    }
}

impl LayerMask {
    /// Clamp refine attributes into UI/GPU-safe ranges.
    pub fn clamp_refine(&mut self) {
        self.contrast = self.contrast.clamp(-1.0, 1.0);
        self.shift = self.shift.clamp(-1.0, 1.0);
        self.density = self.density.clamp(0.0, 1.0);
        self.feather = self.feather.max(0.0);
    }
}

/// Lock flags for professional layer workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LockFlags {
    pub pixels: bool,
    pub position: bool,
    pub all: bool,
    pub alpha: bool,
}

/// Path-based vector mask metadata (pixels/path body filled by host later).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorMask {
    pub enabled: bool,
    pub linked: bool,
}

impl Default for VectorMask {
    fn default() -> Self {
        Self {
            enabled: true,
            linked: true,
        }
    }
}

/// Blend modes for GPU composite (expanded set; unknown modes reject at IO).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
    PassThrough,
}

impl BlendMode {
    pub const ALL: [BlendMode; 17] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::HardLight,
        BlendMode::SoftLight,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
        BlendMode::PassThrough,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Multiply => "multiply",
            Self::Screen => "screen",
            Self::Overlay => "overlay",
            Self::Darken => "darken",
            Self::Lighten => "lighten",
            Self::ColorDodge => "color_dodge",
            Self::ColorBurn => "color_burn",
            Self::HardLight => "hard_light",
            Self::SoftLight => "soft_light",
            Self::Difference => "difference",
            Self::Exclusion => "exclusion",
            Self::Hue => "hue",
            Self::Saturation => "saturation",
            Self::Color => "color",
            Self::Luminosity => "luminosity",
            Self::PassThrough => "pass_through",
        }
    }

    pub fn from_str_label(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().replace('-', "_").as_str() {
            "normal" => Some(Self::Normal),
            "multiply" => Some(Self::Multiply),
            "screen" => Some(Self::Screen),
            "overlay" => Some(Self::Overlay),
            "darken" => Some(Self::Darken),
            "lighten" => Some(Self::Lighten),
            "color_dodge" => Some(Self::ColorDodge),
            "color_burn" => Some(Self::ColorBurn),
            "hard_light" => Some(Self::HardLight),
            "soft_light" => Some(Self::SoftLight),
            "difference" => Some(Self::Difference),
            "exclusion" => Some(Self::Exclusion),
            "hue" => Some(Self::Hue),
            "saturation" => Some(Self::Saturation),
            "color" => Some(Self::Color),
            "luminosity" => Some(Self::Luminosity),
            "pass_through" | "passthrough" => Some(Self::PassThrough),
            _ => None,
        }
    }

    /// Integer code for GPU uniform packing.
    pub fn as_u32(self) -> u32 {
        match self {
            Self::Normal => 0,
            Self::Multiply => 1,
            Self::Screen => 2,
            Self::Overlay => 3,
            Self::Darken => 4,
            Self::Lighten => 5,
            Self::ColorDodge => 6,
            Self::ColorBurn => 7,
            Self::HardLight => 8,
            Self::SoftLight => 9,
            Self::Difference => 10,
            Self::Exclusion => 11,
            Self::Hue => 12,
            Self::Saturation => 13,
            Self::Color => 14,
            Self::Luminosity => 15,
            Self::PassThrough => 16,
        }
    }
}

/// Text layer payload (shaped by Qt; rasterized/cached for composite).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextContent {
    pub text: String,
    pub font_family: String,
    pub font_size_pt: f32,
    pub color_rgba: [f32; 4],
    pub alignment: u8,
    pub tracking: f32,
    pub line_spacing: f32,
    /// Text frame width in document pixels (`0` = use bake buffer width).
    #[serde(default)]
    pub frame_w: f32,
    /// Text frame height in document pixels (`0` = use bake buffer height).
    #[serde(default)]
    pub frame_h: f32,
    /// Word-wrap within the frame when baking.
    #[serde(default)]
    pub wrap: bool,
}

impl Default for TextContent {
    fn default() -> Self {
        Self {
            text: String::new(),
            font_family: "Noto Sans".into(),
            font_size_pt: 24.0,
            color_rgba: [0.0, 0.0, 0.0, 1.0],
            alignment: 0,
            tracking: 0.0,
            line_spacing: 1.2,
            frame_w: 0.0,
            frame_h: 0.0,
            wrap: false,
        }
    }
}

/// Adjustment layer parameters (nondestructive).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdjustmentParams {
    BrightnessContrast {
        brightness: f32,
        contrast: f32,
    },
    Levels {
        black: f32,
        white: f32,
        gamma: f32,
    },
    HueSaturation {
        hue: f32,
        saturation: f32,
        lightness: f32,
    },
    Invert,
    Threshold {
        level: f32,
    },
    Posterize {
        levels: u32,
    },
    /// EV exposure + optional gamma (DR-028 adjustment spine).
    Exposure {
        stops: f32,
        gamma: f32,
    },
}

impl Default for AdjustmentParams {
    fn default() -> Self {
        Self::BrightnessContrast {
            brightness: 0.0,
            contrast: 0.0,
        }
    }
}

impl AdjustmentParams {
    /// Clamp parameters into UI/GPU-safe ranges.
    pub fn clamped(self) -> Self {
        match self {
            Self::BrightnessContrast {
                brightness,
                contrast,
            } => Self::BrightnessContrast {
                brightness: brightness.clamp(-1.0, 1.0),
                contrast: contrast.clamp(-1.0, 1.0),
            },
            Self::Levels {
                black,
                white,
                gamma,
            } => {
                let black = black.clamp(0.0, 1.0);
                let white = white.clamp(0.0, 1.0).max(black + 1e-4);
                Self::Levels {
                    black,
                    white,
                    gamma: gamma.clamp(0.01, 10.0),
                }
            }
            Self::HueSaturation {
                hue,
                saturation,
                lightness,
            } => Self::HueSaturation {
                hue: hue.clamp(-1.0, 1.0),
                saturation: saturation.clamp(-1.0, 1.0),
                lightness: lightness.clamp(-1.0, 1.0),
            },
            Self::Invert => Self::Invert,
            Self::Threshold { level } => Self::Threshold {
                level: level.clamp(0.0, 1.0),
            },
            Self::Posterize { levels } => Self::Posterize {
                levels: levels.clamp(2, 256),
            },
            Self::Exposure { stops, gamma } => Self::Exposure {
                stops: stops.clamp(-5.0, 5.0),
                gamma: gamma.clamp(0.01, 10.0),
            },
        }
    }

    /// Short kind key for QML (`brightness`, `levels`, …).
    pub fn kind_key(&self) -> &'static str {
        match self {
            Self::BrightnessContrast { .. } => "brightness",
            Self::Levels { .. } => "levels",
            Self::HueSaturation { .. } => "hue",
            Self::Invert => "invert",
            Self::Threshold { .. } => "threshold",
            Self::Posterize { .. } => "posterize",
            Self::Exposure { .. } => "exposure",
        }
    }
}

/// One nondestructive filter/effect node on a layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterEffect {
    pub id: u64,
    pub name: String,
    pub enabled: bool,
    pub opacity: f32,
    pub blend: BlendMode,
    pub params: FilterParams,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FilterParams {
    GaussianBlur {
        radius: f32,
    },
    BoxBlur {
        radius: f32,
    },
    Sharpen {
        amount: f32,
    },
    Invert,
    Offset {
        x: i32,
        y: i32,
    },
    /// Directional blur (wave 2).
    MotionBlur {
        distance: f32,
        angle_deg: f32,
    },
    /// Emboss height-map style (wave 2).
    Emboss {
        strength: f32,
        angle_deg: f32,
    },
    /// Monochrome/noise grain (DR-028).
    Noise {
        amount: f32,
    },
}

/// Maximum Gaussian/box blur radius accepted by the engine.
pub const MAX_BLUR_RADIUS: f32 = 64.0;

impl FilterParams {
    /// Clamp parameters into UI/GPU-safe ranges.
    pub fn clamped(self) -> Self {
        match self {
            Self::GaussianBlur { radius } => Self::GaussianBlur {
                radius: radius.clamp(0.0, MAX_BLUR_RADIUS),
            },
            Self::BoxBlur { radius } => Self::BoxBlur {
                radius: radius.clamp(0.0, MAX_BLUR_RADIUS),
            },
            Self::Sharpen { amount } => Self::Sharpen {
                amount: amount.clamp(0.0, 4.0),
            },
            Self::Invert => Self::Invert,
            Self::Offset { x, y } => Self::Offset { x, y },
            Self::MotionBlur {
                distance,
                angle_deg,
            } => Self::MotionBlur {
                distance: distance.clamp(0.0, MAX_BLUR_RADIUS),
                angle_deg: angle_deg.rem_euclid(360.0),
            },
            Self::Emboss {
                strength,
                angle_deg,
            } => Self::Emboss {
                strength: strength.clamp(0.0, 4.0),
                angle_deg: angle_deg.rem_euclid(360.0),
            },
            Self::Noise { amount } => Self::Noise {
                amount: amount.clamp(0.0, 1.0),
            },
        }
    }
}

impl FilterEffect {
    /// Create an enabled Gaussian Blur effect.
    pub fn gaussian_blur(id: u64, radius: f32) -> Self {
        Self {
            id,
            name: "Gaussian Blur".into(),
            enabled: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            params: FilterParams::GaussianBlur {
                radius: radius.clamp(0.0, MAX_BLUR_RADIUS),
            },
        }
    }

    /// Create an enabled Motion Blur effect.
    pub fn motion_blur(id: u64, distance: f32, angle_deg: f32) -> Self {
        Self {
            id,
            name: "Motion Blur".into(),
            enabled: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            params: FilterParams::MotionBlur {
                distance,
                angle_deg,
            }
            .clamped(),
        }
    }

    /// Create an enabled Emboss effect.
    pub fn emboss(id: u64, strength: f32, angle_deg: f32) -> Self {
        Self {
            id,
            name: "Emboss".into(),
            enabled: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            params: FilterParams::Emboss {
                strength,
                angle_deg,
            }
            .clamped(),
        }
    }

    /// Create an enabled Sharpen effect.
    pub fn sharpen(id: u64, amount: f32) -> Self {
        Self {
            id,
            name: "Sharpen".into(),
            enabled: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            params: FilterParams::Sharpen { amount }.clamped(),
        }
    }

    /// Create an enabled Noise effect.
    pub fn noise(id: u64, amount: f32) -> Self {
        Self {
            id,
            name: "Noise".into(),
            enabled: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            params: FilterParams::Noise { amount }.clamped(),
        }
    }
}

/// Where brush/eraser strokes apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaintTarget {
    #[default]
    LayerPixels,
    LayerMask,
}

/// One layer in the ordered / hierarchical graph (metadata; pixels on GPU).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub kind: LayerKind,
    pub opacity: f32,
    pub visible: bool,
    pub locked: bool,
    pub locks: LockFlags,
    pub blend: BlendMode,
    pub parent: Option<LayerId>,
    pub transform: LayerTransform,
    pub mask: Option<LayerMask>,
    /// Optional vector mask (path-based); coverage rasterized by host when present.
    #[serde(default)]
    pub vector_mask: Option<VectorMask>,
    /// Photoshop-style clip to the nearest non-clipping layer below.
    #[serde(default)]
    pub clips_to_below: bool,
    pub label_color: u8,
    pub text: Option<TextContent>,
    pub adjustment: Option<AdjustmentParams>,
    #[serde(default)]
    pub shape: Option<ShapeContent>,
    #[serde(default)]
    pub fill: Option<FillContent>,
    pub effects: Vec<FilterEffect>,
    /// Nondestructive layer styles (shadow / stroke v1).
    #[serde(default)]
    pub styles: Vec<super::layer_style::LayerStyle>,
    /// Declarative filter / effect plan (handbook 15); executors apply nodes later.
    #[serde(default)]
    pub filter_plan: crate::filter_plan::FilterPlan,
    /// Serialization key for raster bytes inside `.ptx` (e.g. `layer-3`).
    pub asset_key: Option<String>,
}

impl Layer {
    pub fn new(id: LayerId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            kind: LayerKind::Raster,
            opacity: 1.0,
            visible: true,
            locked: false,
            locks: LockFlags::default(),
            blend: BlendMode::Normal,
            parent: None,
            transform: LayerTransform::default(),
            mask: None,
            vector_mask: None,
            clips_to_below: false,
            label_color: 0,
            text: None,
            adjustment: None,
            shape: None,
            fill: None,
            effects: Vec::new(),
            styles: Vec::new(),
            filter_plan: crate::filter_plan::FilterPlan::new(),
            asset_key: Some(format!("layer-{}", id.0)),
        }
    }

    pub fn group(id: LayerId, name: impl Into<String>) -> Self {
        let mut layer = Self::new(id, name);
        layer.kind = LayerKind::Group;
        layer.blend = BlendMode::PassThrough;
        layer.asset_key = None;
        layer
    }

    pub fn text_layer(id: LayerId, name: impl Into<String>, content: TextContent) -> Self {
        let mut layer = Self::new(id, name);
        layer.kind = LayerKind::Text;
        layer.text = Some(content);
        layer
    }

    pub fn shape_layer(id: LayerId, name: impl Into<String>, content: ShapeContent) -> Self {
        let mut layer = Self::new(id, name);
        layer.kind = LayerKind::Shape;
        layer.shape = Some(content);
        layer
    }

    pub fn fill_layer(id: LayerId, name: impl Into<String>, content: FillContent) -> Self {
        let mut layer = Self::new(id, name);
        layer.kind = LayerKind::Fill;
        layer.fill = Some(content);
        layer.asset_key = None;
        layer
    }

    pub fn adjustment_layer(
        id: LayerId,
        name: impl Into<String>,
        params: AdjustmentParams,
    ) -> Self {
        let mut layer = Self::new(id, name);
        layer.kind = LayerKind::Adjustment;
        layer.adjustment = Some(params);
        layer.asset_key = None;
        layer
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }

    pub fn paint_blocked(&self) -> bool {
        self.locked
            || self.locks.all
            || self.locks.pixels
            || !matches!(self.kind, LayerKind::Raster)
    }

    pub fn position_blocked(&self) -> bool {
        self.locked || self.locks.all || self.locks.position
    }

    /// `0` = no mask, `1` = mask enabled, `2` = mask disabled.
    pub fn mask_flag(&self) -> u8 {
        match &self.mask {
            None => 0,
            Some(m) if m.enabled => 1,
            Some(_) => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_clamped() {
        let mut l = Layer::new(LayerId(1), "A");
        l.set_opacity(2.0);
        assert!((l.opacity - 1.0).abs() < f32::EPSILON);
        l.set_opacity(-1.0);
        assert!((l.opacity - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn blend_roundtrip() {
        for b in BlendMode::ALL {
            assert_eq!(BlendMode::from_str_label(b.as_str()), Some(b));
        }
    }

    #[test]
    fn paint_blocked_on_group() {
        let g = Layer::group(LayerId(2), "G");
        assert!(g.paint_blocked());
    }

    #[test]
    fn position_blocked_respects_flags() {
        let mut layer = Layer::new(LayerId(1), "A");
        assert!(!layer.position_blocked());
        layer.locks.position = true;
        assert!(layer.position_blocked());
        layer.locks.position = false;
        layer.locks.all = true;
        assert!(layer.position_blocked());
        assert!(layer.paint_blocked());
    }
}
