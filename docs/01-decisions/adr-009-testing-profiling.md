# ADR-009: Testing & Profiling Tooling

## Status

Accepted

## Context

GPU + QML app needs layered verification without requiring GPU in every unit test.

## Devil's advocate

**Case for “manual only until late”:** Speed.  
**Hidden cost:** Engine regressions invisible.  
**Failure mode:** Untestable blob.  
**Reversibility:** Medium.

## Options Considered

### Option 1: Layered — pure Rust tests + optional GPU + manual QML + perf HUD

- **Pros**: Matches architecture
- **Cons**: GPU CI optional/flaky
- **Reversibility**: Easy

### Option 2: Full Qt Test automation from day one

- **Pros**: UI coverage
- **Cons**: Heavy for beta bridge
- **Reversibility**: Medium

## Decision

**Option 1.** Owner lock (grill R2): **G10 = A**.

- `phototux-engine`: unit + property tests (no Qt)
- `phototux-gpu`: shader/pipeline tests behind feature `gpu-tests` when device present
- QML: manual checklist Phase 1; automate later if stable
- Profiling: `tracing`, debug FPS overlay, Tracy when optimizing
- **No** ship of features that only pass visual vibe-check against ADR-008 gates when those gates apply

## Consequences

- **Positive**: Fast feedback on pure logic
- **Negative**: UI regressions more manual early
- **Neutral**: CI can be Linux host with optional GPU job later

## Revisit Date

End of Phase 3 (when blend correctness needs golden images).

## Dependencies

- **Depends on**: ADR-006, ADR-008
- **Blocks**: none

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| | | |
