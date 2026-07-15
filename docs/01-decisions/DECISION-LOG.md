# Decision Log

## Interactive grill — Round 1 (2026-07-15)

| ID | Decision | Choice | ADR |
|----|----------|--------|-----|
| G1 | Platform | **A** Linux/Wayland v1 only | ADR-001 |
| G2 | UI toolkit | **A** Qt 6 QML | ADR-002 |
| G3 | FFI | **C** Hybrid qtbridge + canvas C++ | ADR-003 |
| G4 | GPU API | **A** wgpu 30 Vulkan-first | ADR-004 |
| G5 | Zero-copy | **A** + mandatory interop spike | ADR-005, ADR-010 |

## Interactive grill — Round 2 (2026-07-15)

| ID | Decision | Choice | ADR |
|----|----------|--------|-----|
| G6 | Workspace crates | **A** multi-crate, strict naming | ADR-006 |
| G7 | Threading | **B** command queue / worker phased | ADR-007 |
| G8 | QML chrome | **C** Controls 2 first; Kirigami deferred | ADR-002 |
| G9 | SLOs | **A** hard phase gates; ≥60 FPS fluid UX | ADR-008 |
| G10 | Testing | **A** layered tests + HUD/tracing | ADR-009 |
| G11 | Document model | **A** full graph in Phase 3 only | ADR-011 |
| G12 | License | **A** GPL-3.0-or-later; public OSS late | ADR-012 |

## Interactive grill — Round 3 (2026-07-15)

| ID | Decision | Choice | ADR |
|----|----------|--------|-----|
| G13 | New document size | **C** ask + presets 720p/1080p/2K/4K | ADR-013 |
| G14 | Multi-document | **A** single document v1 | ADR-013 |
| G15 | Icons | **B** bundled FOSS pack in `assets/` | ADR-013 |
| G16 | Undo granularity | **A** one committed gesture = one undo | ADR-013 |
| G17 | CI | **A** local Arch only for now | ADR-013 |
| G18 | Zoom on open/new | **A** zoom to fit | ADR-013 |

## Full ADR table

| Date | ADR | Decision | Reversibility | Revisit Date |
|------|-----|----------|---------------|--------------|
| 2026-07-15 | ADR-001 | Linux / Wayland only for v1 | Hard | After Phase 5 or funded port |
| 2026-07-15 | ADR-002 | Qt 6 QML; Controls 2; Kirigami deferred | Hard | End Phase 1 / Qt 7 |
| 2026-07-15 | ADR-003 | qtbridge primary; hybrid canvas C++ | Medium | End Phase 1 & 2 |
| 2026-07-15 | ADR-004 | wgpu 30, Vulkan-first | Medium–Hard | Phase 2 interop |
| 2026-07-15 | ADR-005 | Zero-copy only (ship) | Hard | Spike report |
| 2026-07-15 | ADR-006 | Multi-crate `phototux_*` layout | Medium | End Phase 2 |
| 2026-07-15 | ADR-007 | Command queue threading | Medium | Phase 4 |
| 2026-07-15 | ADR-008 | SLOs as phase gates (≥60 FPS) | Easy | Each phase exit |
| 2026-07-15 | ADR-009 | Layered testing + profiling | Easy | End Phase 3 |
| 2026-07-15 | ADR-010 | Interop spike before Phase 2 | Easy | After spike report |
| 2026-07-15 | ADR-011 | Layer graph timing = Phase 3 | Medium | Start Phase 3 |
| 2026-07-15 | ADR-012 | GPL-3.0-or-later; OSS publish late | Hard after publish | First public release |
| 2026-07-15 | ADR-013 | Product prefs G13–G18 (new doc, single doc, icons, undo, CI, zoom) | Easy–Medium | Per-topic revisit in ADR-013 |
| 2026-07-15 | ADR-014 | Desktop GUI only — no CLI/TUI product (v1) | Hard for v1 | Post–Phase 5 if batch tool demanded |
