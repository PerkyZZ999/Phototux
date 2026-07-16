# Phase 4 journal — Tools & brush

**Branch:** `feat/phase4-tools-brush`  
**Date:** 2026-07-15  
**Host:** Intel Arc B580 / Wayland / Qt 6.11  

## Delivered

| Piece | Status |
|-------|--------|
| Brush + eraser tools | Done |
| Circular dab stamps (WGSL) on active layer | Done |
| Spacing interpolator (`StrokeBuilder`) | Done |
| Paint worker thread + `EngineCommand` queue (ADR-007) | Done |
| Stroke undo/redo (layer texture backup) | Done |
| Hardness + RGB color properties | Done |
| Mouse paint path; pressure field when present | Done |
| Latency + FPS HUD hooks | Done |

## Architecture

UI enqueues `BeginStroke` / `StrokePoint` / `EndStroke` only.  
Worker stamps into layer textures, coalesces composite (~every 4 dabs), publishes result handle.  
AppSession polls worker events from `FrameAnimation`.

## Gates

| Gate | Approach |
|------|----------|
| ≥60 FPS while brushing | FrameAnimation EMA (existing); paint off UI thread |
| Input→render &lt; 8 ms | `strokeLatencyMs` from first dab submit+poll |
| Worker path | `phototux-paint` thread + mpsc |

Measure on host while scribbling; document observed values in follow-up if needed.

## Run

```bash
export PATH=/usr/lib/qt6/bin:$PATH QMAKE=/usr/lib/qt6/bin/qmake QSG_RHI_BACKEND=vulkan
cargo run -p phototux
# New Document → Brush → drag on canvas
# Eraser → Undo stroke
```

## Notes

- Present path still uses RHI document quad + import attempt; GPU paint/composite is authoritative for document pixels.
- Full texture sampling of composite into RHI remains follow-up polish (Phase 2/3 carry).
