# Journal — Character chrome + guides/grid/rulers/snap (2026-07-16)

## Intent

Continue Phase 4 polish after foundation: ship Character Properties chrome and make ViewGuides visually real (grid, rulers, snap) without opening Phase 5 gates.

## Shipped

- **Character:** `textLayerActive` + typography qprops; `updateActiveText`; Properties Character section; live QML text preview; Bake button
- **Guides:** prefs for grid/rulers/snap; View menu + Preferences; canvas grid/guide overlays; top/left rulers; `addGuide` / `clearGuides`; `snap_value` for guide place + marquee commit
- Engine: `ViewGuides::guides_json`, `snap_value`, orientation parse

## Checks

`./scripts/check-rust.sh` (run with this slice).
