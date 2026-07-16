//! Document selection channel state (Phase 7 / selection polish).

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

    pub fn parse(label: &str) -> Self {
        match label {
            "add" => Self::Add,
            "subtract" => Self::Subtract,
            "intersect" => Self::Intersect,
            _ => Self::Replace,
        }
    }
}

/// Geometry used for the last committed selection outline (QML ants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionShape {
    #[default]
    Rect,
    Ellipse,
}

impl SelectionShape {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Ellipse => "ellipse",
        }
    }

    pub fn parse(label: &str) -> Self {
        match label {
            "ellipse" => Self::Ellipse,
            _ => Self::Rect,
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

impl SelectionRect {
    pub fn union(self, other: Self) -> Self {
        let x0 = self.x.min(other.x);
        let y0 = self.y.min(other.y);
        let x1 = self
            .x
            .saturating_add(i32::try_from(self.width).unwrap_or(i32::MAX))
            .max(
                other
                    .x
                    .saturating_add(i32::try_from(other.width).unwrap_or(i32::MAX)),
            );
        let y1 = self
            .y
            .saturating_add(i32::try_from(self.height).unwrap_or(i32::MAX))
            .max(
                other
                    .y
                    .saturating_add(i32::try_from(other.height).unwrap_or(i32::MAX)),
            );
        Self {
            x: x0,
            y: y0,
            width: u32::try_from((x1 - x0).max(0)).unwrap_or(0),
            height: u32::try_from((y1 - y0).max(0)).unwrap_or(0),
        }
    }

    pub fn intersect(self, other: Self) -> Option<Self> {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = self
            .x
            .saturating_add(i32::try_from(self.width).unwrap_or(i32::MAX))
            .min(
                other
                    .x
                    .saturating_add(i32::try_from(other.width).unwrap_or(i32::MAX)),
            );
        let y1 = self
            .y
            .saturating_add(i32::try_from(self.height).unwrap_or(i32::MAX))
            .min(
                other
                    .y
                    .saturating_add(i32::try_from(other.height).unwrap_or(i32::MAX)),
            );
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        Some(Self {
            x: x0,
            y: y0,
            width: u32::try_from(x1 - x0).unwrap_or(0),
            height: u32::try_from(y1 - y0).unwrap_or(0),
        })
    }
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
    pub shape: SelectionShape,
    pub feather: f32,
    pub bounds: Option<SelectionRect>,
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            active: false,
            combine: SelectionCombine::Replace,
            shape: SelectionShape::Rect,
            feather: 0.0,
            bounds: None,
        }
    }
}

impl SelectionState {
    pub fn clear(&mut self) {
        self.active = false;
        self.bounds = None;
        self.shape = SelectionShape::Rect;
    }

    pub fn select_all(&mut self, width: u32, height: u32) {
        self.active = width > 0 && height > 0;
        self.shape = SelectionShape::Rect;
        self.bounds = self.active.then_some(SelectionRect {
            x: 0,
            y: 0,
            width,
            height,
        });
    }

    pub fn set_rect(&mut self, rect: SelectionRect, combine: SelectionCombine) {
        self.apply_shape(rect, SelectionShape::Rect, combine);
    }

    pub fn set_ellipse(&mut self, rect: SelectionRect, combine: SelectionCombine) {
        self.apply_shape(rect, SelectionShape::Ellipse, combine);
    }

    fn apply_shape(
        &mut self,
        rect: SelectionRect,
        shape: SelectionShape,
        combine: SelectionCombine,
    ) {
        self.combine = combine;
        if rect.width == 0 || rect.height == 0 {
            if matches!(combine, SelectionCombine::Replace) {
                self.clear();
            }
            return;
        }
        match combine {
            SelectionCombine::Replace => {
                self.active = true;
                self.shape = shape;
                self.bounds = Some(rect);
            }
            SelectionCombine::Add => {
                self.active = true;
                match self.bounds {
                    Some(prev) => {
                        self.bounds = Some(prev.union(rect));
                        // Mixed geometry: ants use axis-aligned union outline.
                        self.shape = SelectionShape::Rect;
                    }
                    None => {
                        self.bounds = Some(rect);
                        self.shape = shape;
                    }
                }
            }
            SelectionCombine::Subtract => {
                // Keep previous outline bounds when active; GPU mask is authoritative.
            }
            SelectionCombine::Intersect => {
                let Some(prev) = self.bounds else {
                    self.clear();
                    return;
                };
                match prev.intersect(rect) {
                    Some(next) => {
                        self.active = true;
                        self.shape = shape;
                        self.bounds = Some(next);
                    }
                    None => self.clear(),
                }
            }
        }
    }

    pub fn invert_bounds(&mut self, width: u32, height: u32) {
        // After GPU invert, outline covers the full document.
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
        assert_eq!(s.shape, SelectionShape::Rect);
        s.clear();
        assert!(!s.active);
    }

    #[test]
    fn set_ellipse_replace() {
        let mut s = SelectionState::default();
        s.set_ellipse(
            SelectionRect {
                x: 10,
                y: 20,
                width: 40,
                height: 30,
            },
            SelectionCombine::Replace,
        );
        assert!(s.active);
        assert_eq!(s.shape, SelectionShape::Ellipse);
        assert_eq!(s.bounds.map(|b| b.x), Some(10));
    }

    #[test]
    fn add_unions_bounds() {
        let mut s = SelectionState::default();
        s.set_rect(
            SelectionRect {
                x: 0,
                y: 0,
                width: 10,
                height: 10,
            },
            SelectionCombine::Replace,
        );
        s.set_rect(
            SelectionRect {
                x: 5,
                y: 5,
                width: 10,
                height: 10,
            },
            SelectionCombine::Add,
        );
        let b = s.bounds.expect("bounds");
        assert_eq!(b.x, 0);
        assert_eq!(b.y, 0);
        assert_eq!(b.width, 15);
        assert_eq!(b.height, 15);
    }

    #[test]
    fn combine_parse() {
        assert_eq!(SelectionCombine::parse("add"), SelectionCombine::Add);
        assert_eq!(SelectionCombine::parse("nope"), SelectionCombine::Replace);
    }
}
