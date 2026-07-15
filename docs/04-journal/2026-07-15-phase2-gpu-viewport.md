# Phase 2 journal — GPU viewport

**Branch:** `feat/phase2-gpu-viewport`  
**Date:** 2026-07-15  
**Host:** Intel Arc B580 (BMG G21), Mesa 26.1.4, Qt 6.11.1, Wayland / Plasma  

## Delivered

| Piece | Status |
|-------|--------|
| `Camera2D` + `FpsTracker` in `phototux_engine` | Done |
| `AppSession` pan/zoom/FPS slots (`ConvertToCamelCase`) | Done |
| Production `phototux_canvas` `QQuickRhiItem` (`PhototuxCanvas`) | Done |
| Wire into `Main.qml` + pan/zoom input + FPS HUD | Done |
| GPU present path (letterbox + camera-transformed document quad, GPU shaders) | Done |
| wgpu probe + VkImage export + `QRhiTexture::createFrom` attempt | Done |

## FPS gate (ADR-008)

| Metric | Result |
|--------|--------|
| Steady-state FPS (FrameAnimation EMA) | **63 FPS** on host Plasma (AT-SPI label `FPS: 63`) |
| ≥ 60 FPS zoom/pan target | **Met** |

Present path is continuous GPU RHI (no full-frame CPU `QImage` upload).

## Interop attempt (ADR-005 / ADR-010 carry)

1. **wgpu** creates Vulkan device on Arc B580; GPU-clear texture; `as_hal` **VkImage export OK**.  
2. Handle published to C++ via `phototux_canvas_set_wgpu_export`.  
3. Renderer calls `QRhiTexture::createFrom(NativeTexture{object=VkImage})`.  
4. Log line observed:  
   `wgpu import: createFrom(VkImage) OK — zero-copy path available`  

**Caveat:** Present content for Phase 2 is still the **GPU RHI procedural document quad** (camera pan/zoom). Sampling the imported texture into the document fill is deferred (wire as Phase 3 content pipeline). `createFrom` succeeding is a stronger result than the spike (which stopped at export only); end-to-end **sampled** zero-copy for document pixels is not yet the product present path.

**Not used:** CPU full-frame upload (forbidden).

## Shell / QA notes

- Fixed QML path: `crates/phototux` → `../../qml` (was `../../../`).  
- Renamed `onSurface` → `colorOnSurface` (QML treats `on*` as signal handlers).  
- Enabled `#[qobject(Singleton, ConvertToCamelCase)]` so slots match QML (`setViewportSize`, `panBy`, …).  
- Manual/agent check: New Document 1080p → status `1920×1080 · zoom 53% · pan (960,540)`; FPS 63.

## Run

```bash
export PATH=/usr/lib/qt6/bin:$PATH
export QMAKE=/usr/lib/qt6/bin/qmake
export QSG_RHI_BACKEND=vulkan
cargo run -p phototux
```

## Next (Phase 3)

- Layer graph + composite into wgpu textures.  
- Sample imported / shared texture in `PhototuxCanvas` fragment path.  
- Prefer DMA-BUF if `createFrom` proves invalid for actual sampling across VkDevices.
