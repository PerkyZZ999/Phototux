# ADR-018: Layered PSD Interchange Subset

## Status

Accepted

## Context

Professional raster workflows often exchange Photoshop documents. Full PSD fidelity is intractable for v1. Native `.ptx` (ADR-016) remains authoritative; PSD is interchange with explicit disclosure.

## Decision

- Import a documented subset after graph v2 stabilizes: raster layers when available, hierarchy/opacity/common blends/masks/text where representable.
- Always show a **Compatibility Report** listing unsupported constructs (effects, smart objects, unsupported depths/modes).
- Never claim lossless round-trip for unsupported features.
- Export PSD later, only for the supported subset, validated against fixtures.
- Prefer Save to `.ptx` after import.

Current implementation validates the PSD signature/header and opens a placeholder raster layer with a full compatibility report when channel decompression is incomplete—so users never experience silent data loss.

## Consequences

- Users see warnings instead of silent drops.
- Fixtures and golden tests gate expansion of the subset.
- 16-bit/HDR and ICC policy remain under a future color-depth ADR.

## Revisit Date

After Phase 10 filter stacks and `.ptx` migrations stabilize.

## Dependencies

- ADR-016, ADR-017, ADR-015
