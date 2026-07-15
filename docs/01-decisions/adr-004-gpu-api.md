# ADR-004: GPU API — wgpu (Vulkan-first)

## Status

Accepted

## Context

Engine needs GPU textures, compute blend modes, high-FPS viewport. Linux primary → Vulkan.

## Devil's advocate

**Case for vulkano/ash only:** Cleaner external-memory control for Qt sharing.  
**Case for Qt RHI shaders only:** One graphics stack; no second device.  
**Hidden cost of wgpu:** Abstraction may hide import handles; dual-stack with Qt RHI.  
**Failure mode:** Cannot import wgpu texture into QSG → force CPU path or rewrite on ash.  
**Reversibility:** Medium if engine behind trait; Hard if WGSL everywhere without abstraction.

**Defense:** SPEC + ecosystem + WGSL compute for Phase 3. Keep Vulkan escape hatch. Pure Qt RHI for all painting abandons Rust engine pillar.

## Options Considered

### Option 1: wgpu 30, Vulkan backend preferred

- **Pros**: Safe API; compute; portable
- **Cons**: Interop friction with Qt
- **Reversibility**: Medium

### Option 2: vulkano

- **Pros**: Direct Vulkan
- **Cons**: Less ecosystem than wgpu; no WGSL
- **Reversibility**: Medium

### Option 3: ash raw + custom

- **Pros**: Max control
- **Cons**: Unsafe surface; slow delivery
- **Reversibility**: Hard

### Option 4: Qt RHI only (no wgpu)

- **Pros**: Single GPU stack
- **Cons**: Engine logic in C++/Qt; abandons Rust GPU story
- **Reversibility**: Hard

## Decision

**Option 1.** Pin `wgpu` major **30**. Prefer Vulkan on Linux. Optional thin `ash` only inside interop module for external memory if required.

## Consequences

- **Positive**: Engine productivity; Phase 3 compute path
- **Negative**: Must solve cross-API texture share
- **Neutral**: Other backends unused on v1

## Revisit Date

Phase 2 interop checkpoint; if blocked >1 week continuous, re-grill Option 2/3 for interop layer only.

## Dependencies

- **Depends on**: ADR-001
- **Blocks**: ADR-005, ADR-008

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| | | |
