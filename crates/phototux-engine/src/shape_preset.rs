//! Starting geometry for a new shape layer.
//!
//! "New Rectangle" has to decide how big a rectangle, and where. Those numbers
//! were a `match` on an untyped kind string inside the host slot that creates
//! the layer, mixed in with the GPU upload that follows — so the proportions
//! could not be read without reading the upload, and could not be tested at
//! all. They are document geometry and belong here, next to the path helpers
//! they call.

use crate::layer::{ShapeContent, ShapeGradient};
use crate::paths::{PathPoint, VectorPath, ellipse_path, polygon_path, rect_path};

/// Which shape a "new shape layer" action creates.
///
/// The kind arrives as a string from the action registry and from the canvas,
/// and it used to be matched with a `_` arm that produced a rectangle. That
/// made every unhandled kind — including a typo — a silent rectangle. The
/// variants are named so an unknown one can be refused instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapePreset {
    Rect,
    Ellipse,
    Line,
    Polygon,
    /// A rectangle carrying a linear gradient fill.
    Gradient,
    /// A rectangle kept as live vector geometry rather than baked.
    LiveRect,
}

impl ShapePreset {
    /// Every preset, for exhaustiveness checks against the action registry.
    pub const ALL: [Self; 6] = [
        Self::Rect,
        Self::Ellipse,
        Self::Line,
        Self::Polygon,
        Self::Gradient,
        Self::LiveRect,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Ellipse => "ellipse",
            Self::Line => "line",
            Self::Polygon => "polygon",
            Self::Gradient => "gradient",
            Self::LiveRect => "live",
        }
    }

    /// Parse a registry or QML kind, `None` when it names no preset.
    ///
    /// No fallback, for the same reason [`crate::SelectionModifyOp::parse`]
    /// has none and unlike [`crate::tool_id::is_known`], which does fall back:
    /// picking an unknown *tool* would otherwise leave the user with no tool
    /// at all, while creating an unrequested *layer* is a document mutation
    /// the user then has to notice and undo. Making nothing is the recoverable
    /// answer.
    #[must_use]
    pub fn parse(kind: &str) -> Option<Self> {
        match kind {
            "rect" => Some(Self::Rect),
            "ellipse" => Some(Self::Ellipse),
            "line" => Some(Self::Line),
            "polygon" => Some(Self::Polygon),
            "gradient" => Some(Self::Gradient),
            "live" => Some(Self::LiveRect),
            _ => None,
        }
    }

    /// Display name for menus.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Rect => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Line => "Line",
            Self::Polygon => "Polygon",
            Self::Gradient => "Gradient Fill",
            Self::LiveRect => "Live Vector Shape",
        }
    }

    /// The `kind` key recorded on the layer.
    ///
    /// Not the same as [`Self::as_str`]: a gradient and a live rectangle are
    /// both rectangles as far as the path is concerned, and differ only in
    /// what decorates them.
    #[must_use]
    pub fn kind_key(self) -> &'static str {
        match self {
            Self::Rect | Self::Gradient | Self::LiveRect => "rect",
            Self::Ellipse => "ellipse",
            Self::Line => "line",
            Self::Polygon => "polygon",
        }
    }

    /// Whether the layer stays live vector geometry instead of being baked.
    #[must_use]
    pub fn is_live_vector(self) -> bool {
        matches!(self, Self::Polygon | Self::Gradient | Self::LiveRect)
    }

    /// Whether the shape encloses an area to fill. A line does not.
    #[must_use]
    pub fn is_filled(self) -> bool {
        !matches!(self, Self::Line)
    }

    /// Starting content for this preset in a `width` × `height` document.
    ///
    /// Every dimension is a fraction of the document so the shape lands in a
    /// usable place at any size, and so a 4K canvas does not open with a shape
    /// too small to grab.
    #[must_use]
    pub fn content(self, width: u32, height: u32) -> ShapeContent {
        let w = width as f32;
        let h = height as f32;
        ShapeContent {
            path: self.path(w, h),
            filled: self.is_filled(),
            stroked: true,
            kind: self.kind_key().into(),
            live_vector: self.is_live_vector(),
            gradient: self.gradient(w, h),
            ..ShapeContent::default()
        }
    }

    fn path(self, w: f32, h: f32) -> VectorPath {
        match self {
            Self::Ellipse => ellipse_path("Ellipse", w * 0.5, h * 0.5, w * 0.2, h * 0.15),
            Self::Polygon => polygon_path("Polygon", w * 0.5, h * 0.5, w.min(h) * 0.22, 6),
            Self::Line => VectorPath::polyline(
                "Line",
                vec![
                    PathPoint {
                        x: w * 0.2,
                        y: h * 0.5,
                    },
                    PathPoint {
                        x: w * 0.8,
                        y: h * 0.5,
                    },
                ],
                false,
            ),
            Self::Gradient => rect_path("Gradient", w * 0.25, h * 0.25, w * 0.5, h * 0.4),
            Self::Rect | Self::LiveRect => {
                rect_path("Rectangle", w * 0.25, h * 0.25, w * 0.5, h * 0.4)
            }
        }
    }

    fn gradient(self, w: f32, h: f32) -> Option<ShapeGradient> {
        matches!(self, Self::Gradient).then(|| ShapeGradient {
            x0: w * 0.25,
            y0: h * 0.25,
            x1: w * 0.75,
            y1: h * 0.65,
            c0_rgba: [0.2, 0.45, 0.9, 1.0],
            c1_rgba: [0.95, 0.35, 0.2, 1.0],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_round_trips_through_its_kind_string() {
        for preset in ShapePreset::ALL {
            assert_eq!(
                ShapePreset::parse(preset.as_str()),
                Some(preset),
                "{} did not survive a parse of its own name",
                preset.as_str()
            );
        }
    }

    #[test]
    fn an_unknown_kind_makes_nothing_rather_than_a_rectangle() {
        // The behaviour this type exists to change: the old `_` arm turned a
        // typo into a rectangle the user did not ask for.
        assert_eq!(ShapePreset::parse("rectangle"), None);
        assert_eq!(ShapePreset::parse("Rect"), None);
        assert_eq!(ShapePreset::parse(""), None);
    }

    #[test]
    fn a_gradient_and_a_live_rectangle_are_rectangles_on_the_layer() {
        // kind_key deliberately differs from as_str for these two, which is
        // the sort of thing a reader assumes is a bug unless it is asserted.
        assert_eq!(ShapePreset::Gradient.kind_key(), "rect");
        assert_eq!(ShapePreset::LiveRect.kind_key(), "rect");
        assert_eq!(ShapePreset::Rect.kind_key(), "rect");
        assert_eq!(ShapePreset::Ellipse.kind_key(), "ellipse");
    }

    #[test]
    fn only_the_gradient_preset_carries_a_gradient() {
        for preset in ShapePreset::ALL {
            let content = preset.content(800, 600);
            assert_eq!(
                content.gradient.is_some(),
                preset == ShapePreset::Gradient,
                "{} disagrees about carrying a gradient",
                preset.as_str()
            );
        }
    }

    #[test]
    fn a_line_is_the_one_preset_that_is_not_filled() {
        for preset in ShapePreset::ALL {
            assert_eq!(
                preset.content(800, 600).filled,
                preset != ShapePreset::Line,
                "{} disagrees about being filled",
                preset.as_str()
            );
        }
        assert!(!ShapePreset::Line.content(800, 600).path.closed);
    }

    #[test]
    fn live_vector_presets_are_the_ones_the_host_must_not_bake() {
        let live: Vec<&str> = ShapePreset::ALL
            .into_iter()
            .filter(|p| p.content(800, 600).live_vector)
            .map(ShapePreset::as_str)
            .collect();
        assert_eq!(live, vec!["polygon", "gradient", "live"]);
    }

    #[test]
    fn every_preset_lands_inside_the_document() {
        // The proportions are fractions of the document precisely so this
        // holds at any size; a shape opening off-canvas cannot be grabbed.
        for (w, h) in [(64_u32, 64_u32), (1920, 1080), (3840, 2160), (100, 4000)] {
            for preset in ShapePreset::ALL {
                let content = preset.content(w, h);
                for anchor in &content.path.anchors {
                    assert!(
                        anchor.x >= 0.0
                            && anchor.y >= 0.0
                            && anchor.x <= w as f32
                            && anchor.y <= h as f32,
                        "{} puts an anchor at ({}, {}) outside a {w}x{h} document",
                        preset.as_str(),
                        anchor.x,
                        anchor.y
                    );
                }
            }
        }
    }

    /// Horizontal span of a preset's anchors in a `w` × `h` document.
    fn span_x(preset: ShapePreset, w: u32, h: u32) -> f32 {
        let content = preset.content(w, h);
        let xs = content.path.anchors.iter().map(|a| a.x);
        let (lo, hi) = xs.fold((f32::MAX, f32::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
        hi - lo
    }

    #[test]
    fn every_preset_scales_with_the_document() {
        // The property that makes fractional proportions worth having, and the
        // one a weaker "is it bigger than nothing" check misses: a preset that
        // clamped its size to a constant would still enclose *something* at
        // every document size, just the wrong something.
        for (w, h) in [(8_u32, 8_u32), (64, 64), (960, 540)] {
            for preset in ShapePreset::ALL {
                let single = span_x(preset, w, h);
                let double = span_x(preset, w * 2, h * 2);
                assert!(
                    (double - single * 2.0).abs() <= single * 0.01,
                    "{} spans {single} at {w}x{h} but {double} at twice that — \
                     it is not proportional to the document",
                    preset.as_str()
                );
            }
        }
    }

    #[test]
    fn every_preset_is_big_enough_to_grab_in_a_small_document() {
        // A shape that opens a couple of pixels wide is a layer the user
        // cannot see or click, even though it technically exists.
        for (w, h) in [(8_u32, 8_u32), (16, 16), (64, 64), (100, 4000)] {
            let floor = w.min(h) as f32 * 0.2;
            for preset in ShapePreset::ALL {
                let span = span_x(preset, w, h);
                assert!(
                    span >= floor,
                    "{} opens {span}px wide in a {w}x{h} document, under the {floor}px floor",
                    preset.as_str()
                );
            }
        }
    }
}
