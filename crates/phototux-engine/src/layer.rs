//! Layer types for the document stack (ADR-011).

/// Stable id for a layer within a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayerId(pub u64);

/// Blend modes for MVP composite (WGSL-backed on GPU).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
}

impl BlendMode {
    pub const ALL: [BlendMode; 4] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Multiply => "multiply",
            Self::Screen => "screen",
            Self::Overlay => "overlay",
        }
    }

    pub fn from_str_label(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "normal" => Some(Self::Normal),
            "multiply" => Some(Self::Multiply),
            "screen" => Some(Self::Screen),
            "overlay" => Some(Self::Overlay),
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
        }
    }
}

/// One layer in the ordered stack (metadata only; pixels live on GPU).
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub opacity: f32,
    pub visible: bool,
    pub locked: bool,
    pub blend: BlendMode,
}

impl Layer {
    pub fn new(id: LayerId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            opacity: 1.0,
            visible: true,
            locked: false,
            blend: BlendMode::Normal,
        }
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
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
}
