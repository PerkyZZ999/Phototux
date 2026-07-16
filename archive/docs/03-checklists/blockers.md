# Current Blockers

## Active

None.

## Resolved

| # | Blocker | Date Raised | Date Resolved | Resolution |
|---|---------|-------------|---------------|------------|
| B3 | Release cold boot exceeded the original 250 ms target | 2026-07-15 | 2026-07-15 | **Closed** — QML and startup icons are embedded in a statically linked Qt AOT module; the optimized 10-run release median is 685.94 ms (648.17 ms best). ADR-008 now gates Phase 5 at <1,000 ms while retaining <250 ms as a stretch target |
| B2 | Canvas presented a procedural RHI tint, not the wgpu document composite | 2026-07-15 | 2026-07-15 | **Closed** — wgpu owns Vulkan instance/device/queue; Qt Quick adopts them, samples retained `VkImage` through `QRhiTexture`, and frame/worker queue use is mutex-serialized |
| B0 | Spike validation skipped by request | Owner | 2026-07-15 | **Closed** — spike + Phase 2 interop attempt done |
| B1 | Early code scaffold outpaced design docs | Owner | 2026-07-15 | Scaffold removed; DESIGN_BRIEF + IA + DESIGN.md added; re-scaffold after review |
