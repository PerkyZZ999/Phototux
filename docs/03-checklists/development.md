# Development Checklist

Living document. Update status during `build-with-checklist`.

Legend: `[ ]` todo · `[~]` in progress · `[x]` done · `[!]` blocked

---

## Phase 1 — Toolchain & Pure Rust-Qt Bootstrap (ADR-003, ADR-006)

### 1.1 Workspace foundation
- [x] Create Cargo workspace root `Cargo.toml` with members per ADR-006
- [x] Scaffold `crates/phototux` binary crate
- [x] Scaffold `crates/phototux-ui` (qtbridge QObjects)
- [x] Scaffold `crates/phototux-engine` stub (no Qt)
- [x] Reserve `phototux-gpu` / `phototux-canvas` as empty or stub crates (or defer files until Phase 2 — names reserved in docs)
- [x] Root `qml/` directory with `Main.qml`
- [x] `.gitignore` covers `target/`, build artifacts
- **Done when:** `cargo build -p phototux` resolves workspace — **met**

### 1.2 qtbridge integration
- [x] Pin `qtbridge = "0.2"` (or latest 0.2.x)
- [x] `QApp` entry loads `Main.qml`
- [x] Sample `#[qobject]` backend with `#[qproperty]` + `#[qslot]`
- [x] QML binds slider/label to Rust property (round-trip)
- **Done when:** `cargo run -p phototux` opens window; slider updates Rust state and label — **met** (window title PhotoTux, 1440×900)

### 1.3 QML shell skeleton (KDE-dense)
- [x] ApplicationWindow dark theme baseline
- [x] Top toolbar placeholder (tools)
- [x] Left tool strip placeholder
- [x] Right properties dock placeholder
- [x] Bottom status bar (FPS placeholder text OK)
- [x] Center canvas placeholder `Rectangle` (not GPU yet)
- **Done when:** Layout matches multi-pane editor silhouette at 1280×800+ — **met**

### 1.4 Phase 1 quality
- [x] Engine stub unit test runs under `cargo test -p phototux_engine`
- [x] Document run instructions in README
- [x] Measure cold-start ballpark (note in journal; gate is Phase 5)
- **Done when:** checklist 1.1–1.3 complete + README run section — **met**

**Phase 1 exit:** Rust↔QML binding validated; UI skeleton; no requirement for wgpu yet. — **COMPLETE 2026-07-15**

---

## Phase 2 — High-Performance GPU Viewport (ADR-004, ADR-005, ADR-008)

- [ ] `phototux-gpu`: wgpu device/queue init (Vulkan)
- [ ] `phototux-canvas`: custom item path (qtbridge or hybrid C++ RHI item)
- [ ] Zero-copy or shared texture present path (no CPU full-frame default)
- [ ] Pan/zoom camera in engine
- [ ] FPS overlay / frame timing
- [ ] **Gate:** ≥60 FPS zoom/pan on large test canvas
- **Blocked by:** Phase 1 exit; interop risk (spike skipped)

---

## Phase 3 — Composite Layer Engine (ADR-004, ADR-007, ADR-008)

- [ ] Image state graph in `phototux-engine`
- [ ] WGSL blend modes
- [ ] Transactional undo/redo
- [ ] **Gate:** 10×4K composite < 2 ms GPU

---

## Phase 4 — Brush & Tools (ADR-007, ADR-008)

- [ ] Brush core + Wayland tablet pressure
- [ ] Selection, eyedropper, transform tool states
- [ ] Layer panel QML (interactive)
- [ ] Engine worker command queue mandatory
- [ ] **Gate:** input-to-render < 8 ms path instrumented

---

## Phase 5 — Desktop Integration & Release (ADR-001, ADR-008)

- [ ] KDE menu bar / global menu hooks as applicable
- [ ] XDG portals open/save
- [ ] HDR path profile
- [ ] **Gate:** cold boot < 250 ms interactive (stretch; document if missed)
- [ ] Packaging notes (distro/Flatpak deferred OK)

---

## Cross-cutting

- [ ] Keep `docs/03-checklists/blockers.md` current
- [ ] ADR revisit dates checked at each phase exit
- [ ] No new major deps without ADR
