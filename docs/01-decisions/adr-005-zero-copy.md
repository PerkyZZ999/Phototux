# ADR-005: Zero-Copy Compositing Strategy

## Status

Accepted

## Context

Hard pillar: raw pixels never traverse FFI. Canvas texture lives on GPU; Qt displays via imported texture / shared image.

## Devil's advocate

**Case for double-buffered CPU staging “just for MVP”:** Ships demos faster.  
**Hidden cost:** Becomes permanent; kills latency SLO; teaches wrong architecture.  
**Failure mode:** Drivers refuse external memory → project stalls (spike skipped).  
**Reversibility of accepting copies:** Easy to add, **Hard to remove** once UI depends on it.

**Defense:** Hard constraint. Debug-only readback allowed; never default path.

## Options Considered

### Option 1: Shared GPU image (external memory / DMA-BUF) into Qt RHI / QSGTexture

- **Pros**: Meets pillar + SLOs
- **Cons**: Highest implementation risk
- **Reversibility**: Hard once working (good)

### Option 2: Per-frame CPU upload (QImage / staging buffer)

- **Pros**: Easy
- **Cons**: **Fails hard constraint**
- **Reversibility**: Easy

### Option 3: Separate native window for canvas

- **Pros**: Avoids QSG import
- **Cons**: Broken desktop UX; focus/input hell
- **Reversibility**: Medium

## Decision

**Option 1** as the only shippable path.

- **Allowed:** Debug/test CPU readback behind `cfg` or explicit debug flag; golden tests may read back.
- **Forbidden:** Steady-state interactive view using full-frame CPU copies.
- **Interop order of attempt:** (1) shared Vulkan image / external memory with Qt RHI, (2) DMA-BUF FD import, (3) ADR amendment if both fail.

Spike status: **skipped by owner** — Phase 2 first vertical slice owns validation.

## Consequences

- **Positive**: Architecture honesty; SLO path possible
- **Negative**: Phase 2 schedule risk elevated
- **Neutral**: Thumbnail generation may still use downscaled readback

## Revisit Date

First successful Phase 2 present path, or after 2 failed interop approaches.

## Dependencies

- **Depends on**: ADR-002, ADR-003, ADR-004
- **Blocks**: Phase 2 acceptance, ADR-008 measurement of composite budget

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| | | |
