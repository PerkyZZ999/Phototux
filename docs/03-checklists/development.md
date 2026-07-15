# Development Checklist

Living, **phase-level** tracker. Detailed implementation plans are written **at the start of each phase**, not here.

Legend: `[ ]` todo · `[~]` in progress · `[x]` done · `[!]` blocked

**Product surface (ADR-014):** desktop GUI only for MVP/v1 — **no** CLI product, **no** TUI, **no** web shell. `cargo` commands are developer tooling only.

**Governing docs:** ADRs `docs/01-decisions/`, `docs/DESIGN.md`, IA, `AGENTS.md`, `SPEC.md`.

---

## Phase 0 — Foundation (docs & decisions)

- [x] Inception, constraints, research dossier
- [x] Design brief, IA, DESIGN.md tokens
- [x] Grill R1–R3 + ADR-001…013; desktop surface ADR-014
- [x] AGENTS.md + pre-commit (fmt / clippy / rust-doctor)
- [x] Doc alignment review (2026-07-15)
- [ ] Owner: FOSS icon pack → `assets/icons/` + license note (when ready)
- [ ] Optional: human design pass on DESIGN.md

**Exit:** Decisions and design sufficient to implement Phase 1.

---

## Phase 1 — Desktop shell bootstrap

**Goal:** Launchable **GUI** workspace; Rust↔QML bindings; chrome matches design intent. No GPU canvas requirement yet.

**Covers (plan in detail when starting):** workspace crates (ADR-006), qtbridge UI (ADR-003), QML shell per IA + DESIGN.md, New Document presets (ADR-013), quality hooks green.

**Exit:** App window runs as desktop editor shell; state binding works; design/IA respected.

**Refs:** ADR-002, 003, 006, 009, 012, 013, 014 · design docs

---

## Phase 1.5 — Interop spike (mandatory)

**Goal:** Prove zero-copy (or document hard fail) **before** production GPU viewport.

**Covers:** Time-boxed throwaway spike (ADR-010); findings journal; ADR amendments only if needed.

**Exit:** Written recipe **or** written blocker — **no** silent CPU full-frame product path.

**Refs:** ADR-003, 004, 005, 010

---

## Phase 2 — GPU viewport

**Goal:** Canvas is real GPU surface; pan/zoom fluid.

**Exit gate:** ≥ **60 FPS** zoom/pan (ADR-008); zero-copy hot path (ADR-005).

**Refs:** ADR-004, 005, 007, 008, 010 findings · `phototux_gpu` / `phototux_canvas`

---

## Phase 3 — Layer / composite engine

**Goal:** Non-destructive graph, blends, undo (gesture-level, ADR-013).

**Exit gate:** 10×4K composite **&lt; 2 ms** GPU (ADR-008).

**Refs:** ADR-011, 004, 007, 008, 009

---

## Phase 4 — Tools & brush

**Goal:** Painting and essential tools; tablet path; layers UI real.

**Exit gate:** ≥60 FPS while brushing; input→render **&lt; 8 ms** path (ADR-008); worker for heavy work (ADR-007).

**Refs:** ADR-007, 008, 013 · IA tool/layer flows

---

## Phase 5 — Desktop integration

**Goal:** Feel like a finished Plasma citizen (menus, portals, polish).

**Exit gate:** Cold boot target **&lt; 250 ms** interactive (stretch; document if missed). Packaging notes OK deferred.

**Refs:** ADR-001, 008, 012, 014 · IA open/export flows (GUI + portals, not CLI)

---

## Standing rules (all phases)

- [ ] Update this file’s phase status when starting/finishing a phase
- [ ] Log blockers in `blockers.md` immediately
- [ ] No major deps / surface changes without ADR
- [ ] UI changes respect `DESIGN.md` (extend tokens if needed)
- [ ] `./scripts/check-rust.sh` green when Rust workspace exists
- [ ] Fix code rather than paragraph-long workaround comments (`AGENTS.md`)
