# Journal — Phase 1 command spine + Phase 2 generation (2026-07-16)

## Phase 1

- `phototux_engine::commands` — `command_id`, `CommandArgs`, `SessionState::invoke`, `CommandEffects` / host history follow-ups.
- `AppSession` routes layer/history/view/new-doc slots through invoke.
- Headless tests in `commands` module; taxonomy lists shipped IDs.

## Phase 2

- `DocumentGraph.generation` + `bump_generation`.
- `DocumentSnapshotLease`, `mark_persisted`, `is_dirty_vs_persisted`.
- History entries carry `generation`; save pins `pending_save_generation`.

## Next

Phase 3 shell contracts (descriptors, preferences, action menus).
