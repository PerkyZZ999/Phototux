# Stack Probe Dossier — PhotoTux

**Date:** 2026-07-15  
**Inputs:** `README.md`, `CONSTRAINTS.md`, `SPEC.md`  
**Host probe:** Qt **6.11.1**, rustc **1.95.0**, `qtbridge` **0.2.0**, `cxx-qt` **0.9.1**, `wgpu` **30.0.0**, `vulkaninfo` present

---

## 1. Executive summary

PhotoTux is constrained into a narrow stack: **Linux/Wayland**, **Rust engine**, **Qt 6 QML shell**, **zero-copy GPU canvas**. Research confirms that combination is **coherent but high-risk at the GPU↔Qt seam**.

**Qt Bridges for Rust (`qtbridge` 0.2)** is real official beta, crates.io-published, and **matches the host toolchain** (needs Qt ≥ 6.10, Rust ≥ 1.87). It is the right Phase 1 tool for QObjects, properties, slots, and QML models **without writing app C++**. It is **not proven** for custom Scene Graph / `QQuickRhiItem` zero-copy import — docs explicitly redirect to **CXX-Qt** when C++-only Qt APIs are required.

**wgpu 30** remains the correct engine abstraction for Vulkan-first Linux and WGSL blend compute. The hard problem is **sharing a GPU texture with Qt RHI** without CPU copies (DMA-BUF / external memory). That was the natural spike target; spike **skipped by request** — Phase 2 must treat interop as first vertical risk, with a **documented fallback** (thin C++ `QQuickRhiItem` + handle import, or temporary debug path that is **not** allowed to ship).

Alternatives **GTK/libadwaita**, **iced/egui/Slint**, **OpenGL FBO copy**, and **stale qmetaobject-first** fail hard constraints or product vision. **CXX-Qt 0.9.1** is the strongest **Plan B / hybrid** for the canvas item only.

---

## 2. Decision points researched

| # | Decision point | Detail doc |
|---|----------------|------------|
| 1 | UI shell | [ui-shell.md](./ui-shell.md) |
| 2 | FFI bridge | [ffi-bridge.md](./ffi-bridge.md) |
| 3 | Graphics engine | [graphics-engine.md](./graphics-engine.md) |
| 4 | Engine architecture / undo / threads / test / obs | [engine-architecture.md](./engine-architecture.md) |

Skipped as N/A for MVP: cloud auth, multi-tenant DB, web deployment.

---

## 3. Top candidate per decision point

| Decision | Top candidate | Rationale | Risk |
|----------|---------------|-----------|------|
| UI shell | Qt 6.11 QML (Controls 2, dense Breeze-dark) | Only stack matching KDE-native multi-pane + RHI | Low |
| FFI (app logic) | `qtbridge` 0.2.x | Official, pure Rust macros, host-compatible | Med (beta churn) |
| FFI (canvas item) | Hybrid: C++/`cxx-qt` QQuickRhiItem if needed | qtbridge may not cover custom items | **High** |
| GPU engine | `wgpu` 30, Vulkan backend | SPEC, WGSL compute, ecosystem | Med (interop) |
| Doc model (post-MVP) | Graph + transactional undo | Phase 3 roadmap | Med |
| Threading | UI commands → engine worker queue | Protect UI thread / latency SLO | Med |
| Testing | Rust unit + GPU golden later + FPS HUD | Match SLOs | Low |
| Observability | `tracing` + Tracy + GPU timestamps | SLO proof | Low |

---

## 4. Compatibility matrix (stack-level)

| Stack combo | Hard constraints | Success criteria path | Reversibility | Overall risk |
|-------------|------------------|----------------------|---------------|--------------|
| qtbridge + QML + wgpu + zero-copy RHI | Pass if interop solved | Pass | Hard | **High** (interop unknown) |
| qtbridge + QML + wgpu + CPU upload | **Fail** zero-copy hard constraint | Fail FPS/latency long-term | Easy | Disqualified for ship |
| CXX-Qt + QML + wgpu + RHI item | Pass | Pass | Hard | Medium |
| GTK-rs + wgpu | Fail KDE shell vision | Partial | Hard | Disqualified |
| Pure iced/egui + wgpu | Fail Qt/KDE shell | Partial | Medium | Disqualified |

---

## 5. Red flags & disqualifications

1. **CPU full-frame upload as product path** — fails hard constraint + SLOs.
2. **qtbridge beta** — API/docs churn; commercial pre-release legal notes for Qt commercial licensees (OSS: verify LICENSE files at vendor pin time).
3. **Custom QQuickItem may require C++** — SPEC “no C++” is **softened** to “no C++ for app logic”; canvas boundary may need ≤1 thin C++ type.
4. **Spike skipped** — highest technical uncertainty unvalidated in code.
5. **RefCell borrow panics** across QML re-entrancy — design command queue carefully.
6. **Private Qt headers** — may need `qt6-base` private devel packages on Arch for advanced RHI.

---

## 6. Open questions before lock

1. Confirm whether `qtbridge` 0.2 can register external `QQuickItem` types or only Rust `#[qobject]` types.
2. Choose zero-copy primitive: external Vulkan memory vs DMA-BUF on Wayland.
3. Single shared VkDevice with Qt RHI vs texture import across devices.
4. Kirigami vs pure Controls 2 for dense desktop chrome.
5. Exact crate pins: `qtbridge = "=0.2.0"`, `wgpu = "30"`.
6. License compliance check of qtbridge + system Qt for intended distribution (AUR/Flatpak later).

---

## 7. Suggested ADR set for grill phase

1. ADR-001: Platform (Linux/Wayland only for v1)
2. ADR-002: UI toolkit (Qt 6 QML)
3. ADR-003: FFI strategy (qtbridge primary, hybrid canvas allowed)
4. ADR-004: GPU API (wgpu Vulkan-first)
5. ADR-005: Zero-copy compositing strategy
6. ADR-006: Workspace crate layout
7. ADR-007: Threading & command queue
8. ADR-008: Performance SLO acceptance gates
9. ADR-009: Testing & profiling tooling

---

## 8. Research commit note

Research only — **no decisions locked**. Next: `grill-with-docs`.
