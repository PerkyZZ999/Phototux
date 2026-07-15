# Decision Log

## Interactive grill — Round 1 (2026-07-15)

Owner-confirmed locks (this session):

| ID | Decision | Choice | Notes |
|----|----------|--------|-------|
| G1 | Platform | **A** Linux/Wayland v1 only | ADR-001 |
| G2 | UI toolkit | **A** Qt 6 QML | ADR-002 |
| G3 | FFI | **C** Hybrid qtbridge + canvas C++ allowed | ADR-003 |
| G4 | GPU API | **A** wgpu 30 Vulkan-first | ADR-004 |
| G5 | Zero-copy | **A** only + **mandatory pre–Phase 2 interop spike** | ADR-005, ADR-010 |

## Full log

| Date | ADR | Decision | Reversibility | Revisit Date |
|------|-----|----------|---------------|--------------|
| 2026-07-15 | ADR-001 | Linux / Wayland only for v1 | Hard | After Phase 5 or funded port |
| 2026-07-15 | ADR-002 | Qt 6 QML (Controls 2, dense desktop) | Hard | End Phase 1 / Qt 7 |
| 2026-07-15 | ADR-003 | qtbridge 0.2 primary; hybrid canvas C++/cxx-qt allowed | Medium | End Phase 1 & 2 |
| 2026-07-15 | ADR-004 | wgpu 30, Vulkan-first | Medium–Hard | Phase 2 interop checkpoint |
| 2026-07-15 | ADR-005 | Zero-copy GPU share only (no ship CPU path) | Hard | Spike report / 2 failed approaches |
| 2026-07-15 | ADR-006 | Multi-crate workspace (ui/engine/gpu/canvas) | Medium | End Phase 2 |
| 2026-07-15 | ADR-007 | Command queue threading model | Medium | Start Phase 4 |
| 2026-07-15 | ADR-008 | SPEC SLOs as phase exit gates | Easy (process) | Each phase exit |
| 2026-07-15 | ADR-009 | Layered testing + profiling | Easy | End Phase 3 |
| 2026-07-15 | ADR-010 | Mandatory interop spike before Phase 2 | Easy (process) | After spike report |
