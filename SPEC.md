# SPEC.md: PhotoTux Project Specification

> **Non-normative bridge.** Authoritative engineering contracts live in [`internal_docs/`](internal_docs/README.md) (Engineering Handbook) and the [Decision Register](internal_docs/Appendix/Decision-Register.md). Tech stack locks: [DR-023](internal_docs/Appendix/Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase). Former ADR ids → live DRs: [Archived-ADR-to-DR-Map.md](internal_docs/Appendix/Archived-ADR-to-DR-Map.md). Prefer handbook chapters over this file when they disagree.

## 1. Project Overview & Vision

**PhotoTux** is a professional-grade, high-performance raster and vector image editor designed natively for modern Linux systems running Wayland.

### The Vision

While applications like GIMP and Krita are powerful, the Linux desktop ecosystem lacks a modern, ultra-responsive creative suite that pairs the strict performance and safety guarantees of **Rust** with the highly polished UI layout mechanics of **KDE Plasma 6 / Qt Quick**. PhotoTux fills this void by offering a workspace designed from the ground up for massive, high-resolution canvas manipulation at **60+ FPS** (up to 120/144Hz) with minimal input latency.

### Key Architectural Pillars

* **Zero-Copy GPU Compositing:** All heavy pixel transformations, brush engines, and layer blending execute directly on the GPU using **Rust** and **`wgpu`** (targeting Vulkan natively).
* **KDE-Native Modern Shell:** A highly dense, dark-mode, multi-pane utility layout designed in **QML (Qt Quick)** that aligns with the KDE Plasma 6 visual style.
* **Official Rust Integration:** Powered by the cutting-edge **`qtbridge-rust`** framework, bypassing traditional manual C++ FFI bindings completely.

---

## 2. Technical Stack

| Layer | Technology | Purpose |
| --- | --- | --- |
| **Frontend UI** | **Qt 6 / QML (Qt Quick)** | Multi-pane layouts, toolbars, properties panel, collapsible docks, and dialog window menus. |
| **Backend Logic** | **Rust (Latest Stable)** | Image state graph, layer configurations, brush dynamics, and event orchestration. |
| **FFI Bridge** | **`qtbridge-rust` (Official Beta)** | Direct generation of native Qt wrappers from pure Rust structs using safe attributes (`#[qobject]`). |
| **Graphics Engine** | **`wgpu`** | Low-level GPU-accelerated canvas rendering, layer compositing shaders, and viewport panning/zooming. |
| **Host Target** | **Linux (Wayland Native)** | Smooth high-DPI desktop scaling, Wayland graphics tablet input streams, and native portals. |

---

## 3. High-Level Architecture

The principal performance challenge of combining a declarative UI with a high-speed canvas is preventing memory copying. PhotoTux implements a **decoupled render path** to ensure high framerates.

```
┌────────────────────────────────────────────────────────┐
│               QML FRONTEND (Qt Quick)                  │
│  - Dense Toolbars, Layers list, Slider UI components   │
└──────────────────────────┬─────────────────────────────┘
                           │  Passes UI Events & Signals
                           │  (Using safe #[qobject] macros)
┌──────────────────────────▼─────────────────────────────┐
│              qtbridge-rust Bridge Layer                │
│  - Bidirectional, compile-time Rust/QML FFI wrapper     │
└──────────────────────────┬─────────────────────────────┘
                           │  High-Level Controls
                           │  (e.g., DrawBrush(X, Y))
┌──────────────────────────▼─────────────────────────────┐
│                 RUST BACKEND ENGINE                    │
│  - Multi-threaded canvas state, Undo/Redo DAG tree     │
└──────────────────────────┬─────────────────────────────┘
                           │  Direct GPU draw commands
                           │  (wgpu / Vulkan API)
┌──────────────────────────▼─────────────────────────────┐
│                 GPU TEXTURE / CANVAS                   │
│  - Shared texture mapped directly into Qt RHI Viewport │
└────────────────────────────────────────────────────────┘

```

### The Rendering Strategy

1. **Rendering Target:** The Rust core allocates and manages the target canvas texture entirely in GPU memory using `wgpu`.
2. **Shared Canvas Context:** Qt's Rendering Hardware Interface (RHI) imports this native GPU texture handle directly as a `QSGTexture` inside a custom QML `QQuickItem`.
3. **No FFI Copying:** Raw pixel data never leaves the GPU or traverses the FFI bridge. The bridge only transmits light control commands (e.g., brush settings, layer opacity).

---

## 4. Feature Roadmap & Development Phases

The project is structured into five chronological milestones designed to systematically build and validate the application's performance.

```
┌─────────────────────────────────────────────────────────────────────────┐
│ PHASE 1: Toolchain & Pure Rust-Qt App Bootstrap                         │
│ - Set up a Cargo-driven workspace utilizing qtbridge                    │
│ - Implement a clean QML UI skeleton matching KDE HIG                    │
│ - Validate successful Rust-to-QML state binding (sliders, labels)       │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ PHASE 2: High-Performance GPU Viewport                                  │
│ - Create custom QQuickItem hooked into Qt RHI                           │
│ - Initialize Rust wgpu pipeline on top of active Vulkan instance        │
│ - Achieve stable 60+ FPS canvas zooming/panning viewport benchmark      │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ PHASE 3: Composite Layer Engine                                         │
│ - Design the non-destructive Image State Graph in Rust                  │
│ - Write WGSL compute shaders for standard Blend Modes (Multiply, etc.)  │
│ - Build a transactional, memory-efficient undo/redo system              │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ PHASE 4: Creative Brush Subsystem & Tools                               │
│ - Build dynamic brush core supporting Wayland tablet pressure curves     │
│ - Add essential tool states: selections, color pickers, and transform   │
│ - Develop a highly detailed, interactive Layer Panel in QML             │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ PHASE 5: Desktop Integration & Release                                  │
│ - Bind application actions to global KDE system menu bars               │
│ - Incorporate native file portals (XDG Portals) for image exports       │
│ - Profile and optimize rendering paths for HDR monitor configurations    │
└─────────────────────────────────────────────────────────────────────────┘

```

---

## 5. Performance Budgets & Service Level Objectives (SLOs)

* **Steady-State Frame Rate:** $\geq 60\text{ FPS}$ (supporting 120/144Hz high-refresh displays) during active zooming, panning, or heavy brush strokes.
* **Input-to-Render Latency:** $< 8\text{ ms}$ on standard graphics tablets.
* **Cold Boot Execution Time:** $< 1000\text{ ms}$ Phase 5 gate to a fully interactive editor workspace; $< 250\text{ ms}$ remains the stretch target.
* **Compositing Budget:** All calculations for blending a 10-layer 4K image ($3840 \times 2160$ pixels) must complete on the GPU in under **$2\text{ ms}$**.

---

## 6. Development Environment & Verification Setup

### Recommended Host Requirements

* **Operating System:** Arch Linux / CachyOS.
* **Display Compositor:** Wayland (native scaling).
* **Terminal Environment:** Ghostty, ZSH, and TMUX.

### Compilation Dependencies

```bash
# Install core compiler and Qt 6 packages
sudo pacman -S rustup clang cmake qt6-base qt6-declarative vulkan-headers

```

### Verification (developer)

PhotoTux is a **desktop GUI** app (not a CLI/TUI product). Use **Qt 6** `qmake` on `PATH`, then from the workspace root:

```bash
export PATH=/usr/lib/qt6/bin:$PATH
export QMAKE=/usr/lib/qt6/bin/qmake
cargo run -p phototux    # launches the editor window (dev)
./scripts/check-rust.sh  # rustfmt + clippy + rust-doctor
```

Crate layout and phases: see [`internal_docs/`](internal_docs/README.md), [Alignment Roadmap](internal_docs/Appendix/Alignment-Roadmap.md), and [Implementation Checklist](internal_docs/Appendix/Implementation-Checklist.md). Former ADR ids → [Archived-ADR-to-DR-Map.md](internal_docs/Appendix/Archived-ADR-to-DR-Map.md).

---
