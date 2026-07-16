# Journal: Professional Raster Roadmap Slice (2026-07-16)

## Scope

Implement Phases 6–12 foundation from the professional raster roadmap: design-doc alignment, graph v2, `.ptx`, unified history, selections/clipboard/transforms, layers/masks/blends, creation tools, adjustments/filters, polish hooks, and PSD/format interchange.

## Delivered

- **Docs:** `DESIGN_BRIEF.md`, `INFORMATION_ARCHITECTURE.md` rewritten for professional raster; ADR-016 (`.ptx`), ADR-017 (graph v2/history), ADR-018 (PSD subset); ADR-015 amended; checklist Phases 6–12 marked.
- **Engine:** Typed `LayerKind` nodes, transforms/masks/locks/text/adjustments/effects; `HistoryService` timeline; selection/color/guides/brush presets/`CancelToken`.
- **I/O:** Versioned `.ptx` encode/decode + atomic save; autosave recovery journal; WebP/TIFF/BMP/GIF; PSD header import + compatibility report.
- **GPU:** Expanded blend modes; `SelectionMask` R8; filter/adjustment pass descriptors + CPU invert/brightness refs; layer RGBA readback for Save.
- **UI:** Save/Save As, open `.ptx`/PSD, History dock, expanded tools, select/copy/paste, groups/masks/adjustments/text actions, guides toggle, cancel I/O.

## Verification

`./scripts/check-rust.sh` green (fmt, clippy `-D warnings`, rust-doctor errors=0).

## Follow-ups

- Full GPU selection overlay / marching ants paint path
- Free-transform handle chrome + quality resampling
- Complete PSD channel decompression + layered export
- Tile/delta undo for large documents; multi-doc only after ADR-013 amendment
