# ADR-001: Platform Scope (Linux / Wayland)

## Status

Accepted

## Context

PhotoTux targets high-refresh creative work with tablet input and dense desktop chrome. Cross-platform early dilutes zero-copy and portal work.

## Devil's advocate

**Case for multi-platform early:** Larger audience; macOS Metal path via wgpu is real.  
**Hidden cost of Linux-only:** Contributor pool smaller; no “it runs on my Mac” demos.  
**Failure mode:** If Linux Vulkan external-memory path is uniquely broken, nowhere to hide.  
**Reversibility:** Medium — abstract GPU early; UI is Qt (already multi-OS capable later).

**Defense (SPEC + constraints):** Hard constraint. Wayland tablet + KDE HIG + Vulkan native are the product. Port later only after SLO path proven.

**Owner lock (grill 2026-07-15):** **G1 = A** — Linux/Wayland only for v1.

## Options Considered

### Option 1: Linux Wayland only (v1)

- **Pros**: Focus; matches host; portals/tablet first-class
- **Cons**: No other OS until later
- **Reversibility**: Medium

### Option 2: Linux + Windows/macOS from day one

- **Pros**: Broader testing surface
- **Cons**: Explodes FFI/RHI matrix; violates resource constraint
- **Reversibility**: Hard once promised

### Option 3: Linux X11 primary

- **Pros**: Older stack familiarity
- **Cons**: Fights tablet/high-DPI future
- **Reversibility**: Medium

## Decision

**Option 1.** Primary host: Arch/CachyOS + KDE Plasma Wayland. X11 not a target.

## Consequences

- **Positive**: Clear QA surface; deep Wayland integration
- **Negative**: No non-Linux CI requirement initially
- **Neutral**: Qt/wgpu still portable if revisited

## Revisit Date

After Phase 5 desktop integration milestone, or if a funded port appears.

## Dependencies

- **Depends on**: none
- **Blocks**: ADR-002, ADR-004, ADR-005

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| 2026-07-15 | Confirmed Option 1 in interactive grill (G1=A) | Owner explicit lock |
