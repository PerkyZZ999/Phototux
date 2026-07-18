# AGENTS.md

Agent-facing constitution for **PhotoTux**.

**Authoritative engineering docs:** [`internal_docs/`](internal_docs/README.md) (Engineering Handbook).  
**Historical docs:** [`archive/docs/`](archive/docs/README.md) (former `/docs/` — ADRs, journals, checklists).  
**Alignment (complete):** [`internal_docs/Appendix/Alignment-Roadmap.md`](internal_docs/Appendix/Alignment-Roadmap.md) (stack frozen — [DR-023](internal_docs/Appendix/Decision-Register.md)).  
**Product roadmap (handbook parity):** [`internal_docs/Appendix/Handbook-Parity-Roadmap.md`](internal_docs/Appendix/Handbook-Parity-Roadmap.md).  
**Product checklist:** [`internal_docs/Appendix/Handbook-Parity-Checklist.md`](internal_docs/Appendix/Handbook-Parity-Checklist.md).  
**Gap inventory:** [`internal_docs/Appendix/Codebase-Handbook-Gap-Analysis.md`](internal_docs/Appendix/Codebase-Handbook-Gap-Analysis.md).  
**Alignment checklist (history):** [`internal_docs/Appendix/Implementation-Checklist.md`](internal_docs/Appendix/Implementation-Checklist.md).

If handbook Decision Register conflicts with archived ADRs or code: **surface the conflict** (never silent) → update Decision Register or gap analysis → prefer **measured shipped code + promoted DR** over silent drift. Root `SPEC.md` / `CONSTRAINTS.md` are **non-normative bridges** → handbook + Decision Register. Archived ADR map: [`internal_docs/Appendix/Archived-ADR-to-DR-Map.md`](internal_docs/Appendix/Archived-ADR-to-DR-Map.md).

---

## Project overview

PhotoTux is a **Linux/Wayland**, **Rust + Qt 6 QML** professional image editor with **zero-copy GPU** canvas (`wgpu`/Vulkan) and a dense KDE Plasma–aligned shell.

| Layer | Choice | Notes |
|-------|--------|-------|
| Platform | Linux / Wayland v1 | Handbook local-first + Linux host |
| UI | Qt 6.10+ QML, Controls 2; Kirigami deferred | DR-023 Accepted (DR-008 superseded) |
| FFI | `qtbridge` 0.2; thin C++ canvas + QML AOT only | DR-023 |
| GPU | `wgpu` Vulkan-first | DR-006 / DR-023 |
| Present | Zero-copy interactive; debug readback only | Keep; CPU = tests/degraded only |
| Crates | Multi-crate `phototux_*` | DR-025 coarse; handbook 32 = ownership map |
| Threads | Paint queue + `SessionState::invoke` document spine | Document commits routed; paint stream host-only until stroke-end |
| Doc model | Graph v2 layers in engine | Single doc v1 (DR-024) until DR amend |
| License | GPL-3.0-or-later | |
| Surface | **Desktop GUI only** | No CLI/TUI/web product |

**Design tokens (historical):** `archive/docs/DESIGN.md` until migrated into handbook Themes/UX.  
**Product form:** windowed desktop editor. `cargo` / tests = developer tooling.

---

## Setup commands

```bash
# Host (Arch/CachyOS)
sudo pacman -S rustup clang cmake qt6-base qt6-declarative vulkan-headers
rustup component add rustfmt clippy
cargo install rust-doctor   # or: already on PATH / ~/.bun/bin/rust-doctor

# Qt 6 on PATH (critical: default qmake may be Qt 5)
export PATH=/usr/lib/qt6/bin:$PATH
export QMAKE=/usr/lib/qt6/bin/qmake

# Git hooks (fmt + clippy on every commit; rust-doctor opt-in)
./scripts/install-git-hooks.sh

# When workspace exists:
cargo build -p phototux
cargo run -p phototux
cargo test -p phototux_engine
./scripts/check-rust.sh              # fmt + clippy (pre-commit default)
CHECK_RUST_FULL=1 ./scripts/check-rust.sh   # + rust-doctor
```

---

## Development workflow

1. Read relevant handbook chapters + Decision Register before coding; pick slices from [Handbook-Parity-Checklist](internal_docs/Appendix/Handbook-Parity-Checklist.md).
2. Prefer vertical slices toward handbook parity; respect P11/P12 gates (tiling evidence, DR-024, plugin need).
3. Do not invent a second doc tree under `/docs/` — handbook only; archive is read-only history.
4. Commit only after `./scripts/check-rust.sh` passes (or pre-commit does).

### Workspace layout (when scaffolded)

```
crates/phototux/         # binary package name: phototux
crates/phototux-ui/      # package: phototux_ui  — qtbridge only, no wgpu
crates/phototux-engine/  # package: phototux_engine — pure Rust, no Qt
crates/phototux-gpu/     # package: phototux_gpu — Phase 2+
crates/phototux-canvas/  # package: phototux_canvas — interop ± thin C++
qml/                     # QML; tokens from archive/docs/DESIGN.md until handbook Themes migrate
assets/icons/phosphor/   # Phosphor Icons MIT (core 2.1.1); default weight regular
```

Paths **kebab-case**; Cargo package names **`phototux_*` underscores**.

---

## Mandatory skill compliance

Agents **must load and apply** these skills when the task matches. Web-oriented wording maps to **desktop QML/Qt** (density, hierarchy, a11y, icons)—not HTML/CSS frameworks.

### Rust (all Rust edits)

| Skill | Role |
|-------|------|
| `ms-rust` | Microsoft Pragmatic Rust Guidelines — `must` = hard gates; `should` = defaults |
| `rust` | Performance patterns (allocation, ownership, iterators, async) |
| `rust-reference` | Language semantics, unsafe, types, macros — when correctness depends on the Reference |
| `rust-skills` | Broad idiomatic rules (ownership, errors, API, testing, anti-patterns) |
| `rust-optimise` | Hot-path optimization (mem/own/ds/iter first; micro last) |
| `rust-doctor` | Health scan tool; opt-in via `CHECK_RUST_FULL=1` — re-scan after large fixes |

**Rust hard defaults for this repo:**

- Prefer **clear ownership** and borrowing over `clone` in hot paths.
- Engine/public logic: **`Result` + typed errors** (`thiserror` style); no `unwrap`/`expect` in library paths except tests or documented invariants.
- **`unsafe` only** in `phototux_canvas` / FFI interop; minimal blocks; `// SAFETY:` states the invariant (not a paragraph of excuses).
- Lint overrides: **`#[expect(..., reason = "...")]`**, not silent `#[allow]` (ms-rust).
- Structured logging via **`tracing`** with fields, not stringly `format!` spam in hot paths.
- No artificial “guideline compliant” marker comments in source.
- American English in comments/docs unless asked otherwise.

### Frontend UI/UX (QML shell, chrome, icons)

| Skill | Desktop adaptation |
|-------|--------------------|
| `craft-beautiful-frontend` | Use **dense** density (editor); tokens from `archive/docs/DESIGN.md` / handbook Themes; Gestalt/hierarchy/a11y; no web-card padding; canvas-first; motion only for structure (docks), never paint delay |
| `iconography-frontend-ui` | Icons from `assets/icons/phosphor/`; **map:** `assets/icons/ICON_MAP.md`; function over decoration; labels+tooltips; contrast; states; size on grid (tool strip ~36px hit) |

**Never** invent a second palette or spacing scale—extend archived `DESIGN.md` or handbook Themes.

---

## Engineering doctrine

> **If you need a paragraph-long comment to justify why the workaround is OK, the code is wrong — fix the code.**

- No long apology comments for hacks, race paper-overs, or “temporary” CPU uploads.
- Fix architecture, types, or boundaries instead.
- Short `// SAFETY:` / one-line `reason` on `expect` lints are fine; essays are a smell.
- Forbidden product path: steady-state full-frame **CPU canvas upload** (ADR-005).

---

## Pre-commit & quality checks

| Check | Command / behavior |
|-------|-------------------|
| Install hooks | `./scripts/install-git-hooks.sh` → `core.hooksPath=.githooks` |
| Pre-commit / default | `./scripts/check-rust.sh` → rustfmt + clippy |
| Full gate | `CHECK_RUST_FULL=1 ./scripts/check-rust.sh` or `--full` → + rust-doctor |
| rustfmt | `cargo fmt --all -- --check` |
| clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| rust-doctor | `rust-doctor . --offline --fail-on error -v` (full gate only) |

**Hook behavior:**

- No `Cargo.toml` → skip checks (exit 0), unless staged `*.rs` → fail.
- Clippy warnings are **errors** (`-D warnings`).
- rust-doctor is **not** in the pre-commit path; run the full gate manually or in CI when needed.
- Prefer fixing findings over suppressions.

Agents: after non-trivial Rust changes, run `./scripts/check-rust.sh` even if hooks are not installed in the environment.

---

## Testing instructions

```bash
cargo test -p phototux_engine          # pure logic (no Qt)
cargo test --workspace                 # when more crates exist
# GPU tests (optional, device present):
cargo test -p phototux_gpu --features gpu-tests
```

- Unit tests for engine graph/undo when present (ADR-009).
- QML: manual checklist early; no requirement for full Qt Test in Phase 1.
- Do not claim phase exit without ADR-008 SLO evidence when that gate applies.

### Performance gates (ADR-008)

| Gate | Target | From |
|------|--------|------|
| Zoom/pan FPS | ≥ 60 | Phase 2 exit |
| Brush stroke FPS | ≥ 60 | Phase 4 exit |
| Tablet input→render | < 8 ms | Phase 4 exit |
| Cold boot interactive | < 1,000 ms gate; < 250 ms stretch | Phase 5 (measure earlier) |
| 10×4K composite | < 2 ms GPU | Phase 3 exit |
| Hot path copies | No full-frame CPU upload | Phase 2+ |

---

## Code style & organization

### Rust

- Edition **2024**; `rustfmt.toml` / `clippy.toml` at repo root.
- Crate boundaries: no Qt in `phototux_engine`; no wgpu in `phototux_ui`.
- Naming: types `UpperCamelCase`, functions/modules `snake_case`, constants `SCREAMING_SNAKE`.
- Public API docs on non-trivial exported items; module-level docs on crate roots.

### QML / UI

- Controls 2; Breeze-dark / archived `DESIGN.md` tokens (migrate to handbook Themes).
- Layout per handbook IA (`internal_docs/01-Information-Architecture.md`) + workspace chapters.
- New document: **ask + presets** 720p / 1080p / 2K / 4K.
- Single document v1 until Decision Register amend; **zoom-to-fit** on open/new.
- Strings user-facing: `qsTr(...)`.

### Git / commits

- Atomic commits; conventional-ish subjects (`feat:`, `fix:`, `docs:`, `chore:`).
- Reference ADRs when changing architecture.
- Do not commit secrets, `target/`, or large binaries without need.

---

## Decision boundaries

### Allowed without new ADR

`qtbridge` 0.2.x, `wgpu` 30.x, `tracing`, `thiserror`, `serde`, small pure-Rust utils, Phosphor SVGs under `assets/icons/phosphor/`.

### Requires ADR amendment

UI toolkit change, primary FFI switch, abandoning zero-copy, multi-doc, non-Linux v1, spreading handwritten C++ beyond canvas or ADR-003's QML AOT anchor, new major subsystems (cloud, plugins store).

### Forbidden

Electron/web shell, **CLI or TUI as product** (ADR-014), GTK as main UI, CPU full-frame canvas as default, Kirigami in Phase 1–2, silent scope to Windows/macOS, paragraph-length workaround comments instead of fixes.

---

## Revisit triggers

1. qtbridge blocks custom item → ADR-003  
2. Zero-copy fails two real approaches → ADR-005 (+ spike report)  
3. wgpu/Qt share fails → ADR-004 interop  
4. RefCell re-entrancy forces model change → ADR-007  
5. SLO unachievable on reference hardware → ADR-008  
6. New major dependency → ADR  

---

## Debugging tips

| Symptom | Check |
|---------|--------|
| Build links wrong Qt | `PATH`/`QMAKE` → `/usr/lib/qt6/bin` |
| QML import missing | Package name of `#[qobject]` crate (`import phototux_ui`) |
| Pre-commit skip forever | Missing `Cargo.toml` (expected docs-only) |
| rust-doctor exit 2 | Project does not compile — fix `cargo build` first |
| Hook not running | `./scripts/install-git-hooks.sh`; `git config core.hooksPath` |

---

## Key doc map

| Path | Use |
|------|-----|
| `internal_docs/` | **Engineering Handbook** (authoritative) |
| `internal_docs/Appendix/Decision-Register.md` | Architectural decisions index |
| `internal_docs/Appendix/Alignment-Roadmap.md` | Alignment complete (contracts) |
| `internal_docs/Appendix/Handbook-Parity-Roadmap.md` | Product phases to full handbook parity |
| `internal_docs/Appendix/Handbook-Parity-Checklist.md` | Living product slice tracker |
| `internal_docs/Appendix/Interactive-Stability-Checklist.md` | Living GUI / edge-case QA suite |
| `internal_docs/Appendix/Implementation-Checklist.md` | Alignment history (Phases 0–4) |
| `internal_docs/Appendix/Codebase-Handbook-Gap-Analysis.md` | Code vs handbook diffs |
| `archive/docs/` | Archived former `/docs/` (ADRs, journals, old IA) |
| `SPEC.md` / `CONSTRAINTS.md` | Non-normative bridges → handbook + Decision Register |
| `internal_docs/Appendix/Archived-ADR-to-DR-Map.md` | Archived ADR → live DR |
| `scripts/check-rust.sh` | Quality gate |
| `.githooks/pre-commit` | Commit gate |

---

## PR / handoff checklist

- [ ] `./scripts/check-rust.sh` green (when Rust workspace exists)
- [ ] Tests for engine logic touched
- [ ] UI matches handbook UX / Themes (historical tokens in `archive/docs/DESIGN.md` until migrated)
- [ ] No forbidden steady-state CPU canvas upload
- [ ] Gap analysis / Decision Register updated if architecture changes
- [ ] No paragraph-long workaround comments
