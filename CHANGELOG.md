# Changelog

All notable decision milestones and project state changes.

## [engineering-handbook] — 2026-07-16

### Docs

- Adopted `internal_docs/` as the authoritative Engineering Handbook.
- Archived former `/docs/` → `archive/docs/` (historical ADRs, journals, checklists).
- Added `internal_docs/Appendix/Codebase-Handbook-Gap-Analysis.md` (code vs handbook diffs + hybrid alignment plan).
- Pointed `README.md` / `AGENTS.md` at handbook; root `SPEC.md` / `CONSTRAINTS.md` remain bridge docs.

### Alignment stance

Keep shipping Qt + wgpu zero-copy + `.ptx` spine; evolve toward handbook command/snapshot/workspace contracts. Do not big-bang rewrite to the proposed fine-grained crate layout.

---

## [ia-parity-roadmap] — 2026-07-16

### Docs

- Merged owner `PREFERED_IA.md` into normative `INFORMATION_ARCHITECTURE.md` with Current / Planned / Blocked / Deferred tags (codebase = shipped truth).
- Retargeted `docs/03-checklists/development.md` production slices for full IA parity.
- Synced `DESIGN_BRIEF.md`, `FEATURES_TODO.md`, `AGENTS.md`, `README.md`; logged ADR tensions (multi-doc, Shape kind, plugins) in `conflicts.md`.

### Still gated

| Item | Gate |
|------|------|
| Document tabs / multi-doc | ADR-013 amendment |
| Shape layers | ADR-017 kind amendment |
| Plugin / script product surface | New ADR |

---

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

## [grill-round-2-owner-lock] — 2026-07-15

| ID | Lock |
|----|------|
| G6 | Multi-crate workspace, strict `phototux_*` naming (ADR-006) |
| G7 | Command queue / phased worker (ADR-007) |
| G8 | Controls 2 first; Kirigami deferred (ADR-002) |
| G9 | SLOs hard gates; ≥60 FPS fluid UX (ADR-008) |
| G10 | Layered testing + HUD/tracing (ADR-009) |
| G11 | Full document graph in Phase 3 only (ADR-011) |
| G12 | GPL-3.0-or-later; public OSS late (ADR-012) |

**Grill status:** Rounds 1–2 complete. Core stack + process locked. Optional Round 3 only for secondary product prefs.

---

## [grill-round-3-owner-lock] — 2026-07-15

| ID | Lock |
|----|------|
| G13 | New doc: ask every time + presets 720p / 1080p / 2K / 4K |
| G14 | Single document only (v1) |
| G15 | Bundled FOSS icon pack under `assets/` (owner supplies pack) |
| G16 | Undo = one committed action/gesture per step |
| G17 | CI: local Arch/CachyOS only for now |
| G18 | Zoom-to-fit on open/new |

**Grill status:** Rounds 1–3 complete. Architecture + product prefs locked (ADR-001…013).

---

## [doc-review-and-desktop-surface] — 2026-07-15

- Doc alignment review: `docs/04-journal/2026-07-15-doc-review.md`
- **ADR-014:** MVP/v1 = **desktop GUI only** (no CLI/TUI product)
- Fixed IA F1 vs New Document presets; SPEC verify path; checklist phase-level rewrite
- CONSTRAINTS hard list updated

---
