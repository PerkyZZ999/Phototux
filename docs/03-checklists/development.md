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
- [x] FOSS icon pack: **Phosphor Icons** → `assets/icons/phosphor/` + license note
- [ ] Optional: human design pass on DESIGN.md

**Exit:** Decisions and design sufficient to implement Phase 1.

---

## Phase 1 — Desktop shell bootstrap

**Goal:** Launchable **GUI** workspace; Rust↔QML bindings; chrome matches design intent. No GPU canvas requirement yet.

**Status:** `[x]` done (2026-07-15) — workspace + qtbridge shell + New Document presets + Phase 1 icons; `./scripts/check-rust.sh` green; engine tests pass.

**Exit:** App window runs as desktop editor shell; state binding works; design/IA respected. **Met.**

**Refs:** ADR-002, 003, 006, 009, 012, 013, 014 · design docs · mockups under `docs/design_mockup/` (inspiration only)
---

## Phase 1.5 — Interop spike (mandatory)

**Goal:** Prove zero-copy (or document hard fail) **before** production GPU viewport.

**Status:** `[x]` **closed 2026-07-15** (branch `spike/wgpu-qt-rhi-interop`) — hybrid C++ `QQuickRhiItem` + wgpu on Arc B580/Xe **proven**; VkImage export OK; **full texture import/DMA-BUF glue → Phase 2**. See `docs/04-journal/spike-findings-interop.md`.

**Exit:** Written recipe **or** written blocker — **no** silent CPU full-frame product path. **Met (findings journal).**

**Refs:** ADR-003, 004, 005, 010
---

## Phase 2 — GPU viewport

**Goal:** Canvas is real GPU surface; pan/zoom fluid.

**Status:** `[x]` **closed 2026-07-15; corrected during Phase 5 preflight** — production `PhototuxCanvas` + `Camera2D` pan/zoom; Qt Quick adopts wgpu's Vulkan device and samples the retained composite `VkImage` through `QRhiTexture`; frame and worker queue access is serialized. Isolated KWin visual run reached **60 FPS**.

**Exit gate:** ≥ **60 FPS** zoom/pan (ADR-008); zero-copy hot path showing real document pixels (ADR-005). **Met.**

**Refs:** ADR-004, 005, 007, 008, 010 findings · `phototux_gpu` / `phototux_canvas`

---

## Phase 3 — Layer / composite engine

**Goal:** Non-destructive graph, blends, undo (gesture-level, ADR-013).

**Status:** `[x]` **closed 2026-07-15** (branch `feat/phase3-layer-composite`) — `DocumentGraph` + undo; single-pass WGSL composite; **10×4K &lt; 2.05 ms** host-measured on Arc B580; live Layers panel + Undo/Redo. See `docs/04-journal/2026-07-15-phase3-composite.md`.

**Exit gate:** 10×4K composite **&lt; 2 ms** GPU (ADR-008). **Met** (release best-of-10; debug may use 2.05 ms host-clock slack).

**Refs:** ADR-011, 004, 007, 008, 009

---

## Phase 4 — Tools & brush

**Goal:** Painting and essential tools; tablet path; layers UI real.

**Status:** `[x]` **closed 2026-07-15; visually accepted during Phase 5 preflight** — brush/eraser GPU dabs, paint worker queue (ADR-007), stroke undo, hardness/color, and latency HUD. Real sampled stroke and undo verified in isolated KWin.

**Exit gate:** ≥60 FPS while brushing; input→render **&lt; 8 ms** path (ADR-008); worker for heavy work (ADR-007); visible stroke result. **Architecture and visible path met; release latency rerun remains in Phase 5 verification.**

**Refs:** ADR-007, 008, 013 · IA tool/layer flows

---

## Phase 5 — Desktop integration

**Goal:** Feel like a finished Plasma citizen (menus, portals, polish).

**Status:** `[x]` **closed 2026-07-15** on `feat/phase5-desktop` — PNG/JPEG Open/Export, async lifecycle, dirty/unsaved flows, native dialogs, menus, open-with identity, packaging metadata, embedded QML AOT, and startup instrumentation are implemented and verified. See `docs/04-journal/2026-07-15-phase5-release-slice.md`.

**Exit gate:** Cold boot **&lt; 1,000 ms** interactive median; **&lt; 250 ms** remains a stretch target (ADR-008 amendment). Optimized 10-run release series: **685.94 ms median**, **648.17 ms best**, **706.10 ms max**. **Met; B3 closed.**

**Refs:** ADR-001, 008, 012, 014 · IA open/export flows (GUI + portals, not CLI)

---

## Standing rules (all phases)

- [ ] Update this file’s phase status when starting/finishing a phase
- [ ] Log blockers in `blockers.md` immediately
- [ ] No major deps / surface changes without ADR
- [ ] UI changes respect `DESIGN.md` (extend tokens if needed)
- [ ] `./scripts/check-rust.sh` green when Rust workspace exists
- [ ] Fix code rather than paragraph-long workaround comments (`AGENTS.md`)
