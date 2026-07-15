# Spike findings: wgpu–Qt RHI interop

**Branch:** `spike/wgpu-qt-rhi-interop`  
**Date:** 2026-07-15  
**ADR:** 010 / 003 / 004 / 005  

## Host

| Item | Value |
|------|--------|
| GPU | **Intel Arc B580 (BMG G21), 12 GB** — discrete |
| Vulkan | API 1.4.x, `Intel open-source Mesa driver` |
| Mesa / vulkan-intel | **26.1.4-arch3.1** |
| Qt | **6.11.1** |
| RHI backend | `QSG_RHI_BACKEND=vulkan` (forced in spike) |
| OS | CachyOS / Wayland / KDE Plasma |
| wgpu | **30.0.0** (Vulkan backend) |

## What we built

| Piece | Role |
|-------|------|
| `crates/phototux-gpu` | wgpu Vulkan device + GPU clear to texture + `as_hal` VkImage export |
| `crates/phototux-spike-interop` | Tiny Qt/QML window + **C++ `QQuickRhiItem`** (`SpikeCanvas`) GPU clear animation |
| Unit test | `phototux_gpu` creates device+texture on Arc |

## Attempt 1 — export GPU image / shared path

| Step | Result |
|------|--------|
| wgpu Vulkan adapter on Arc B580 | **OK** — adapter name includes Arc B580; backend=Vulkan |
| GPU clear render pass to texture (no CPU pixel loop) | **OK** |
| `Texture::as_hal::<Vulkan>()` → raw **VkImage** handle | **OK** — non-null handle logged |
| Import that VkImage into Qt RHI as the `SpikeCanvas` color buffer | **Not completed** in this time-box |

**Qt side (hybrid):** `QQuickRhiItem` + `QRhiCommandBuffer::beginPass` clear is **GPU-only present** into QML (no `QImage` upload). Window verified running via KWin (`phototux-spike-interop`, title “PhotoTux Spike — wgpu / Qt RHI interop”).

So Attempt 1 is **partial**: export from wgpu works; **import into the same Quick item** is the remaining glue.

## Attempt 2 — DMA-BUF

| Step | Result |
|------|--------|
| DMA-BUF export/import end-to-end | **Not implemented** this session |
| Rationale | Focus spent on hybrid C++ item + wgpu probe + handle export; DMA-BUF is the **recommended next** path on **Intel + Wayland + Xe** |

## Result

**Outcome: B-progress (hybrid proven; full zero-copy glue open)**

| Criteria | Status |
|----------|--------|
| Hybrid C++ canvas item in QML | **Pass** |
| wgpu on Arc B580 / Xe / Vulkan | **Pass** |
| VkImage handle export via wgpu-hal | **Pass** |
| Live **imported** wgpu texture in Qt item (zero-copy end-to-end) | **Open** → Phase 2 |
| CPU full-frame upload as success path | **Not used** (forbidden for product) |

Not **C** (total fail) — core building blocks work on this host.  
Not full **B pass** until import path samples the wgpu texture.

## Recipe (current)

### Run spike

```bash
export PATH=/usr/lib/qt6/bin:$PATH
export QMAKE=/usr/lib/qt6/bin/qmake
export QSG_RHI_BACKEND=vulkan
cargo run -p phototux_spike_interop
```

### Proven pieces for Phase 2

1. **`phototux_gpu`:** `GpuContext::new()` + `create_cleared_texture` + optional `texture_vk_image_handle`.  
2. **`SpikeCanvas` (C++):** `QQuickRhiItem` registered as `PhototuxSpike 1.0 / SpikeCanvas`; GPU clear each frame.  
3. **Hybrid expectation confirmed:** custom canvas item is **C++**, not pure qtbridge macros (ADR-003).  
4. **Prefer texture import / DMA-BUF** over shared `VkDevice` for Phase 2 (plan recommendation).

### Phase 2 should

1. Move `SpikeCanvas` → production `phototux_canvas` (clean, not spike-ugly).  
2. Implement **Attempt 1 complete**: import exported handle **or** (preferred on Arc/Wayland) **DMA-BUF FD** into `QRhiTexture` and sample in `SpikeCanvasRenderer::render`.  
3. May need thin **ash** + external memory flags on allocation if wgpu public API cannot mark textures exportable.  
4. Keep product path free of full-frame CPU upload (ADR-005).  
5. Then pan/zoom + ≥60 FPS gate on the real shell.

### Do not

- Treat Qt RHI clear alone as “zero-copy from wgpu.”  
- Merge spike C++ into `main` as-is without cleanup.  
- Ship CPU staging as the default present path.

## qtbridge vs C++

| Question | Answer |
|----------|--------|
| qtbridge alone for canvas? | **No for RHI item** — hybrid C++ `QQuickRhiItem` required (expected Outcome B) |
| Shared VkDevice first? | **No** — continue with **import/DMA-BUF** first |

## Manual / agent UI check

- Process ran: `./target/debug/phototux-spike-interop`  
- KWin listed window: `app_id=phototux-spike-interop`, title contains “Spike”  
- Log line confirms Arc B580 + VkImage export  

## Checklist

- [x] Branch `spike/wgpu-qt-rhi-interop`  
- [x] wgpu + hybrid canvas spike builds  
- [x] Host Arc/Xe documented  
- [x] This findings file  
- [ ] Full DMA-BUF / import (carry to Phase 2)  
