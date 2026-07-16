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
}

impl LayerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Raster => "raster",
            Self::Group => "group",
            Self::Text => "text",
            Self::Adjustment => "adjustment",
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
}

impl Default for LayerMask {
    fn default() -> Self {
        Self {
            enabled: true,
            linked: true,
            density: 1.0,
            feather: 0.0,
            inverted: false,
        }
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
}

impl Default for AdjustmentParams {
    fn default() -> Self {
        Self::BrightnessContrast {
            brightness: 0.0,
            contrast: 0.0,
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
    GaussianBlur { radius: f32 },
    BoxBlur { radius: f32 },
    Sharpen { amount: f32 },
    Invert,
    Offset { x: i32, y: i32 },
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
    pub label_color: u8,
    pub text: Option<TextContent>,
    pub adjustment: Option<AdjustmentParams>,
    pub effects: Vec<FilterEffect>,
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
            label_color: 0,
            text: None,
            adjustment: None,
            effects: Vec::new(),
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
        self.locked || self.locks.all || self.locks.pixels || self.kind != LayerKind::Raster
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
}
