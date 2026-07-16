# Journal: Adjustment / filter GPU slice (2026-07-16)

## What shipped

- Engine: `set_adjustment`, `set_effects`, `add_gaussian_blur`, `set_gaussian_radius`; `GraphCommand::SetAdjustment` / `SetEffects` undo.
- GPU: Brightness/Contrast + Levels evaluate in the single-pass composite FS loop (adjustment kind); adjustment layers no longer get palette tint fills.
- GPU: Separable Gaussian Blur as nondestructive `FilterEffect` pre-pass when packing layer array slices.
- UI: Layer → New Adjustment → Brightness/Contrast | Levels; Filter → Gaussian Blur; Properties sliders for params/radius.
- CPU refs + unit tests for levels/gaussian; engine undo tests for adjustment/effects.

## Deferred

Invert polish, Curves/Hue, Box/Sharpen, modal filter dialogs, PSD effect import, destructive Apply.
