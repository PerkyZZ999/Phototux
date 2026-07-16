# ADR-006: Cargo Workspace Layout

## Status

Accepted

## Context

Need clear boundaries: app binary, UI bridge types, engine, canvas interop, shared types.

## Devil's advocate

**Case for monocrates:** Faster start.  
**Hidden cost:** Circular deps; hard to test engine headless.  
**Failure mode:** Everything in `main` → rewrite at Phase 3.  
**Reversibility:** Easy early, Hard later.

## Options Considered

### Option 1: Multi-crate workspace

- **Pros**: Testable engine; isolate unsafe interop
- **Cons**: Slight bootstrap overhead
- **Reversibility**: Easy

### Option 2: Single crate

- **Pros**: Simple
- **Cons**: No boundaries
- **Reversibility**: Medium

## Decision

**Option 1 — multi-crate, strict naming.** Owner lock (grill R2): **G6 = A**.

### Naming rules (logical & organised)

| Directory (kebab) | Cargo package name | Rust crate import | Role |
|-------------------|--------------------|-------------------|------|
| `crates/phototux/` | `phototux` | binary only | `QApp` entry, wires deps |
| `crates/phototux-ui/` | `phototux_ui` | `phototux_ui` | qtbridge QObjects, models (no wgpu) |
| `crates/phototux-engine/` | `phototux_engine` | `phototux_engine` | pure Rust state (**no Qt**) |
| `crates/phototux-gpu/` | `phototux_gpu` | `phototux_gpu` | wgpu pipelines, WGSL assets |
| `crates/phototux-canvas/` | `phototux_canvas` | `phototux_canvas` | QQuick/RHI interop (± thin C++) |

- Prefer **`phototux_` prefix** on all library packages; binary is plain `phototux`.
- Hyphen in **paths**, underscore in **package/crate** names (Cargo convention).
- No free-floating `utils` / `common` crate until a third consumer needs it — prefer the owning crate.
- QML module import follows package name of the crate that owns `#[qobject]` (e.g. `import phototux_ui`).

### Tree

```
PhotoTux/
├── Cargo.toml                 # workspace
├── crates/
│   ├── phototux/              # binary: QApp entry
│   ├── phototux-ui/           # qtbridge QObjects, models
│   ├── phototux-engine/       # pure Rust document/canvas state (no Qt)
│   ├── phototux-gpu/          # wgpu pipelines, shaders
│   └── phototux-canvas/       # QQuick item interop (may include C++ later)
├── qml/                       # QML assets (tokens from DESIGN.md)
└── assets/                    # icons, brushes later
```

### Create when

| Phase | Crates that must exist |
|-------|------------------------|
| Phase 1 | `phototux`, `phototux_ui`, `phototux_engine` |
| Phase 1.5 spike | may add minimal gpu/canvas **on spike branch** |
| Phase 2+ | `phototux_gpu`, `phototux_canvas` on main |

## Consequences

- **Positive**: Engine unit-testable without Qt; canvas unsafe isolated
- **Negative**: More `Cargo.toml` files
- **Neutral**: QML at repo root `qml/` for clarity

## Revisit Date

End of Phase 2 (if interop forces different package boundaries).

## Dependencies

- **Depends on**: ADR-003, ADR-004
- **Blocks**: build checklist structure

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| 2026-07-15 | G6=A; naming table + phased create | Interactive grill R2 |
