# Journal: Handbook ↔ codebase alignment complete

| Field | Value |
| --- | --- |
| Date | 2026-07-16 |
| Status | Complete |
| Roadmap | [`internal_docs/Appendix/Alignment-Roadmap.md`](../../../internal_docs/Appendix/Alignment-Roadmap.md) |

## What “aligned” means (exit)

Codebase and Engineering Handbook are contract-aligned so future work can follow `internal_docs/` directly. Not every handbook feature is implemented; Deferred/Provisional items are labeled with DR/phase.

## Commits (local, this pass)

| Phase | Subject (approx) |
| --- | --- |
| A | Accepted v1 contracts (DR-005 leases, DR-013/026 `.ptx` v2, DR-015 shell, ch.08 host-only) |
| B | Document mutations through `SessionState::invoke` (selection/mask/filter/style/text/shape/raster) |
| C | Gap analysis, taxonomy, checklist, chapter Accepted-v1 notes |
| D | SPEC/CONSTRAINTS bridges + Archived-ADR-to-DR-Map + AGENTS |
| E | Alignment roadmap exit + this journal |

## Evidence checklist

- [x] Document-authoritative edits route via named commands
- [x] Host-only classes documented (previews, paint stream, prefs, I/O)
- [x] Gap analysis no longer claims missing Shape / prefs / convert / ptx v2 / command router
- [x] Root SPEC/CONSTRAINTS non-normative banners
- [x] Phase 5 still gated (tiling, multi-doc, plugins, history spill)
- [x] `./scripts/check-rust.sh` green after Phase B

## Next

Build product features from handbook chapters. Do not reopen stack (DR-023) or treat alignment as unfinished because Phase 5 depth remains.
