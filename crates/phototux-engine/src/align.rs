//! Aligning and distributing layers by their visible content.
//!
//! Every raster layer in a PhotoTux document is document-sized, so "align these
//! layers" cannot mean "align their rectangles" — every rectangle is the same
//! rectangle, and the command would be a no-op on exactly the documents where
//! the user wants it. What a person means by the left edge of a layer is the
//! left edge of the pixels they can *see*, which is the bounding box of its
//! non-transparent pixels, placed into the document by the layer's transform.
//!
//! Measuring that needs pixels, and pixels live on the GPU, which this crate
//! cannot reach. So the split is: the host measures, this module decides. The
//! host reads each target layer back and calls [`content_bounds`] and
//! [`placed_bounds`]; the resulting boxes come back in as command arguments and
//! every rule about *where the layers end up* — which frame they align to, how
//! many targets an operation needs, how the spacing works — is decided here,
//! where it can be tested without a device.

use crate::camera::Rect;
use crate::layer::{LayerId, LayerTransform};

/// The axis an operation moves layers along.
///
/// Nothing in this module aligns diagonally: an operation touches exactly one
/// of the two translation components, and the other must be left alone rather
/// than helpfully zeroed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignAxis {
    Horizontal,
    Vertical,
}

/// One align-or-distribute operation.
///
/// Named as a vocabulary — wire key, label, icon and target count all live on
/// the variant — because the same set has to appear in the action registry, in
/// the menus, in the options bar and in the command dispatch. Every previous
/// set that was written out separately in those four places drifted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignOp {
    Left,
    HorizontalCenter,
    Right,
    Top,
    VerticalCenter,
    Bottom,
    /// Even horizontal spacing between the centres of three or more layers.
    DistributeHorizontal,
    /// Even vertical spacing between the centres of three or more layers.
    DistributeVertical,
}

impl AlignOp {
    /// Every operation, in menu order: the six alignments, then the two
    /// distributions.
    pub const ALL: [Self; 8] = [
        Self::Left,
        Self::HorizontalCenter,
        Self::Right,
        Self::Top,
        Self::VerticalCenter,
        Self::Bottom,
        Self::DistributeHorizontal,
        Self::DistributeVertical,
    ];

    /// The key used on the wire, in action ids and in QML.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::HorizontalCenter => "h-center",
            Self::Right => "right",
            Self::Top => "top",
            Self::VerticalCenter => "v-center",
            Self::Bottom => "bottom",
            Self::DistributeHorizontal => "distribute-h",
            Self::DistributeVertical => "distribute-v",
        }
    }

    /// Parse a wire key, `None` when it names no operation.
    ///
    /// No fallback arm, for the reason [`crate::ShapePreset::parse`] has none:
    /// a mistyped key that quietly became "align left" would move the user's
    /// layers somewhere they did not ask for.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|op| op.as_str() == key)
    }

    /// Menu and tooltip text.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Left => "Align Left Edges",
            Self::HorizontalCenter => "Align Horizontal Centers",
            Self::Right => "Align Right Edges",
            Self::Top => "Align Top Edges",
            Self::VerticalCenter => "Align Vertical Centers",
            Self::Bottom => "Align Bottom Edges",
            Self::DistributeHorizontal => "Distribute Horizontal Centers",
            Self::DistributeVertical => "Distribute Vertical Centers",
        }
    }

    /// Phosphor icon name, without weight or extension.
    #[must_use]
    pub fn icon_key(self) -> &'static str {
        match self {
            Self::Left => "align-left",
            Self::HorizontalCenter => "align-center-horizontal",
            Self::Right => "align-right",
            Self::Top => "align-top",
            Self::VerticalCenter => "align-center-vertical",
            Self::Bottom => "align-bottom",
            Self::DistributeHorizontal => "arrows-out-line-horizontal",
            Self::DistributeVertical => "arrows-out-line-vertical",
        }
    }

    /// Which translation component this operation writes.
    #[must_use]
    pub fn axis(self) -> AlignAxis {
        match self {
            Self::Left | Self::HorizontalCenter | Self::Right | Self::DistributeHorizontal => {
                AlignAxis::Horizontal
            }
            Self::Top | Self::VerticalCenter | Self::Bottom | Self::DistributeVertical => {
                AlignAxis::Vertical
            }
        }
    }

    #[must_use]
    pub fn is_distribute(self) -> bool {
        matches!(self, Self::DistributeHorizontal | Self::DistributeVertical)
    }

    /// How many layers the operation needs to mean anything.
    ///
    /// Aligning needs one, because a single layer aligns to the canvas (see
    /// [`align_frame`]). Distributing needs three: with two, the ends are
    /// already fixed and there is nothing in between to space out, so a
    /// two-layer distribute would silently do nothing.
    #[must_use]
    pub fn min_targets(self) -> usize {
        if self.is_distribute() { 3 } else { 1 }
    }
}

/// Every operation as `[{id, label, icon, distribute}]`, for the chrome.
///
/// The options bar and the menus both build themselves from this rather than
/// restating the eight operations in QML, where a ninth added to the enum
/// would silently never appear.
#[must_use]
pub fn align_ops_json() -> String {
    let rows: Vec<serde_json::Value> = AlignOp::ALL
        .iter()
        .map(|op| {
            serde_json::json!({
                "id": op.as_str(),
                "label": op.label(),
                "icon": op.icon_key(),
                "distribute": op.is_distribute(),
                "minTargets": op.min_targets(),
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

/// One thing an alignment treats as a single object.
///
/// Usually one layer, so `members` holds one id. A group is the reason this is
/// not simply a `(LayerId, Rect)`: the compositor does not pass a group's
/// transform down to its children, so a group can only move by moving every
/// member by the same amount — and it has to count as *one* box, or
/// distributing three groups would space out their members instead.
#[derive(Debug, Clone, PartialEq)]
pub struct AlignTarget {
    /// Where the target's visible content sits in the document.
    pub bounds: Rect,
    /// Every layer that moves when this target moves.
    pub members: Vec<LayerId>,
}

impl AlignTarget {
    /// A target that is exactly one layer.
    #[must_use]
    pub fn single(id: LayerId, bounds: Rect) -> Self {
        Self {
            bounds,
            members: vec![id],
        }
    }
}

/// Bounding box of the non-transparent pixels in an RGBA8 buffer.
///
/// `None` when the layer is fully transparent — a layer with nothing visible
/// has no edges to align, and returning the whole buffer instead would drag
/// every other layer towards an empty rectangle.
///
/// The test is `alpha > 0` rather than a threshold. A pixel at alpha 1/255 is
/// invisible in practice, but it is *there*, and a soft-edged brush stroke
/// fades to exactly those values — thresholding would move the box by however
/// wide the falloff happened to be, which is not a number the user can see or
/// predict.
#[must_use]
pub fn content_bounds(pixels: &[u8], width: u32, height: u32) -> Option<Rect> {
    let (w, h) = (width as usize, height as usize);
    if w == 0 || h == 0 || pixels.len() < w * h * 4 {
        return None;
    }
    let (mut min_x, mut min_y) = (usize::MAX, usize::MAX);
    let (mut max_x, mut max_y) = (0_usize, 0_usize);
    for y in 0..h {
        let row = y * w * 4;
        for x in 0..w {
            if pixels[row + x * 4 + 3] == 0 {
                continue;
            }
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }
    if min_x == usize::MAX {
        return None;
    }
    // Half-open: a single opaque pixel at (3, 3) spans [3, 4), so the box is
    // one pixel wide rather than zero.
    Some(Rect::new(
        min_x as f32,
        min_y as f32,
        (max_x - min_x + 1) as f32,
        (max_y - min_y + 1) as f32,
    ))
}

/// Where a layer-space box lands in the document under `transform`.
///
/// The pivot is the document centre, matching the compositor
/// (`inverse_affine_coeffs`), so the box this returns is the one actually on
/// screen. All four corners are mapped and the axis-aligned hull taken: under
/// rotation the corners no longer agree on which is leftmost.
#[must_use]
pub fn placed_bounds(source: Rect, transform: LayerTransform, doc_w: u32, doc_h: u32) -> Rect {
    let affine = transform.forward_affine(doc_w as f32 * 0.5, doc_h as f32 * 0.5);
    let (x0, y0) = (source.x, source.y);
    let (x1, y1) = (source.x + source.width, source.y + source.height);
    let corners = [(x0, y0), (x1, y0), (x0, y1), (x1, y1)];
    let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
    let (mut max_x, mut max_y) = (f32::MIN, f32::MIN);
    for (cx, cy) in corners {
        let (mx, my) = affine.map_point(cx, cy);
        min_x = min_x.min(mx);
        max_x = max_x.max(mx);
        min_y = min_y.min(my);
        max_y = max_y.max(my);
    }
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

/// The rectangle an alignment aligns *to*.
///
/// With two or more layers the frame is their combined bounding box, so
/// "align left" pulls them to the leftmost layer and nothing escapes the group.
/// With exactly one it is the canvas, because a single layer aligned to its own
/// bounding box is a no-op — the only reading of "align this layer" that does
/// anything is "align it to the document". Photoshop makes the same choice, and
/// it means the common case needs no extra control in the options bar.
#[must_use]
pub fn align_frame(boxes: &[Rect], canvas: Rect) -> Rect {
    if boxes.len() < 2 {
        return canvas;
    }
    let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
    let (mut max_x, mut max_y) = (f32::MIN, f32::MIN);
    for b in boxes {
        min_x = min_x.min(b.x);
        min_y = min_y.min(b.y);
        max_x = max_x.max(b.x + b.width);
        max_y = max_y.max(b.y + b.height);
    }
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

/// How far each box must move, in document pixels, to satisfy `op`.
///
/// One offset per input box, in the same order. Offsets are deltas rather than
/// absolute positions so the caller can add them to whatever translation the
/// layer already carries, and the untouched axis is always exactly `0.0`.
#[must_use]
pub fn align_offsets(op: AlignOp, boxes: &[Rect], frame: Rect) -> Vec<(f32, f32)> {
    let scalars = if op.is_distribute() {
        distribute_scalars(op, boxes)
    } else {
        boxes.iter().map(|b| align_scalar(op, *b, frame)).collect()
    };
    let horizontal = op.axis() == AlignAxis::Horizontal;
    scalars
        .into_iter()
        .map(|d| if horizontal { (d, 0.0) } else { (0.0, d) })
        .collect()
}

/// Offset along the operation's axis that snaps one box to `frame`.
fn align_scalar(op: AlignOp, b: Rect, frame: Rect) -> f32 {
    match op {
        AlignOp::Left => frame.x - b.x,
        AlignOp::Right => (frame.x + frame.width) - (b.x + b.width),
        AlignOp::HorizontalCenter => (frame.x + frame.width * 0.5) - (b.x + b.width * 0.5),
        AlignOp::Top => frame.y - b.y,
        AlignOp::Bottom => (frame.y + frame.height) - (b.y + b.height),
        AlignOp::VerticalCenter => (frame.y + frame.height * 0.5) - (b.y + b.height * 0.5),
        // Distribution is not a per-box question; `align_offsets` routes it away.
        AlignOp::DistributeHorizontal | AlignOp::DistributeVertical => 0.0,
    }
}

/// Even spacing of centres between the two outermost boxes.
///
/// The extremes stay put and everything between them is respaced. Distributing
/// by *centre* rather than by gap is the more predictable of the two readings:
/// boxes of wildly different sizes end up on an even rhythm instead of an even
/// gap that looks uneven, and the result does not change when a layer's
/// transparent margin changes.
fn distribute_scalars(op: AlignOp, boxes: &[Rect]) -> Vec<f32> {
    let horizontal = op.axis() == AlignAxis::Horizontal;
    let centre = |b: &Rect| {
        if horizontal {
            b.x + b.width * 0.5
        } else {
            b.y + b.height * 0.5
        }
    };
    if boxes.len() < 3 {
        return vec![0.0; boxes.len()];
    }
    // Rank by current centre. The result must be indexed back by the caller's
    // order, so the ranking carries the original index rather than reordering
    // the boxes.
    let mut order: Vec<(usize, f32)> = boxes.iter().map(centre).enumerate().collect();
    order.sort_by(|a, b| a.1.total_cmp(&b.1));
    let first = order[0].1;
    let last = order[order.len() - 1].1;
    let step = (last - first) / (order.len() - 1) as f32;
    let mut out = vec![0.0; boxes.len()];
    for (rank, (index, current)) in order.into_iter().enumerate() {
        out[index] = (first + step * rank as f32) - current;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::new(x, y, w, h)
    }

    /// An RGBA8 buffer of `w`×`h` with one opaque rectangle in it.
    fn buffer_with_box(w: u32, h: u32, bx: u32, by: u32, bw: u32, bh: u32) -> Vec<u8> {
        let mut px = vec![0_u8; (w * h * 4) as usize];
        for y in by..by + bh {
            for x in bx..bx + bw {
                let i = ((y * w + x) * 4) as usize;
                px[i..i + 4].copy_from_slice(&[255, 128, 0, 255]);
            }
        }
        px
    }

    #[test]
    fn every_op_round_trips_through_its_wire_key() {
        for op in AlignOp::ALL {
            assert_eq!(AlignOp::parse(op.as_str()), Some(op), "{}", op.as_str());
        }
        assert_eq!(AlignOp::parse("centre"), None);
        assert_eq!(AlignOp::parse(""), None);
    }

    #[test]
    fn the_two_distributions_are_the_ones_needing_three_layers() {
        for op in AlignOp::ALL {
            assert_eq!(op.min_targets(), if op.is_distribute() { 3 } else { 1 });
        }
    }

    #[test]
    fn content_bounds_finds_the_opaque_rectangle() {
        let px = buffer_with_box(32, 32, 4, 6, 10, 3);
        assert_eq!(content_bounds(&px, 32, 32), Some(rect(4.0, 6.0, 10.0, 3.0)));
    }

    #[test]
    fn a_single_opaque_pixel_is_one_pixel_wide_not_zero() {
        // Half-open boxes: an inclusive max would make this box zero-sized,
        // and a zero-width box centres wrongly and distributes to nowhere.
        let px = buffer_with_box(8, 8, 3, 3, 1, 1);
        assert_eq!(content_bounds(&px, 8, 8), Some(rect(3.0, 3.0, 1.0, 1.0)));
    }

    #[test]
    fn a_fully_transparent_layer_has_no_bounds() {
        // Not `Some(whole buffer)`: an empty layer reported as document-sized
        // would drag every other layer towards a rectangle with nothing in it.
        let px = vec![0_u8; 16 * 16 * 4];
        assert_eq!(content_bounds(&px, 16, 16), None);
    }

    #[test]
    fn the_faintest_pixel_still_counts_as_content() {
        // Soft brush edges fade to alpha 1. Thresholding them away would move
        // a layer's measured edge by however wide its falloff happened to be.
        let mut px = vec![0_u8; 8 * 8 * 4];
        px[(2 * 8 + 5) * 4 + 3] = 1;
        assert_eq!(content_bounds(&px, 8, 8), Some(rect(5.0, 2.0, 1.0, 1.0)));
    }

    #[test]
    fn content_bounds_refuses_a_buffer_too_small_for_its_size() {
        assert_eq!(content_bounds(&[0; 16], 32, 32), None);
        assert_eq!(content_bounds(&[], 0, 0), None);
    }

    #[test]
    fn an_untransformed_box_is_placed_where_it_already_is() {
        let b = rect(10.0, 20.0, 30.0, 40.0);
        let placed = placed_bounds(b, LayerTransform::identity(), 800, 600);
        assert!((placed.x - 10.0).abs() < 1e-3 && (placed.y - 20.0).abs() < 1e-3);
        assert!((placed.width - 30.0).abs() < 1e-3 && (placed.height - 40.0).abs() < 1e-3);
    }

    #[test]
    fn a_translated_layer_is_placed_by_its_translation() {
        let b = rect(10.0, 20.0, 30.0, 40.0);
        let t = LayerTransform {
            translate_x: 15.0,
            translate_y: -5.0,
            ..LayerTransform::identity()
        };
        let placed = placed_bounds(b, t, 800, 600);
        assert!((placed.x - 25.0).abs() < 1e-3, "x was {}", placed.x);
        assert!((placed.y - 15.0).abs() < 1e-3, "y was {}", placed.y);
    }

    #[test]
    fn a_rotated_box_is_measured_by_its_hull_not_its_corners() {
        // The property that makes mapping all four corners worth the arithmetic:
        // under rotation no single corner is reliably the leftmost one.
        let b = rect(390.0, 290.0, 20.0, 20.0);
        let t = LayerTransform {
            rotation_deg: 45.0,
            ..LayerTransform::identity()
        };
        let placed = placed_bounds(b, t, 800, 600);
        let diagonal = 20.0_f32 * std::f32::consts::SQRT_2;
        assert!(
            (placed.width - diagonal).abs() < 0.5,
            "a square rotated 45° should span its diagonal, got {}",
            placed.width
        );
    }

    #[test]
    fn one_layer_aligns_to_the_canvas_and_several_align_to_each_other() {
        let canvas = rect(0.0, 0.0, 800.0, 600.0);
        let a = rect(100.0, 100.0, 50.0, 50.0);
        assert_eq!(align_frame(&[a], canvas), canvas);
        let b = rect(300.0, 200.0, 40.0, 20.0);
        assert_eq!(
            align_frame(&[a, b], canvas),
            rect(100.0, 100.0, 240.0, 120.0)
        );
    }

    #[test]
    fn align_left_pulls_every_layer_to_the_leftmost_one() {
        let boxes = [
            rect(100.0, 0.0, 50.0, 50.0),
            rect(300.0, 0.0, 50.0, 50.0),
            rect(220.0, 0.0, 50.0, 50.0),
        ];
        let frame = align_frame(&boxes, rect(0.0, 0.0, 800.0, 600.0));
        let out = align_offsets(AlignOp::Left, &boxes, frame);
        assert_eq!(out, vec![(0.0, 0.0), (-200.0, 0.0), (-120.0, 0.0)]);
    }

    #[test]
    fn an_alignment_never_touches_the_other_axis() {
        // A layer nudged sideways by "align top" is the kind of bug that only
        // shows up once, on a document the user cared about.
        let boxes = [rect(10.0, 10.0, 20.0, 20.0), rect(90.0, 70.0, 40.0, 10.0)];
        let frame = align_frame(&boxes, rect(0.0, 0.0, 400.0, 400.0));
        for op in AlignOp::ALL {
            for (dx, dy) in align_offsets(op, &boxes, frame) {
                match op.axis() {
                    AlignAxis::Horizontal => assert_eq!(dy, 0.0, "{} moved y", op.as_str()),
                    AlignAxis::Vertical => assert_eq!(dx, 0.0, "{} moved x", op.as_str()),
                }
            }
        }
    }

    /// Apply `align_offsets` to the boxes so a test can assert on the result.
    fn moved(op: AlignOp, boxes: &[Rect], frame: Rect) -> Vec<Rect> {
        align_offsets(op, boxes, frame)
            .into_iter()
            .zip(boxes)
            .map(|((dx, dy), b)| rect(b.x + dx, b.y + dy, b.width, b.height))
            .collect()
    }

    #[test]
    fn aligned_edges_actually_coincide_afterwards() {
        let boxes = [
            rect(100.0, 40.0, 50.0, 90.0),
            rect(300.0, 200.0, 30.0, 20.0),
            rect(220.0, 90.0, 70.0, 60.0),
        ];
        let frame = align_frame(&boxes, rect(0.0, 0.0, 800.0, 600.0));
        let left = moved(AlignOp::Left, &boxes, frame);
        assert!(left.iter().all(|b| (b.x - frame.x).abs() < 1e-3));
        let right = moved(AlignOp::Right, &boxes, frame);
        let edge = frame.x + frame.width;
        assert!(right.iter().all(|b| (b.x + b.width - edge).abs() < 1e-3));
        let bottom = moved(AlignOp::Bottom, &boxes, frame);
        let floor = frame.y + frame.height;
        assert!(bottom.iter().all(|b| (b.y + b.height - floor).abs() < 1e-3));
    }

    #[test]
    fn centring_lands_boxes_of_different_sizes_on_the_same_centre() {
        // Centring by edge instead of centre is a plausible-looking mistake
        // that only misbehaves when the boxes differ in size.
        let boxes = [rect(0.0, 0.0, 10.0, 10.0), rect(200.0, 0.0, 90.0, 10.0)];
        let frame = align_frame(&boxes, rect(0.0, 0.0, 800.0, 600.0));
        let out = moved(AlignOp::HorizontalCenter, &boxes, frame);
        let centres: Vec<f32> = out.iter().map(|b| b.x + b.width * 0.5).collect();
        assert!((centres[0] - centres[1]).abs() < 1e-3, "{centres:?}");
    }

    #[test]
    fn distributing_evens_the_spacing_and_leaves_the_ends_alone() {
        let boxes = [
            rect(0.0, 0.0, 10.0, 10.0),
            rect(20.0, 0.0, 10.0, 10.0),
            rect(200.0, 0.0, 10.0, 10.0),
        ];
        let frame = align_frame(&boxes, rect(0.0, 0.0, 800.0, 600.0));
        let out = moved(AlignOp::DistributeHorizontal, &boxes, frame);
        assert_eq!(out[0].x, 0.0, "the leftmost layer must not move");
        assert_eq!(out[2].x, 200.0, "the rightmost layer must not move");
        assert!((out[1].x - 100.0).abs() < 1e-3, "middle at {}", out[1].x);
    }

    #[test]
    fn distributing_respaces_by_position_not_by_stack_order() {
        // The middle layer is listed first. Distributing by list order would
        // move the wrong two layers and leave the result unevenly spaced.
        let boxes = [
            rect(20.0, 0.0, 10.0, 10.0),
            rect(200.0, 0.0, 10.0, 10.0),
            rect(0.0, 0.0, 10.0, 10.0),
        ];
        let frame = align_frame(&boxes, rect(0.0, 0.0, 800.0, 600.0));
        let out = moved(AlignOp::DistributeHorizontal, &boxes, frame);
        let mut centres: Vec<f32> = out.iter().map(|b| b.x + b.width * 0.5).collect();
        centres.sort_by(f32::total_cmp);
        let first_gap = centres[1] - centres[0];
        let second_gap = centres[2] - centres[1];
        assert!(
            (first_gap - second_gap).abs() < 1e-3,
            "gaps {first_gap} and {second_gap} are not even"
        );
    }

    #[test]
    fn distributing_fewer_than_three_layers_moves_nothing() {
        let frame = rect(0.0, 0.0, 800.0, 600.0);
        for boxes in [
            &[][..],
            &[rect(10.0, 10.0, 5.0, 5.0)][..],
            &[rect(10.0, 10.0, 5.0, 5.0), rect(90.0, 10.0, 5.0, 5.0)][..],
        ] {
            let out = align_offsets(AlignOp::DistributeHorizontal, boxes, frame);
            assert_eq!(out.len(), boxes.len());
            assert!(out.iter().all(|&(dx, dy)| dx == 0.0 && dy == 0.0));
        }
    }

    #[test]
    fn distributing_layers_already_stacked_on_one_spot_is_a_no_op() {
        // Coincident centres make the span zero; the step must come out zero
        // rather than a division that scatters them.
        let boxes = [
            rect(50.0, 0.0, 10.0, 10.0),
            rect(50.0, 0.0, 10.0, 10.0),
            rect(50.0, 0.0, 10.0, 10.0),
        ];
        let out = align_offsets(
            AlignOp::DistributeHorizontal,
            &boxes,
            rect(0.0, 0.0, 80.0, 80.0),
        );
        assert!(out.iter().all(|&(dx, _)| dx.abs() < 1e-6), "{out:?}");
    }

    #[test]
    fn aligning_is_idempotent() {
        // Running the same alignment twice must not creep: the second pass is
        // what a user does when they are not sure the first one landed.
        let boxes = [
            rect(100.0, 40.0, 50.0, 90.0),
            rect(300.0, 200.0, 30.0, 20.0),
            rect(220.0, 90.0, 70.0, 60.0),
        ];
        for op in AlignOp::ALL {
            let frame = align_frame(&boxes, rect(0.0, 0.0, 800.0, 600.0));
            let once = moved(op, &boxes, frame);
            let frame2 = align_frame(&once, rect(0.0, 0.0, 800.0, 600.0));
            let twice = moved(op, &once, frame2);
            for (a, b) in once.iter().zip(&twice) {
                assert!(
                    (a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3,
                    "{} moved again on a second pass",
                    op.as_str()
                );
            }
        }
    }

    #[test]
    fn every_op_has_its_own_key_label_and_icon() {
        for (i, op) in AlignOp::ALL.iter().enumerate() {
            for other in &AlignOp::ALL[i + 1..] {
                assert_ne!(op.as_str(), other.as_str());
                assert_ne!(op.label(), other.label());
                assert_ne!(op.icon_key(), other.icon_key());
            }
        }
    }
}
