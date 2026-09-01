//! Which document features become which render passes (handbook 17 / DR-006).
//!
//! This translation is document policy, not graphics: it decides that a
//! Gaussian blur effect and a drop-shadow style produce passes at all, which
//! ones win when two features overlap, and what counts as small enough to skip.
//! It lived in `phototux_gpu`, which is the crate that knows how to *run* a
//! pass, not which passes a layer deserves — so answering "what does a layer
//! with a shadow and a Levels adjustment actually draw as" meant reading across
//! two crates, and none of it was reachable from a headless test.
//!
//! The plan is deliberately backend-neutral: radii, offsets, opacities and
//! colours. Turning a descriptor into a pipeline stays in `phototux_gpu`.

use crate::layer::{FilterParams, Layer};
use crate::layer_style::LayerStyle;

/// A drop shadow or outer glow, resolved to offsets and blur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowPlan {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub opacity: f32,
    pub color_rgba: [f32; 4],
}

/// A flat colour laid over the layer's coverage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorOverlayPlan {
    pub opacity: f32,
    pub color_rgba: [f32; 4],
}

/// An outline drawn around the layer's coverage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokePlan {
    pub width: f32,
    pub opacity: f32,
    pub color_rgba: [f32; 4],
}

/// Everything a layer's effects and styles ask the renderer to do.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerRenderPlan {
    /// Filters to run, in stack order.
    ///
    /// A list rather than one slot per kind. The slots discarded the user's
    /// ordering — a sharpen before a blur is not the same picture as after —
    /// merged repeated effects by taking the larger parameter, and swallowed
    /// every kind with no slot of its own through a `_ => {}` arm, which is
    /// how Box Blur, Invert and Offset came to be unrunnable while sitting in
    /// the vocabulary.
    pub filters: Vec<FilterParams>,
    pub drop_shadow: Option<ShadowPlan>,
    /// Outer glow, kept separate from the drop shadow so a layer carrying both
    /// renders both. A glow is a shadow at zero offset, which is why they share
    /// a shape — but not a slot.
    pub outer_glow: Option<ShadowPlan>,
    pub color_overlay: Option<ColorOverlayPlan>,
    pub stroke: Option<StrokePlan>,
}

impl Default for LayerRenderPlan {
    fn default() -> Self {
        Self::identity()
    }
}

impl LayerRenderPlan {
    /// A plan that asks for nothing.
    #[must_use]
    pub fn identity() -> Self {
        Self {
            filters: Vec::new(),
            drop_shadow: None,
            outer_glow: None,
            color_overlay: None,
            stroke: None,
        }
    }

    /// Whether this plan needs an effect pass at all.
    ///
    /// Insignificant filters are dropped when the plan is built — a blur of
    /// 0.001 px is not worth an offscreen target and two passes — so a
    /// non-empty list is by construction a list worth running.
    #[must_use]
    pub fn needs_effects(&self) -> bool {
        !self.filters.is_empty()
            || self.drop_shadow.is_some()
            || self.outer_glow.is_some()
            || self.color_overlay.is_some()
            || self.stroke.is_some()
    }

    /// Resolve a layer's enabled effects and styles into a plan.
    ///
    /// Filters keep the order the user stacked them in, and repeated kinds
    /// stay repeated: two Gaussian blurs are two blurs. The plan used to merge
    /// them by taking the larger radius, which is a different picture and one
    /// the effect stack in the panel did not describe.
    ///
    /// A drop shadow and an outer glow occupy separate slots. They share a
    /// shape — a glow is a shadow at zero offset — and for a while shared one
    /// slot too, which meant a layer carrying both silently lost the glow.
    #[must_use]
    pub fn from_layer(layer: &Layer) -> Self {
        let mut plan = Self::identity();
        plan.filters = layer
            .effects
            .iter()
            .filter(|effect| effect.enabled)
            .map(|effect| effect.params)
            .filter(FilterParams::is_significant)
            .collect();
        for style in &layer.styles {
            match style {
                LayerStyle::DropShadow {
                    enabled: true,
                    offset_x,
                    offset_y,
                    blur,
                    opacity,
                    color_rgba,
                } => {
                    plan.drop_shadow = Some(ShadowPlan {
                        offset_x: *offset_x,
                        offset_y: *offset_y,
                        blur: *blur,
                        opacity: *opacity,
                        color_rgba: *color_rgba,
                    });
                }
                LayerStyle::Stroke {
                    enabled: true,
                    width,
                    opacity,
                    color_rgba,
                } => {
                    plan.stroke = Some(StrokePlan {
                        width: *width,
                        opacity: *opacity,
                        color_rgba: *color_rgba,
                    });
                }
                LayerStyle::OuterGlow {
                    enabled: true,
                    radius,
                    opacity,
                    color_rgba,
                } => {
                    plan.outer_glow = Some(ShadowPlan {
                        offset_x: 0.0,
                        offset_y: 0.0,
                        blur: *radius,
                        opacity: *opacity,
                        color_rgba: *color_rgba,
                    });
                }
                LayerStyle::ColorOverlay {
                    enabled: true,
                    opacity,
                    color_rgba,
                } => {
                    plan.color_overlay = Some(ColorOverlayPlan {
                        opacity: *opacity,
                        color_rgba: *color_rgba,
                    });
                }
                _ => {}
            }
        }
        plan
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{BlendMode, FilterEffect};

    fn raster() -> Layer {
        Layer::new(crate::layer::LayerId(1), "L")
    }

    fn effect(params: FilterParams, enabled: bool) -> FilterEffect {
        FilterEffect {
            id: 1,
            name: "fx".into(),
            enabled,
            opacity: 1.0,
            blend: BlendMode::Normal,
            params,
        }
    }

    #[test]
    fn an_untouched_layer_needs_no_effect_pass() {
        let plan = LayerRenderPlan::from_layer(&raster());
        assert_eq!(plan, LayerRenderPlan::identity());
        assert!(!plan.needs_effects());
    }

    #[test]
    fn disabled_effects_are_ignored() {
        let mut layer = raster();
        layer
            .effects
            .push(effect(FilterParams::GaussianBlur { radius: 8.0 }, false));
        assert!(!LayerRenderPlan::from_layer(&layer).needs_effects());
    }

    /// Below-threshold requests are treated as absent: a 0.001 px blur is not
    /// worth an offscreen target and two passes.
    #[test]
    fn negligible_effects_do_not_earn_a_pass() {
        let mut layer = raster();
        layer
            .effects
            .push(effect(FilterParams::GaussianBlur { radius: 0.001 }, true));
        layer
            .effects
            .push(effect(FilterParams::Sharpen { amount: 0.0001 }, true));
        assert!(!LayerRenderPlan::from_layer(&layer).needs_effects());
    }

    /// Two blurs are two blurs, in the order the user stacked them.
    ///
    /// This test used to assert the opposite — that repeats merged by taking
    /// the larger parameter — which is a different picture from the one the
    /// effect stack in the panel describes, and threw away the ordering with
    /// it.
    #[test]
    fn repeated_effects_stay_repeated_and_ordered() {
        let mut layer = raster();
        layer
            .effects
            .push(effect(FilterParams::GaussianBlur { radius: 4.0 }, true));
        layer
            .effects
            .push(effect(FilterParams::GaussianBlur { radius: 9.0 }, true));
        layer
            .effects
            .push(effect(FilterParams::Sharpen { amount: 0.2 }, true));
        layer
            .effects
            .push(effect(FilterParams::Sharpen { amount: 0.7 }, true));
        let plan = LayerRenderPlan::from_layer(&layer);
        assert_eq!(
            plan.filters,
            vec![
                FilterParams::GaussianBlur { radius: 4.0 },
                FilterParams::GaussianBlur { radius: 9.0 },
                FilterParams::Sharpen { amount: 0.2 },
                FilterParams::Sharpen { amount: 0.7 },
            ]
        );
    }

    /// Every kind in the vocabulary must survive the plan. Three did not: the
    /// resolver matched on the kinds it had a slot for and dropped the rest
    /// through a `_ => {}` arm, so Box Blur, Invert and Offset were unrunnable
    /// while remaining perfectly serializable.
    #[test]
    fn every_filter_kind_reaches_the_plan() {
        for &params in FilterParams::ALL_KINDS {
            let mut layer = raster();
            // Offset's default is a no-op by design; move it so it counts.
            let params = if matches!(params, FilterParams::Offset { .. }) {
                FilterParams::Offset { x: 4, y: 4 }
            } else {
                params
            };
            layer.effects.push(effect(params, true));
            let plan = LayerRenderPlan::from_layer(&layer);
            assert_eq!(
                plan.filters,
                vec![params],
                "{} was dropped by the plan",
                params.kind_key()
            );
            assert!(plan.needs_effects());
        }
    }

    /// A disabled effect contributes nothing, whatever its parameters.
    #[test]
    fn disabled_effects_stay_out_of_the_plan() {
        let mut layer = raster();
        layer
            .effects
            .push(effect(FilterParams::GaussianBlur { radius: 9.0 }, false));
        assert!(LayerRenderPlan::from_layer(&layer).filters.is_empty());
    }

    #[test]
    fn a_drop_shadow_keeps_its_offset_and_blur() {
        let mut layer = raster();
        layer.styles.push(LayerStyle::DropShadow {
            enabled: true,
            offset_x: 3.0,
            offset_y: -2.0,
            blur: 5.0,
            opacity: 0.5,
            color_rgba: [0.0, 0.0, 0.0, 1.0],
        });
        let shadow = LayerRenderPlan::from_layer(&layer)
            .drop_shadow
            .expect("shadow");
        assert!((shadow.offset_x - 3.0).abs() < 1e-6);
        assert!((shadow.offset_y + 2.0).abs() < 1e-6);
        assert!((shadow.blur - 5.0).abs() < 1e-6);
    }

    /// A glow is a shadow at zero offset, which is how it reaches the renderer.
    #[test]
    fn an_outer_glow_becomes_a_zero_offset_shadow() {
        let mut layer = raster();
        layer.styles.push(LayerStyle::OuterGlow {
            enabled: true,
            radius: 7.0,
            opacity: 0.8,
            color_rgba: [1.0, 1.0, 0.0, 1.0],
        });
        let shadow = LayerRenderPlan::from_layer(&layer)
            .outer_glow
            .expect("glow is planned");
        assert!((shadow.offset_x).abs() < 1e-6);
        assert!((shadow.offset_y).abs() < 1e-6);
        assert!((shadow.blur - 7.0).abs() < 1e-6);
    }

    /// A layer carrying both must render both. This previously asserted the
    /// opposite — that the glow was dropped — so that fixing it would fail here
    /// first rather than change rendering silently.
    #[test]
    fn a_glow_and_a_drop_shadow_both_survive() {
        let mut layer = raster();
        layer.styles.push(LayerStyle::DropShadow {
            enabled: true,
            offset_x: 4.0,
            offset_y: 4.0,
            blur: 2.0,
            opacity: 1.0,
            color_rgba: [0.0, 0.0, 0.0, 1.0],
        });
        layer.styles.push(LayerStyle::OuterGlow {
            enabled: true,
            radius: 9.0,
            opacity: 1.0,
            color_rgba: [1.0, 1.0, 0.0, 1.0],
        });
        let plan = LayerRenderPlan::from_layer(&layer);
        let shadow = plan.drop_shadow.expect("drop shadow");
        let glow = plan.outer_glow.expect("outer glow");
        assert!(
            (shadow.offset_x - 4.0).abs() < 1e-6,
            "shadow keeps its offset"
        );
        assert!((glow.offset_x).abs() < 1e-6, "a glow has no offset");
        assert!((glow.blur - 9.0).abs() < 1e-6, "glow keeps its radius");
    }

    #[test]
    fn styles_and_effects_accumulate_into_one_plan() {
        let mut layer = raster();
        layer
            .effects
            .push(effect(FilterParams::GaussianBlur { radius: 3.0 }, true));
        layer.styles.push(LayerStyle::Stroke {
            enabled: true,
            width: 2.0,
            opacity: 1.0,
            color_rgba: [0.0, 0.0, 0.0, 1.0],
        });
        layer.styles.push(LayerStyle::ColorOverlay {
            enabled: true,
            opacity: 0.25,
            color_rgba: [1.0, 0.0, 0.0, 1.0],
        });
        let plan = LayerRenderPlan::from_layer(&layer);
        assert!(plan.needs_effects());
        assert!(plan.stroke.is_some());
        assert!(plan.color_overlay.is_some());
        assert_eq!(
            plan.filters,
            vec![FilterParams::GaussianBlur { radius: 3.0 }]
        );
    }
}
