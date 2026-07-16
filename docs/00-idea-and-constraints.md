# Idea & Constraints Snapshot

Living snapshot of project foundation at inception. Original sources: `README.md`, `CONSTRAINTS.md`, `SPEC.md`.

**Captured:** 2026-07-15

## Idea (summary)

PhotoTux = professional raster/vector image editor for Linux Wayland. Rust engine + Qt Quick shell. Zero-copy GPU compositing via `wgpu`/Vulkan imported into Qt RHI. Fills gap between capable but heavy open tools and a high-refresh, low-latency native creative workspace on KDE Plasma 6.

## Value

Pixels stay on GPU. Bridge carries light commands. UI stays dense and desktop-native. Target: ≥60 FPS (path to 120/144Hz), <8 ms tablet latency, <1,000 ms cold-boot gate (<250 ms stretch), <2 ms for 10-layer 4K composite.

## MVP

Phases 1–2: qtbridge bootstrap + QML skeleton + custom QQuickItem + wgpu viewport with ≥60 FPS zoom/pan.

## Hard constraints

- Linux / Wayland
- Rust + Qt 6 QML
- Zero-copy GPU canvas
- Performance SLOs as acceptance gates
- **Desktop GUI only** (no CLI/TUI product for v1) — ADR-014

## Soft constraints

- `qtbridge` app logic + hybrid canvas if needed
- KDE HIG dense dark UI (Controls 2; Kirigami deferred)
- Arch/CachyOS reference host
- Vulkan-first wgpu

## Full docs

- [README.md](../README.md)
- [CONSTRAINTS.md](../CONSTRAINTS.md)
- [SPEC.md](../SPEC.md)
- [AGENTS.md](../AGENTS.md)
- [01-decisions/](./01-decisions/) (ADR-001…014)
