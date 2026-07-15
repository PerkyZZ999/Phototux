# ADR-011: Document Model Timing (Layer Graph)

## Status

Accepted

## Context

SPEC Phase 3 introduces a non-destructive image state graph, blend modes, and undo. Phase 1–2 need a working shell and GPU viewport without boiling the ocean.

## Devil's advocate

**Case for graph in Phase 1:** Avoid rewrite; layers “ready.”  
**Hidden cost:** Interop and shell delayed; untested abstractions.  
**Failure mode:** Graph designed against wrong GPU constraints.  
**Case for never graphing:** Simpler forever; fights product roadmap.

## Options Considered

### Option 1: Phase 3 only for full graph; Phase 1–2 simple canvas

- **Pros**: Critical path = shell + zero-copy viewport first
- **Cons**: Some API evolution when layers land
- **Reversibility**: Medium

### Option 2: Full graph in Phase 1

- **Pros**: Early structure
- **Cons**: Scope explosion
- **Reversibility**: Hard

### Option 3: Mutable bitmap stack forever

- **Pros**: Simple
- **Cons**: Blocks non-destructive vision
- **Reversibility**: Hard

## Decision

**Option 1.** Owner lock (grill R2): **G11 = A**.

- **Phase 1–2:** Camera (pan/zoom), single (or minimal) GPU texture/document surface; engine types may use names that won’t embarrass later (`Document`, `LayerId` stubs OK) but **no** full blend graph, no multi-layer composite pipeline.
- **Phase 3:** Image state graph, WGSL blends, transactional undo.
- **Do not** invent a permanent “flat bitmap only” API that cannot grow.

## Consequences

- **Positive**: Phase 2 interop gets focus
- **Negative**: Layer UI may be placeholder until Phase 3–4
- **Neutral**: IA Layers panel can show mock rows in Phase 1

## Revisit Date

Start of Phase 3.

## Dependencies

- **Depends on**: ADR-004, ADR-006, ADR-008
- **Blocks**: Phase 3 checklist detail

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| 2026-07-15 | Accepted G11=A | Interactive grill R2 |
