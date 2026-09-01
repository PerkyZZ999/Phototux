//! Per-layer blend ranges — Photoshop's "Blend If".
//!
//! A layer can be hidden where *it* is too dark or too light, and hidden where
//! the composite *underneath* it is too dark or too light. That is the whole
//! feature, and it is the cheapest way in the editor to knock a sky out of a
//! photograph or to confine a texture to the shadows of what is beneath it.
//!
//! Each of the two ranges has four stops rather than two. With two, a range is
//! a hard cut and the edge it leaves is aliased and obvious; the inner pair
//! give the cut a ramp, which is what Photoshop's split slider handles are for.
//! A range whose stops coincide is still a hard cut, so nothing is lost by
//! always carrying four.
//!
//! The maths lives here, in the crate with no GPU, because it is the reference
//! the WGSL mirrors: [`BlendIf::coverage`] and the shader's `blend_if_factor`
//! compute the same number, and a device fixture holds them to it.

use serde::{Deserialize, Serialize};

/// Which channel a blend range reads.
///
/// A vocabulary rather than a `u32` because the value crosses into WGSL, into
/// `.ptx` and into the chrome, and the three had no reason to agree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BlendIfChannel {
    /// Rec.601 luma, matching [`crate::blend_rgb`]'s non-separable modes.
    #[default]
    Gray,
    Red,
    Green,
    Blue,
}

impl BlendIfChannel {
    pub const ALL: [Self; 4] = [Self::Gray, Self::Red, Self::Green, Self::Blue];

    /// Wire key, used in `.ptx`, in action arguments and in QML.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gray => "gray",
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
        }
    }

    /// Parse a wire key, `None` when it names no channel.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == key)
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Gray => "Gray",
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Blue => "Blue",
        }
    }

    /// The code the composite shader switches on. Never zero: zero is the
    /// shader's "no blend range", so a channel that mapped to it would silently
    /// disable the feature instead of reading a channel.
    #[must_use]
    pub fn gpu_code(self) -> u32 {
        match self {
            Self::Gray => 1,
            Self::Red => 2,
            Self::Green => 3,
            Self::Blue => 4,
        }
    }

    /// Read this channel out of a linear RGB triple.
    #[must_use]
    pub fn value(self, rgb: [f32; 3]) -> f32 {
        match self {
            Self::Gray => 0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2],
            Self::Red => rgb[0],
            Self::Green => rgb[1],
            Self::Blue => rgb[2],
        }
    }
}

/// One four-stop range: hidden below `black_start`, hidden above `white_end`.
///
/// The stops are `0..=1` rather than Photoshop's `0..=255` because everything
/// else that crosses into the shader is normalised, and the chrome is the only
/// place a 0–255 reading belongs. They are kept in order by
/// [`Self::normalized`] rather than by the setters, so a `.ptx` written by an
/// older build — or by hand — cannot produce a range that inverts itself.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlendRange {
    /// Below this, the layer is fully hidden.
    pub black_start: f32,
    /// At and above this, the dark end is fully shown.
    pub black_end: f32,
    /// Up to this, the light end is fully shown.
    pub white_start: f32,
    /// Above this, the layer is fully hidden.
    pub white_end: f32,
}

impl Default for BlendRange {
    fn default() -> Self {
        Self::FULL
    }
}

impl BlendRange {
    /// The range that hides nothing.
    pub const FULL: Self = Self {
        black_start: 0.0,
        black_end: 0.0,
        white_start: 1.0,
        white_end: 1.0,
    };

    /// Labels for the four stops, in slot order.
    pub const STOP_LABELS: [&'static str; 4] =
        ["Black start", "Black end", "White start", "White end"];

    /// Whether this range lets everything through.
    #[must_use]
    pub fn is_full(self) -> bool {
        let n = self.normalized();
        n.black_start <= 0.0 && n.black_end <= 0.0 && n.white_start >= 1.0 && n.white_end >= 1.0
    }

    /// The stops clamped to `0..=1` and forced into ascending order.
    ///
    /// Sorting rather than rejecting: the four values are four slider handles,
    /// and a user dragging the white pair past the black pair means "swap
    /// them", not "refuse the drag". Doing it here rather than in the setter
    /// also covers values arriving from a file.
    #[must_use]
    pub fn normalized(self) -> Self {
        let mut v = [
            self.black_start,
            self.black_end,
            self.white_start,
            self.white_end,
        ];
        for x in &mut v {
            *x = if x.is_finite() {
                x.clamp(0.0, 1.0)
            } else {
                0.0
            };
        }
        v.sort_by(f32::total_cmp);
        Self {
            black_start: v[0],
            black_end: v[1],
            white_start: v[2],
            white_end: v[3],
        }
    }

    /// The four stops as the shader reads them.
    #[must_use]
    pub fn stops(self) -> [f32; 4] {
        let n = self.normalized();
        [n.black_start, n.black_end, n.white_start, n.white_end]
    }

    /// Rebuild from four slot values, in [`Self::STOP_LABELS`] order.
    #[must_use]
    pub fn from_stops(stops: [f32; 4]) -> Self {
        Self {
            black_start: stops[0],
            black_end: stops[1],
            white_start: stops[2],
            white_end: stops[3],
        }
    }

    /// How much of the layer survives at channel value `v`.
    ///
    /// A ramp between coincident stops would be a division by zero, so an
    /// empty ramp is a step. That is not a special case bolted on — a
    /// non-split slider handle *is* two coincident stops, and it is the
    /// default, so the step is the common path rather than the degenerate one.
    #[must_use]
    pub fn coverage(self, v: f32) -> f32 {
        let n = self.normalized();
        let dark = ramp_up(v, n.black_start, n.black_end);
        let light = ramp_down(v, n.white_start, n.white_end);
        dark.min(light).clamp(0.0, 1.0)
    }
}

/// Dark end: `0` below `lo`, `1` at and above `hi`, linear between.
///
/// The two ramps are separate functions rather than one and its complement,
/// because they break the tie at a coincident pair in *opposite* directions and
/// the tie is the common case — an unsplit slider handle is two coincident
/// stops, and a full range is two of those. A pixel sitting exactly on a
/// threshold is shown, at both ends: the dark ramp therefore tests `hi` first
/// and the light ramp tests `lo` first. Getting either backwards makes the
/// default range hide pure black or pure white, which is a range that is
/// supposed to hide nothing at all.
fn ramp_up(v: f32, lo: f32, hi: f32) -> f32 {
    if v >= hi {
        return 1.0;
    }
    if v <= lo {
        return 0.0;
    }
    (v - lo) / (hi - lo)
}

/// Light end: `1` at and below `lo`, `0` at and above `hi`, linear between.
fn ramp_down(v: f32, lo: f32, hi: f32) -> f32 {
    if v <= lo {
        return 1.0;
    }
    if v >= hi {
        return 0.0;
    }
    (hi - v) / (hi - lo)
}

/// A layer's two blend ranges and the channel they read.
///
/// Stored on the layer and serialised into `.ptx`, so the field carries
/// `#[serde(default)]` at its use site: a document written before this existed
/// must open with the ranges that hide nothing.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct BlendIf {
    pub channel: BlendIfChannel,
    /// Read from this layer's own pixels.
    pub this_layer: BlendRange,
    /// Read from the composite beneath this layer.
    pub underlying: BlendRange,
}

impl BlendIf {
    /// Whether this hides nothing, and can be skipped entirely.
    ///
    /// The composite checks this before doing any of the work, and the chrome
    /// uses it to decide whether the group is worth a badge — an inactive
    /// Blend If that still cost a shader branch per pixel would be the sort of
    /// thing nobody notices until a profile says so.
    #[must_use]
    pub fn is_identity(self) -> bool {
        self.this_layer.is_full() && self.underlying.is_full()
    }

    /// How much of `this_rgb` survives over a composite of `under_rgb`.
    ///
    /// This is the CPU reference the WGSL mirrors. The two ranges multiply:
    /// each is an independent reason to hide the pixel, and a pixel excluded by
    /// either is excluded.
    #[must_use]
    pub fn coverage(self, this_rgb: [f32; 3], under_rgb: [f32; 3]) -> f32 {
        if self.is_identity() {
            return 1.0;
        }
        self.this_layer.coverage(self.channel.value(this_rgb))
            * self.underlying.coverage(self.channel.value(under_rgb))
    }

    /// The channel code the shader switches on, or `0` when there is nothing
    /// to do. Letting the identity case reach the shader as a live channel
    /// would cost the branch for no change in the picture.
    #[must_use]
    pub fn gpu_channel(self) -> u32 {
        if self.is_identity() {
            0
        } else {
            self.channel.gpu_code()
        }
    }
}

/// Every channel as `[{id, label}]`, for the chrome.
#[must_use]
pub fn blend_if_channels_json() -> String {
    let rows: Vec<serde_json::Value> = BlendIfChannel::ALL
        .iter()
        .map(|c| serde_json::json!({ "id": c.as_str(), "label": c.label() }))
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLACK: [f32; 3] = [0.0, 0.0, 0.0];
    const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
    const MID: [f32; 3] = [0.5, 0.5, 0.5];

    #[test]
    fn every_channel_round_trips_and_has_a_nonzero_code() {
        for c in BlendIfChannel::ALL {
            assert_eq!(BlendIfChannel::parse(c.as_str()), Some(c));
            // Zero is the shader's "no blend range"; a channel mapping to it
            // would silently turn the feature off instead of reading a channel.
            assert_ne!(c.gpu_code(), 0, "{} maps to the off code", c.as_str());
        }
        assert_eq!(BlendIfChannel::parse("grey"), None);
    }

    #[test]
    fn the_default_blend_if_hides_nothing() {
        let b = BlendIf::default();
        assert!(b.is_identity());
        assert_eq!(b.gpu_channel(), 0);
        for this in [BLACK, MID, WHITE] {
            for under in [BLACK, MID, WHITE] {
                assert_eq!(b.coverage(this, under), 1.0);
            }
        }
    }

    #[test]
    fn a_hard_range_is_a_step_not_a_division_by_zero() {
        // Both stops coincide, which is what an un-split slider handle is —
        // the default shape, not a degenerate one.
        let r = BlendRange {
            black_start: 0.5,
            black_end: 0.5,
            ..BlendRange::FULL
        };
        assert_eq!(r.coverage(0.49), 0.0);
        assert_eq!(r.coverage(0.5), 1.0);
        assert_eq!(r.coverage(0.51), 1.0);
        assert!(r.coverage(0.5).is_finite());
    }

    #[test]
    fn a_full_range_shows_the_two_values_it_is_easiest_to_hide() {
        // Pure black sits exactly on the default black stops and pure white on
        // the default white stops. Break either tie the wrong way and the
        // range that is supposed to hide nothing hides one end of the tonal
        // scale — invisibly, on any layer whose *other* range is in use.
        let r = BlendRange::FULL;
        assert_eq!(r.coverage(0.0), 1.0, "a full range hid pure black");
        assert_eq!(r.coverage(1.0), 1.0, "a full range hid pure white");
        assert_eq!(r.coverage(0.5), 1.0);
    }

    #[test]
    fn a_pixel_sitting_exactly_on_a_threshold_is_shown_at_either_end() {
        let dark = BlendRange {
            black_start: 0.4,
            black_end: 0.4,
            ..BlendRange::FULL
        };
        assert_eq!(dark.coverage(0.4), 1.0);
        let light = BlendRange {
            white_start: 0.6,
            white_end: 0.6,
            ..BlendRange::FULL
        };
        assert_eq!(light.coverage(0.6), 1.0);
        assert_eq!(light.coverage(0.61), 0.0);
    }

    #[test]
    fn a_split_range_ramps_between_its_stops() {
        let r = BlendRange {
            black_start: 0.2,
            black_end: 0.6,
            ..BlendRange::FULL
        };
        assert_eq!(r.coverage(0.1), 0.0);
        assert!((r.coverage(0.4) - 0.5).abs() < 1e-5, "{}", r.coverage(0.4));
        assert_eq!(r.coverage(0.8), 1.0);
    }

    #[test]
    fn the_white_end_hides_the_bright_side() {
        let r = BlendRange {
            white_start: 0.4,
            white_end: 0.8,
            ..BlendRange::FULL
        };
        assert_eq!(r.coverage(0.2), 1.0);
        assert!((r.coverage(0.6) - 0.5).abs() < 1e-5);
        assert_eq!(r.coverage(0.9), 0.0);
    }

    #[test]
    fn coverage_never_leaves_zero_to_one_however_the_stops_are_arranged() {
        let steps = [0.0_f32, 0.13, 0.37, 0.5, 0.62, 0.88, 1.0];
        for &a in &steps {
            for &b in &steps {
                for &c in &steps {
                    for &d in &steps {
                        let r = BlendRange::from_stops([a, b, c, d]);
                        for &v in &steps {
                            let cov = r.coverage(v);
                            assert!(
                                (0.0..=1.0).contains(&cov) && cov.is_finite(),
                                "stops {a} {b} {c} {d} at {v} gave {cov}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn out_of_order_stops_are_sorted_rather_than_inverting_the_range() {
        // Four slider handles dragged past each other mean "swap", not
        // "refuse" — and a `.ptx` can carry any four numbers at all.
        let tangled = BlendRange::from_stops([0.9, 0.7, 0.3, 0.1]);
        let tidy = BlendRange::from_stops([0.1, 0.3, 0.7, 0.9]);
        for v in [0.0_f32, 0.2, 0.5, 0.8, 1.0] {
            assert!(
                (tangled.coverage(v) - tidy.coverage(v)).abs() < 1e-6,
                "at {v}: {} vs {}",
                tangled.coverage(v),
                tidy.coverage(v)
            );
        }
    }

    #[test]
    fn non_finite_stops_do_not_reach_the_arithmetic() {
        let r = BlendRange::from_stops([f32::NAN, f32::INFINITY, 0.5, f32::NEG_INFINITY]);
        for v in [0.0_f32, 0.5, 1.0] {
            assert!(r.coverage(v).is_finite(), "at {v}");
        }
        assert!(r.stops().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn hiding_the_dark_end_of_this_layer_leaves_its_light_end() {
        let b = BlendIf {
            channel: BlendIfChannel::Gray,
            this_layer: BlendRange {
                black_start: 0.4,
                black_end: 0.4,
                ..BlendRange::FULL
            },
            underlying: BlendRange::FULL,
        };
        assert!(!b.is_identity());
        assert_eq!(b.coverage(BLACK, MID), 0.0);
        assert_eq!(b.coverage(WHITE, MID), 1.0);
    }

    #[test]
    fn the_underlying_range_reads_what_is_beneath_not_the_layer_itself() {
        // The two ranges are easy to swap in the shader and in the panel, and
        // a swap looks like a plausible result rather than a bug.
        let b = BlendIf {
            channel: BlendIfChannel::Gray,
            this_layer: BlendRange::FULL,
            underlying: BlendRange {
                black_start: 0.4,
                black_end: 0.4,
                ..BlendRange::FULL
            },
        };
        assert_eq!(b.coverage(WHITE, BLACK), 0.0, "dark backdrop must hide it");
        assert_eq!(b.coverage(BLACK, WHITE), 1.0, "dark layer must not");
    }

    #[test]
    fn the_two_ranges_multiply_so_either_can_hide_a_pixel() {
        let half = BlendRange {
            black_start: 0.0,
            black_end: 1.0,
            ..BlendRange::FULL
        };
        let b = BlendIf {
            channel: BlendIfChannel::Gray,
            this_layer: half,
            underlying: half,
        };
        // Each range passes half at mid-grey, so together they pass a quarter.
        assert!((b.coverage(MID, MID) - 0.25).abs() < 1e-4);
    }

    #[test]
    fn each_channel_reads_its_own_component() {
        let hide_dark = BlendRange {
            black_start: 0.5,
            black_end: 0.5,
            ..BlendRange::FULL
        };
        let red_only = [1.0, 0.0, 0.0];
        for channel in BlendIfChannel::ALL {
            let b = BlendIf {
                channel,
                this_layer: hide_dark,
                underlying: BlendRange::FULL,
            };
            let visible = b.coverage(red_only, MID) > 0.0;
            assert_eq!(
                visible,
                channel == BlendIfChannel::Red,
                "{} disagrees about a pure red pixel",
                channel.as_str()
            );
        }
    }

    #[test]
    fn an_identity_blend_if_is_switched_off_before_it_reaches_the_shader() {
        let mut b = BlendIf::default();
        assert_eq!(b.gpu_channel(), 0);
        b.channel = BlendIfChannel::Blue;
        assert_eq!(b.gpu_channel(), 0, "an identity range costs no branch");
        b.this_layer.black_start = 0.2;
        b.this_layer.black_end = 0.2;
        assert_eq!(b.gpu_channel(), BlendIfChannel::Blue.gpu_code());
    }

    #[test]
    fn stops_survive_a_round_trip_through_the_slot_projection() {
        let r = BlendRange::from_stops([0.1, 0.25, 0.75, 0.9]);
        assert_eq!(BlendRange::from_stops(r.stops()), r);
        assert_eq!(BlendRange::STOP_LABELS.len(), r.stops().len());
    }
}
