# Project Constraints

## Technical Constraints

| Constraint | Value | Reversibility | Rationale |
|------------|-------|---------------|-----------|
| Host OS | Linux only (Arch/CachyOS primary) | Hard — no Windows/macOS MVP | Spec targets modern Linux + Wayland |
| Display protocol | Wayland native | Hard for primary path | High-DPI, tablet streams, portals |
| Desktop alignment | KDE Plasma 6 visual/HIG style | Medium — QML can restyle | Dense dark multi-pane native feel |
| UI toolkit | Qt 6 / QML (Qt Quick) | Hard | Multi-pane shell, docks, menus |
| Backend language | Rust (stable) | Hard | Safety + perf for engine |
| FFI bridge | `qtbridge-rust` (official beta) | Medium–Hard | Spec pillar; alternatives = cxx-qt / manual FFI |
| GPU API path | `wgpu` → Vulkan preferred | Medium | Portable GPU; Vulkan native on Linux |
| Render strategy | Zero-copy GPU texture shared into Qt RHI / QSGTexture | Hard (product pillar) | No full-frame FFI pixel copies |
| Packaging target (later) | Desktop Linux app | Soft | Flatpak/AppImage/distro packages deferred |
| Product surface (v1) | **Desktop GUI only** | Hard for v1 | No CLI product, no TUI, no web (ADR-014) |

## Resource Constraints

| Constraint | Value | Impact on Scope |
|------------|-------|-----------------|
| Team size | Solo / agent-assisted | Vertical slices only; no parallel platform work |
| Budget | $0 OSS tooling | No proprietary SDKs; rely on system Qt + crates.io |
| Timeline | Phased milestones (1→5) | MVP = Phases 1–2 only |
| Host env | CachyOS/Arch, Wayland, KDE, ZSH/TMUX/Ghostty | Docs & scripts assume Arch package names |

## Quality Constraints

| Constraint | Target | Measurement |
|------------|--------|-------------|
| Steady-state FPS | ≥ 60 (aim 120/144 capable) | Viewport benchmark |
| Input latency | < 8 ms tablet path | Latency instrumentation |
| Cold boot | < 250 ms interactive | Startup timing |
| Compositing | 10×4K layers < 2 ms GPU | GPU timestamps |
| Memory discipline | No steady-state full-canvas CPU copies | Profile + code review |
| Safety | Prefer safe Rust; unsafe confined to GPU/FFI boundary | `unsafe` audit notes |

## Hard Constraints (Non-Negotiable)

1. Linux + Wayland primary platform for MVP and v1
2. Rust backend + Qt 6 QML frontend
3. Zero-copy GPU canvas strategy (pixels stay on GPU; bridge carries commands/state only)
4. Performance SLOs in README Success Criteria remain acceptance gates for canvas work
5. **Desktop GUI only** for MVP/v1 — no CLI product, no TUI, no web/Electron (ADR-014)

## Soft Constraints (Preferred but Reversible)

1. `qtbridge` for app logic; hybrid canvas C++ allowed (ADR-003) if custom item needs it
2. KDE HIG-aligned dense dark shell (not GNOME/libadwaita); Controls 2 first, Kirigami deferred
3. Arch/CachyOS as reference host; local checks only until public OSS (ADR-013)
4. Vulkan-first `wgpu` backend (Metal/DX12 irrelevant for now)

## Constraint Change Log

| Date | Constraint Changed | Reason | Approved By |
|------|-------------------|--------|-------------|
| 2026-07-15 | Initial constraints from SPEC.md | Project inception | Owner + agent |
| 2026-07-15 | Desktop GUI only (v1); soft FFI hybrid clarified | Owner + doc review | Owner + agent |
