# Journal — IA refresh + production parity roadmap (2026-07-16)

## Intent

Merge owner [`PREFERED_IA.md`](../PREFERED_IA.md) into normative [`INFORMATION_ARCHITECTURE.md`](../INFORMATION_ARCHITECTURE.md), retarget production checklist slices for full IA parity, and align surrounding docs. **Codebase** remains source of truth for shipped capability; IA + checklist define the production-ready destination.

## Changes

- Rewrote `INFORMATION_ARCHITECTURE.md` with preferred structure (menus, tools, panels, modules, flows) plus **Current / Planned / Blocked / Deferred** tags.
- Rebuilt `docs/03-checklists/development.md` suggested-next into IA-parity roadmap (shell → editing → vectors → interchange; ADR-gated items explicit).
- Corrected stale checklist items already shipped (layer mask paint, clipping, adjustment GPU, selection ants).
- Updated `DESIGN_BRIEF.md` component inventory to match code + IA.
- Pointed `FEATURES_TODO.md` / `PREFERED_IA.md` at official IA + checklist.
- Logged ADR tensions in `conflicts.md` (multi-doc, Shape kind, plugins).
- Light sync: `AGENTS.md` key doc map, `README.md` status.

## Non-goals

- No ADR amendments this slice (multi-doc / Shape / plugins remain gated).
- No product code changes.
