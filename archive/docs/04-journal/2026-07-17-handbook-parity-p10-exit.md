# Handbook Parity P10 exit — Accessibility projection

**Date:** 2026-07-17  
**Status:** Met (exit)

## Shipped

- Semantic tree: `accessibilityTreeJson` (toolbar, canvas, panels)
- AT-SPI host mapping: `phototux_engine::atspi_map` (`SemanticRole` → AT-SPI role names + states)
- Host property: `atspiProjectionJson` refreshed with the semantic tree

## Deferred (DR-028 / P13)

- Full custom AT-SPI D-Bus provider beyond Qt Accessible + mapping spine
- A11y evidence pack / contrast-focus-scale gates → P13
- Complete keyboard-only workflow parity

## Evidence

- Engine tests: role mapping for toolbar/canvas/panel
- `./scripts/check-rust.sh` green on exit commit

## Next

Ungated: **P13** budget fixture harness + Provisional ledger promotion.
