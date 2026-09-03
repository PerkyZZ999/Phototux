# AGENTS.md

PhotoTux is a **Linux/Wayland** professional image editor: **Rust + Qt 6 QML** (`qtbridge`), **wgpu/Vulkan** zero-copy canvas, dense KDE Plasma–aligned **desktop GUI**. There is no CLI, TUI, or web product.

**Authority:** [Engineering Handbook](internal_docs/README.md) and [Decision Register](internal_docs/Appendix/Decision-Register.md) ([DR-023](internal_docs/Appendix/Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase) stack). If handbook and code disagree, **surface the conflict** — then update the Decision Register or [gap analysis](internal_docs/Appendix/Codebase-Handbook-Gap-Analysis.md). Prefer measured shipped code plus a promoted DR over silent drift. Root `SPEC.md` / `CONSTRAINTS.md` are non-normative bridges. Former ADR ids: [Archived-ADR-to-DR-Map.md](internal_docs/Appendix/Archived-ADR-to-DR-Map.md).

---

## Commands

Qt 6 must be on `PATH`. Host `qmake` is often Qt 5:

```bash
export PATH=/usr/lib/qt6/bin:$PATH
export QMAKE=/usr/lib/qt6/bin/qmake

cargo build -p phototux
cargo run -p phototux
cargo test -p phototux_engine
rust-tc check                           # fastest compiler-only feedback
rust-tc quick                           # fmt + check + clippy + tests + doctests
rust-tc doctor                          # full local Rust-Toolchain gate
./scripts/check-rust.sh                 # rust-tc precommit (fmt + clippy; git hook)
./scripts/check-rust.sh --full          # rust-tc doctor + SonarQube
CHECK_SONAR=0 ./scripts/check-rust.sh --full   # rust-tc doctor only
./scripts/check-sonar.sh                # Clippy JSON + scanner + quality gate
python3 scripts/check-docs-links.py      # internal_docs + web/docs link check
```

One-time: `./scripts/install-git-hooks.sh`. Host packages, crate map, and the rest of the quality matrix: [32 — Developer Guide](internal_docs/32-Developer-Guide.md#build-check-and-test-commands).

---

## How to work

1. Before architecture, UX, or crate-boundary work, read the relevant handbook chapter and the Decision Register. Pick product slices from the [Handbook-Parity-Checklist](internal_docs/Appendix/Handbook-Parity-Checklist.md).
2. Extend [`internal_docs/`](internal_docs/README.md). Do not add a second tree under `/docs/` (archive is history). User-facing behaviour also updates the page in [`web/docs`](web/docs) that describes it ([DR-033](internal_docs/Appendix/Decision-Register.md#dr-033--public-web-presence-is-two-static-astro-sites-not-a-second-handbook)) — handbook for contributors, site for users.
3. After non-trivial Rust, `rust-tc quick` must pass. Add engine tests for engine logic you touch. Commit at your own judgement, whenever the work reaches a sensible commit point. Before finishing substantial work, `rust-tc doctor`.

Cargo workspace: directories kebab-case, packages `phototux_*`. Ownership: Developer Guide [Rust Workspace Boundaries](internal_docs/32-Developer-Guide.md#rust-workspace-boundaries) and [DR-025](internal_docs/Appendix/Decision-Register.md#dr-025--crate-topology-coarse-workspace).

| Package | Owns | Stays free of |
|---------|------|----------------|
| `phototux_engine` | document, commands, history | Qt, wgpu |
| `phototux_ui` | qtbridge QObjects | wgpu |
| `phototux_gpu` | wgpu / Vulkan | Qt |
| `phototux_canvas` | Qt↔wgpu interop, thin C++ | extra handwritten C++ |
| `phototux_io` | `.ptx`, raster, PSD subset | Qt, wgpu |

---

## Guardrails

Interactive present is **zero-copy GPU**. CPU canvas is tests and degraded mode only.

- Library paths: `Result` + typed errors (`thiserror`). `unwrap`/`expect` only in tests or documented invariants.
- `unsafe` only in `phototux_canvas` / FFI; each block has a one-line `// SAFETY:` invariant.
- Lint overrides: `#[expect(..., reason = "...")]`, not silent `#[allow]`.
- Tracing with fields on hot paths, not `format!` spam.
- QML tokens from [`qml/Theme.qml`](qml/Theme.qml) and handbook [25 — Themes](internal_docs/25-Themes.md). Icons from `assets/icons/phosphor/` via [`assets/icons/ICON_MAP.md`](assets/icons/ICON_MAP.md). User-facing strings: `qsTr(...)`.
- New document: ask + presets 720p / 1080p / 2K / 4K; zoom-to-fit on open/new. Tabs are [DR-024](internal_docs/Appendix/Decision-Register.md#dr-024--document-session-model) v2; **multi-window is out of scope**.
- If a workaround needs a paragraph of apology, fix the code.

**Without a new DR:** qtbridge 0.2.x, wgpu 30.x, tracing, thiserror, serde, small pure-Rust utils, Phosphor SVGs.

**Needs a DR:** UI toolkit, primary FFI, abandoning zero-copy, multi-window, non-Linux v1, handwritten C++ beyond canvas or QML AOT, new major subsystems (cloud, plugin store).

**Ship:** Qt 6 QML desktop editor on Linux/Wayland with wgpu present. Not Electron/web, not a CLI/TUI product, not GTK as the main UI, not Kirigami in current phases, not a silent Windows/macOS port.

---

## Skills

When the task matches, load the skill. Web-oriented wording maps to **dense desktop QML**, not HTML/CSS.

- Rust: `ms-rust` (`must` = hard gates) and `rust-skills`. Language-law: `rust-reference`. Hot path: `rust-optimise`. Health scan: `rust-tc doctor` (Rust-Toolchain; not the `rust-doctor` crate).
- QML / chrome / icons: `craft-beautiful-frontend` (dense editor, `Theme.qml`, canvas-first) and `iconography-frontend-ui`.

---

## Quality

Pre-commit is `rust-tc precommit` (fmt + clippy). `rust-tc doctor` is the full **local** Rust-Toolchain gate (fmt, clippy, nextest, doctests, cargo-deny, cargo-shear, cargo-hack). It is not the `rust-doctor` Cargo binary. SonarQube (project key `phototux`, [localhost:9000](http://localhost:9000/dashboard?id=phototux)) stays a separate opt-in via `--full` / `check-sonar.sh`. Token: `SONAR_TOKEN` or gitignored `.sonar/scanner-token`. Prefer fixing findings over suppressions.

Do not add `cargo-audit` (`cargo-deny` owns advisories). Do not blanket-allow Clippy to force green. Device-backed GPU tests stay opt-in: `cargo test -p phototux_gpu --features gpu-tests`. Do not run `rust-tc mutants`, `rust-tc miri`, `rust-tc fuzz`, or `rust-tc features-deep` after every edit.

Clippy cognitive-complexity threshold is **30**; Sonar `S3776` is **15**. Split helpers rather than raising either.

Performance budgets: [DR-017](internal_docs/Appendix/Decision-Register.md#dr-017--performance-budgets-provisional) (former ADR-008). Headless core tests: [DR-022](internal_docs/Appendix/Decision-Register.md#dr-022--headless-testability-of-core). GPU tests (device present): `cargo test -p phototux_gpu --features gpu-tests`. GUI edge cases: [Interactive-Stability-Checklist](internal_docs/Appendix/Interactive-Stability-Checklist.md).

Commits: atomic, conventional-ish (`feat:`, `fix:`, `docs:`, `chore:`). Reference DRs when changing architecture. Do not commit secrets, `target/`, or `.sonar/`. Commit a coherent unit of work once it builds and its checks pass — not a half-applied change, and not a snapshot of whatever is in the tree. When one investigation yields two separate concerns, split them.

**Cursor** loads this file plus path-scoped `.cursor/rules/*.mdc` and nested `AGENTS.md`. **Claude Code** loads root `CLAUDE.md` (imports this file), path-scoped `.claude/rules/`, nested `CLAUDE.md`, and `.claude/settings.json`. Do not copy this constitution into those files.

---

## Pointers

| When | Read |
|------|------|
| Stack, crate topology, session model | Decision Register (DR-023, DR-025, DR-024) |
| Product slice | [Handbook-Parity-Checklist](internal_docs/Appendix/Handbook-Parity-Checklist.md) / [Roadmap](internal_docs/Appendix/Handbook-Parity-Roadmap.md) |
| Code vs handbook | [Gap analysis](internal_docs/Appendix/Codebase-Handbook-Gap-Analysis.md) |
| Shell / IA / tokens | [01](internal_docs/01-Information-Architecture.md), [25](internal_docs/25-Themes.md) |
| Commands / undo | [08](internal_docs/08-Command-System.md), [20](internal_docs/20-History-Undo.md) |
| GPU / present | [17](internal_docs/17-Rendering-Engine.md) |
| Contributor workflow | [32](internal_docs/32-Developer-Guide.md) |
| Websites (phototux.xyz, docs.phototux.xyz) | [`web/README.md`](web/README.md), [DR-033](internal_docs/Appendix/Decision-Register.md#dr-033--public-web-presence-is-two-static-astro-sites-not-a-second-handbook) |
| GUI QA | [Interactive-Stability-Checklist](internal_docs/Appendix/Interactive-Stability-Checklist.md) |

---

## Debug

| Symptom | Check |
|---------|--------|
| Links wrong Qt | `PATH` / `QMAKE` → `/usr/lib/qt6/bin` |
| QML import missing | `import phototux_ui` (package of the `#[qobject]` crate) |
| rust-tc / just missing | `rust-tc` and `just` on PATH (`~/.local/bin`) |
| rust-tc clippy fails to link Qt | `PATH` / `QMAKE` → `/usr/lib/qt6/bin` |
| sonar-scanner auth | `sonar auth status`; token as above |
| SonarQube unreachable | `http://localhost:9000` or `CHECK_SONAR=0` |
| Hook not running | `./scripts/install-git-hooks.sh`; `git config core.hooksPath` |
