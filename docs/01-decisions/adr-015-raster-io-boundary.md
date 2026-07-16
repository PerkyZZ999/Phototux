# ADR-015: Raster I/O and GPU Transfer Boundary

## Status

Accepted

## Context

Phase 5 needs native image open and export without weakening ADR-005. PNG/JPEG codecs operate on CPU memory, while the authoritative interactive document and composite live on the GPU. PhotoTux has no versioned project-file format yet, so raster export must not be mislabeled as lossless document save.

## Options Considered

### Option 1: CPU codec at explicit file-operation boundaries

- Decode/encode on a background worker.
- Upload decoded pixels once when opening.
- Read back the flattened composite once when exporting.
- Keep the steady-state viewport and painting path zero-copy.

**Pros:** Mature codecs, clear boundary, compatible with portals, no raw pixels over Qt FFI.  
**Cons:** Open/export cost scales with image size.  
**Reversibility:** Easy to replace individual codecs later.

### Option 2: GPU-native codecs

**Pros:** Potentially lower transfer cost.  
**Cons:** Large subsystem, weak PNG/JPEG ecosystem fit, no Phase 5 value.  
**Reversibility:** Hard.

### Option 3: Per-frame CPU mirror

**Pros:** Export appears simple.  
**Cons:** Violates ADR-005, doubles memory, creates stale-authority bugs.  
**Reversibility:** Hard after adoption.

## Decision

Choose **Option 1**.

### Supported release-slice operations

| Operation | Contract |
|-----------|----------|
| Open PNG/JPEG | Decode off UI thread, apply EXIF orientation, normalize to 8-bit RGBA sRGB, create one flattened editable layer, upload once |
| Export PNG | One-shot composite readback; preserve RGBA |
| Export JPEG | One-shot composite readback; flatten alpha over white; quality 92 |
| Save | Disabled until a native, versioned document format has its own ADR |

Use the Rust `image` crate with default features disabled and only PNG/JPEG codecs enabled. Decode limits: maximum dimension 32,768, maximum decoded RGBA allocation 512 MiB, and checked width×height arithmetic. Unsupported color/profile metadata is normalized or rejected with a user-visible typed error; ICC/HDR preservation is deferred.

All codec and filesystem work runs asynchronously. Rust owns pixel buffers. QML receives paths, progress/state, dimensions, and typed error text only—never raw pixels.

Output uses a temporary sibling file followed by rename when the portal-provided destination supports it. Failed export must not truncate an existing destination.

### ADR-005 interpretation

Allowed:

- one CPU decode plus one GPU upload on explicit Open;
- one GPU readback plus one CPU encode on explicit Export;
- tests/debug readback.

Forbidden:

- steady-state CPU document mirror;
- per-frame readback or upload;
- raw full-frame pixels crossing Rust↔Qt FFI.

## Consequences

- **Positive:** File I/O is testable as pure Rust; interactive zero-copy path remains intact.
- **Negative:** Very large open/export operations need progress and cancellation later.
- **Neutral:** Native project save, ICC round-trip, 16-bit/HDR, and multi-layer interchange remain deferred.

## Revisit Date

Before native project save or 16-bit/HDR import/export.

## Dependencies

- **Depends on:** ADR-001, ADR-005, ADR-007, ADR-011, ADR-014
- **Blocks:** Phase 5 Open/Export acceptance

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| 2026-07-15 | Accepted | Phase 5 release-slice boundary |
