# Handbook Parity P4 exit — Masks & layer semantics

**Date:** 2026-07-17  
**Status:** Met (exit)

## Shipped

- Atomic multi-select `layer.delete` / `layer.reorder` / `layer.group` + `layer.ungroup`; Ctrl/Shift Layers panel selection; batch undo
- `LayerKind::Fill` + `layer.create-fill` / `layer.set-fill-color`; GPU solid upload; Properties color
- `effect.reorder` / `effect.set-enabled` + Properties effect list
- Clip break-on-delete-base; Layers ↳ tooltip polish
- `mask.apply` host bake (density/invert); Properties density/feather/invert/link; GPU density equation
- Layer styles: OuterGlow + ColorOverlay (CPU + GPU pack)

## Deferred (explicit)

- Vector mask path edit
- Refine contrast / edge shift

## Evidence

- Engine tests: multi-delete, group selection, fill create, effect reorder, clip-base delete
- `./scripts/check-rust.sh` green on exit commit
- Checklist / Roadmap / Command-Taxonomy updated

## Next

Ungated: P5 filter gallery / path edit / text depth (see Roadmap §7).
