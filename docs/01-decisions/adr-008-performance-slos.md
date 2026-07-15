# ADR-008: Performance SLOs as Acceptance Gates

## Status

Accepted

## Context

SPEC defines numeric budgets. Without gates, “feels fine” ships regressions.

## Devil's advocate

**Case for qualitative “smooth enough”:** Faster iteration solo.  
**Hidden cost:** No regression signal; Phase 5 HDR work unconstrained.  
**Failure mode:** 30 FPS becomes normal.  
**Reversibility:** Easy to add metrics later but lost baseline history.

## Options Considered

### Option 1: Enforce SPEC SLOs as phase gates

- **Pros**: Honest product
- **Cons**: Harder Phase 2
- **Reversibility**: Easy (process)

### Option 2: Soft goals only

- **Pros**: Flexible
- **Cons**: Non-goals become reality
- **Reversibility**: Easy

## Decision

**Option 1.** Gates. Owner lock (grill R2): **G9 = A** — smooth/fluid UX; **≥60 FPS** for zoom/pan and later brush strokes is a **real exit criterion**, not a vibe.

| SLO | Target | Enforced from |
|-----|--------|---------------|
| Steady-state FPS zoom/pan | ≥ 60 | Phase 2 exit |
| Steady-state FPS active brush (when tools land) | ≥ 60 | Phase 4 exit (same floor) |
| High-refresh path | design for 120/144 | Phase 2+ capability, not Phase 2 gate |
| Input-to-render (tablet) | < 8 ms | Phase 4 exit |
| Cold boot interactive | < 250 ms | Phase 5 target; measure from Phase 1 |
| 10-layer 4K composite | < 2 ms GPU | Phase 3 exit |
| Zero-copy hot path | no full-frame CPU upload | Phase 2+ continuous |

Instrument early: frame time HUD (debug), `tracing`, Tracy optional, wgpu timestamps for composite.

## Consequences

- **Positive**: Clear done definition
- **Negative**: May delay “pretty demos”
- **Neutral**: Numbers may tighten, not loosen, without ADR amendment

## Revisit Date

Each phase exit review.

## Dependencies

- **Depends on**: ADR-004, ADR-005
- **Blocks**: phase exit checklists

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| | | |
