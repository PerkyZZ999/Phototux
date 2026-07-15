# Development Checklist

Living document. Update status during `build-with-checklist`.

Legend: `[ ]` todo · `[~]` in progress · `[x]` done · `[!]` blocked

---

## Phase 1 — Toolchain & Pure Rust-Qt Bootstrap (ADR-003, ADR-006)

### 1.1 Workspace foundation
- [ ] Create Cargo workspace root `Cargo.toml` with members per ADR-006
- [ ] Scaffold `crates/phototux` binary crate
- [ ] Scaffold `crates/phototux-ui` (qtbridge QObjects)
- [ ] Scaffold `crates/phototux-engine` stub (no Qt)
- [ ] Reserve `phototux-gpu` / `phototux-canvas` as empty or stub crates (or defer files until Phase 2 — names reserved in docs)
- [ ] Root `qml/` directory with `Main.qml`
- [ ] `.gitignore` covers `target/`, build artifacts
- **Done when:** `cargo build -p phototux` resolves workspace

### 1.2 qtbridge integration
- [ ] Pin `qtbridge = "0.2"` (or latest 0.2.x)
- [ ] `QApp` entry loads `Main.qml`
- [ ] Sample `#[qobject]` backend with `#[qproperty]` + `#[qslot]`
- [ ] QML binds slider/label to Rust property (round-trip)
- **Done when:** `cargo run -p phototux` opens window; slider updates Rust state and label

### 1.3 QML shell skeleton (KDE-dense)
- [ ] ApplicationWindow dark theme baseline
- [ ] Top toolbar placeholder (tools)
- [ ] Left tool strip placeholder
- [ ] Right properties dock placeholder
- [ ] Bottom status bar (FPS placeholder text OK)
- [ ] Center canvas placeholder `Rectangle` (not GPU yet)
- **Done when:** Layout matches multi-pane editor silhouette at 1280×800+

### 1.4 Phase 1 quality
- [ ] Engine stub unit test runs under `cargo test -p phototux-engine`
- [ ] Document run instructions in README
- [ ] Measure cold-start ballpark (note in journal; gate is Phase 5)
- **Done when:** checklist 1.1–1.3 complete + README run section

**Phase 1 exit:** Rust↔QML binding validated; UI skeleton; no requirement for wgpu yet.

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
