# Journal: Panels — Swatches, Navigator, Properties (2026-07-16)

## What shipped

- `AppSession`: FG/BG hex, recent colors, `activeBlend`, `setPan` / `centerViewOn`, foreground/background slots; brush RGB stays synced to FG.
- Swatches dock: FG/BG chips, swap, HEX field, default palette + recent picks.
- Geometric Navigator: doc frame + viewport rect; click/drag pans (no GPU thumbnail; ADR-005).
- Properties: blend ComboBox; FG RGB sliders; View Fit/100% (zoom slider removed).

## Deferred

GPU Navigator thumbnail, History jump, layer thumbs, HSV wheel, palette files, Window→docks.
