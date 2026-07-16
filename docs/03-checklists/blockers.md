# Current Blockers

## Active

| # | Blocker | Raised By | Date | Impact | Resolution Plan |
|---|---------|-----------|------|--------|-----------------|
| B3 | Best sampled release cold boot reaches first interactive frame in 653.71 ms, above ADR-008's 250 ms Phase 5 target | Agent | 2026-07-15 | Blocks Phase 5 exit | Package QML as Qt resources with `qmlcachegen` AOT units, profile remaining Qt Quick first-frame work, then rerun the release measurement |

## Resolved

| # | Blocker | Date Raised | Date Resolved | Resolution |
|---|---------|-------------|---------------|------------|
| B2 | Canvas presented a procedural RHI tint, not the wgpu document composite | 2026-07-15 | 2026-07-15 | **Closed** — wgpu owns Vulkan instance/device/queue; Qt Quick adopts them, samples retained `VkImage` through `QRhiTexture`, and frame/worker queue use is mutex-serialized |
| B0 | Spike validation skipped by request | Owner | 2026-07-15 | **Closed** — spike + Phase 2 interop attempt done |
| B1 | Early code scaffold outpaced design docs | Owner | 2026-07-15 | Scaffold removed; DESIGN_BRIEF + IA + DESIGN.md added; re-scaffold after review |
