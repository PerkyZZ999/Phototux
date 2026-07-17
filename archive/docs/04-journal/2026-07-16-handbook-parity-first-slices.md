# Journal: Handbook Parity — first slices batch

| Field | Value |
| --- | --- |
| Date | 2026-07-16 |
| Status | Complete (batch exit) |
| Roadmap | [`internal_docs/Appendix/Handbook-Parity-Roadmap.md`](../../../internal_docs/Appendix/Handbook-Parity-Roadmap.md) |
| Checklist | [`internal_docs/Appendix/Handbook-Parity-Checklist.md`](../../../internal_docs/Appendix/Handbook-Parity-Checklist.md) |

## Scope

P1.1 action registry, P1.2 MenuBar from actions, P1.3 tool strip from tool descriptors, P3.1 edit-target / pixel-selection chrome.

## Commits (local)

| Slice | Subject |
| --- | --- |
| 1 | `feat: engine action descriptors and AppSession invokeAction` |
| 2 | `feat: drive MenuBar from action descriptors` |
| 3 | `feat: tool strip consumes toolDescriptorsJson` |
| 4 | `feat: expose distinct edit-target and selection chrome` |

## Shipped

- Engine `ActionDescriptor` + `default_actions()` / `actions_json()`; host resolves via `invokeAction` / `actionEnabled`.
- MenuBar Instantiator menus from `actionsJson` (File…Help); context menus still hardcoded (P1.4).
- Tool strip from `toolDescriptorsJson` with Phosphor `icon_key` stems + group hairlines.
- QML: `pixelSelectionActive`, `editTarget`, `editTargetLabel`, `activeLayerKind`; enriched `status_summary`; Properties Edit target row.

## Deferred (next batches)

- P1.4 context menus from registry
- P1.5 customizable keymap / P1.6 command palette
- P2 docking / tear-off
- Remaining P3 (object selection, announce suite, mask↔selection flows)

## Gate

`./scripts/check-rust.sh` green after each slice.
