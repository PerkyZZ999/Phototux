# PhotoTux

Professional-grade, high-performance raster/vector image editor for modern Linux (Wayland).

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
| 3 | Cold boot to interactive workspace | < 250 ms | Process start → first interactive frame |
| 4 | 10-layer 4K compositing budget | < 2 ms GPU | GPU timestamp queries on blend pass |
| 5 | Zero-copy canvas path | No CPU pixel upload for steady-state view | Architecture + runtime assert: no full-frame `Map`/readback in hot path |

## Constraints

See [CONSTRAINTS.md](CONSTRAINTS.md). Product intent and architecture pillars: [SPEC.md](SPEC.md).

## Next Steps

1. `stack-probe` — research/validate Rust, Qt 6 QML, `qtbridge-rust`, `wgpu`, Wayland stack
2. `grill-with-docs` — lock ADRs
3. Spike skipped by request → `lock-and-tag` → `agent-bootstrap` → build Phase 1
