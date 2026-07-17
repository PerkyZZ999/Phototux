# Handbook parity P5 depth (2026-07-17)

## Shipped

- CPU dab stamp reference (`stamp_dab_rgba` / `paint_dabs_rgba`) matching GPU soft circular coverage
- Brush dynamics on `BrushParams`: opacity, flow, spacing_ratio, scatter, size/opacity pressure
- Deterministic scatter in `StrokeBuilder`; GPU stamp uses `stamp_alpha` (no double pressure on radius)
- Sharpen: `FilterEffect::sharpen`, Filter menu action, `cpu_sharpen_rgba` fixture

## Still open (DR-028)

Stroke journal/recovery hooks; texture tips; GPU sharpen pack path; filter gallery UX.
