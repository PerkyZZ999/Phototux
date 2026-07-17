# Journal: Handbook Parity — P1 chrome completion batch

| Field | Value |
| --- | --- |
| Date | 2026-07-16 |
| Status | Complete (batch exit) |
| Roadmap | [`internal_docs/Appendix/Handbook-Parity-Roadmap.md`](../../../internal_docs/Appendix/Handbook-Parity-Roadmap.md) |
| Checklist | [`internal_docs/Appendix/Handbook-Parity-Checklist.md`](../../../internal_docs/Appendix/Handbook-Parity-Checklist.md) |

## Scope

P1.4 context menus from action registry, P1.5 shortcut resolve + persisted keymap, P1.6 command palette.

## Commits (local)

| Slice | Subject |
| --- | --- |
| 1 | `feat: drive context menus from action descriptors` |
| 2 | `feat: resolve keyboard shortcuts via action map` |
| 3 | `feat: persist customizable keymap in preferences` |
| 4 | `feat: command palette over action registry` |

## Shipped

- `ActionDescriptor.contexts` + layer/canvas/selection/mask menus via Instantiator + `invokeAction`.
- Chord → action map, ApplicationShortcut Instantiator, text-field / prefs yield.
- Preferences `keymap` overrides (schema v2), conflict steal, Keyboard section UI.
- Command palette (`Ctrl+Shift+P` / `action.app.command-palette`), substring filter, Enter invoke.

## Still open (P1)

- P1.7 descriptor taxonomy axes / workspace-scope command IDs polish
- Path context menus; full fuzzy palette; every dialog keyboard path

## Next batches

P2 docking / tear-off, remaining P3 selection concepts, or P5/P6 per product priority.

## Gate

`./scripts/check-rust.sh` green after each slice.
