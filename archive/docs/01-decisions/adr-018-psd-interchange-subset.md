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

Current implementation (2026-07-16): PSD v1 RGB 8-bit import with Raw/PackBits composite and simple raster layer channels; subset PSD export (Raw, no groups/masks/effects). Compatibility report always discloses subset limits; ZIP/16-bit/non-RGB fail or warn without silent garbage. No lossless round-trip claim.

## Consequences

- Users see warnings instead of silent drops.
- Fixtures and golden tests gate expansion of the subset.
- 16-bit/HDR and ICC policy remain under a future color-depth ADR.

## Revisit Date

After Phase 10 filter stacks and `.ptx` migrations stabilize.

## Dependencies

- ADR-016, ADR-017, ADR-015
