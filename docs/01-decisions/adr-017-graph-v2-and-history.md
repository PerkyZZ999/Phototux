# ADR-017: Document Graph v2 and Transactional History

## Status

Accepted

## Context

Phase 3 shipped a flat `Vec<Layer>` metadata graph and a separate GPU stroke snapshot stack. Professional raster features need typed nodes, hierarchy, masks/transforms metadata, and one chronological undo timeline (ADR-013 gesture granularity).

## Decision

### Graph v2

- Stable `LayerId` within a document; document-scoped `document_id` (`u128`) for recovery identity.
- Typed `LayerKind`: `Raster`, `Group`, `Text`, `Adjustment` (extensible).
- Shared node fields: name, opacity, visibility, locks, blend, parent id, transform, optional mask metadata, optional effect stack metadata.
- Pixels remain GPU-authoritative; graph stores asset keys for serialization only.
- Capability validation rejects unsupported combinations at command boundaries.

### History

- Every user gesture is one `HistoryEntry` on a bounded timeline.
- Graph mutations store invertible `GraphCommand` payloads.
- Stroke commits store markers that restore via GPU texture snapshots (same budget as today; later tile/delta).
- Undo/Redo follows timeline order, not “all strokes then all graph.”
- History panel shows human labels; serialization of full pixel undo stacks into `.ptx` is out of scope for v1 (dirty flag + graph + pixels only).

## Consequences

- UI Undo/Redo must consult the unified timeline.
- Compositor may ignore non-raster nodes until their GPU paths land; metadata still round-trips in `.ptx`.
- Groups/masks/text/adjustments land incrementally without another ADR unless the kind set changes.

## Revisit Date

After Phase 10 filter stacks stabilize (tile/delta undo budget).

## Dependencies

- ADR-011, ADR-013, ADR-016
