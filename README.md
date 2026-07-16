# PhotoTux

Professional-grade, high-performance raster/vector **desktop** image editor for modern Linux (Wayland). **GUI application only** for MVP/v1 (no CLI/TUI product).

## Problem Statement

Linux creative tooling is capable (GIMP, Krita) but rarely pairs **strict systems performance** with a **polished, dense desktop shell**. Editors either feel heavy (slow canvas, high latency, CPU-bound compositing) or do not feel native on KDE Plasma 6 / Wayland. Artists and technical users need an editor that sustains 60+ FPS (and high-refresh 120/144Hz) on large canvases with minimal tablet input latency — without abandoning a modern multi-pane desktop UX.

## Target Audience

- Digital artists and illustrators on **Linux / Wayland**, especially **KDE Plasma 6**
- Technical power users who prefer native performance over Electron/web shells
- Contributors comfortable with **Rust + QML** who want a systems-first graphics app

## Core Value Proposition

Zero-copy GPU compositing (Rust + `wgpu`/Vulkan) behind a KDE-native Qt Quick shell — massive canvases stay responsive because pixels never cross FFI.

## MVP Scope

Minimum viable product = **Phase 1 + Phase 2** from the product roadmap (interactive shell + GPU viewport).

1. Cargo workspace with `qtbridge-rust` bootstrap; Rust↔QML state binding works (sliders, labels, basic panels)
2. QML UI skeleton matching KDE HIG: dense dark multi-pane layout (toolbars, docks placeholders)
3. Custom QQuickItem canvas hooked to Qt RHI; Rust `wgpu` pipeline on shared Vulkan surface
4. Stable **≥60 FPS** zoom/pan viewport benchmark on a large test canvas

Explicitly out of scope for MVP:

- Full brush engine, pressure curves, tablet tool stack (Phase 4)
- Non-destructive layer graph, blend-mode compute, undo DAG (Phase 3)
- Selections, transform tools, color pickers (Phase 4)
- KDE global menus, XDG portals export polish, HDR path (Phase 5)
- Vector editing, plugin marketplace, multi-document tabs

Full phase plan lives in [SPEC.md](SPEC.md).

## Success Criteria

| # | Criterion | Target | Measurement Method |
|---|-----------|--------|-------------------|
| 1 | Steady-state frame rate during zoom/pan | ≥ 60 FPS (path to 120/144Hz) | In-app frame timer / `tracy` or similar on reference 4K canvas |
| 2 | Input-to-render latency (tablet stroke path) | < 8 ms | Timed stroke → present path on Wayland tablet |
| 3 | Cold boot to interactive workspace | < 1,000 ms gate; < 250 ms stretch | Process start → first interactive frame |
| 4 | 10-layer 4K compositing budget | < 2 ms GPU | GPU timestamp queries on blend pass |
| 5 | Zero-copy canvas path | No CPU pixel upload for steady-state view | Architecture + runtime assert: no full-frame `Map`/readback in hot path |

## Constraints

See [CONSTRAINTS.md](CONSTRAINTS.md). Product intent and architecture pillars: [SPEC.md](SPEC.md).

## Documentation map

| Document | Purpose |
|----------|---------|
| [**internal_docs/**](internal_docs/README.md) | **Engineering Handbook** (authoritative) |
| [internal_docs/Appendix/Codebase-Handbook-Gap-Analysis.md](internal_docs/Appendix/Codebase-Handbook-Gap-Analysis.md) | Codebase vs handbook diffs + alignment plan |
| [internal_docs/Appendix/Decision-Register.md](internal_docs/Appendix/Decision-Register.md) | Architectural decision index |
| [SPEC.md](SPEC.md) | Bridge: product architecture / SLOs (migrate into handbook) |
| [CONSTRAINTS.md](CONSTRAINTS.md) | Bridge: hard/soft constraints |
| [AGENTS.md](AGENTS.md) | Agent coding constitution |
| [CHANGELOG.md](CHANGELOG.md) | Decision milestones |
| [archive/docs/](archive/docs/README.md) | Archived former `/docs/` (ADRs, journals, old IA/checklists) |

**Status:** Foundation editor ships GPU canvas, layers/masks, selections, transforms, brush, `.ptx`/PSD subset, adjustments/filters core, fill/gradient. Normative engineering direction = handbook; implementation alignment = gap analysis (hybrid: keep Qt/wgpu/`.ptx` spine, evolve toward command/snapshot/workspace contracts).

## Run (developer)

```bash
export PATH=/usr/lib/qt6/bin:$PATH
export QMAKE=/usr/lib/qt6/bin/qmake
cargo run -p phototux          # GUI editor window
cargo test -p phototux_engine
./scripts/check-rust.sh        # rustfmt + clippy (+ rust-doctor when CHECK_RUST_FULL=1)
```

Requires Qt **6.10+** on PATH. First launch opens **New Document** (presets 720p / 1080p / 2K / 4K).

## Agent & quality gate

- **`AGENTS.md`** — coding constitution (Rust skills, UI skills, ADR stack, doctrine).
- **Pre-commit:** `./scripts/install-git-hooks.sh` then every commit runs `scripts/check-rust.sh`.

## Next Steps

1. Confirm Decision Register promotions in [gap analysis §8](internal_docs/Appendix/Codebase-Handbook-Gap-Analysis.md) (Qt, `.ptx`, single-doc, zero-copy).
2. Phase α alignment: thin command router over existing ops (see gap analysis §6.3).
3. Multi-doc / Shape layers only after explicit Decision Register amendments.
