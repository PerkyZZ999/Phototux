# Handbook Parity P7 exit — History & lifecycle

**Date:** 2026-07-17  
**Status:** Met (exit)

## Shipped

- Unified history timeline + panel jump (prior)
- Recovery UX + restore/discard chooser (prior)
- Safe-start: `safe_start_next` + `PHOTOTUX_SAFE_START=1` (prefs essentials chrome)
- Retention budget UI: `Preferences::history_retention_limit` (8–512), prefs SpinBox, `HistoryService::set_limit`
- GPU recover / `renderer_generation` (prior / P6)

## Deferred / gated

- Spill-to-disk history → P11 (DR-029)
- Multi-window / multi-doc → P11
- Formal lifecycle controller + fuller coalescing suite → DR-028 depth

## Evidence

- Prefs unit: retention clamp
- Engine: `HistoryService::set_limit` + budget harness B9 soft gate
- `./scripts/check-rust.sh` green on exit commit

## Next

Ungated: **DR-028 depth** slices; **DR-017** device-tier present/boot evidence.
