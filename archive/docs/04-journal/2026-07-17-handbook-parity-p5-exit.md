# Handbook Parity P5 exit — Creative engines depth

**Date:** 2026-07-17  
**Status:** Met (exit)

## Shipped

- Filter gallery: `app.show-filter-gallery` + QML dialog; `filter.preview` / `filter.set-preview-params` / `filter.commit` / `filter.cancel-preview`
- Preview session on `SessionState` (ephemeral GPU overlay via host recomposite); commit writes `Layer.effects` + mirrors `FilterPlanNode`
- Cancel / stale policy: `CancelToken` + document generation snapshot; reject commit when cancelled or generation moved; invalidate on tool/layer/document change
- Path edit: `tool.path-edit`; `path.set-closed` / `path.move-anchor` / `path.add-anchor` / `path.delete-anchor` (shape path when active Shape, else document `PathDocument`); host re-rasterize upload
- Text: `TextContent.frame_w` / `frame_h` / `wrap` (serde defaults); bake word-wrap; Character panel frame/wrap + bake-vs-keep copy + bake announce

## Deferred (explicit, DR-028)

- Texture brush tips / full pressure curves
- GPU noise / full handbook 15 filter catalog
- On-canvas text caret + font host discovery/shaping
- Vector-preserving boolean; live vector present without raster upload
- Shape gradients / new primitives beyond rect/ellipse/line
- Tile-aware stroke planner (P11)

## Evidence

- Engine tests: preview non-dirty; cancel; stale/cancelled commit reject; path edit undo round-trip; text wrap bake + serde defaults
- `./scripts/check-rust.sh` green on exit commit
- Checklist / Roadmap / Command-Taxonomy updated

## Next

Ungated: **P6** color/render depth (see Roadmap §7).
