# AGENTS.md — PhotoTux Coding Constitution

> Governs AI agents and humans. Overrides general knowledge with locked ADRs (`decisions-locked-v1`).

## Tech Stack Rules

### Platform
- **Use**: Linux + Wayland only for v1 (Arch/CachyOS reference)
- **Never use**: Windows/macOS as build targets, X11-first paths
- **Rationale**: ADR-001

### UI shell
- **Use**: Qt **6.10+** / QML (Qt Quick Controls 2), dense desktop dark chrome (Breeze-inspired)
- **Never use**: GTK/libadwaita, iced, egui, Slint, Electron/web shell for main UI
- **Rationale**: ADR-002

### FFI / app logic
- **Use**: `qtbridge` **0.2.x** for QObjects, properties, slots, models
- **Allowed hybrid**: thin C++ or `cxx-qt` **only** for canvas `QQuickRhiItem` / RHI interop (`phototux-canvas`)
- **Never use**: qmetaobject as primary stack; large hand-written Qt C++ app layer
- **Rationale**: ADR-003

### GPU engine
- **Use**: `wgpu` **30.x**, Vulkan preferred on Linux
- **Allowed**: thin `ash`/Vulkan only inside interop module
- **Never use**: steady-state OpenGL FBO CPU round-trip as product path; Qt RHI as sole engine (abandons Rust GPU)
- **Rationale**: ADR-004, ADR-005

### Language
- **Use**: Rust stable (host ≥ 1.87; prefer latest stable)
- **QML** for presentation; keep business/document logic in Rust crates

## Code Patterns

### Crate boundaries (ADR-006)
- `phototux` — binary / `QApp` entry
- `phototux_ui` (dir `phototux-ui`) — qtbridge types only
- `phototux_engine` — pure Rust document/canvas state (**no Qt deps**)
- `phototux_gpu` — wgpu pipelines/shaders (from Phase 2)
- `phototux_canvas` — Scene Graph / RHI interop (unsafe + optional C++)
- Paths kebab-case; package names `phototux_*` underscore
- QML lives in repo-root `qml/`
- Controls 2 first; **no Kirigami** until ADR-002 need is documented
- Full layer graph only from Phase 3 (ADR-011)
- License: **GPL-3.0-or-later** (ADR-012)

### Threading (ADR-007)
- Shape UI→engine interactions as **commands**, not ad-hoc mutates
- Phase 1: sync slots OK for light property updates only
- No heavy GPU/composite work on UI thread long-term
- Never hold `RefCell` mut borrow across await or re-entrant QML calls

### Rendering (ADR-005, ADR-010)
- Canvas pixels stay on GPU
- Bridge carries commands/state only
- Debug readback only behind `cfg` / debug flag — never default interactive path
- **Mandatory** time-boxed interop spike after Phase 1, **before** Phase 2 production canvas (`docs/01-decisions/adr-010-interop-spike.md`)

### Styling
- Dense multi-pane editor layout; dark theme
- Prefer Qt Quick Controls 2; avoid mobile-first Kirigami layouts unless justified

### Error handling
- Engine: `Result` / thiserror-style; no panics for recoverable document errors
- FFI boundary: convert errors to user-visible signals/status; log with `tracing`

## Quality Gates

### Performance (ADR-008)
| Gate | Target | From phase |
|------|--------|------------|
| Zoom/pan FPS | ≥ 60 | Phase 2 exit |
| Tablet input→render | < 8 ms | Phase 4 exit |
| Cold boot interactive | < 250 ms | Phase 5 (measure earlier) |
| 10-layer 4K composite | < 2 ms GPU | Phase 3 exit |
| Hot path copies | No full-frame CPU upload | Phase 2+ |

### Testing (ADR-009)
- Unit tests for `phototux-engine` required for graph/undo logic when present
- GPU tests optional feature `gpu-tests`
- QML: manual checklist until bridge stable
- Prefer `cargo test` for pure Rust before GUI runs

### Safety
- Confine `unsafe` to `phototux-canvas` / interop
- No secrets in repo

### Build
- `qmake` (or distro Qt) on PATH; system Qt 6 packages OK
- Prefer `cargo run -p phototux` from workspace root

## Decision Boundaries

### Dependency policy
- **Allowed without new ADR**: `qtbridge` 0.2.x, `wgpu` 30.x, `tracing`, `thiserror`, `serde` as needed, tiny pure-Rust utils
- **Requires ADR amendment**: alternate UI toolkit, primary FFI switch, abandoning zero-copy, adding network/cloud backends
- **Forbidden**: Electron, full-frame CPU canvas path as default, non-Linux v1 scope creep

### Architecture boundaries
- Do not put document graph logic inside QML
- Do not put wgpu inside `phototux-ui`
- Do not expand C++ beyond canvas interop without ADR-003 amendment
- Do not skip ADR-008 gates at phase exit

## Revisit Triggers

Flag for ADR review (do not silent-pivot) if:

1. qtbridge cannot register custom item / blocks Phase 2 → ADR-003
2. Zero-copy interop fails two approaches → ADR-005
3. wgpu cannot share memory with Qt RHI → ADR-004 interop layer
4. RefCell panics force architecture change → ADR-007
5. SLO unachievable on reference hardware → ADR-008
6. Need new major dependency outside allowed list

## Conflict Resolution

1. More recent ADR wins  
2. Log in `docs/04-journal/conflicts.md`  
3. Surface conflict in agent output — never silent pick  

## Workflow reminder

- Living checklists: `docs/03-checklists/`
- Spec: `SPEC.md` | Constraints: `CONSTRAINTS.md` | Research: `docs/00-research/DOSSIER.md`
- Baseline tag: `decisions-locked-v1`
