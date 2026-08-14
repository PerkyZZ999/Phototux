//! The QML canvas' in-progress selection path, and what it has to satisfy
//! before it can become a polygon selection.
//!
//! The shell builds a lasso or polygon as a `"x,y|x,y|…"` string and hands it
//! back on commit. Parsing it and deciding whether it is usable were three
//! guards inline in `select_polygon`, each with its own early return to the
//! same cleanup — which is both hard to read at the call site and impossible
//! to test without a live QObject.

use phototux_engine::SelectionState;

/// Whether a path string can become a polygon selection.
#[derive(Debug, Clone, PartialEq)]
pub enum PathVerdict {
    /// Not enough usable points, or they enclose nothing. The caller drops the
    /// in-progress path; there is nothing to commit and nothing to report.
    NotAPolygon,
    /// Usable: at least three finite points enclosing a non-empty area.
    Polygon(Vec<(f32, f32)>),
}

/// Parse the shell's `"x,y|x,y|…"` path into document-space points.
///
/// Malformed pairs are skipped rather than failing the whole path: the string
/// is built incrementally by the canvas, and one unreadable segment should not
/// discard a lasso the user spent a gesture drawing.
///
/// Non-finite coordinates are skipped on the same terms, and that part is not
/// cosmetic. [`SelectionState::polygon_bounds`] checks that the *bounds* it
/// computed are finite, which does not check the points: `f32::min` and
/// `f32::max` ignore NaN, so a NaN point leaves the bounds finite and passes
/// the guard. It would then reach the polygon rasteriser and the recorded
/// command arguments. Nothing demonstrates the shell producing one — the
/// screen-to-document helpers guard their divisor — so this closes the gap at
/// the boundary rather than fixing an observed failure.
#[must_use]
pub fn parse(points: &str) -> Vec<(f32, f32)> {
    points
        .split('|')
        .filter_map(|part| {
            let (xs, ys) = part.split_once(',')?;
            let x = xs.trim().parse::<f32>().ok()?;
            let y = ys.trim().parse::<f32>().ok()?;
            (x.is_finite() && y.is_finite()).then_some((x, y))
        })
        .collect()
}

/// Parse a path and decide whether it describes a committable polygon.
///
/// Three conditions, all of which used to sit inline: it parses to at least
/// three points, and those points have bounds — which is where a path that is
/// long but degenerate, every point on one line, is rejected.
#[must_use]
pub fn classify(points: &str) -> PathVerdict {
    let parsed = parse(points);
    if parsed.len() < 3 {
        return PathVerdict::NotAPolygon;
    }
    if SelectionState::polygon_bounds(&parsed).is_none() {
        return PathVerdict::NotAPolygon;
    }
    PathVerdict::Polygon(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_well_formed_path_keeps_its_points_in_order() {
        assert_eq!(
            parse("10,20|30.5,40|0,0"),
            vec![(10.0, 20.0), (30.5, 40.0), (0.0, 0.0)]
        );
    }

    #[test]
    fn surrounding_space_is_tolerated() {
        assert_eq!(parse(" 10 , 20 "), vec![(10.0, 20.0)]);
    }

    #[test]
    fn an_unreadable_segment_is_skipped_not_fatal() {
        // The rest of the gesture survives one bad segment.
        assert_eq!(parse("10,20|junk|30,40"), vec![(10.0, 20.0), (30.0, 40.0)]);
        assert_eq!(parse("10,20|50|30,40"), vec![(10.0, 20.0), (30.0, 40.0)]);
        assert_eq!(parse("10,20|,|30,40"), vec![(10.0, 20.0), (30.0, 40.0)]);
    }

    #[test]
    fn a_non_finite_coordinate_never_leaves_the_parser() {
        // "NaN" is what Qt's string conversion produces, and Rust's f32 parser
        // accepts it, so this is a coordinate that arrives looking valid.
        assert_eq!(parse("10,20|NaN,5|30,40"), vec![(10.0, 20.0), (30.0, 40.0)]);
        assert_eq!(parse("10,20|5,nan"), vec![(10.0, 20.0)]);
        assert_eq!(parse("inf,20|10,20"), vec![(10.0, 20.0)]);
        assert_eq!(parse("10,20|-inf,-inf"), vec![(10.0, 20.0)]);
        assert!(parse("NaN,NaN").is_empty());
    }

    #[test]
    fn an_empty_or_pointless_path_parses_to_nothing() {
        assert!(parse("").is_empty());
        assert!(parse("|||").is_empty());
    }

    #[test]
    fn three_finite_points_enclosing_area_are_a_polygon() {
        assert_eq!(
            classify("0,0|10,0|5,8"),
            PathVerdict::Polygon(vec![(0.0, 0.0), (10.0, 0.0), (5.0, 8.0)])
        );
    }

    #[test]
    fn fewer_than_three_points_are_not_a_polygon() {
        assert_eq!(classify("0,0|10,10"), PathVerdict::NotAPolygon);
        assert_eq!(classify("0,0"), PathVerdict::NotAPolygon);
        assert_eq!(classify(""), PathVerdict::NotAPolygon);
    }

    #[test]
    fn points_that_enclose_nothing_are_not_a_polygon() {
        // Collinear: three points, no area, so no bounds to select with.
        assert_eq!(classify("0,0|5,0|10,0"), PathVerdict::NotAPolygon);
    }

    #[test]
    fn dropping_non_finite_points_can_take_a_path_below_the_polygon_floor() {
        // Four segments arrive, three are usable coordinates but only after
        // the NaN goes — and what is left is a straight line. The count check
        // alone would have passed this.
        assert_eq!(classify("0,0|NaN,4|5,0|10,0"), PathVerdict::NotAPolygon);
    }
}
