# Changelog

All notable decision milestones and project state changes.

## [decisions-locked-v1] — 2026-07-15

### Decisions Locked

| ADR | Decision | Reversibility | Revisit Date |
|-----|----------|---------------|--------------|
| ADR-001 | Linux / Wayland v1 only | Hard | After Phase 5 |
| ADR-002 | Qt 6 QML shell | Hard | End Phase 1 / Qt 7 |
| ADR-003 | qtbridge primary + hybrid canvas | Medium | End Phase 1–2 |
| ADR-004 | wgpu 30 Vulkan-first | Medium–Hard | Phase 2 interop |
| ADR-005 | Zero-copy compositing only | Hard | Phase 2 present path |
| ADR-006 | Multi-crate Cargo workspace | Medium | End Phase 2 |
| ADR-007 | Command queue threading model | Medium | Phase 4 |
| ADR-008 | SLOs as acceptance gates | Easy | Each phase exit |
| ADR-009 | Layered testing + profiling | Easy | End Phase 3 |

### Constraints at Lock

| Constraint | Status | Notes |
|------------|--------|-------|
| Linux / Wayland | Satisfied | ADR-001 |
| Rust + Qt 6 QML | Satisfied | ADR-002, ADR-003 |
| Zero-copy GPU canvas | Satisfied (design) | ADR-005; runtime unvalidated (spike skipped) |
| Performance SLOs | Satisfied (gates) | ADR-008 |
| qtbridge preferred | Satisfied | ADR-003 with hybrid escape |

### Validated Assumptions

| Assumption | Spike Branch | Result |
|------------|--------------|--------|
| qtbridge builds on host Qt 6.11 / Rust 1.95 | *(spike skipped)* | **Unvalidated in-repo** — host packages present |
| Zero-copy wgpu ↔ Qt RHI | *(spike skipped)* | **Unvalidated** — Phase 2 owns risk |

### Success Criteria Baseline

| Criterion | Target | Current Status |
|-----------|--------|----------------|
| Steady-state FPS | ≥ 60 | Not started |
| Input latency | < 8 ms | Not started |
| Cold boot | < 250 ms | Not started |
| 10×4K composite | < 2 ms GPU | Not started |
| Zero-copy hot path | No full-frame CPU upload | Design locked |

### Known Risks at Lock

1. **qtbridge 0.2 beta API churn**
2. **Custom QQuickItem / RHI import may require C++** (hybrid ADR-003)
3. **Spike skipped** — interop is highest technical risk
4. **RefCell re-entrancy panics** if command boundaries sloppy

### Next Milestone

`agent-bootstrap` → `AGENTS.md` + development checklists → **Phase 1** Cargo/qtbridge bootstrap + QML skeleton.

---

## [grill-round-1-owner-lock] — 2026-07-15

### Owner-confirmed (interactive grill)

| ID | Lock |
|----|------|
| G1 | Linux/Wayland v1 only (ADR-001) |
| G2 | Qt 6 QML (ADR-002) |
| G3 | Hybrid FFI: qtbridge + canvas C++ allowed (ADR-003) |
| G4 | wgpu 30 Vulkan-first (ADR-004) |
| G5 | Zero-copy only + **mandatory interop spike before Phase 2** (ADR-005, **ADR-010**) |

### Process change

Earlier “spike skipped” is **partially reversed**: interop spike is required before Phase 2 production canvas code.

---
