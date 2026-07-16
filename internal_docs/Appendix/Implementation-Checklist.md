# Implementation Checklist (alignment)

Living tracker for [Alignment Roadmap](Alignment-Roadmap.md).  
Legend: `[ ]` todo · `[~]` partial · `[x]` done · `[!]` blocked · `[P]` post-v1

**Tech stack:** frozen ([DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase)) — do not track toolkit swaps here.

---

## Phase 0 — Documentation lock

- [x] Archive former `/docs/` → `archive/docs/`
- [x] Handbook authoritative (`internal_docs/`)
- [x] Gap analysis written
- [x] Alignment roadmap + DR-023…026
- [x] Charter / lifecycle / developer guide stack language
- [x] This checklist seeded

---

## Phase 1 — Command spine

- [x] CommandId + registry + invoke in `phototux_engine`
- [x] Wrap core graph mutations (undo/redo, layer ops) as commands
- [x] `AppSession` routes core slots through router (behavior-stable)
- [x] Command Taxonomy lists shipped IDs
- [x] Headless command tests (no Qt)
- [ ] Remaining AppSession mutations (masks, filters, selection…) still direct — wrap in later slices

---

## Phase 2 — Version + snapshot leases

- [x] Document generation / version on commit
- [x] Snapshot metadata lease for recomposite
- [x] Save pin generation + receipt (`mark_persisted` / dirty vs persisted)
- [x] History entries reference generation

---

## Phase 3 — Shell contracts (Qt)

- [x] Panel / tool descriptors (`shell.rs` + JSON props)
- [x] Preferences service + dialog (XDG `preferences.json`)
- [x] Window menu panel toggles + layer context menu v1
- [x] Theme tokens documented in `Theme.qml` (archived DESIGN.md source)
- [x] Workspace Reset → Essentials panel visibility

---

## Phase 4 — Engine depth

- [ ] CPU reference composite (test subset)
- [ ] Text bake + Character chrome
- [ ] Selection modify ops
- [ ] Color assign / convert foundation
- [ ] Paths engine + panel
- [ ] Shape kind + tools (after graph amend)
- [ ] Adjustment/filter wave 2
- [ ] Layer styles v1
- [ ] Guides / grid / rulers / snap
- [ ] `.ptx` chunk/integrity evolution (compat)

---

## Phase 5 — Gated

- [!] Tiling / pyramid (evidence gate)
- [!] Multi-document (amend DR-024)
- [P] Plugin capability seams (after Phase 1; ABI deferred)
- [P] History spill format

---

## Standing rules

- [ ] Update this file when starting/finishing a slice
- [ ] Update gap analysis rows for closed architecture items
- [ ] `./scripts/check-rust.sh` green
- [ ] No paragraph-long workaround comments
