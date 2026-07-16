# ADR-016: Native `.ptx` Document Format

## Status

Accepted

## Context

ADR-015 deferred native project save until a versioned document format existed. Professional raster work (layers, masks, transforms, adjustments, history-safe dirty state) requires an authoritative container distinct from flattened PNG/JPEG export. The format must preserve GPU-authored pixels at explicit save boundaries without introducing a steady-state CPU mirror (ADR-005).

## Options Considered

### Option 1: Versioned binary container (chosen)

- Magic + schema version + deflate-compressed JSON manifest + per-layer PNG assets
- Atomic temp-sibling write + rename
- Forward-version rejection; corruption diagnostics; migration hooks

**Pros:** Simple, testable, no new archive ecosystem risk.  
**Cons:** Not a standard archive; tooling must use PhotoTux or dedicated readers.

### Option 2: ZIP/`OpenRaster`-like package

**Pros:** Inspectable with common tools.  
**Cons:** Extra dependency surface; ORA semantics diverge from typed graph v2.

### Option 3: Embed in PSD as native

**Pros:** Interchange convenience.  
**Cons:** PSD is lossy/complex; must not be authoritative (Phase 12 subset only).

## Decision

Choose **Option 1**. File extension `.ptx`. MIME hint `application/x-phototux-document`.

### Contract

| Field | Rule |
|-------|------|
| Magic | `PHOTOTUX` (8 bytes) |
| Schema | `u32` little-endian; v1 = current |
| Manifest | Deflate JSON: document id, size, active layer, typed nodes, next_id |
| Assets | Deflate PNG blobs keyed by layer id for raster nodes |
| Write | Temp sibling → fsync → rename; failed save must not damage prior file |
| Read | Reject unknown future major schema; surface typed errors |

CPU transfer occurs only on Save/Open/Recover. Interactive pixels remain GPU-authoritative.

Crash recovery uses a sibling journal (`.ptx.journal` / autosave directory) that stores the last successful snapshot metadata and optional asset blobs; recovery never silently overwrites the user’s last explicit Save.

## Consequences

- **Positive:** Layered round-trip; Save enabled in UI; export remains flattened interchange.
- **Negative:** Schema migrations required as graph v2 grows (masks, text, filters).
- **Neutral:** PSD import/export stays a separate ADR-018 track; `.ptx` remains authoritative.

## Revisit Date

Before 16-bit/HDR texture formats or multi-document sessions.

## Dependencies

- ADR-005 (zero-copy), ADR-011 (graph), ADR-015 (I/O boundary), ADR-017 (graph v2 / history)
