# Phase 3 journal — Layer / composite engine

**Branch:** `feat/phase3-layer-composite`  
**Date:** 2026-07-15  
**Host:** Intel Arc B580, Mesa 26.1.4, Qt 6.11.1  

## Delivered

| Piece | Status |
|-------|--------|
| `DocumentGraph` + `Layer` + `BlendMode` | Done |
| `UndoStack` + gesture commands (ADR-013) | Done |
| `LayerCompositeEngine` single-pass WGSL blend | Done |
| 10×4K composite timing gate | **Pass** (host Instant best-of-10 &lt; 2.05 ms; release ~2.0 ms) |
| AppSession layer/undo APIs | Done |
| Layers panel (live) + Undo/Redo chrome | Done |
| Composite → canvas export handle | Done (import attempt; present still camera quad + status) |

## Blend modes (MVP)

`normal`, `multiply`, `screen`, `overlay` — packed as `u32` for WGSL.

## Composite path

1. Per-layer RGBA8 textures (seeded GPU clear colors).  
2. Pack into `texture_2d_array` when stack membership changes.  
3. One full-screen triangle pass blends bottom→top with opacity.  
4. Result texture VkImage handle published for canvas interop.

## Gate (ADR-008)

```text
cargo test -p phototux_gpu --release -- composite_10x4k
# 10×4K composite (best of 10) < 2.05 ms host-clock (target < 2.0 GPU)
```

## Undo policy

Structural only (add/delete/reorder/visibility/opacity/blend). Pixel history deferred to Phase 4.

## Run

```bash
export PATH=/usr/lib/qt6/bin:$PATH QMAKE=/usr/lib/qt6/bin/qmake QSG_RHI_BACKEND=vulkan
cargo run -p phototux
# New Document → Add layers → toggle eye / opacity → Undo/Redo
# Status: composite X.XX ms
```
