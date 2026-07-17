# Journal: Handbook Parity — P1.7 + P2 workspace batch

| Field | Value |
| --- | --- |
| Date | 2026-07-16 |
| Status | Complete (batch exit) |
| Roadmap | [`internal_docs/Appendix/Handbook-Parity-Roadmap.md`](../../../internal_docs/Appendix/Handbook-Parity-Roadmap.md) |
| Checklist | [`internal_docs/Appendix/Handbook-Parity-Checklist.md`](../../../internal_docs/Appendix/Handbook-Parity-Checklist.md) |

## Scope

P1.7 CommandMeta + host-only catalog; P2.1 WorkspaceState; P2.2 DockTopology v1; P2.3 QML `panels_json` / topology-driven chrome.

## Commits (local)

| Slice | Subject |
| --- | --- |
| 1 | `feat: command meta registry for taxonomy axes` |
| 2 | `feat: app/workspace command IDs and host-only catalog` |
| 3 | `feat: engine WorkspaceState separate from document dirty` |
| 4 | `feat: dock topology model for right-stack workspace` |
| 5 | `feat: drive dock chrome from panelDescriptorsJson` |

## Shipped

- `CommandMeta` axes for every `command_id::ALL`; taxonomy doc + host-only exemption catalog; P1.7 checked.
- App/workspace IDs (`app.show-preferences`, `workspace.reset`, `workspace.toggle-panel`) + `HostFollowUp` chrome path.
- `WorkspaceState` visibility map + revision; prefs schema panel map; dirty-isolation tests.
- `DockTopology` right-stack validate + prefs persist.
- QML: titles/visibility/prefs from descriptors; Paths/Character placeholders; Window toggles for catalog panels.

## Deferred (next P2 batch)

Tear-off floating Window + restore clamp; auto-hide / pin; drag/drop placement; named user presets beyond Essentials; Layers/History virtualization; full split topology.

## Gate

`./scripts/check-rust.sh` green after each slice.
