# PhotoTux Interactive Testing Checklist

**Started:** 2026-07-17  
**Updated:** 2026-07-17 (pass 2)  
**Goal:** Exercise shipped UI/spines; find and fix edge cases until smoke + depth paths are green.

Legend: `[ ]` todo · `[~]` in progress / flaky · `[x]` pass · `[!]` fail (tracked below) · `[N]` skipped

## Environment

- [x] Release/debug build runs (`PATH=/usr/lib/qt6/bin:$PATH cargo run -p phototux`)
- [x] Window focuses under Wayland (KWin / kwinmcp)
- [x] Cold boot reaches interactive shell (AppSession ready + first interactive frame)

## Smoke — document & chrome

- [x] New document (preset) opens; zoom-to-fit (1080p via Ctrl+N → Enter)
- [x] Welcome closes on Ctrl+N / document create (no stuck overlay)
- [x] Tool strip: Brush…Path Edit named in AT-SPI; icons bundled in qrc
- [x] Layers: status shows layer count after edits
- [x] Undo after paint (Ctrl+Z) — pass 1
- [x] Dirty indicator (`Untitled*`, Unsaved)
- [x] Command palette (Ctrl+Shift+P)
- [N] Multi-doc tabs: not exercised this pass

## DR-028 depth spines

- [x] A1 Brush texture strength slider (`Brush tip texture strength`)
- [x] A2 Filter Noise via command palette (rejects on text layer with clear status — raster required)
- [x] A2 Exposure via command palette
- [x] A3 Text tool: create layer; Character panel; on-canvas “Text” frame
- [x] A4 Soft-proof control present (`Soft-proof with display ICC`)
- [x] A5 Shape: canvas click grows layer count (5 layers after shape)
- [N] A5 Shape boolean — deferred
- [N] A6 Mask contrast/shift interact — controls present; not drag-tested
- [x] A7 Grid overlay via palette “grid”
- [x] A8 Accessible names on tool strip buttons + canvas + New File

## Edge / conflict watch

- [x] Empty document → Welcome + New Document flow
- [x] Host status marker `host:document.new` cleared after handling
- [x] Filter on non-raster layer → rejected without crash
- [N] Switch tool mid-stroke — deferred
- [N] Rapid zoom/pan — deferred
- [N] Close last document — deferred

## Issues log

| ID | Severity | Symptom | Status |
| --- | --- | --- | --- |
| T-001 | blocker | App never showed a window: `AppSession` emitted `*_changed` during `Default` before qtbridge proxy existed → panic “No proxy” | **fixed** |
| T-002 | blocker | QML root failed silently: Qt Quick `TextEdit` had invalid `background:` | **fixed** |
| T-003 | high | New Document Create no-op / Cancel stacked | **fixed** |
| T-004 | med | Welcome New/Open without AT button roles | **fixed** |
| T-005 | med | Status stuck on `host:document.new` | **fixed** |
| T-006 | low | colord `busctl` hang risk | **fixed** |
| T-007 | med | Tool strip overflow + missing AT names + missing qrc icons | **fixed** — Accessible on tools; denser strip packing; phosphor icons in qml-aot |
| T-008 | high | Welcome stayed open after Ctrl+N / document create | **fixed** — close welcome on destructive new/open, dialog open, `hasDocument` |
| T-009 | info | Noise filter on text layer → `command rejected: effect requires raster layer` | expected — switch to raster first |

## Sign-off

- [x] Blockers T-001–T-003, T-008 fixed and retested under kwinmcp
- [x] Smoke green for this pass
- [x] Depth spines mostly `[x]`; boolean / mask drag / multi-doc still `[N]`
- [x] Commits for fixes + checklist updates
