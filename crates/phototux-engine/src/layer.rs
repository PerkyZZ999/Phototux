//! Layer types for the document stack (ADR-011, ADR-017).

use std::borrow::Cow;

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

    /// Coverage this mask yields for one stored sample, in `0.0..=1.0`.
    ///
    /// This is the single definition of mask semantics. The order — invert,
    /// then contrast/shift refine, then density — is what the composite shader
    /// applies, and it matters: refining after density would fold the density
    /// floor into the contrast pivot. Baking the mask into pixels previously
    /// open-coded its own version that applied invert and density but dropped
    /// contrast and shift entirely, so Apply Layer Mask produced pixels that
    /// did not match the canvas the user was looking at.
    ///
    /// `feather` is deliberately not applied here, and its absence is the
    /// point rather than an omission: it softens a texel using its neighbours,
    /// so it cannot be a function of one sample. [`Self::feathered`] is the
    /// other half — run it over the stored mask first, then this over each
    /// resulting sample. The composite does exactly that, blurring the mask
    /// before packing it and applying these four in the shader.
    #[must_use]
    pub fn coverage(&self, sample: f32) -> f32 {
        let mut m = sample.clamp(0.0, 1.0);
        if self.inverted {
            m = 1.0 - m;
        }
        let contrast = self.contrast.clamp(-1.0, 1.0);
        let shift = self.shift.clamp(-1.0, 1.0);
        m = ((m - 0.5) * (1.0 + contrast) + 0.5 + shift).clamp(0.0, 1.0);
        let density = self.density.clamp(0.0, 1.0);
        1.0 - density * (1.0 - m)
    }

    /// Soften the stored mask by this mask's feather radius.
    ///
    /// The neighbourhood half of the mask definition; [`Self::coverage`] is the
    /// per-sample half, and both must run — in this order — for a baked result
    /// to match what the canvas shows.
    ///
    /// Borrows unchanged when there is nothing to do, which is the common case:
    /// feather defaults to zero and most masks never set it.
    ///
    /// The kernel is a box average where the GPU uses a Gaussian, so the two
    /// soften by the same radius but not identically — the same approximation
    /// the Gaussian blur effect already documents in `phototux_gpu::parity`.
    #[must_use]
    pub fn feathered<'a>(&self, width: u32, height: u32, mask_r8: &'a [u8]) -> Cow<'a, [u8]> {
        let radius = self.feather.max(0.0).round() as u32;
        if radius == 0 {
            return Cow::Borrowed(mask_r8);
        }
        match crate::selection::feather_mask_r8(width, height, mask_r8, radius) {
            Ok(softened) => Cow::Owned(softened),
            // A size disagreement is the caller's to report; softening is not
            // the place to fail a bake that would otherwise succeed.
            Err(_) => Cow::Borrowed(mask_r8),
        }
    }

    /// Multiply an RGBA8 buffer's alpha by this mask's coverage, in place.
    ///
    /// `mask_r8` carries one coverage byte per pixel, so it must be exactly a
    /// quarter of `rgba`'s length; a mismatch means the caller paired a mask
    /// with the wrong layer and is reported rather than silently truncated.
    ///
    /// # Errors
    /// Returns the mismatched lengths when the buffers do not correspond.
    pub fn bake_into_rgba8(&self, rgba: &mut [u8], mask_r8: &[u8]) -> Result<(), (usize, usize)> {
        if mask_r8.len() * 4 != rgba.len() {
            return Err((rgba.len(), mask_r8.len()));
        }
        for (px, &m) in rgba.chunks_exact_mut(4).zip(mask_r8.iter()) {
            let coverage = self.coverage(f32::from(m) / 255.0);
            let alpha = (f32::from(px[3]) / 255.0) * coverage;
            px[3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        }
        Ok(())
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

/// Blend modes for GPU composite (unknown modes reject at IO).
///
/// Declares the wire name, the GPU code, the menu family and the display
/// label together. Those four facts used to live in four parallel `match`
/// arms plus `ALL` plus a `rename_all` serde attribute, and the QML combo
/// listed a sixth copy that had drifted to eight of the seventeen modes — so
/// half the set was unreachable from the Properties panel. One list makes a
/// new mode reachable everywhere or nowhere.
///
/// GPU codes are explicit rather than positional because the shader switches
/// on them and `.ptx` documents do not record them: reordering the list for
/// the menu must not silently repaint saved work.
macro_rules! blend_modes {
    ($($(#[$vattr:meta])* $variant:ident => $wire:literal, $code:literal, $family:literal, $label:literal);+ $(;)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
        pub enum BlendMode {
            $($(#[$vattr])* #[serde(rename = $wire)] $variant),+
        }

        impl BlendMode {
            /// Every mode, in menu order.
            pub const ALL: &'static [BlendMode] = &[$(BlendMode::$variant),+];

            /// Stable wire name used by `.ptx`, the command layer and QML.
            #[must_use]
            pub fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $wire),+ }
            }

            /// Parse a wire name; `None` when it names no mode this build ships.
            #[must_use]
            pub fn from_str_label(s: &str) -> Option<Self> {
                match s.to_ascii_lowercase().replace('-', "_").as_str() {
                    $($wire => Some(Self::$variant),)+
                    "passthrough" => Some(Self::PassThrough),
                    _ => None,
                }
            }

            /// Integer code for GPU uniform packing.
            #[must_use]
            pub fn as_u32(self) -> u32 {
                match self { $(Self::$variant => $code),+ }
            }

            /// Menu family, so the combo can band the modes the way the
            /// darken/lighten/contrast grouping teaches them.
            #[must_use]
            pub fn family(self) -> &'static str {
                match self { $(Self::$variant => $family),+ }
            }

            /// Display label for chrome.
            #[must_use]
            pub fn label(self) -> &'static str {
                match self { $(Self::$variant => $label),+ }
            }
        }
    };
}

blend_modes! {
    #[default]
    Normal       => "normal",        0, "normal",   "Normal";
    PassThrough  => "pass_through", 16, "normal",   "Pass Through";

    Darken       => "darken",        4, "darken",   "Darken";
    Multiply     => "multiply",      1, "darken",   "Multiply";
    ColorBurn    => "color_burn",    7, "darken",   "Color Burn";
    LinearBurn   => "linear_burn",  17, "darken",   "Linear Burn";
    DarkerColor  => "darker_color", 18, "darken",   "Darker Color";

    Lighten      => "lighten",       5, "lighten",  "Lighten";
    Screen       => "screen",        2, "lighten",  "Screen";
    ColorDodge   => "color_dodge",   6, "lighten",  "Color Dodge";
    LinearDodge  => "linear_dodge", 19, "lighten",  "Linear Dodge (Add)";
    LighterColor => "lighter_color",20, "lighten",  "Lighter Color";

    Overlay      => "overlay",       3, "contrast", "Overlay";
    SoftLight    => "soft_light",    9, "contrast", "Soft Light";
    HardLight    => "hard_light",    8, "contrast", "Hard Light";
    VividLight   => "vivid_light",  21, "contrast", "Vivid Light";
    LinearLight  => "linear_light", 22, "contrast", "Linear Light";
    PinLight     => "pin_light",    23, "contrast", "Pin Light";
    HardMix      => "hard_mix",     24, "contrast", "Hard Mix";

    Difference   => "difference",   10, "compare",  "Difference";
    Exclusion    => "exclusion",    11, "compare",  "Exclusion";
    Subtract     => "subtract",     25, "compare",  "Subtract";
    Divide       => "divide",       26, "compare",  "Divide";

    Hue          => "hue",          12, "component","Hue";
    Saturation   => "saturation",   13, "component","Saturation";
    Color        => "color",        14, "component","Color";
    Luminosity   => "luminosity",   15, "component","Luminosity";
}

impl BlendMode {
    /// Whether the mode is defined one channel at a time.
    ///
    /// The component modes mix a channel of the backdrop with two of the
    /// source, and the two whole-colour modes pick a pixel by its luminosity,
    /// so neither has a per-channel form at all. Asking for one is the mistake
    /// that used to leave the component modes rendering as plain Normal.
    #[must_use]
    pub fn is_separable(self) -> bool {
        !matches!(
            self,
            Self::Hue
                | Self::Saturation
                | Self::Color
                | Self::Luminosity
                | Self::DarkerColor
                | Self::LighterColor
        )
    }
}

/// Every blend mode as JSON for the chrome, in menu order.
///
/// Each entry carries `id`, `label` and `family`; the combo draws a separator
/// wherever the family changes.
#[must_use]
pub fn blend_modes_json() -> String {
    let entries: Vec<serde_json::Value> = BlendMode::ALL
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.as_str(),
                "label": m.label(),
                "family": m.family(),
            })
        })
        .collect();
    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into())
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
        /// Output black point — the value input black maps to.
        #[serde(default)]
        out_black: f32,
        /// Output white point — the value input white maps to.
        #[serde(default = "one")]
        out_white: f32,
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
    /// Saturation boost weighted against what is already saturated.
    Vibrance {
        amount: f32,
    },
    /// Monochrome conversion with explicit per-channel luminance weights.
    BlackAndWhite {
        red: f32,
        green: f32,
        blue: f32,
    },
    /// Temperature and tint, as a warm/cool and green/magenta shift.
    WhiteBalance {
        temperature: f32,
        tint: f32,
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

/// `serde` default for a levels output-white that predates the field.
fn one() -> f32 {
    1.0
}

/// Editor slots one adjustment kind may expose.
///
/// The composite shader carries this many floats per layer, so raising it is a
/// uniform-layout change, not a free one. Eight covers every kind shipped; a
/// kind needing a curve or a gradient wants a lookup texture rather than a
/// longer slot list.
pub const MAX_ADJUSTMENT_SLOTS: usize = 8;

/// Widen a kind's parameters into the fixed slot array.
fn pad<const N: usize>(values: [f32; N]) -> [f32; MAX_ADJUSTMENT_SLOTS] {
    let mut out = [0.0; MAX_ADJUSTMENT_SLOTS];
    out[..N].copy_from_slice(&values);
    out
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
                out_black,
                out_white,
            } => {
                let black = black.clamp(0.0, 1.0);
                let white = white.clamp(0.0, 1.0).max(black + 1e-4);
                Self::Levels {
                    black,
                    white,
                    gamma: gamma.clamp(0.01, 10.0),
                    out_black: out_black.clamp(0.0, 1.0),
                    out_white: out_white.clamp(0.0, 1.0),
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
            Self::Vibrance { amount } => Self::Vibrance {
                amount: amount.clamp(-1.0, 1.0),
            },
            Self::BlackAndWhite { red, green, blue } => Self::BlackAndWhite {
                red: red.clamp(0.0, 2.0),
                green: green.clamp(0.0, 2.0),
                blue: blue.clamp(0.0, 2.0),
            },
            Self::WhiteBalance { temperature, tint } => Self::WhiteBalance {
                temperature: temperature.clamp(-1.0, 1.0),
                tint: tint.clamp(-1.0, 1.0),
            },
        }
    }

    /// Short kind key for QML (`brightness`, `levels`, …).
    #[must_use]
    pub fn kind_key(&self) -> &'static str {
        match self {
            Self::BrightnessContrast { .. } => "brightness",
            Self::Levels { .. } => "levels",
            Self::HueSaturation { .. } => "hue",
            Self::Invert => "invert",
            Self::Threshold { .. } => "threshold",
            Self::Posterize { .. } => "posterize",
            Self::Exposure { .. } => "exposure",
            Self::Vibrance { .. } => "vibrance",
            Self::BlackAndWhite { .. } => "black-white",
            Self::WhiteBalance { .. } => "white-balance",
        }
    }

    /// Display name for the layer and for menus.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::BrightnessContrast { .. } => "Brightness/Contrast",
            Self::Levels { .. } => "Levels",
            Self::HueSaturation { .. } => "Hue/Saturation",
            Self::Invert => "Invert",
            Self::Threshold { .. } => "Threshold",
            Self::Posterize { .. } => "Posterize",
            Self::Exposure { .. } => "Exposure",
            Self::Vibrance { .. } => "Vibrance",
            Self::BlackAndWhite { .. } => "Black & White",
            Self::WhiteBalance { .. } => "White Balance",
        }
    }

    /// The composite shader's op code for this kind.
    ///
    /// `0` means "no adjustment", so a kind that forgets its code renders as
    /// an invisible layer rather than as an error — which is what four of
    /// these kinds did for as long as this mapping lived in the GPU crate,
    /// where the `_ => 0` arm silently absorbed every kind added here.
    /// [`Self::ALL_KINDS`] and the shader are held to each other by test.
    #[must_use]
    pub fn gpu_op(&self) -> u32 {
        match self {
            Self::BrightnessContrast { .. } => 1,
            Self::Levels { .. } => 2,
            Self::Exposure { .. } => 3,
            Self::HueSaturation { .. } => 4,
            Self::Invert => 5,
            Self::Threshold { .. } => 6,
            Self::Posterize { .. } => 7,
            Self::Vibrance { .. } => 8,
            Self::BlackAndWhite { .. } => 9,
            Self::WhiteBalance { .. } => 10,
        }
    }

    /// One default instance of every kind, in menu order.
    pub const ALL_KINDS: &'static [Self] = &[
        Self::BrightnessContrast {
            brightness: 0.0,
            contrast: 0.0,
        },
        Self::Levels {
            black: 0.0,
            white: 1.0,
            gamma: 1.0,
            out_black: 0.0,
            out_white: 1.0,
        },
        Self::Exposure {
            stops: 0.0,
            gamma: 1.0,
        },
        Self::HueSaturation {
            hue: 0.0,
            saturation: 0.0,
            lightness: 0.0,
        },
        Self::Invert,
        Self::Threshold { level: 0.5 },
        Self::Posterize { levels: 8 },
        Self::Vibrance { amount: 0.0 },
        Self::BlackAndWhite {
            red: 0.299,
            green: 0.587,
            blue: 0.114,
        },
        Self::WhiteBalance {
            temperature: 0.0,
            tint: 0.0,
        },
    ];

    /// The default parameters for a kind key; `None` for an unknown key.
    ///
    /// No fallback: the caller is creating a layer, and a Brightness/Contrast
    /// layer the user did not ask for is one they have to notice and undo.
    #[must_use]
    pub fn default_for_kind(kind: &str) -> Option<Self> {
        Self::ALL_KINDS
            .iter()
            .find(|p| p.kind_key() == kind)
            .cloned()
    }

    /// Parameters projected onto the editor slots.
    ///
    /// The slots are the chrome's whole vocabulary for an adjustment — QML
    /// binds one slider per slot and the command carries the same values — so
    /// the projection and its inverse belong together, next to the ranges that
    /// bound them. Slots past [`Self::editor_slots`]'s length are unused and
    /// read as zero.
    ///
    /// A fixed array rather than a `Vec`: this runs once per layer on every
    /// composite sync, and the budget is small enough that an allocation there
    /// would cost more than the eight floats.
    #[must_use]
    pub fn slots(&self) -> [f32; MAX_ADJUSTMENT_SLOTS] {
        match *self {
            Self::BrightnessContrast {
                brightness,
                contrast,
            } => pad([brightness, contrast]),
            Self::Levels {
                black,
                white,
                gamma,
                out_black,
                out_white,
            } => pad([black, white, gamma, out_black, out_white]),
            Self::HueSaturation {
                hue,
                saturation,
                lightness,
            } => pad([hue, saturation, lightness]),
            Self::Invert => [0.0; MAX_ADJUSTMENT_SLOTS],
            Self::Threshold { level } => pad([level]),
            #[expect(
                clippy::cast_precision_loss,
                reason = "posterize levels are clamped to 2..=256 and fit f32 exactly"
            )]
            Self::Posterize { levels } => pad([levels as f32]),
            Self::Exposure { stops, gamma } => pad([stops, gamma]),
            Self::Vibrance { amount } => pad([amount]),
            Self::BlackAndWhite { red, green, blue } => pad([red, green, blue]),
            Self::WhiteBalance { temperature, tint } => pad([temperature, tint]),
        }
    }

    /// Rebuild this kind from editor slot values.
    #[must_use]
    pub fn with_slots(&self, p: [f32; MAX_ADJUSTMENT_SLOTS]) -> Self {
        match self {
            Self::BrightnessContrast { .. } => Self::BrightnessContrast {
                brightness: p[0],
                contrast: p[1],
            },
            Self::Levels { .. } => Self::Levels {
                black: p[0],
                white: p[1],
                gamma: p[2],
                out_black: p[3],
                out_white: p[4],
            },
            Self::HueSaturation { .. } => Self::HueSaturation {
                hue: p[0],
                saturation: p[1],
                lightness: p[2],
            },
            Self::Invert => Self::Invert,
            Self::Threshold { .. } => Self::Threshold { level: p[0] },
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "rounded and clamped to 2..=256 before the cast"
            )]
            Self::Posterize { .. } => Self::Posterize {
                levels: p[0].round().clamp(2.0, 256.0) as u32,
            },
            Self::Exposure { .. } => Self::Exposure {
                stops: p[0],
                gamma: p[1].max(0.01),
            },
            Self::Vibrance { .. } => Self::Vibrance { amount: p[0] },
            Self::BlackAndWhite { .. } => Self::BlackAndWhite {
                red: p[0],
                green: p[1],
                blue: p[2],
            },
            Self::WhiteBalance { .. } => Self::WhiteBalance {
                temperature: p[0],
                tint: p[1],
            },
        }
    }

    /// Editor slots this kind exposes, as `(label, min, max)`.
    ///
    /// Position in this list *is* the slot index, so a parameter cannot be
    /// described by one index and read from another. An empty list means the
    /// kind has no parameters — Invert is the whole adjustment — not that its
    /// editor is missing.
    #[must_use]
    pub fn editor_slots(&self) -> &'static [(&'static str, f32, f32)] {
        match self {
            Self::BrightnessContrast { .. } => {
                &[("Brightness", -1.0, 1.0), ("Contrast", -1.0, 1.0)]
            }
            Self::Levels { .. } => &[
                ("Black", 0.0, 1.0),
                ("White", 0.0, 1.0),
                ("Gamma", 0.1, 3.0),
                ("Output Black", 0.0, 1.0),
                ("Output White", 0.0, 1.0),
            ],
            Self::Exposure { .. } => &[("Stops", -5.0, 5.0), ("Gamma", 0.1, 3.0)],
            Self::HueSaturation { .. } => &[
                ("Hue", -1.0, 1.0),
                ("Saturation", -1.0, 1.0),
                ("Lightness", -1.0, 1.0),
            ],
            Self::Invert => &[],
            Self::Threshold { .. } => &[("Level", 0.0, 1.0)],
            Self::Posterize { .. } => &[("Levels", 2.0, 32.0)],
            Self::Vibrance { .. } => &[("Amount", -1.0, 1.0)],
            Self::BlackAndWhite { .. } => {
                &[("Red", 0.0, 2.0), ("Green", 0.0, 2.0), ("Blue", 0.0, 2.0)]
            }
            Self::WhiteBalance { .. } => &[("Temperature", -1.0, 1.0), ("Tint", -1.0, 1.0)],
        }
    }

    /// Apply this adjustment to one straight-alpha RGB colour.
    ///
    /// The reference the composite shader mirrors. Keeping it here rather than
    /// only in WGSL is what makes an adjustment testable without a device, and
    /// what a parity fixture compares the shader against.
    #[must_use]
    pub fn apply_rgb(&self, rgb: [f32; 3]) -> [f32; 3] {
        let clamp3 = |c: [f32; 3]| c.map(|v| v.clamp(0.0, 1.0));
        match *self {
            Self::BrightnessContrast {
                brightness,
                contrast,
            } => {
                let c = contrast + 1.0;
                clamp3(rgb.map(|v| (v - 0.5) * c + 0.5 + brightness))
            }
            Self::Levels {
                black,
                white,
                gamma,
                out_black,
                out_white,
            } => {
                let span = (white - black).max(1e-5);
                let g = gamma.max(0.01);
                let out_span = out_white - out_black;
                clamp3(rgb.map(|v| {
                    let t = ((v - black) / span).clamp(0.0, 1.0).powf(1.0 / g);
                    out_black + t * out_span
                }))
            }
            Self::Exposure { stops, gamma } => {
                let mul = stops.clamp(-5.0, 5.0).exp2();
                let g = gamma.max(0.01);
                clamp3(rgb.map(|v| (v * mul).clamp(0.0, 1.0).powf(1.0 / g)))
            }
            Self::HueSaturation {
                hue,
                saturation,
                lightness,
            } => {
                let mut hsl = rgb_to_hsl(rgb);
                hsl[0] = (hsl[0] + hue).rem_euclid(1.0);
                // Saturation and lightness read as "toward grey / toward the
                // end of the range", so a negative value scales down and a
                // positive one closes the remaining distance. A plain multiply
                // would make +1 mean "unchanged" on an already-saturated pixel.
                hsl[1] = if saturation >= 0.0 {
                    hsl[1] + (1.0 - hsl[1]) * saturation
                } else {
                    hsl[1] * (1.0 + saturation)
                };
                hsl[2] = if lightness >= 0.0 {
                    hsl[2] + (1.0 - hsl[2]) * lightness
                } else {
                    hsl[2] * (1.0 + lightness)
                };
                clamp3(hsl_to_rgb([
                    hsl[0],
                    hsl[1].clamp(0.0, 1.0),
                    hsl[2].clamp(0.0, 1.0),
                ]))
            }
            Self::Invert => rgb.map(|v| 1.0 - v.clamp(0.0, 1.0)),
            Self::Threshold { level } => {
                let luma = 0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2];
                let v = if luma >= level { 1.0 } else { 0.0 };
                [v; 3]
            }
            // Boost weighted by how much room a pixel has left: already
            // saturated colours barely move, which is what separates this from
            // the saturation slider next door.
            Self::Vibrance { amount } => {
                let max = rgb[0].max(rgb[1]).max(rgb[2]);
                let min = rgb[0].min(rgb[1]).min(rgb[2]);
                let sat = max - min;
                let gain = amount * (1.0 - sat);
                let luma = 0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2];
                clamp3(rgb.map(|v| luma + (v - luma) * (1.0 + gain)))
            }
            Self::BlackAndWhite { red, green, blue } => {
                // Normalized so a set of weights cannot brighten or darken the
                // image as a side effect of summing to something other than 1.
                let sum = (red + green + blue).max(1e-4);
                let grey = (red * rgb[0] + green * rgb[1] + blue * rgb[2]) / sum;
                [grey.clamp(0.0, 1.0); 3]
            }
            Self::WhiteBalance { temperature, tint } => {
                // Warm lifts red and drops blue; tint trades green against the
                // red/blue pair. Both are gains around 1.0 so zero is neutral.
                let warm = temperature * 0.3;
                let green_shift = tint * 0.3;
                clamp3([
                    rgb[0] * (1.0 + warm) * (1.0 - green_shift * 0.5),
                    rgb[1] * (1.0 + green_shift),
                    rgb[2] * (1.0 - warm) * (1.0 - green_shift * 0.5),
                ])
            }
            Self::Posterize { levels } => {
                let n = levels.clamp(2, 256);
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "level counts are clamped to 2..=256"
                )]
                let steps = (n - 1) as f32;
                clamp3(rgb.map(|v| (v.clamp(0.0, 1.0) * steps).round() / steps))
            }
        }
    }
}

/// RGB → HSL, all components in 0..1 (hue wraps).
#[must_use]
pub fn rgb_to_hsl(rgb: [f32; 3]) -> [f32; 3] {
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
pub fn hsl_to_rgb(hsl: [f32; 3]) -> [f32; 3] {
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    /// Radial streaking outward from the centre.
    ZoomBlur {
        amount: f32,
    },
    /// The classic controllable sharpener: blur, then add back the difference.
    UnsharpMask {
        radius: f32,
        amount: f32,
    },
    /// Edge isolation — the image minus its own blur, recentred on grey.
    HighPass {
        radius: f32,
    },
    /// Local midtone contrast.
    Clarity {
        radius: f32,
        amount: f32,
    },
    /// Edge-preserving noise reduction (bilateral).
    Denoise {
        radius: f32,
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
            Self::ZoomBlur { amount } => Self::ZoomBlur {
                amount: amount.clamp(0.0, 1.0),
            },
            Self::UnsharpMask { radius, amount } => Self::UnsharpMask {
                radius: radius.clamp(0.0, MAX_BLUR_RADIUS),
                amount: amount.clamp(0.0, 4.0),
            },
            Self::HighPass { radius } => Self::HighPass {
                radius: radius.clamp(0.0, MAX_BLUR_RADIUS),
            },
            Self::Clarity { radius, amount } => Self::Clarity {
                radius: radius.clamp(0.0, MAX_BLUR_RADIUS),
                amount: amount.clamp(-1.0, 1.0),
            },
            Self::Denoise { radius, amount } => Self::Denoise {
                radius: radius.clamp(0.0, 16.0),
                amount: amount.clamp(0.0, 1.0),
            },
        }
    }

    /// Short kind key for the registry, the gallery and `.ptx`.
    #[must_use]
    pub fn kind_key(&self) -> &'static str {
        match self {
            Self::GaussianBlur { .. } => "gaussian",
            Self::BoxBlur { .. } => "box",
            Self::Sharpen { .. } => "sharpen",
            Self::Invert => "invert",
            Self::Offset { .. } => "offset",
            Self::MotionBlur { .. } => "motion",
            Self::Emboss { .. } => "emboss",
            Self::Noise { .. } => "noise",
            Self::ZoomBlur { .. } => "zoom",
            Self::UnsharpMask { .. } => "unsharp",
            Self::HighPass { .. } => "high-pass",
            Self::Clarity { .. } => "clarity",
            Self::Denoise { .. } => "denoise",
        }
    }

    /// Display name for the effect stack and menus.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::GaussianBlur { .. } => "Gaussian Blur",
            Self::BoxBlur { .. } => "Box Blur",
            Self::Sharpen { .. } => "Sharpen",
            Self::Invert => "Invert",
            Self::Offset { .. } => "Offset",
            Self::MotionBlur { .. } => "Motion Blur",
            Self::Emboss { .. } => "Emboss",
            Self::Noise { .. } => "Add Noise",
            Self::ZoomBlur { .. } => "Zoom Blur",
            Self::UnsharpMask { .. } => "Unsharp Mask",
            Self::HighPass { .. } => "High Pass",
            Self::Clarity { .. } => "Clarity",
            Self::Denoise { .. } => "Denoise",
        }
    }

    /// The effect shader's mode for this kind.
    ///
    /// `0` is the shader's copy-through, so a kind that reaches it renders as
    /// no filter at all — the failure that hid four adjustment kinds. Modes
    /// 3–6 and 8 belong to layer styles and compositing, not to filters.
    #[must_use]
    pub fn shader_mode(&self) -> u32 {
        match self {
            // Gaussian and box run through `SeparableBlur`, not this shader.
            Self::GaussianBlur { .. } | Self::BoxBlur { .. } => 0,
            Self::MotionBlur { .. } => 1,
            Self::Emboss { .. } => 2,
            Self::Sharpen { .. } => 7,
            Self::Noise { .. } => 9,
            Self::Invert => 10,
            Self::Offset { .. } => 11,
            Self::ZoomBlur { .. } => 12,
            // Unsharp, high pass, clarity and denoise all read a blurred copy
            // alongside the source, so they share the two-input path.
            Self::UnsharpMask { .. } => 13,
            Self::HighPass { .. } => 14,
            Self::Clarity { .. } => 15,
            Self::Denoise { .. } => 16,
        }
    }

    /// Whether this kind needs a blurred copy of the source alongside it.
    ///
    /// The blur radius is the kind's first slot. Grouping the four here rather
    /// than at each call site is what keeps the executor from having to know
    /// which filters happen to be built on a blur.
    #[must_use]
    pub fn blur_radius_input(&self) -> Option<f32> {
        match *self {
            Self::UnsharpMask { radius, .. }
            | Self::HighPass { radius }
            | Self::Clarity { radius, .. } => Some(radius.max(0.5)),
            Self::Denoise { radius, .. } => Some(radius.max(0.5)),
            _ => None,
        }
    }

    /// One default instance of every kind, in menu order.
    pub const ALL_KINDS: &'static [Self] = &[
        Self::GaussianBlur { radius: 4.0 },
        Self::BoxBlur { radius: 4.0 },
        Self::MotionBlur {
            distance: 8.0,
            angle_deg: 0.0,
        },
        Self::ZoomBlur { amount: 0.25 },
        Self::Sharpen { amount: 1.0 },
        Self::UnsharpMask {
            radius: 2.0,
            amount: 1.0,
        },
        Self::HighPass { radius: 3.0 },
        Self::Clarity {
            radius: 6.0,
            amount: 0.4,
        },
        Self::Denoise {
            radius: 3.0,
            amount: 0.5,
        },
        Self::Noise { amount: 0.35 },
        Self::Emboss {
            strength: 1.0,
            angle_deg: 135.0,
        },
        Self::Invert,
        Self::Offset { x: 0, y: 0 },
    ];

    /// The default parameters for a kind key; `None` for an unknown key.
    #[must_use]
    pub fn default_for_kind(kind: &str) -> Option<Self> {
        Self::ALL_KINDS
            .iter()
            .find(|p| p.kind_key() == kind)
            .copied()
    }

    /// Parameters projected onto the editor slots.
    #[must_use]
    pub fn slots(&self) -> [f32; MAX_ADJUSTMENT_SLOTS] {
        match *self {
            Self::GaussianBlur { radius }
            | Self::BoxBlur { radius }
            | Self::HighPass { radius } => pad([radius]),
            Self::Sharpen { amount } | Self::Noise { amount } | Self::ZoomBlur { amount } => {
                pad([amount])
            }
            Self::Invert => [0.0; MAX_ADJUSTMENT_SLOTS],
            #[expect(
                clippy::cast_precision_loss,
                reason = "offsets are document pixels and fit f32 well below the mantissa limit"
            )]
            Self::Offset { x, y } => pad([x as f32, y as f32]),
            Self::MotionBlur {
                distance,
                angle_deg,
            } => pad([distance, angle_deg]),
            Self::Emboss {
                strength,
                angle_deg,
            } => pad([strength, angle_deg]),
            Self::UnsharpMask { radius, amount }
            | Self::Clarity { radius, amount }
            | Self::Denoise { radius, amount } => pad([radius, amount]),
        }
    }

    /// Rebuild this kind from editor slot values.
    #[must_use]
    pub fn with_slots(&self, p: [f32; MAX_ADJUSTMENT_SLOTS]) -> Self {
        match self {
            Self::GaussianBlur { .. } => Self::GaussianBlur { radius: p[0] },
            Self::BoxBlur { .. } => Self::BoxBlur { radius: p[0] },
            Self::Sharpen { .. } => Self::Sharpen { amount: p[0] },
            Self::Invert => Self::Invert,
            #[expect(
                clippy::cast_possible_truncation,
                reason = "offsets are clamped to the document by the executor"
            )]
            Self::Offset { .. } => Self::Offset {
                x: p[0] as i32,
                y: p[1] as i32,
            },
            Self::MotionBlur { .. } => Self::MotionBlur {
                distance: p[0],
                angle_deg: p[1],
            },
            Self::Emboss { .. } => Self::Emboss {
                strength: p[0],
                angle_deg: p[1],
            },
            Self::Noise { .. } => Self::Noise { amount: p[0] },
            Self::ZoomBlur { .. } => Self::ZoomBlur { amount: p[0] },
            Self::UnsharpMask { .. } => Self::UnsharpMask {
                radius: p[0],
                amount: p[1],
            },
            Self::HighPass { .. } => Self::HighPass { radius: p[0] },
            Self::Clarity { .. } => Self::Clarity {
                radius: p[0],
                amount: p[1],
            },
            Self::Denoise { .. } => Self::Denoise {
                radius: p[0],
                amount: p[1],
            },
        }
    }

    /// Editor slots this kind exposes, as `(label, min, max)`; position is the
    /// slot index.
    #[must_use]
    pub fn editor_slots(&self) -> &'static [(&'static str, f32, f32)] {
        match self {
            Self::GaussianBlur { .. } | Self::BoxBlur { .. } => &[("Radius", 0.0, MAX_BLUR_RADIUS)],
            Self::Sharpen { .. } => &[("Amount", 0.0, 4.0)],
            Self::Invert => &[],
            Self::Offset { .. } => &[("X", -512.0, 512.0), ("Y", -512.0, 512.0)],
            Self::MotionBlur { .. } => &[("Distance", 0.0, MAX_BLUR_RADIUS), ("Angle", 0.0, 360.0)],
            Self::Emboss { .. } => &[("Strength", 0.0, 4.0), ("Angle", 0.0, 360.0)],
            Self::Noise { .. } => &[("Amount", 0.0, 1.0)],
            Self::ZoomBlur { .. } => &[("Amount", 0.0, 1.0)],
            Self::UnsharpMask { .. } => &[("Radius", 0.0, MAX_BLUR_RADIUS), ("Amount", 0.0, 4.0)],
            Self::HighPass { .. } => &[("Radius", 0.0, MAX_BLUR_RADIUS)],
            Self::Clarity { .. } => &[("Radius", 0.0, MAX_BLUR_RADIUS), ("Amount", -1.0, 1.0)],
            Self::Denoise { .. } => &[("Radius", 0.0, 16.0), ("Amount", 0.0, 1.0)],
        }
    }

    /// Whether this kind is worth an effect pass at these parameters.
    ///
    /// A blur of 0.001 px is not worth a render target; nor is a sharpen of
    /// zero. Kinds with no threshold — Invert — always contribute.
    #[must_use]
    pub fn is_significant(&self) -> bool {
        match *self {
            Self::GaussianBlur { radius }
            | Self::BoxBlur { radius }
            | Self::HighPass { radius } => radius > 0.01,
            Self::MotionBlur { distance, .. } => distance > 0.01,
            Self::Emboss { strength, .. } => strength > 0.01,
            Self::Sharpen { amount } | Self::Noise { amount } | Self::ZoomBlur { amount } => {
                amount > 0.001
            }
            Self::UnsharpMask { amount, .. } | Self::Denoise { amount, .. } => amount > 0.001,
            Self::Clarity { amount, .. } => amount.abs() > 0.001,
            Self::Offset { x, y } => x != 0 || y != 0,
            Self::Invert => true,
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
    /// Blend ranges — hide this layer where it, or what is under it, falls
    /// outside a channel range. `default` so documents written before this
    /// existed open with the ranges that hide nothing.
    #[serde(default)]
    pub blend_if: crate::BlendIf,
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
            blend_if: crate::BlendIf::default(),
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

    /// Zero feather must not copy: most masks never set it, and the bake runs
    /// on every apply.
    #[test]
    fn no_feather_borrows_the_mask_unchanged() {
        let mask = LayerMask::default();
        let stored = vec![0_u8, 128, 255, 64];
        let out = mask.feathered(2, 2, &stored);
        assert!(matches!(out, std::borrow::Cow::Borrowed(_)));
        assert_eq!(&*out, &stored[..]);
    }

    /// Feathering a hard edge must soften it — the whole point of the control,
    /// and what it did not do while nothing consumed the stored value.
    #[test]
    fn feather_softens_a_hard_edge() {
        let mask = LayerMask {
            feather: 1.0,
            ..Default::default()
        };
        // A 4x1 step from fully hidden to fully revealed.
        let stored = vec![0_u8, 0, 255, 255];
        let out = mask.feathered(4, 1, &stored);
        assert!(matches!(out, std::borrow::Cow::Owned(_)));
        assert!(
            out[1] > 0 && out[1] < 255,
            "the texel before the edge picks up its neighbour: {out:?}"
        );
        assert!(out[2] > 0 && out[2] < 255, "and the one after it: {out:?}");
    }

    /// Feather and the per-sample parameters are two halves of one definition,
    /// applied in that order. Baking without softening first is what made the
    /// control appear to do nothing.
    #[test]
    fn a_baked_mask_reflects_feather_before_coverage() {
        let mask = LayerMask {
            feather: 1.0,
            ..Default::default()
        };
        let stored = vec![0_u8, 0, 255, 255];
        let mut hard = vec![255_u8; 16];
        let mut soft = vec![255_u8; 16];

        mask.bake_into_rgba8(&mut hard, &stored).expect("hard bake");
        let softened = mask.feathered(4, 1, &stored);
        mask.bake_into_rgba8(&mut soft, &softened)
            .expect("soft bake");

        assert_eq!(hard[7], 0, "unsoftened, the second texel is fully hidden");
        assert!(
            soft[7] > 0,
            "softened, it takes some coverage from its neighbour"
        );
    }

    #[test]
    fn opacity_clamped() {
        let mut l = Layer::new(LayerId(1), "A");
        l.set_opacity(2.0);
        assert!((l.opacity - 1.0).abs() < f32::EPSILON);
        l.set_opacity(-1.0);
        assert!((l.opacity - 0.0).abs() < f32::EPSILON);
    }

    /// `0` is "no adjustment" in the shader, so a kind that reaches it renders
    /// as an invisible layer. Four kinds did exactly that.
    #[test]
    fn every_adjustment_kind_has_a_live_gpu_op() {
        let mut seen = Vec::new();
        for params in AdjustmentParams::ALL_KINDS {
            let op = params.gpu_op();
            assert_ne!(op, 0, "{} renders as nothing", params.kind_key());
            assert!(!seen.contains(&op), "{} reuses op {op}", params.kind_key());
            seen.push(op);
        }
    }

    /// The command that creates an adjustment layer looks kinds up by key, so
    /// a kind whose key does not round-trip cannot be created at all.
    #[test]
    fn every_adjustment_kind_is_reachable_by_key() {
        for params in AdjustmentParams::ALL_KINDS {
            let found = AdjustmentParams::default_for_kind(params.kind_key());
            assert_eq!(found.as_ref(), Some(params));
            assert!(!params.label().is_empty());
        }
        assert_eq!(AdjustmentParams::default_for_kind("nonsense"), None);
    }

    /// The chrome edits an adjustment only through its slots, so a slot the
    /// projection drops is a control that silently does nothing — and a slot
    /// with no declared range is a slider with no bounds.
    #[test]
    fn editor_slots_round_trip_through_the_projection() {
        for params in AdjustmentParams::ALL_KINDS {
            let slots = params.slots();
            assert_eq!(
                &params.with_slots(slots),
                params,
                "{} does not survive its own projection",
                params.kind_key()
            );
            assert!(
                params.editor_slots().len() <= MAX_ADJUSTMENT_SLOTS,
                "{} declares more slots than the shader carries",
                params.kind_key()
            );
            for (index, (label, min, max)) in params.editor_slots().iter().enumerate() {
                assert!(!label.is_empty());
                assert!(
                    min < max,
                    "{} slot {index}: {min}..{max}",
                    params.kind_key()
                );
            }
        }
    }

    /// Every kind must be able to change a pixel. A kind whose `apply_rgb` is
    /// the identity has no implementation, however complete its metadata.
    #[test]
    fn every_adjustment_changes_some_pixel() {
        let probe = [0.25, 0.55, 0.8];
        for params in AdjustmentParams::ALL_KINDS {
            // Defaults are deliberately neutral, so each kind is nudged off it.
            let mut slots = params.slots();
            // Defaults are deliberately neutral, so each slot is nudged off it
            // within the range its editor declares.
            for (i, (_, min, max)) in params.editor_slots().iter().enumerate() {
                slots[i] = (slots[i] + (max - min) * 0.25).clamp(*min, *max);
            }
            let moved = params.with_slots(slots);
            let out = moved.apply_rgb(probe);
            assert!(
                out != probe,
                "{} left the pixel untouched",
                params.kind_key()
            );
            for v in out {
                assert!(
                    (0.0..=1.0).contains(&v),
                    "{} escaped range",
                    params.kind_key()
                );
            }
        }
    }

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

    /// A neutral Hue/Saturation is the one a freshly created layer carries, and
    /// it must not tint the document the moment it appears.
    #[test]
    fn a_neutral_hue_saturation_is_a_no_op() {
        let neutral = AdjustmentParams::HueSaturation {
            hue: 0.0,
            saturation: 0.0,
            lightness: 0.0,
        };
        let probe = [0.8, 0.3, 0.45];
        let out = neutral.apply_rgb(probe);
        for (a, b) in out.iter().zip(probe) {
            assert!((a - b).abs() < 1e-4, "{probe:?} -> {out:?}");
        }
    }

    #[test]
    fn blend_roundtrip() {
        for &b in BlendMode::ALL {
            assert_eq!(BlendMode::from_str_label(b.as_str()), Some(b));
        }
    }

    /// The shader switches on these codes and `.ptx` does not record them, so
    /// a collision repaints one mode as another and a change repaints saved
    /// documents. Both are silent.
    #[test]
    fn blend_gpu_codes_are_unique_and_pinned() {
        let mut seen = Vec::new();
        for &b in BlendMode::ALL {
            let code = b.as_u32();
            assert!(!seen.contains(&code), "{b:?} reuses GPU code {code}");
            seen.push(code);
        }
        // The codes that shipped before the set was completed.
        for (mode, code) in [
            (BlendMode::Normal, 0),
            (BlendMode::Multiply, 1),
            (BlendMode::Screen, 2),
            (BlendMode::Overlay, 3),
            (BlendMode::Darken, 4),
            (BlendMode::Lighten, 5),
            (BlendMode::ColorDodge, 6),
            (BlendMode::ColorBurn, 7),
            (BlendMode::HardLight, 8),
            (BlendMode::SoftLight, 9),
            (BlendMode::Difference, 10),
            (BlendMode::Exclusion, 11),
            (BlendMode::Hue, 12),
            (BlendMode::Saturation, 13),
            (BlendMode::Color, 14),
            (BlendMode::Luminosity, 15),
            (BlendMode::PassThrough, 16),
        ] {
            assert_eq!(mode.as_u32(), code, "{mode:?} changed GPU code");
        }
    }

    /// The combo bands the list by family, so a family that resumes after
    /// another draws a separator through the middle of itself.
    #[test]
    fn blend_families_are_contiguous() {
        let mut seen: Vec<&str> = Vec::new();
        let mut previous = "";
        for &b in BlendMode::ALL {
            if b.family() != previous {
                assert!(
                    !seen.contains(&b.family()),
                    "family {} resumes after {previous}",
                    b.family()
                );
                seen.push(b.family());
                previous = b.family();
            }
        }
    }

    /// The chrome reads this JSON instead of restating the list, which is how
    /// the combo came to offer eight of the seventeen modes.
    #[test]
    fn blend_modes_json_lists_every_mode() {
        let json = blend_modes_json();
        for &b in BlendMode::ALL {
            assert!(json.contains(b.as_str()), "{b:?} missing from chrome JSON");
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

    /// The shader applies invert, then contrast/shift, then density. Baking
    /// dropped the middle step, so a mask with contrast or shift baked to
    /// different pixels than the canvas showed.
    #[test]
    fn coverage_applies_refine_between_invert_and_density() {
        let mask = LayerMask {
            contrast: 0.5,
            ..LayerMask::default()
        };
        // Refine pivots on 0.5, so the midpoint is a fixed point ...
        assert!((mask.coverage(0.5) - 0.5).abs() < 1e-6);
        // ... while everything else is pushed away from it.
        assert!(mask.coverage(0.75) > 0.75);
        assert!(mask.coverage(0.25) < 0.25);
    }

    #[test]
    fn coverage_shift_raises_every_sample() {
        let plain = LayerMask::default();
        let shifted = LayerMask {
            shift: 0.25,
            ..LayerMask::default()
        };
        for step in 0..=10 {
            let sample = step as f32 / 10.0;
            assert!(
                shifted.coverage(sample) >= plain.coverage(sample),
                "shift must not lower coverage at {sample}"
            );
        }
    }

    #[test]
    fn coverage_inverts_before_refining() {
        let mask = LayerMask {
            inverted: true,
            ..LayerMask::default()
        };
        assert!((mask.coverage(1.0) - 0.0).abs() < 1e-6);
        assert!((mask.coverage(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn zero_density_yields_full_coverage_whatever_the_sample() {
        let mask = LayerMask {
            density: 0.0,
            ..LayerMask::default()
        };
        for step in 0..=10 {
            assert!((mask.coverage(step as f32 / 10.0) - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn coverage_stays_in_range_across_extremes() {
        for &contrast in &[-1.0, 0.0, 1.0] {
            for &shift in &[-1.0, 0.0, 1.0] {
                for &density in &[0.0, 0.5, 1.0] {
                    let mask = LayerMask {
                        contrast,
                        shift,
                        density,
                        ..LayerMask::default()
                    };
                    for step in 0..=20 {
                        let c = mask.coverage(step as f32 / 20.0);
                        assert!((0.0..=1.0).contains(&c), "coverage {c} out of range");
                    }
                }
            }
        }
    }

    #[test]
    fn bake_multiplies_alpha_and_rejects_mismatched_buffers() {
        let mask = LayerMask::default();
        let mut rgba = vec![255u8; 8];
        mask.bake_into_rgba8(&mut rgba, &[255, 0]).expect("bake");
        assert_eq!(rgba[3], 255, "full coverage keeps alpha");
        assert_eq!(rgba[7], 0, "zero coverage clears alpha");

        let mut short = vec![255u8; 8];
        assert!(mask.bake_into_rgba8(&mut short, &[255]).is_err());
    }
}
