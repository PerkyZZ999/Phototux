# Journal — DR-028 depth pass + DR-017 present-path budgets

**Date:** 2026-07-17  
**Commits:** `83571b8` (A1/A2), `7b1752d` (A3), `52670f7` (A4–A8), docs/B follow.

## DR-028 depth spines closed

| Spine | Shipping evidence |
| --- | --- |
| A1 Brush texture | `BrushTextureKind` + CPU/GPU stamp + Properties strength |
| A2 Filters | `FilterParams::Noise` + `AdjustmentParams::Exposure` |
| A3 Text | `fc-list` fonts + on-canvas `TextEdit` |
| A4 Display ICC | colord/env/xdg/`sRGB` + soft-proof “Use display profile” |
| A5 Shape | polygon/gradient/`live_vector` + `ShapeBooleanPartner` |
| A6 Mask refine | `contrast`/`shift` on `LayerMask` + composite shader |
| A7 Dirty region | `dirty_rect` + overlay view generation; grid clip |
| A8 A11y | AT-SPI evidence fixture + Accessible tool strip/canvas |

**Residual `[P]`:** lcms2 transform engine; custom AT-SPI D-Bus server; GPU-resident live vectors @ 60 Hz; full brush curves / filter chapter depth.

## DR-017

Present-path soft proxies added to `budget_harness` (B1 dirty-mark, B2 nav intervals, B3 warm construct). Ledger + DR-017 amended with Tier M synthetic 4K evidence; photon/GPU present remains Provisional when CI has no display.

## Out of scope (still gated)

P11 tiling/spill/sparse; P12 plugin ABI.
