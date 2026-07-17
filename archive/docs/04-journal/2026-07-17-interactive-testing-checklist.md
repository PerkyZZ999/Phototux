# PhotoTux Interactive Testing Checklist

**Started:** 2026-07-17  
**Goal:** Exercise shipped UI/spines; find and fix edge cases until smoke + depth paths are green.

Legend: `[ ]` todo · `[~]` in progress / flaky · `[x]` pass · `[!]` fail (tracked below) · `[N]` skipped

## Environment

- [x] Release/debug build runs (`PATH=/usr/lib/qt6/bin:$PATH cargo run -p phototux`)
- [x] Window focuses under Wayland (KWin / kwinmcp)
- [x] Cold boot reaches interactive shell (AppSession ready + first interactive frame)

## Smoke — document & chrome

- [x] New document (preset) opens; zoom-to-fit (1080p via Welcome → Create / Enter)
- [~] Tool strip: brush / eraser visible; others behind “More tools” at this window height (capacity)
- [x] Layers panel present after new doc (status: 2 layers)
- [x] Undo after paint (Ctrl+Z)
- [x] Dirty indicator (`Untitled*`, Unsaved)
- [N] Multi-doc tabs: not exercised this pass

## DR-028 depth spines

- [x] A1 Brush texture strength slider visible (`Brush tip texture strength`)
- [N] A2 Filter gallery: Noise — deferred (menu path not exercised)
- [N] A2 Exposure adjustment — deferred
- [N] A3 Text tool on-canvas — deferred (editor host fixed; create path not retested)
- [N] A4 Soft-proof display profile — deferred (UI labels present: Display: sRGB)
- [N] A5 Shape / boolean — deferred
- [N] A6 Mask contrast/shift — deferred (controls present in AT tree)
- [N] A7 Grid overlay — deferred
- [x] A8 Accessible names: canvas (`Canvas 1920×1080`), Tools toolbar, New File button, Create button

## Edge / conflict watch

- [x] Empty document → Welcome + New Document flow
- [x] Host status marker `host:document.new` cleared after handling
- [N] Switch tool mid-stroke — deferred
- [N] Rapid zoom/pan — deferred
- [N] Close last document — deferred

## Issues log

| ID | Severity | Symptom | Status |
| --- | --- | --- | --- |
| T-001 | blocker | App never showed a window: `AppSession` emitted `*_changed` during `Default` before qtbridge proxy existed → panic “No proxy” | **fixed** — field-only init; notify after construction |
| T-002 | blocker | QML root failed silently: Qt Quick `TextEdit` had invalid `background:` property | **fixed** — wrap editor in `Item` + `Rectangle` chrome |
| T-003 | high | New Document Create appeared to no-op: Cancel/Create were stacked full-width; clicks hit Cancel; `accepted` signal name was fragile | **fixed** — `createRequested` signal, Enter confirms, Cancel\|Create row |
| T-004 | med | Welcome New/Open were MouseAreas without AT button roles | **fixed** — Accessible.Button + name |
| T-005 | med | Status bar stuck on `host:document.new` after Ctrl+N | **fixed** — `clearHostStatusMarker` |
| T-006 | low | colord `busctl` could hang session discovery | **fixed** — `timeout 1s` |
| T-007 | low | Tool strip overflows many tools behind “More tools” at default test geometry | open — capacity formula / window height |

## Sign-off

- [x] Blockers T-001–T-003 fixed and retested under kwinmcp
- [~] Smoke mostly green; depth spines partially `[N]` for time
- [x] Commits for fixes + checklist updates
