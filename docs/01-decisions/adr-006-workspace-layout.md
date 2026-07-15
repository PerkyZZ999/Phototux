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

**Option 1.** Layout:

```
PhotoTux/
├── Cargo.toml                 # workspace
├── crates/
│   ├── phototux/              # binary: QApp entry
│   ├── phototux-ui/           # qtbridge QObjects, models
│   ├── phototux-engine/       # pure Rust document/canvas state (no Qt)
│   ├── phototux-gpu/          # wgpu pipelines, shaders
│   └── phototux-canvas/       # QQuick item interop (may include C++ later)
├── qml/                       # QML assets
└── assets/
```

Phase 1 may start with `phototux` + `phototux-ui` + stub engine; add gpu/canvas crates when Phase 2 starts — **crate names reserved now**.

## Consequences

- **Positive**: Engine unit-testable without Qt
- **Negative**: More `Cargo.toml` files
- **Neutral**: QML lives at repo root `qml/` for clarity

## Revisit Date

End of Phase 2 (if interop forces different package boundaries).

## Dependencies

- **Depends on**: ADR-003, ADR-004
- **Blocks**: build checklist structure

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| | | |
