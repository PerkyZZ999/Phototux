//! Gradient shapes — how a point maps to a position along the ramp.
//!
//! The colour interpolation and the buffer walk live with the fill code in
//! `phototux_gpu`; what varies between a linear gradient and a radial one is
//! only this parameter, and *that* is document policy: it decides what the
//! user's drag means, not how to write pixels.

use serde::{Deserialize, Serialize};

/// Which shape a gradient sweeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradientKind {
    /// Along the drag, perpendicular bands.
    #[default]
    Linear,
    /// Outward from the drag's start, circular bands.
    Radial,
    /// Around the drag's start, a sweep of angle.
    Angle,
    /// Along the drag and mirrored back, so the start is the centre.
    Reflected,
    /// Outward from the drag's start in a rotated square.
    Diamond,
}

impl GradientKind {
    /// Every kind, in menu order.
    pub const ALL: [Self; 5] = [
        Self::Linear,
        Self::Radial,
        Self::Angle,
        Self::Reflected,
        Self::Diamond,
    ];

    /// Stable wire name for the registry, prefs and `.ptx`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Radial => "radial",
            Self::Angle => "angle",
            Self::Reflected => "reflected",
            Self::Diamond => "diamond",
        }
    }

    /// Display name for the tool options.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::Radial => "Radial",
            Self::Angle => "Angle",
            Self::Reflected => "Reflected",
            Self::Diamond => "Diamond",
        }
    }

    /// Icon stem for the tool options; see `assets/icons/ICON_MAP.md`.
    #[must_use]
    pub fn icon_key(self) -> &'static str {
        match self {
            Self::Linear => "gradient",
            Self::Radial => "circle-half",
            Self::Angle => "circle-notch",
            Self::Reflected => "arrows-left-right",
            Self::Diamond => "diamond",
        }
    }

    /// Parse a wire name; `None` when it names no kind this build ships.
    ///
    /// No fallback: the caller is about to paint, and sweeping a different
    /// shape than the one asked for is an edit the user has to notice and undo.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == name)
    }

    /// Position along the ramp, `0..=1`, for a point given the drag `p0`→`p1`.
    ///
    /// `p0` is where the drag started, which every kind treats as the origin;
    /// `p1` sets the direction and the distance at which the ramp reaches its
    /// far end.
    #[must_use]
    pub fn parameter_at(self, p0: [f32; 2], p1: [f32; 2], x: f32, y: f32) -> f32 {
        let dx = p1[0] - p0[0];
        let dy = p1[1] - p0[1];
        let len2 = dx * dx + dy * dy;
        if len2 < 1e-6 {
            return 0.0;
        }
        let px = x - p0[0];
        let py = y - p0[1];
        let t = match self {
            Self::Linear => (px * dx + py * dy) / len2,
            Self::Radial => ((px * px + py * py) / len2).sqrt(),
            // Angle sweeps a full turn, so the drag sets where zero points
            // rather than how far the ramp reaches.
            Self::Angle => {
                let a = py.atan2(px) - dy.atan2(dx);
                a.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU
            }
            // Mirrored about the start: the drag's midpoint behaviour of a
            // linear ramp, folded so the start is the centre of the band.
            Self::Reflected => ((px * dx + py * dy) / len2).abs(),
            // Chebyshev distance in the drag's frame draws a rotated square.
            Self::Diamond => {
                let len = len2.sqrt();
                let (ux, uy) = (dx / len, dy / len);
                let along = (px * ux + py * uy).abs();
                let across = (px * -uy + py * ux).abs();
                (along + across) / len
            }
        };
        t.clamp(0.0, 1.0)
    }
}

/// The gradient a drag describes: its shape, its endpoints and its two colours.
///
/// One value rather than five parameters, because they are five halves of one
/// answer — a caller with the endpoints but not the shape cannot paint, and
/// threading them separately pushed the fill past the argument limit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientRamp {
    pub kind: GradientKind,
    /// Where the drag began; every kind treats this as the origin.
    pub start: [f32; 2],
    /// Where the drag ended; sets direction and the distance to the far end.
    pub end: [f32; 2],
    pub start_rgba: [f32; 4],
    pub end_rgba: [f32; 4],
}

impl GradientRamp {
    /// Colour at a point, interpolated along the ramp.
    #[must_use]
    pub fn color_at(&self, x: f32, y: f32) -> [f32; 4] {
        let t = self.kind.parameter_at(self.start, self.end, x, y);
        std::array::from_fn(|c| self.start_rgba[c] + (self.end_rgba[c] - self.start_rgba[c]) * t)
    }

    /// Whether the drag has a direction at all.
    ///
    /// A click without a drag names no gradient; the caller fills flat rather
    /// than dividing by zero.
    #[must_use]
    pub fn has_direction(&self) -> bool {
        let dx = self.end[0] - self.start[0];
        let dy = self.end[1] - self.start[1];
        dx * dx + dy * dy >= 1e-6
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_round_trips_and_is_labelled() {
        let mut seen = Vec::new();
        for kind in GradientKind::ALL {
            assert_eq!(GradientKind::parse(kind.as_str()), Some(kind));
            assert!(!kind.label().is_empty());
            assert!(
                !seen.contains(&kind.as_str()),
                "{kind:?} reuses a wire name"
            );
            seen.push(kind.as_str());
        }
        assert_eq!(GradientKind::parse("nonsense"), None);
    }

    /// Every kind must read 0 at the drag's start: that is what makes the
    /// first colour appear where the user pressed.
    #[test]
    fn every_kind_starts_at_zero() {
        let p0 = [10.0, 20.0];
        let p1 = [110.0, 20.0];
        for kind in GradientKind::ALL {
            let t = kind.parameter_at(p0, p1, p0[0], p0[1]);
            assert!(t.abs() < 1e-4, "{kind:?} starts at {t}");
        }
    }

    /// A zero-length drag has no direction, so every kind reports the start of
    /// the ramp rather than dividing by zero.
    #[test]
    fn a_zero_length_drag_is_not_a_division() {
        for kind in GradientKind::ALL {
            let t = kind.parameter_at([5.0, 5.0], [5.0, 5.0], 40.0, 90.0);
            assert!(
                t.is_finite() && (0.0..=1.0).contains(&t),
                "{kind:?} gave {t}"
            );
        }
    }

    /// The parameter is a ramp position; nothing may escape 0..=1 however far
    /// the point is from the drag.
    #[test]
    fn the_parameter_never_escapes_the_ramp() {
        let p0 = [50.0, 50.0];
        let p1 = [80.0, 20.0];
        for kind in GradientKind::ALL {
            for &(x, y) in &[
                (-500.0, -500.0),
                (500.0, 500.0),
                (0.0, 500.0),
                (500.0, 0.0),
                (50.0, 50.0),
            ] {
                let t = kind.parameter_at(p0, p1, x, y);
                assert!((0.0..=1.0).contains(&t), "{kind:?} at ({x}, {y}) gave {t}");
            }
        }
    }

    /// The ramp's ends must be its two colours, whatever shape it sweeps.
    #[test]
    fn a_ramp_lands_on_its_own_colours() {
        for kind in GradientKind::ALL {
            let ramp = GradientRamp {
                kind,
                start: [0.0, 0.0],
                end: [100.0, 0.0],
                start_rgba: [1.0, 0.0, 0.0, 1.0],
                end_rgba: [0.0, 0.0, 1.0, 1.0],
            };
            assert!(ramp.has_direction());
            let at_start = ramp.color_at(0.0, 0.0);
            for (a, b) in at_start.iter().zip(ramp.start_rgba) {
                assert!((a - b).abs() < 1e-4, "{kind:?} start: {at_start:?}");
            }
        }
    }

    /// A click without a drag names no gradient.
    #[test]
    fn a_click_without_a_drag_has_no_direction() {
        let ramp = GradientRamp {
            kind: GradientKind::Linear,
            start: [7.0, 7.0],
            end: [7.0, 7.0],
            start_rgba: [0.0; 4],
            end_rgba: [1.0; 4],
        };
        assert!(!ramp.has_direction());
    }

    /// Linear runs one way; reflected folds it, so a point behind the start
    /// reads the same as its mirror ahead of it. That fold is the difference
    /// between the two kinds.
    #[test]
    fn reflected_mirrors_what_linear_clamps() {
        let p0 = [50.0, 50.0];
        let p1 = [150.0, 50.0];
        // 30px behind the start.
        let behind = (20.0, 50.0);
        let ahead = (80.0, 50.0);
        assert!(
            GradientKind::Linear.parameter_at(p0, p1, behind.0, behind.1) < 1e-6,
            "linear should clamp behind the start"
        );
        let a = GradientKind::Reflected.parameter_at(p0, p1, behind.0, behind.1);
        let b = GradientKind::Reflected.parameter_at(p0, p1, ahead.0, ahead.1);
        assert!((a - b).abs() < 1e-4, "reflected did not mirror: {a} vs {b}");
    }

    /// Radial rings are circles, so two points the same distance from the
    /// start read the same wherever they sit around it.
    #[test]
    fn radial_depends_only_on_distance() {
        let p0 = [0.0, 0.0];
        let p1 = [100.0, 0.0];
        let east = GradientKind::Radial.parameter_at(p0, p1, 40.0, 0.0);
        let north = GradientKind::Radial.parameter_at(p0, p1, 0.0, 40.0);
        let diagonal = GradientKind::Radial.parameter_at(p0, p1, 28.284_27, 28.284_27);
        assert!((east - north).abs() < 1e-4);
        assert!((east - diagonal).abs() < 1e-3);
    }

    /// Distinct kinds must map the same point differently, or one of them is
    /// unreachable however well the chrome offers it.
    #[test]
    fn the_kinds_disagree_somewhere() {
        let p0 = [0.0, 0.0];
        let p1 = [100.0, 0.0];
        let probes = [(30.0, 40.0), (-20.0, 10.0), (70.0, -60.0), (10.0, 90.0)];
        for (i, a) in GradientKind::ALL.into_iter().enumerate() {
            for b in GradientKind::ALL.into_iter().skip(i + 1) {
                assert!(
                    probes.iter().any(|&(x, y)| {
                        (a.parameter_at(p0, p1, x, y) - b.parameter_at(p0, p1, x, y)).abs() > 1e-3
                    }),
                    "{a:?} and {b:?} map every probe identically"
                );
            }
        }
    }
}
