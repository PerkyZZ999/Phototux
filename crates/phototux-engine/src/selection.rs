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

/// Geometry used for the last committed selection outline.
///
/// `Rect` / `Ellipse` use QML Shape ants; `Mask` uses GPU edge ants on the R8 channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionShape {
    #[default]
    Rect,
    Ellipse,
    /// Irregular / polygonal / freehand — outline from GPU selection mask.
    Mask,
}

impl SelectionShape {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Ellipse => "ellipse",
            Self::Mask => "mask",
        }
    }

    pub fn parse(label: &str) -> Self {
        match label {
            "ellipse" => Self::Ellipse,
            "mask" => Self::Mask,
            _ => Self::Rect,
        }
    }
}

/// Which morphological edit `selection.modify` performs.
///
/// The op used to travel as a bare `String` from the action registry through
/// two parsers into the command, and each stop had its own idea of which
/// strings were valid: the registry wrote three literals, the host matched
/// three, and the command recognised only `feather`. Nothing checked that the
/// three lists agreed. Naming the vocabulary once makes an unknown op a parse
/// failure at the boundary instead of a silent no-op three frames later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionModifyOp {
    /// Box-blur the mask edge.
    Feather,
    /// Dilate the mask.
    Expand,
    /// Erode the mask.
    Contract,
}

impl SelectionModifyOp {
    /// Every op, for exhaustiveness checks across the registry and menus.
    pub const ALL: [Self; 3] = [Self::Feather, Self::Expand, Self::Contract];

    /// Radius used when an argument names an op but no distance.
    pub const DEFAULT_RADIUS: u32 = 4;

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Feather => "feather",
            Self::Expand => "expand",
            Self::Contract => "contract",
        }
    }

    /// Parse a registry or QML label, `None` when it names no known op.
    ///
    /// Unlike [`SelectionCombine::parse`] and [`SelectionShape::parse`], which
    /// fall back to their defaults, an unrecognised op has no safe default:
    /// those two describe *how* to interpret an edit the caller already asked
    /// for, whereas this one chooses the edit. Quietly feathering because a
    /// label was misspelled would change pixels nobody asked to change.
    #[must_use]
    pub fn parse(label: &str) -> Option<Self> {
        match label {
            "feather" => Some(Self::Feather),
            "expand" => Some(Self::Expand),
            "contract" => Some(Self::Contract),
            _ => None,
        }
    }

    /// Run this op over an R8 selection mask.
    ///
    /// # Errors
    /// Propagates the buffer-length error from the underlying mask function.
    pub fn apply(
        self,
        width: u32,
        height: u32,
        mask: &[u8],
        radius: u32,
    ) -> Result<Vec<u8>, String> {
        match self {
            Self::Feather => feather_mask_r8(width, height, mask, radius),
            Self::Expand => expand_mask_r8(width, height, mask, radius),
            Self::Contract => contract_mask_r8(width, height, mask, radius),
        }
    }
}

/// Parse a `selection.modify` action argument: `"<op>"` or `"<op>:<radius>"`.
///
/// `None` when the op is unknown, when the radius is present but not a
/// non-negative integer, or when a third colon-separated field follows. A
/// present-but-unparsable radius is refused rather than defaulted: the caller
/// wrote something it believed was a distance, and substituting a different
/// one silently applies an edit at the wrong size. An *absent* radius is a
/// different statement — "this op, at the usual distance" — and takes
/// [`SelectionModifyOp::DEFAULT_RADIUS`].
#[must_use]
pub fn parse_selection_modify_arg(arg: &str) -> Option<(SelectionModifyOp, u32)> {
    let mut parts = arg.split(':');
    // `str::split` always yields at least one item, so this never defaults —
    // an empty argument parses as the empty op and is rejected below.
    let op = SelectionModifyOp::parse(parts.next().unwrap_or_default().trim())?;
    let radius = match parts.next() {
        None => SelectionModifyOp::DEFAULT_RADIUS,
        Some(raw) => raw.trim().parse::<u32>().ok()?,
    };
    if parts.next().is_some() {
        return None;
    }
    Some((op, radius))
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

    /// Commit a polygonal / freehand selection whose outline is the GPU mask.
    pub fn set_mask_polygon(&mut self, bounds: SelectionRect, combine: SelectionCombine) {
        self.apply_shape(bounds, SelectionShape::Mask, combine);
        // Mixed geometry with Add still prefers GPU ants when the new piece is irregular.
        if matches!(combine, SelectionCombine::Add) && self.active {
            self.shape = SelectionShape::Mask;
        }
    }

    /// Axis-aligned bounds of a document-space polygon (empty if fewer than two points).
    pub fn polygon_bounds(points: &[(f32, f32)]) -> Option<SelectionRect> {
        if points.len() < 2 {
            return None;
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for &(x, y) in points {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
            return None;
        }
        let x0 = min_x.floor() as i32;
        let y0 = min_y.floor() as i32;
        let x1 = max_x.ceil() as i32;
        let y1 = max_y.ceil() as i32;
        let width = u32::try_from((x1 - x0).max(0)).unwrap_or(0);
        let height = u32::try_from((y1 - y0).max(0)).unwrap_or(0);
        if width == 0 || height == 0 {
            return None;
        }
        Some(SelectionRect {
            x: x0,
            y: y0,
            width,
            height,
        })
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

/// Box-blur feather of an R8 selection mask (`radius` in pixels, 0 = no-op).
///
/// # Errors
/// Returns an error when the buffer length does not match `width * height`.
pub fn feather_mask_r8(
    width: u32,
    height: u32,
    mask: &[u8],
    radius: u32,
) -> Result<Vec<u8>, String> {
    let w = width as usize;
    let h = height as usize;
    let expected = w.checked_mul(h).ok_or_else(|| "overflow".to_owned())?;
    if mask.len() != expected {
        return Err(format!("mask length {} != expected {expected}", mask.len()));
    }
    if radius == 0 || w == 0 || h == 0 {
        return Ok(mask.to_vec());
    }
    let r = radius as i32;
    let mut out = vec![0_u8; expected];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            out[y as usize * w + x as usize] = feather_neighborhood(mask, w, h, x, y, r);
        }
    }
    Ok(out)
}

fn feather_neighborhood(mask: &[u8], w: usize, h: usize, x: i32, y: i32, r: i32) -> u8 {
    let mut sum = 0_u32;
    let mut count = 0_u32;
    for dy in -r..=r {
        for dx in -r..=r {
            let xx = x + dx;
            let yy = y + dy;
            if xx >= 0 && yy >= 0 && (xx as usize) < w && (yy as usize) < h {
                sum += u32::from(mask[yy as usize * w + xx as usize]);
                count += 1;
            }
        }
    }
    sum.checked_div(count).map(|v| v as u8).unwrap_or(0)
}

/// Morphological expand (dilate) of an R8 selection mask.
///
/// # Errors
/// Returns an error when the buffer length does not match `width * height`.
pub fn expand_mask_r8(
    width: u32,
    height: u32,
    mask: &[u8],
    radius: u32,
) -> Result<Vec<u8>, String> {
    morph_mask_r8(width, height, mask, radius, true)
}

/// Morphological contract (erode) of an R8 selection mask.
///
/// # Errors
/// Returns an error when the buffer length does not match `width * height`.
pub fn contract_mask_r8(
    width: u32,
    height: u32,
    mask: &[u8],
    radius: u32,
) -> Result<Vec<u8>, String> {
    morph_mask_r8(width, height, mask, radius, false)
}

fn morph_mask_r8(
    width: u32,
    height: u32,
    mask: &[u8],
    radius: u32,
    dilate: bool,
) -> Result<Vec<u8>, String> {
    let w = width as usize;
    let h = height as usize;
    let expected = w.checked_mul(h).ok_or_else(|| "overflow".to_owned())?;
    if mask.len() != expected {
        return Err(format!("mask length {} != expected {expected}", mask.len()));
    }
    if radius == 0 || w == 0 || h == 0 {
        return Ok(mask.to_vec());
    }
    let r = radius as i32;
    let mut out = vec![0_u8; expected];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            out[y as usize * w + x as usize] = morph_neighborhood(mask, w, h, x, y, r, dilate);
        }
    }
    Ok(out)
}

fn morph_neighborhood(mask: &[u8], w: usize, h: usize, x: i32, y: i32, r: i32, dilate: bool) -> u8 {
    let mut best = if dilate { 0_u8 } else { 255_u8 };
    let tap = MorphTap {
        mask,
        w,
        h,
        r,
        dilate,
    };
    for dy in -r..=r {
        for dx in -r..=r {
            apply_morph_sample(&tap, x + dx, y + dy, dx, dy, &mut best);
        }
    }
    best
}

struct MorphTap<'a> {
    mask: &'a [u8],
    w: usize,
    h: usize,
    r: i32,
    dilate: bool,
}

fn apply_morph_sample(tap: &MorphTap<'_>, xx: i32, yy: i32, dx: i32, dy: i32, best: &mut u8) {
    if dx * dx + dy * dy > tap.r * tap.r {
        return;
    }
    if xx < 0 || yy < 0 || (xx as usize) >= tap.w || (yy as usize) >= tap.h {
        if !tap.dilate {
            *best = 0;
        }
        return;
    }
    let v = tap.mask[yy as usize * tap.w + xx as usize];
    if tap.dilate {
        *best = (*best).max(v);
    } else {
        *best = (*best).min(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feather_softens_edge() {
        let w = 5_u32;
        let h = 5_u32;
        let mut mask = vec![0_u8; 25];
        mask[12] = 255; // center
        let out = feather_mask_r8(w, h, &mask, 1).expect("feather");
        assert!(out[12] > 0);
        assert!(out[12] < 255);
        assert!(out[7] > 0); // neighbor
    }

    #[test]
    fn expand_grows_selection() {
        let w = 7_u32;
        let h = 7_u32;
        let mut mask = vec![0_u8; 49];
        mask[24] = 255; // center
        let out = expand_mask_r8(w, h, &mask, 1).expect("expand");
        assert_eq!(out[24], 255);
        assert_eq!(out[23], 255);
        assert_eq!(out[17], 255);
    }

    #[test]
    fn contract_shrinks_selection() {
        let w = 5_u32;
        let h = 5_u32;
        let mask = vec![255_u8; 25];
        let out = contract_mask_r8(w, h, &mask, 1).expect("contract");
        assert_eq!(out[12], 255); // center remains
        assert_eq!(out[0], 0); // corner eroded
    }

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

    #[test]
    fn shape_parse_mask() {
        assert_eq!(SelectionShape::parse("mask"), SelectionShape::Mask);
        assert_eq!(SelectionShape::Mask.as_str(), "mask");
    }

    #[test]
    fn polygon_bounds_and_mask_commit() {
        let bounds = SelectionState::polygon_bounds(&[(1.2, 2.8), (10.0, 2.0), (5.0, 12.5)])
            .expect("bounds");
        assert_eq!(bounds.x, 1);
        assert_eq!(bounds.y, 2);
        assert_eq!(bounds.width, 9);
        assert_eq!(bounds.height, 11);
        let mut s = SelectionState::default();
        s.set_mask_polygon(bounds, SelectionCombine::Replace);
        assert!(s.active);
        assert_eq!(s.shape, SelectionShape::Mask);
    }

    #[test]
    fn every_modify_op_round_trips_through_its_label() {
        for op in SelectionModifyOp::ALL {
            assert_eq!(
                SelectionModifyOp::parse(op.as_str()),
                Some(op),
                "{} did not survive a parse of its own label",
                op.as_str()
            );
        }
    }

    #[test]
    fn an_unknown_modify_label_is_refused_rather_than_defaulted() {
        // The distinguishing property against SelectionCombine and
        // SelectionShape, which both fall back to a default.
        assert_eq!(SelectionModifyOp::parse("blur"), None);
        assert_eq!(SelectionModifyOp::parse("Feather"), None);
        assert_eq!(SelectionModifyOp::parse(""), None);
    }

    #[test]
    fn each_modify_op_dispatches_to_a_different_edit() {
        // Guards the `apply` match against a copy-paste that points two arms
        // at the same mask function — the arms are one word apart.
        let (w, h) = (7_u32, 7_u32);
        let mut mask = vec![0_u8; 49];
        for i in 16..19 {
            mask[i] = 255;
            mask[i + 7] = 255;
            mask[i + 14] = 255;
        }
        let feathered = SelectionModifyOp::Feather
            .apply(w, h, &mask, 1)
            .expect("feather");
        let expanded = SelectionModifyOp::Expand
            .apply(w, h, &mask, 1)
            .expect("expand");
        let contracted = SelectionModifyOp::Contract
            .apply(w, h, &mask, 1)
            .expect("contract");
        assert_eq!(feathered, feather_mask_r8(w, h, &mask, 1).expect("direct"));
        assert_eq!(expanded, expand_mask_r8(w, h, &mask, 1).expect("direct"));
        assert_eq!(
            contracted,
            contract_mask_r8(w, h, &mask, 1).expect("direct")
        );
        assert_ne!(feathered, expanded);
        assert_ne!(expanded, contracted);
        assert_ne!(feathered, contracted);
    }

    #[test]
    fn a_modify_argument_carries_its_op_and_radius() {
        assert_eq!(
            parse_selection_modify_arg("feather:4"),
            Some((SelectionModifyOp::Feather, 4))
        );
        assert_eq!(
            parse_selection_modify_arg("contract:2"),
            Some((SelectionModifyOp::Contract, 2))
        );
        assert_eq!(
            parse_selection_modify_arg(" expand : 12 "),
            Some((SelectionModifyOp::Expand, 12))
        );
    }

    #[test]
    fn an_omitted_radius_takes_the_default() {
        assert_eq!(
            parse_selection_modify_arg("expand"),
            Some((SelectionModifyOp::Expand, SelectionModifyOp::DEFAULT_RADIUS))
        );
    }

    #[test]
    fn a_present_but_unreadable_radius_is_refused() {
        // The case worth separating from the one above: substituting the
        // default here would apply the edit at a distance nobody asked for.
        assert_eq!(parse_selection_modify_arg("feather:"), None);
        assert_eq!(parse_selection_modify_arg("feather:-3"), None);
        assert_eq!(parse_selection_modify_arg("feather:wide"), None);
        assert_eq!(parse_selection_modify_arg("feather:2.5"), None);
        assert_eq!(parse_selection_modify_arg("feather:4:9"), None);
    }

    #[test]
    fn an_empty_argument_names_no_op() {
        // The old parser wrote `unwrap_or("feather")` here, which never ran:
        // `str::split` always yields at least one item, so an empty argument
        // produced the empty op rather than the intended default.
        assert_eq!(parse_selection_modify_arg(""), None);
        assert_eq!(parse_selection_modify_arg(":4"), None);
    }
}
