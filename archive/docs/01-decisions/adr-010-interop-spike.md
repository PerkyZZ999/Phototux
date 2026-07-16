# ADR-010: Mandatory Interop Spike Before Phase 2

## Status

Accepted

## Context

ADR-005 requires zero-copy GPU present into Qt. ADR-003 allows hybrid canvas C++. The wgpu↔Qt RHI seam is the highest technical risk; an earlier project-wide spike skip left this unvalidated. Owner grill (G5) requires a **short, throwaway interop spike** before Phase 2 production canvas code.

## Devil's advocate

**Case for no spike:** Move faster into “real” app code; learn only in product branches.  
**Hidden cost:** Weeks of entangled QML/engine work before discovering import is impossible on target drivers.  
**Failure mode:** Pressure to ship CPU upload “temporarily.”  
**Reversibility of skipping spike:** Easy to skip, expensive to recover.

## Options Considered

### Option 1: Time-boxed throwaway spike before Phase 2 production

- **Pros**: Isolates risk; informs hybrid vs pure qtbridge; documents driver reality
- **Cons**: Short delay before feature work
- **Reversibility**: Easy (delete branch)

### Option 2: Learn only inside Phase 2 mainline

- **Pros**: No context switch
- **Cons**: Contaminates architecture; harder to abandon bad paths
- **Reversibility**: Medium

### Option 3: Skip validation entirely

- **Pros**: Fastest calendar
- **Cons**: Violates owner lock G5
- **Reversibility**: N/A

## Decision

**Option 1.**

### Spike rules

| Rule | Detail |
|------|--------|
| **When** | After Phase 1 shell/bootstrap works; **before** production `phototux-canvas` feature work merges as “done” |
| **Duration** | Time-box **≤ 3 working days** (or ~3 focused sessions); stop and write findings |
| **Branch** | `spike/wgpu-qt-rhi-interop` (throwaway OK) |
| **Success criteria** | Show **one** shared/imported GPU image (or DMA-BUF path) presented in a QML/Qt Quick item at interactive rates **without** full-frame CPU upload as the default path |
| **Secondary learnings** | Can qtbridge register the item, or is thin C++/`cxx-qt` required? Shared VkDevice vs import? NVIDIA/AMD notes on CachyOS host |
| **Outputs** | `docs/04-journal/spike-findings-interop.md` + ADR-003/005 amendments if needed |
| **Allowed in spike** | Ugly C++, private headers, minimal QML, no design polish |
| **Forbidden to promote** | CPU-upload default path into `main` without ADR-005 amendment |

### Attempt order (inside spike)

1. External Vulkan memory / shared image into Qt RHI / `QQuickRhiItem`
2. DMA-BUF export/import on Wayland
3. Document failure modes; if both fail → escalate (re-grill ADR-005), do **not** silently adopt CPU path

## Consequences

- **Positive**: Phase 2 starts with known interop recipe or known hard fail
- **Negative**: ~days delay before pretty canvas features
- **Neutral**: Spike code may be rewritten cleanly into `phototux-canvas`

## Revisit Date

Immediately after spike report lands; again if drivers change major Qt/wgpu versions.

## Dependencies

- **Depends on**: ADR-003, ADR-004, ADR-005 (and practical Phase 1 qtbridge build)
- **Blocks**: Phase 2 exit criteria / production canvas merge as complete

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| 2026-07-15 | Accepted at interactive grill (G5 + spike:yes) | Owner lock |
