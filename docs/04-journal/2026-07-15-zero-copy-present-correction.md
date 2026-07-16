# Zero-copy presentation correction

**Branch:** `feat/phase5-desktop`  
**Date:** 2026-07-15  
**Host:** Intel Arc B580 / Mesa 26.1.4 / Qt 6.11 / wgpu 30 / Vulkan

## Finding

Phase 5 preflight found that the canvas imported a raw `VkImage` only as a probe, deleted the `QRhiTexture` wrapper immediately, bound no sampled texture, and displayed a procedural tint. Phase 2 present and Phase 4 visual acceptance were reopened as blocker B2.

## Correction

- wgpu creates and owns the process-lifetime Vulkan instance, physical device, logical device, and graphics queue.
- `PhototuxCanvasItem` gives those borrowed handles to Qt Quick before scene-graph initialization with `QVulkanInstance::setVkInstance` and `QQuickGraphicsDevice::fromDeviceObjects`.
- The composite result explicitly transitions to `TextureUses::RESOURCE` / `VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL`.
- `PhototuxCanvasRenderer` retains the imported `QRhiTexture`, binds it with a sampler, and draws real document pixels.
- Qt frame submission and paint-worker wgpu submission share a native mutex from `beforeFrameBegin` through `afterFrameEnd`, satisfying Vulkan queue external synchronization.
- Repeated exports of the same image no longer rebuild the QRhi wrapper or pipeline.

## Additional defect found

The first real sampled paint run exposed missing `COPY_DST` usage on layer/undo textures. Stroke begin panicked during GPU backup. Texture usage flags were corrected and a clone/restore regression test added.

## Evidence

- Isolated KWin visual run: blue two-layer composite visible.
- Brush drag: dark stroke visibly sampled from the wgpu composite.
- Undo: stroke visibly removed.
- HUD during debug isolated run: 60–61 FPS, composite 0.49–1.06 ms.
- `cargo test -p phototux_gpu`: 3 passed, including clone/restore and 10×4K composite gate.
- No full-frame CPU upload or readback in the present path.

Release input-latency measurement remains part of Phase 5 final verification.
