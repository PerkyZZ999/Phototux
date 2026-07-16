# Journal: Fill + Linear Gradient (2026-07-16)

## Interpretation of checklist #8

Suggested-next “Vector / shapes / rich text” pivoted to **Color & Fill**: Fill/Gradient tools were dead strip stubs; true vector/`LayerKind::Shape` needs ADR-017. Text bake deferred to next candidate.

## What shipped

- `phototux_gpu::fill`: solid fill + linear gradient + sample helpers (selection-mask aware).
- Canvas: `fill_layer`, `apply_linear_gradient`, `sample_layer_at` / `sample_composite_at`.
- UI: Paint Bucket click, Gradient drag (FG→BG) with preview line, Eyedropper sample → FG.
- Undo via transform pixel snapshots (`Fill` / `Gradient` history labels).

## Deferred

Flood-fill tolerance, non-linear gradients, Shape kind ADR, text glyph rasterization.
