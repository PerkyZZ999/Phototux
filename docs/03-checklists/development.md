# Development Checklist

Living document. Update status during `build-with-checklist`.

Legend: `[ ]` todo · `[~]` in progress · `[x]` done · `[!]` blocked

**Note (2026-07-15):** Early Phase 1 code scaffold was **removed** so design docs could land first. Implementation items below are **open** again; follow `docs/DESIGN.md` + IA when re-scaffolding.

---

## Phase 0 — Design readiness (docs)

- [x] Design brief — `docs/DESIGN_BRIEF.md`
- [x] Information architecture — `docs/INFORMATION_ARCHITECTURE.md`
- [x] Design system tokens — `docs/DESIGN.md` (DESIGN.md format)
- [x] Engineering ADRs locked — `decisions-locked-v1`
- [ ] Design review pass (human) on brief + tokens vs KDE Plasma feel
- [ ] Optional: lint `docs/DESIGN.md` with `@google/design.md`

---

## Phase 1 — Toolchain & Pure Rust-Qt Bootstrap (ADR-003, ADR-006)

### 1.1 Workspace foundation
- [ ] Create Cargo workspace root `Cargo.toml` with members per ADR-006
- [ ] Scaffold `crates/phototux` binary crate
- [ ] Scaffold `crates/phototux-ui` (qtbridge QObjects)
- [ ] Scaffold `crates/phototux-engine` stub (no Qt)
- [ ] Reserve `phototux-gpu` / `phototux-canvas` (names reserved in ADRs)
- [ ] Root `qml/` directory with `Main.qml` styled from `DESIGN.md` tokens
- [ ] `.gitignore` covers `target/`, build artifacts
- **Done when:** `cargo build -p phototux` resolves workspace

### 1.2 qtbridge integration
- [ ] Pin `qtbridge = "0.2"` (or latest 0.2.x)
- [ ] `QApp` entry loads `Main.qml`
- [ ] Sample `#[qobject]` backend with `#[qproperty]` + `#[qslot]`
- [ ] QML binds slider/label to Rust property (round-trip)
- **Done when:** `cargo run -p phototux` opens window; slider updates Rust state and label

### 1.3 QML shell skeleton (per IA + DESIGN.md)
- [ ] ApplicationWindow tokens (neutral/surface)
- [ ] Top toolbar (`toolbar-height`)
- [ ] Left tool strip (`tool-strip-width`)
- [ ] Right Properties + Layers docks (`dock-width`)
- [ ] Status bar (`statusbar-height`, HUD placeholders)
- [ ] Center canvas placeholder (`canvas-viewport` / sunken)
- **Done when:** Layout matches IA blueprint; visual audit vs DESIGN.md

### 1.4 Phase 1 quality
- [ ] Engine stub unit tests
- [ ] README run instructions restored
- [ ] Cold-start ballpark noted in journal
- **Done when:** 1.1–1.3 complete

**Phase 1 exit:** Rust↔QML binding validated; UI skeleton matches design system; no wgpu required yet.

---

## Phase 1.5 — Interop spike (ADR-010) — **before Phase 2 production**

- [ ] Branch `spike/wgpu-qt-rhi-interop`
- [ ] Time-box ≤ ~3 days / 3 sessions
- [ ] Attempt shared Vulkan / external memory → Qt RHI item
- [ ] Attempt DMA-BUF path if needed
- [ ] Record whether qtbridge alone can host item vs thin C++
- [ ] Write `docs/04-journal/spike-findings-interop.md`
- [ ] Amend ADR-003/005 only if outcomes force it
- **Done when:** Documented success recipe **or** documented hard fail (no silent CPU default)
- **Forbidden:** Merging CPU full-frame upload as product default

---

## Phase 2 — High-Performance GPU Viewport (ADR-004, ADR-005, ADR-008)

- [ ] `phototux-gpu`: wgpu device/queue init (Vulkan)
- [ ] `phototux-canvas`: custom item path (qtbridge or hybrid C++ RHI item)
- [ ] Zero-copy or shared texture present path (no CPU full-frame default)
- [ ] Pan/zoom camera in engine
- [ ] FPS overlay / frame timing (mono-hud tokens)
- [ ] **Gate:** ≥60 FPS zoom/pan on large test canvas
- **Blocked by:** Phase 1 exit; interop risk (spike skipped)

---

## Phase 3 — Composite Layer Engine (ADR-004, ADR-007, ADR-008)

- [ ] Image state graph in `phototux-engine`
- [ ] WGSL blend modes
- [ ] Transactional undo/redo
- [ ] Layers panel wired to graph (IA F3)
- [ ] **Gate:** 10×4K composite < 2 ms GPU

---

## Phase 4 — Brush & Tools (ADR-007, ADR-008)

- [ ] Brush core + Wayland tablet pressure
- [ ] Selection, eyedropper, transform tool states
- [ ] Interactive Layer Panel complete
- [ ] Engine worker command queue mandatory
- [ ] **Gate:** input-to-render < 8 ms path instrumented

---

## Phase 5 — Desktop Integration & Release (ADR-001, ADR-008)

- [ ] KDE menu bar / global menu hooks as applicable
- [ ] XDG portals open/save (IA F4/F5)
- [ ] HDR path profile
- [ ] **Gate:** cold boot < 250 ms interactive (stretch; document if missed)
- [ ] Packaging notes (distro/Flatpak deferred OK)

---

## Cross-cutting

- [ ] Keep `docs/03-checklists/blockers.md` current
- [ ] ADR revisit dates checked at each phase exit
- [ ] No new major deps without ADR
- [ ] UI changes update `DESIGN.md` tokens when needed
