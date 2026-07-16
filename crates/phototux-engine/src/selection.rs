//! Document selection channel state (Phase 7).

use serde::{Deserialize, Serialize};

/// How a new selection combines with the current channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionCombine {
    #[default]
    Replace,
    Add,
    Subtract,
    Intersect,
}

impl SelectionCombine {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Replace => "replace",
            Self::Add => "add",
            Self::Subtract => "subtract",
            Self::Intersect => "intersect",
        }
    }
}

/// Axis-aligned rectangle in document pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Ellipse inscribed in a document-space bounds rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionEllipse {
    pub bounds: SelectionRect,
}

/// CPU-side selection metadata; GPU owns the R8 mask when active.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionState {
    pub active: bool,
    pub combine: SelectionCombine,
    pub feather: f32,
    pub bounds: Option<SelectionRect>,
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            active: false,
            combine: SelectionCombine::Replace,
            feather: 0.0,
            bounds: None,
        }
    }
}

impl SelectionState {
    pub fn clear(&mut self) {
        self.active = false;
        self.bounds = None;
    }

    pub fn select_all(&mut self, width: u32, height: u32) {
        self.active = true;
        self.bounds = Some(SelectionRect {
            x: 0,
            y: 0,
            width,
            height,
        });
    }

    pub fn set_rect(&mut self, rect: SelectionRect, combine: SelectionCombine) {
        self.combine = combine;
        self.active = rect.width > 0 && rect.height > 0;
        self.bounds = if self.active { Some(rect) } else { None };
    }

    pub fn invert_bounds(&mut self, width: u32, height: u32) {
        if !self.active {
            self.select_all(width, height);
            return;
        }
        // Metadata invert: full canvas when a partial rect exists (GPU mask does true invert).
        self.select_all(width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_all_and_clear() {
        let mut s = SelectionState::default();
        s.select_all(100, 50);
        assert!(s.active);
        assert_eq!(s.bounds.map(|b| b.width), Some(100));
        s.clear();
        assert!(!s.active);
    }
}
