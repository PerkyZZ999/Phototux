# Risk Checklist

Score = Likelihood (1–5) × Impact (1–5). Review at phase exits.

| # | Risk | L | I | Score | Mitigation | Trigger |
|---|------|---|---|-------|------------|---------|
| R1 | qtbridge 0.2 beta API/breakage | 3 | 4 | 12 | Pin version; thin ui crate; hybrid fallback ADR-003 | Build fails after upgrade |
| R2 | Cannot share wgpu texture with Qt RHI (zero-copy) | 4 | 5 | 20 | Phase 2 first vertical; hybrid C++ item; try DMA-BUF + external memory | 2 approaches fail → ADR-005 |
| R3 | Spike skipped — unknown interop | 5 | 4 | 20 | Treat Phase 2 week-1 as de-facto spike; journal findings | Schedule slip >1 week |
| R4 | RefCell re-entrancy panics | 3 | 3 | 9 | Command queue; short borrows; ADR-007 | Panic in UI interaction |
| R5 | Dual bridge (qtbridge + C++) complexity | 3 | 3 | 9 | Confine C++ to `phototux-canvas` only | C++ spreads to app logic |
| R6 | SLO ≥60 FPS missed on target HW | 2 | 4 | 8 | Instrument early; profile before features | Phase 2 exit fail |
| R7 | Qt private headers / packaging pain | 2 | 3 | 6 | Prefer public RHI APIs; document Arch packages | Build needs private-devel |
| R8 | License/distribution of Qt + qtbridge | 2 | 3 | 6 | Review LICENSE at pin; LGPLv3 system Qt | Before public release |
| R9 | Scope creep (brushes before viewport) | 3 | 3 | 9 | Phase order in SPEC; AGENTS.md | PR adds Phase 4 before 2 |
| R10 | Solo bandwidth | 4 | 2 | 8 | Vertical slices; checklist discipline | Parallel work thrash |

## Escalation

- Score ≥ 16: log blocker in `blockers.md`, consider ADR amendment or spike branch
- R2/R3: do not implement CPU-upload “temporary” product path without ADR-005 change
