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
    pub gaussian: f32,
    pub motion: Option<(f32, f32)>,
    pub emboss: Option<(f32, f32)>,
    pub sharpen: Option<f32>,
    pub noise: Option<f32>,
    pub drop_shadow: Option<ShadowPlan>,
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
            gaussian: 0.0,
            motion: None,
            emboss: None,
            sharpen: None,
            noise: None,
            drop_shadow: None,
            color_overlay: None,
            stroke: None,
        }
    }

    /// Whether this plan needs an effect pass at all.
    ///
    /// The thresholds are the point: a blur of 0.001 px is not worth an
    /// offscreen target and two passes, so it is treated as absent. They match
    /// the guards in [`Self::from_layer`], and both must move together.
    #[must_use]
    pub fn needs_effects(&self) -> bool {
        self.gaussian > 0.01
            || self.motion.is_some()
            || self.emboss.is_some()
            || self.sharpen.is_some_and(|a| a > 0.001)
            || self.noise.is_some_and(|a| a > 0.001)
            || self.drop_shadow.is_some()
            || self.color_overlay.is_some()
            || self.stroke.is_some()
    }

    /// Resolve a layer's enabled effects and styles into a plan.
    ///
    /// Repeated effects of the same kind combine rather than replace: two
    /// Gaussian blurs take the larger radius, because the second is a stronger
    /// request for the same thing, not a second blur to run.
    ///
    /// **Known defect, preserved deliberately:** an outer glow is only honoured
    /// when the layer has no drop shadow, because both are expressed through the
    /// single `drop_shadow` slot — a glow is a shadow at zero offset. A layer
    /// carrying both silently loses the glow. Representing them separately means
    /// a second shadow pass in the renderer, so the fix is not local to this
    /// function; it is recorded in the gap analysis rather than papered over.
    #[must_use]
    pub fn from_layer(layer: &Layer) -> Self {
        let mut plan = Self::identity();
        for effect in &layer.effects {
            if !effect.enabled {
                continue;
            }
            match effect.params {
                FilterParams::GaussianBlur { radius } if radius > 0.01 => {
                    plan.gaussian = plan.gaussian.max(radius);
                }
                FilterParams::MotionBlur {
                    distance,
                    angle_deg,
                } if distance > 0.01 => {
                    plan.motion = Some((distance, angle_deg));
                }
                FilterParams::Emboss {
                    strength,
                    angle_deg,
                } if strength > 0.01 => {
                    plan.emboss = Some((strength, angle_deg));
                }
                FilterParams::Sharpen { amount } if amount > 0.001 => {
                    plan.sharpen = Some(plan.sharpen.map_or(amount, |a| a.max(amount)));
                }
                FilterParams::Noise { amount } if amount > 0.001 => {
                    plan.noise = Some(plan.noise.map_or(amount, |a| a.max(amount)));
                }
                _ => {}
            }
        }
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
                } if plan.drop_shadow.is_none() => {
                    plan.drop_shadow = Some(ShadowPlan {
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

    /// Two blurs are a stronger request for one blur, not two blurs to run.
    #[test]
    fn repeated_effects_take_the_stronger_request() {
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
        assert!((plan.gaussian - 9.0).abs() < 1e-6);
        assert!((plan.sharpen.expect("sharpen") - 0.7).abs() < 1e-6);
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
            .drop_shadow
            .expect("glow becomes a shadow");
        assert!((shadow.offset_x).abs() < 1e-6);
        assert!((shadow.offset_y).abs() < 1e-6);
        assert!((shadow.blur - 7.0).abs() < 1e-6);
    }

    /// Pins the known defect so the behaviour is a recorded decision rather
    /// than a surprise, and so fixing it fails loudly here first.
    #[test]
    fn a_glow_is_currently_lost_when_a_drop_shadow_is_present() {
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
        let shadow = LayerRenderPlan::from_layer(&layer)
            .drop_shadow
            .expect("shadow");
        assert!(
            (shadow.offset_x - 4.0).abs() < 1e-6,
            "the drop shadow wins the single slot; the glow is dropped"
        );
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
        assert!((plan.gaussian - 3.0).abs() < 1e-6);
    }
}
