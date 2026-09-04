# QA Issues

Issues found during the [QA pass](QA_CHECKLIST.md) that were **not** fixable
inside the checklist item that found them — anything needing more than a
localised change, or whose resolution is a product judgement rather than a
correction.

Trivial fixes are made in place and the checklist item is simply marked `[x]`;
they appear in the commit history, not here.

**Status** — `open` · `in progress` · `fixed` (with the commit) · `wontfix`
(with the reason) · `gated` (blocked on a Decision Register amendment).

Each entry states what was observed, the exact steps to reproduce it, and the
root cause — not a guess at the root cause. Where the cause was not established,
the entry says so rather than inventing one.

## Index

| ID | Severity | Area | Summary | Status |
|---|---|---|---|---|
| [QA-001](#qa-001--lock-all-does-not-block-the-three-things-that-restyle-a-layer) | medium | `phototux_engine` / layer locks | Lock All permits opacity, blend mode and effects | **fixed** |
| [QA-002](#qa-002--the-transparency-lock-is-state-nothing-sets-and-nothing-reads) | low | `phototux_engine` / layer locks | `LockFlags::alpha` is persisted, unreachable and unread | open |
| [QA-003](#qa-003--canvas-overlay-colours-are-a-second-palette) | low | `qml/Main.qml` | Six canvas-overlay colours are literals, not tokens | **fixed** |
| [QA-004](#qa-004--an-adjustments-editor-range-and-its-clamp-disagree) | medium | `phototux_engine` / adjustments | Editor slider ranges are narrower than the values the engine keeps | **fixed** |
| [QA-005](#qa-005--a-selection-entirely-off-canvas-reports-itself-as-a-selection) | low | `phototux_engine` / selection | A marquee dragged beside the canvas reports a selection covering no pixels | **fixed** |
| [QA-006](#qa-006--select--modify-blocks-the-ui-thread-for-minutes) | **high** | `phototux_engine` / selection | Select ▸ Modify blocks the UI thread for up to an hour at the radius the UI allows | **fixed** |
| [QA-007](#qa-007--the-text-and-shape-tools-discard-the-click-that-creates-the-layer) | medium | `qml/CanvasInput.qml` / commands | Text and shape layers land at the origin, not where the canvas was clicked | open |
| [QA-008](#qa-008--bake-text-rasterizes-in-a-bitmap-face-not-the-one-the-editor-shows) | medium | `phototux_engine` / text | Bake Text uses a 5×7 bitmap alphabet instead of the layer's font | open |
| [QA-009](#qa-009--path-edit-asks-the-user-to-drag-anchors-it-never-draws) | medium | `qml/Main.qml` / canvas overlays | Path anchors are draggable but never drawn | open |
| [QA-010](#qa-010--the-free-transform-box-is-drawn-outside-the-canvas-viewport) | low | `qml/Main.qml` / canvas overlays | The transform box paints over the tab strip when a layer moves up | open |
| [QA-011](#qa-011--a-freshly-opened-document-is-already-marked-modified-and-the-tab-strip-reorders-itself) | medium | session model / tabs | Opening marks a document dirty; tabs reorder; the same file opens twice | **fixed** |
| [QA-012](#qa-012--a-torn-off-panel-is-a-window-with-no-panel-in-it) | low | `qml/Main.qml` / floating panels | Tear-off opens a window containing a message rather than the panel | open |
| [QA-013](#qa-013--one-seam-drag-can-evict-every-panel-below-it) | medium | dock / resize clamp | Dragging a seam to the bottom hides every panel below with no way back on screen | open |
| [QA-014](#qa-014--convert-to-profile-rewrites-every-pixel-with-nothing-to-undo-it) | medium | `phototux_engine` / colour management | A profile conversion that rewrites pixels cannot be undone | open |

---

## Entries

## QA-001 — Lock All does not block the three things that restyle a layer

| | |
|---|---|
| **Severity** | medium |
| **Area** | `phototux_engine` — `commands.rs`, `layer.rs` |
| **Checklist item** | [E-29](QA_CHECKLIST.md) |
| **Status** | **fixed** |
| **Also logged as** | [T-043](internal_docs/Appendix/Interactive-Stability-Checklist.md) |

**Observed.** With **Lock All** set on a layer, painting, deleting and flipping
are all refused, as they should be. Changing the layer's **opacity**, its
**blend mode**, or **adding a filter effect** all succeed.

The command's own refusal message states the intent it is not keeping:
`"this layer is locked — unlock it to change it"`. Opacity, blend mode and
effects plainly change it.

**Steps to reproduce.**
1. New document, select `Layer 1`.
2. Layer ▸ Lock ▸ Lock All.
3. Drag the Opacity slider in the Layers panel — it moves, and the canvas
   updates.
4. Change the blend mode combo — it applies.
5. Filter ▸ add a Gaussian Blur — it applies.

Headless equivalent: invoke `layer.set-locks` with `all: true`, then
`layer.set-opacity`, `layer.set-blend` and `filter.add-effect`. All three
return `Ok`.

**Root cause.** `reject_locked_layers`
([commands.rs:3358](crates/phototux-engine/src/commands.rs#L3358)) is the only
check that consults `locks.all`, and it has exactly **one** caller:
`cmd_layer_delete` ([commands.rs:869](crates/phototux-engine/src/commands.rs#L869)).

The other two lock flags are enforced through predicates on `Layer` —
`paint_blocked()` (`locked || locks.all || locks.pixels`) and
`position_blocked()` (`locked || locks.all || locks.position`) — and both of
those *do* fold in `locks.all`, which is why paint and flip are correctly
refused. There is no equivalent predicate for "this layer may be restyled", so
nothing consults `locks.all` on the opacity, blend or effect paths.

So Lock All currently means "cannot delete, cannot paint, cannot move", not
"cannot change".

**Why this is not a quick fix.** The correction is not mechanical: it needs a
decision about which commands count as *changing* a layer, and that decision
should be made once and expressed as a predicate rather than sprinkled across
call sites. Candidates a reader would expect Lock All to cover, beyond the three
observed here: layer style add/edit/remove, mask attribute edits, rename,
clipping toggle, `blend-if`, and the adjustment-slot commands on an adjustment
layer. Photoshop greys out opacity, fill, blend mode and layer styles under Lock
All, which is a reasonable target, but the set is a product call.

There is also a second-order question: enablement. Refusing at the command is
the correctness fix; the entries and sliders should also *look* disabled, which
means an enablement tag (`active_layer_unlocked` or similar) that the Layers
panel and the Properties inspector both bind. Doing only the refusal would leave
a slider that moves and then snaps back.

**Suggested shape.** A `Layer::change_blocked()` predicate beside the two that
exist, one enablement tag derived from it, and a conformance test asserting that
every command whose `MutationClass` says it edits a layer refuses under Lock All
— so the set cannot silently grow a hole again.

**Resolution.** Fixed as suggested, with the set expressed as a partition rather
than a `MutationClass` filter — `MutationClass::Document` covers canvas resizes
and selection edits too, so it cannot answer "does this change a *layer*".

`Layer::change_blocked()` reads `locked || locks.all` and nothing else: locking
pixels stops the brush and leaves the blend mode editable, which is what
Photoshop does. `command_id::CHANGES_ACTIVE_LAYER` names the forty commands the
lock refuses and `KEEPS_WORKING_WHEN_LOCKED` names the rest with a reason for
each, and `every_command_is_classified_against_the_lock` partitions
`command_id::ALL` between them, so a new command fails the build until it is
classified. The check itself runs once, at the top of `SessionState::invoke`,
rather than as a precondition repeated in thirty bodies — which is how the hole
opened in the first place.

The enablement half reads the same list. `AppSession::action_is_enabled` greys
any action whose command is in `CHANGES_ACTIVE_LAYER`, so the whole Filter menu,
the Layer Style entries and the mask entries dim together, and `activeLayerLocked`
disables the Layers panel's blend combo and opacity slider. Filter Gallery is
gated by the new `active_layer_unlocked` tag: opening it changes nothing, but
everything it then offers is refused, and a dialog that opens only to say no is
worse than an entry that says so up front.

Two things surfaced once the state was visible. The three lock buttons had no
checked state at all — they looked identical whether the lock was on or off, so
the only way to find out was to try an edit and be refused; they now take the
primary prominence when engaged, and the Layer menu's three entries are
checkable. And Lock All was a one-way trap: it set pixels and position through
an `||` in the arguments the action built, and left them set when it was turned
off, so locking and unlocking left the layer pinned and unpaintable with every
button showing nothing. Lock All is now a superset switch both ways, and
clearing any individual lock releases it.

Verified live at 1920×1080: Lock All greys the Filter menu including the
gallery, disables the blend combo and the opacity slider and its readout, and a
second click returns all of it. Six engine tests, four of them watched failing
first — including `the_narrow_locks_leave_a_layer_restylable`, the counterweight
that fails if the predicate is made over-broad.

---

## QA-002 — The transparency lock is state nothing sets and nothing reads

| | |
|---|---|
| **Severity** | low |
| **Area** | `phototux_engine` — `layer.rs`, `commands.rs` |
| **Checklist item** | [E-29](QA_CHECKLIST.md) |
| **Status** | open |

**Observed.** `LockFlags` carries an `alpha` field — Photoshop's *Lock
transparent pixels*. It is serialised with the layer, round-trips through
`.ptx`, and `layer.set-locks` accepts it. Nothing in the workspace ever reads
it, and nothing in the shipped UI can set it.

**Steps to reproduce.** There is no user-facing path, which is the finding. From
the source:

1. `grep -rn '\.alpha' crates/ | grep -i lock` returns three hits: the field
   declaration, the toggle arm in `args_for_action`, and the `SetLocks`
   construction. No reader.
2. `default_actions()` declares `action.layer.lock-pixels`,
   `lock-position` and `lock-all`. There is no `lock-alpha`, so the
   `Some("alpha")` arm in `SessionState::args_for_action`
   ([commands.rs:271](crates/phototux-engine/src/commands.rs#L271)) is
   unreachable.
3. `paint_blocked()` — the predicate the brush consults — is
   `locked || locks.all || locks.pixels || kind != Raster`. `alpha` is absent.

**Root cause.** The field was added with the rest of `LockFlags` and its
behaviour was never implemented. Unlike the pixel and position locks, a
transparency lock is not a refusal — it is a *masking* rule applied during
paint, so honouring it means restricting the brush to pixels that are already
opaque. That is a change in `phototux_gpu`'s brush path, not a precondition
check, which is presumably why it was left.

This is dead state rather than a lying control: no user can set it, so nobody is
being told a lock is on when it is not. The cost is that a `.ptx` carries a
field with no meaning, and a future reader of `LockFlags` will reasonably assume
all four are enforced.

**Why this is not a quick fix.** Two honest resolutions, and choosing between
them is a product call:

- **Implement it.** Add `action.layer.lock-transparency`, a `lock-alpha` toggle
  in the Layers panel beside the other three, and alpha-preserving compositing
  in the brush stamp — the GPU side is the real work, and it needs a parity
  fixture like the other blend paths.
- **Remove it.** Drop the field, the command arm and the serialised key, with a
  `.ptx` migration that ignores it on read. Cheaper, and honest about what
  ships.

Leaving it as-is is the one option that should not stand, because it is the one
that misleads.

**Note.** The unreachable `Some("alpha")` arm is the same class as the three
dead `dispatch_host_op` arms removed in `75e616b`, which the host-op guard now
prevents. There is no equivalent guard for command *arguments* — nothing asserts
that every `arg` string an `args_for_action` arm matches is one some action
supplies. That guard would have found this, and is worth adding whichever
resolution is chosen.

**Resolution.** *(pending)*

---

## QA-003 — Canvas overlay colours are a second palette

| | |
|---|---|
| **Severity** | low |
| **Area** | `qml/Main.qml`, `qml/Theme.qml` |
| **Checklist item** | [U-15](QA_CHECKLIST.md) |
| **Status** | **fixed** |

**Observed.** Six colours are written as hex literals in `Main.qml` rather than
taken from `Theme.qml`:

| Line | Value | What it paints |
|---|---|---|
| 2511 | `#40FFFFFF` | grid lines |
| 2565 | `#E0FF6A00` | a guide |
| 2599 | `#22000000` | selection-preview fill |
| 2866 | `#000000` | the marching-ants shape stroke |
| 2919 | `#1F3DAEE9` | crop-preview fill |
| 3424 | `#2a2a2e` / `#222226` | navigator transparency checkerboard |

`.claude/rules/qml.md` says tokens come from `Theme.qml` and "do not invent a
second palette". These are one.

**Steps to reproduce.** `grep -n '"#[0-9A-Fa-f]\{6,8\}"' qml/Main.qml`. The
swatch palette at 3773–3775 and the shape fill/stroke defaults in
`PropertiesPanel.qml` are *document* colours, not chrome, and are correctly
excluded.

**Root cause.** `Theme.qml` has tokens for panel chrome and none for canvas
overlays, so each overlay picked its own value at the point of use. Nothing
checks, because the guard family covers controls, icons and dialog widths but
not colour literals.

**Why this is not a quick fix, and why it should not be done mechanically.**
None of the six is a safe swap:

- `#1F3DAEE9` is the accent at 12%; the nearest token, `primarySubtle`, is the
  accent at 10%. Substituting changes what the user sees.
- `#2a2a2e` matches `surfaceContainerHigh` exactly, but its partner `#222226`
  matches nothing — replacing one half of a checkerboard and not the other is
  worse than leaving both.
- The remaining four have no token at all.

The fix is to *name* six overlay tokens in `Theme.qml`, which is a theme design
decision, not a refactor.

**Why it is worth doing.** The trap has already fired here. The comment above
line 2919 records it: *"Accent at 12%, alpha first: an eight-digit hex is
`#AARRGGBB` to Qt, so this had been a pale green fill inside a cyan border."*
An eight-digit literal at the point of use is exactly where that mistake is
invisible; a token named once is where it is not. A `no_colour_literals_in_qml`
guard, with the document-colour sites listed as the exceptions they are, would
keep the count from growing.

**Resolution.** Seven tokens named in `Theme.qml` — `canvasGrid`, `canvasGuide`,
`canvasSelectionPreview`, `canvasOutline`, `canvasCropPreview`, and the
checkerboard's `checkerLight` / `checkerDark`, which had to be a pair for the
reason this issue gives. Each carries the value the literal already had, so
nothing on screen changed; the design decision was what to *call* them and where
they belong — the overlay group in handbook [25](internal_docs/25-Themes.md) —
not which colours to use.

`chrome_colours_come_from_the_theme` fails the build on a colour literal
anywhere in `qml/` outside `Theme.qml`. The two document-colour sites are
excepted by name and by the marker they sit beside rather than by line number,
so moving them does not silently widen the exception. Watched failing on a
restored literal before being trusted.

---

## QA-004 — An adjustment's editor range and its clamp disagree

| | |
|---|---|
| **Severity** | medium |
| **Area** | `phototux_engine` — `layer.rs` |
| **Checklist item** | [E-05](QA_CHECKLIST.md) |
| **Status** | **fixed** |

**Observed.** `AdjustmentParams::editor_slots()` declares the range each slider
binds to. `AdjustmentParams::clamped()` enforces a different one. Three slots
disagree:

| Kind | Slot | `editor_slots` says | `clamped` enforces |
|---|---|---|---|
| Levels | Gamma | `0.1 ..= 3` | `0.01 ..= 10` |
| Exposure | Gamma | `0.1 ..= 3` | `0.01 ..= 10` |
| Posterize | Levels | `2 ..= 32` | `2 ..= 256` |

**Steps to reproduce.** For each kind, set a slot past its declared maximum and
read it back:

```
Levels.with_slots([.., gamma: 1003., ..]).clamped()  ->  gamma == 10
Levels.with_slots([.., gamma: -999.9, ..]).clamped() ->  gamma == 0.01
Posterize.with_slots([levels: 1032]).clamped()       ->  levels == 256
```

The declared range is `0.1 ..= 3` and `2 ..= 32`.

**Root cause.** The two are written independently — `editor_slots()` returns a
literal table for the UI, `clamped()` a literal `clamp` per arm — and nothing
compares them. There is no evidence either is wrong on its own; they were
simply never required to agree.

**Why this is a real problem and not a curiosity.** The engine can hold a value
the slider cannot express. A `.ptx` written by a future build with a wider
range, or any caller that is not the slider, produces a document whose Gamma is
5 while the slider is pinned at 3 — and the first touch of that slider snaps the
value to 3, silently changing the document. The user never asked for that and
has no way to see it coming.

**Why it is not a quick fix.** Which range is authoritative is a product call.
Two coherent answers:

- **The editor range is the truth.** `clamped()` narrows to match, and any
  wider value in an existing document is clamped on load — a migration, and a
  visible change to documents that already exist.
- **The clamp is the truth.** The sliders widen to `0.01 ..= 10` and
  `2 ..= 256`, which makes the useful part of the Gamma slider a sliver at one
  end. A non-linear slider mapping would fix that, and is a design change.

Either way the fix is one table, not two, with the other derived from it — and
a test asserting every slot's `editor_slots` range is exactly what `clamped()`
enforces, so they cannot drift again.

**Resolution.** One table: `clamped()` now reads its bounds from
`editor_slots()` slot by slot and restates none of them, and the editor range is
the one kept.

The product call went that way because the migration the other answer needs is
empty — the slider is the only writer of these values, so no document can hold
anything outside the narrower range — while widening gamma to the clamp's
`0.01..=10` would put neutral at 1.0 nine percent along a linear track. Photoshop
reaches `0.10..=9.99` and keeps 1.00 in the middle by mapping the slider
non-linearly; doing that here is a design change and is the honest follow-up, not
this fix.

Two more private bounds surfaced while collapsing the table and are gone with
it: `with_slots` clamped Posterize to a literal `2..=256` of its own and floored
Exposure gamma at `0.01`. Posterize keeps a bound there — the cast to `u32` has
to stay sound for a caller that does not go on to `clamped` — but it now reads
the declared range rather than a third literal.

`every_adjustment_slot_keeps_exactly_the_range_its_editor_offers` walks every
slot of every kind in both directions. Watched failing against a reintroduced
private clamp before being trusted.

---

## QA-005 — A selection entirely off-canvas reports itself as a selection

| | |
|---|---|
| **Severity** | low |
| **Area** | `phototux_engine` — `commands.rs`; `qml/CanvasInput.qml` |
| **Checklist item** | [E-08](QA_CHECKLIST.md) |
| **Status** | **fixed** |

**Observed.** A rectangular marquee dragged entirely in the letterbox area
beside the document is accepted. `selection.active` becomes true, the status
bar reads `pixel selection`, and the marching ants draw — over a region that
contains no document pixels. Every command that needs a selection then runs and
does nothing.

A zero-area drag is correctly refused (`"the selection is empty"`). A drag that
runs *past* the edge is correctly kept whole, since the useful behaviour is to
intersect with the canvas. Only the entirely-outside case is wrong.

**Steps to reproduce.**
1. New document. The canvas is letterboxed, so there is dark chrome on both
   sides of the white page.
2. Pick the Rectangular Marquee and drag a box wholly within that dark area.
3. The status bar reads `pixel selection`.
4. Fill (Paint Bucket, or Edit ▸ Fill) does nothing, and reports nothing.

Headless equivalent: `selection.replace` with `rect { x: 5000, y: 5000, width:
10, height: 10 }` on a 1280×720 document returns `Ok` and leaves
`selection.active == true`.

**Root cause.** `cmd_selection_replace` rejects a rect whose *own* area is zero
but never intersects it with the document, so "empty" means "empty rectangle"
rather than "selects no pixels". `CanvasInput.qml` does not clamp either: the
marquee commit converts raw pointer coordinates through `screenToDocX` /
`screenToDocY` with no bound, which is what makes the letterbox reachable.

**Why this is not a quick fix.** The correct rule is a judgement, and the two
answers differ in what the user sees:

- **Refuse when the intersection is empty.** Matches Photoshop, and matches the
  existing "the selection is empty" refusal — arguably it is the same rule
  stated properly. But "empty" must then be computed against the document,
  which means the command needs the document size at that point.
- **Clamp the rect to the canvas.** Simpler, but changes the past-the-edge case
  too: the stored bounds would no longer be what the user dragged, and anything
  that later re-derives from those bounds (a transform, an expand) would work
  from the clamped rect rather than the intended one.

Whichever is chosen, the GPU mask is the real authority on coverage and the
engine's `bounds` is bookkeeping — so the fix should make the two agree about
what "active" means, rather than only tightening the rect.

**Resolution.** Refused when the intersection is empty, which was the answer
this issue leaned towards and is what Photoshop does. It is the existing
"the selection is empty" rule stated properly: both cases are the user asking
for a selection that covers nothing. A drag past the edge is still kept whole,
so the bounds stored are the ones drawn.

Making the two agree took a reordering, which was the substantive half. The
host wrote the GPU mask *before* invoking the command, so a refusal would have
left the engine holding the previous selection while the texture held none of
it, plus a host undo snapshot pushed for an edit that never happened. The engine
is now asked first and the mask is written only on success.

The refusal is said rather than swallowed — the zero-area click that deselects
returns earlier, so anything reaching this point is a deliberate drag that
selected nothing, and the only other feedback would be marching ants that never
appear. Surfacing it turned up four call sites rendering a `CommandError`
straight into a toast, so the user was reading "command rejected: …" —
scaffolding the engine's own `user_message` exists to strip. All four now go
through `report_action_error`, which classifies, strips and announces, and
`a_refused_command_is_reported_not_rendered` fails the build on a fifth.
`DocumentError` is excepted where the value in hand is one: its messages are
already sentences.

`a_marquee_that_covers_nothing_is_refused` now covers all four sides and the
one-pixel corner overlap, and asserts a refused marquee leaves the standing
selection alone. Watched failing before being trusted.

---

## QA-006 — Select ▸ Modify blocks the UI thread for minutes

| | |
|---|---|
| **Severity** | **high** — the window stops responding for as long as the operation runs, which at the radius the UI permits is over an hour |
| **Area** | `phototux_engine` — `selection.rs`; reached from `qml/SelectionModifyDialog.qml` |
| **Checklist item** | [H-35](QA_CHECKLIST.md) |
| **Status** | **fixed** — separable disc morphology |
| **Also logged as** | warrants a `T-nnn` row in the Interactive-Stability-Checklist |

**Observed.** Choosing a radius above the default in Select ▸ Modify and
confirming makes the window stop accepting input — not OK, not Cancel, not
Escape, not the menus. The canvas keeps compositing at 60 fps, so it does not
look frozen; it is simply not listening. The process moves from state `S` to
`R` and pegs a core.

It is **not** a hang. The operation completes; it just takes long enough to look
like one.

**Steps to reproduce.**
1. New document at 1080p. Ctrl+A.
2. Select ▸ Modify ▸ Expand…
3. Type `40` into the Radius field and click OK.
4. The window is unresponsive for roughly half a minute in a release build, and
   several times that in a debug build.

**Root cause — measured, not estimated.** `morph_mask_r8`
([selection.rs:674](crates/phototux-engine/src/selection.rs#L674)) is a naive
per-pixel neighbourhood scan over a **disc** structuring element: for every
pixel it visits every offset in `-r..=r` squared and skips those outside
`dx² + dy² > r²`. That is O(w · h · (2r+1)²), single-threaded, and it runs
synchronously inside the `modifySelection` slot — which is to say, on the UI
thread, with no progress, no cancel, and no worker.

Measured on 1920×1080 in a **release** build:

| Radius | Taps per pixel | Wall clock |
|---|---|---|
| 2 (the default) | 25 | 77 ms |
| 8 | 289 | 1.0 s |
| 20 | 1 681 | 6.7 s |
| 40 | 6 561 | 25.3 s |
| 512 (the spin box maximum) | 1 050 625 | ~67 minutes, extrapolated |

The default radius of 2 is the only value that feels instant, which is why the
dialog appears to work: every path that does not change the radius runs 25 taps
per pixel and returns.

**A hypothesis I had and discarded, recorded so nobody re-runs it.** The first
reading was a QML binding loop in `ThemedSpinBox`, whose editor binds
`text: control.displayText` — the trap `qml/AGENTS.md` documents for
`ThemedTextField`. The control experiment killed it: typing into the **Image
Size** dialog's spin box and confirming is completely healthy (9.5% CPU, state
`S`), and that dialog uses the same component with the same bound-`visible`
lifecycle. The difference is not the spin box. It is what the OK handler calls.

**The fix.** The disc is a union of horizontal spans — for each `dy` the span is
`±⌊√(r²−dy²)⌋` — so a dilate is a vertical accumulation of horizontal
sliding-window maxima. A monotonic-deque sliding max is O(w) per row, which
makes the whole operation **O(w · h · r)** instead of O(w · h · r²): an
r-fold improvement, about 81× at radius 40, taking 25 s to roughly 0.3 s. The
result is bit-identical, so it can be held to the current implementation by
differential test rather than by re-deriving what the right answer is.

Even after that, a radius of 512 is ~4 s, so the operation should also move off
the UI thread with the cancel path `a_running_file_operation_can_be_cancelled`
already guards for file work. Fixing the algorithm first is what makes the
common case correct; the worker is what makes the extreme case honest.

**Resolution — fixed.** `morph_mask_r8` now walks the disc as a union of
horizontal spans and takes a sliding-window extreme along each, using a
monotonic deque so every pass is O(w) whatever the window width. The whole
operation is O(w · h · r).

Measured again on the same 1920×1080 fixture, release build:

| Radius | Before | After | Speedup |
|---|---|---|---|
| 2 | 77 ms | 51 ms | 1.5× |
| 8 | 1.0 s | 157 ms | 6.5× |
| 20 | 6.7 s | 348 ms | 19× |
| 40 | 25.3 s | 671 ms | **38×** |
| 512 | ~67 min | 6.8 s | ~590× |

The result is bit-identical. `the_fast_morphology_agrees_with_the_naive_one`
keeps the old implementation as a `#[cfg(test)]` reference and compares the two
across radii, shapes and edges, and
`the_fast_morphology_agrees_on_a_grey_mask` does the same on non-binary values,
because a max/min filter that only ever sees 0 and 255 cannot show a mistake in
which extreme it keeps.

That test earned its place on its first run: it caught the erode's horizontal
edge rule. The naive form treats an out-of-bounds sample *inside the disc* as
empty, so a pixel within `half` of the left or right edge cannot survive an
erode at all — and the fast path was truncating the window instead, which is
the dilate rule. Only the vertical half of that rule had been carried over.

Verified in an isolated KWin session with the exact reproduction above: the
dialog closes, the selection expands, and the process returns to state `S`.

**Residual, deliberately not fixed here.** 6.8 s at radius 512 is still a
UI-thread block, and the operation still has no progress and no cancel. That is
a smaller and different problem from the one reported — the reported defect was
that the window stopped responding for minutes on an ordinary radius — and
moving selection morphology onto the file worker, with the cancel path
`a_running_file_operation_can_be_cancelled` already guards, remains worth
doing.

---

## QA-007 — The Text and Shape tools discard the click that creates the layer

| | |
|---|---|
| **Severity** | medium |
| **Area** | `qml/CanvasInput.qml`, `phototux_engine` — `commands.rs`, `layer.rs` |
| **Checklist item** | [H-21](QA_CHECKLIST.md), [H-22](QA_CHECKLIST.md) |
| **Status** | open |

**Observed.** With the Text tool active, clicking anywhere on the canvas
creates a text layer whose frame is at the document origin, not at the click.
Clicking at the bottom-right of a 1920×1080 document puts the frame at the top
edge, a thousand pixels away from the pointer. The Shape tool behaves the same
way: `rect` presets land at a fixed position regardless of where the drag
started.

Photoshop places the type insertion point where the Type tool is clicked, and
that is this project's stated placement rule. Missing it costs a second gesture
every time — create, then drag the frame back to where the click already said
it should go.

**Steps to reproduce.**

1. New 1080p document.
2. Press `t` for the Text tool.
3. Click near the bottom-right of the canvas.
4. The text frame appears along the top edge of the document.

**Root cause.** The position never leaves QML. `CanvasInput.qml` has the
document coordinates in hand — it computes `screenToDocX(mouse.x)` for the path
tool three lines further down — but the text branch calls
`AppSession.addTextLayer(qsTr("Text"))`, a slot whose whole signature is the
string. `CommandArgs::TextCreate` carries only `text`, and `cmd_text_create`
builds its content from `TextContent::default()`, whose origin is `(0, 0)`.
Nothing downstream could honour a click position because nothing upstream ever
sends one.

**Why not a quick fix.** The coordinate has to be threaded through four places
that each have their own guard: the QML call site, the `#[qslot]` signature,
the `CommandArgs` variant, and `args_for_action` — which supplies arguments for
the same command when it is invoked from the menu rather than the canvas, and
so needs a defensible default rather than a click. `command_conformance.rs`
holds `every_action_builds_arguments_its_command_accepts` over that last pair,
so the change is safe to make but is not a one-line edit, and the same work
should cover the Shape tool rather than being done twice.

## QA-008 — Bake Text rasterizes in a bitmap face, not the one the editor shows

| | |
|---|---|
| **Severity** | medium |
| **Area** | `phototux_engine` — `text_bake.rs` |
| **Checklist item** | [H-21](QA_CHECKLIST.md) |
| **Status** | open |

**Observed.** The on-canvas text editor and the read-only preview both render
in the layer's chosen family — Noto Sans at 24 pt by default, with real
lowercase and proportional advances. `Layer ▸ Bake Text` produces something
else: a blocky monospaced 5×7 bitmap alphabet with no lowercase. The user sees
one thing while the text is editable and a visibly different thing the instant
it becomes pixels, and the bake is not undoable back into an editable layer
without stepping through history.

**Steps to reproduce.**

1. New 1080p document, press `t`, click the canvas.
2. Type `PhotoTux QA` into the frame — it renders in Noto Sans.
3. `Layer ▸ Bake Text`.
4. The canvas now shows `PHOTOTUX QA` in a bitmap face.

**Root cause.** `text_bake.rs` says so in its own first lines: it "uses a
built-in 5×7 ASCII glyph set so bake works without Qt font shaping". That was
the right call for the crate boundary — `phototux_engine` may not link Qt, and
the bake had to exist for headless tests before any shaping path did. It is
correct as a reference rasterizer. It is not correct as the thing a user gets
when they click Bake Text.

**Why not a quick fix.** Shaping the layer's actual face means either a text
adapter in `phototux_ui` (which may use Qt) that rasterizes and hands the host
a buffer, leaving `bake_text_rgba8` as the headless fallback, or a pure-Rust
shaping stack in the engine. The first is a smaller change and matches
handbook 18's "portable core defines text semantics, Linux-native adapters
resolve fonts" split; it is still a new adapter with its own tests, not an
edit. Handbook 18 describes the full engine, and the
[gap analysis](internal_docs/Appendix/Codebase-Handbook-Gap-Analysis.md) does
not currently record that the shipping bake ignores the selected face — that
omission is part of this issue.

## QA-009 — Path Edit asks the user to drag anchors it never draws

| | |
|---|---|
| **Severity** | medium |
| **Area** | `qml/Main.qml` — canvas overlays |
| **Checklist item** | [H-22](QA_CHECKLIST.md) |
| **Status** | open |

**Observed.** The Path Edit tool works. With a shape layer active, dragging
the point where an anchor happens to be moves it, the outline follows, and the
Shape inspector's W/H/X/Y update. But nothing on the canvas shows where the
anchors *are*. There are no handles, no path outline, no selected-anchor
highlight — the tool's own inspector copy reads "Drag anchors to move. Click
empty to add", and the user has no way to see either one. Free Transform, by
contrast, draws its eight handles.

With a **raster** layer active it is worse. The click still adds an anchor,
but to the document's separate path list rather than to any layer, and
`inspector.path` is registered for the `Shape` subject only — so the Path
group is not in the panel either. The anchor count, the Closed toggle and
Delete Anchor are all invisible, and the only way to find out that anything
happened is `Layer ▸ Shape ▸ Stroke Path to Layer`.

**Steps to reproduce.**

1. New 1080p document, `Layer ▸ Shape ▸ Rectangle`.
2. Press `A` for Path Edit. The canvas is unchanged — no handles appear.
3. Drag the rectangle's top-left corner. It moves, so the anchor was there.
4. Select a raster layer instead and click the canvas twice. Nothing appears,
   and the Properties panel shows no Path group.

**Root cause.** There is no overlay to draw. `Main.qml` renders guides, the
crop rectangle, the free-transform handles, the marquee and the text frame,
but has nothing for `graph.paths` or for a shape layer's own path; the host
publishes `pathAnchorCount`, `pathClosed` and `pathEditSelected` as scalars,
and no anchor geometry at all. Hit-testing happens entirely host-side in
`path_hit_test`, so the pointer finds anchors the screen never showed.

**Why not a quick fix.** The overlay needs anchor positions in document space,
which means a new published projection — the same shape as `guidesJson`, and
subject to the same rule that it must not be rebuilt on every composite (see
T-009 in the [Interactive-Stability-Checklist](internal_docs/Appendix/Interactive-Stability-Checklist.md),
where a per-frame republish flooded AT-SPI and killed the session). Handle
size has to stay constant in *screen* pixels while the canvas zooms, and the
selected anchor needs its own treatment. That is a small feature, not a patch.
Making `inspector.path` visible for more than the `Shape` subject is a
separate and much smaller question, but it should be answered together with
this one rather than exposing a panel for geometry that is still invisible.

## QA-010 — The free-transform box is drawn outside the canvas viewport

| | |
|---|---|
| **Severity** | low |
| **Area** | `qml/Main.qml` — canvas overlays |
| **Checklist item** | [H-37](QA_CHECKLIST.md) |
| **Status** | open |

**Observed.** The transform bounding box follows the layer's document bounds
as they move, and nothing clips it to the canvas viewport. Dragging a layer
upward pushes the box's top edge and its two top handles over the document tab
strip, where they are drawn on top of the tab labels; dragging left or right
takes the horizontal edges under the tool rail and towards the right dock. The
box is correct — it is where the layer now is — but it is painted over chrome
that has nothing to do with it.

**Steps to reproduce.**

1. New 1080p document, paint a stroke.
2. Free Transform, then drag from the middle of the canvas up and to the right.
3. The box's top edge and handles are drawn across the tab strip above the
   canvas.

**Root cause.** The handle `Repeater` and the box sit inside `canvasHost` at
`z: 3` with no `clip: true` on the item that bounds the viewport, and their
positions come straight from `docToScreenX`/`docToScreenY`, which are free to
land outside it. Guides and the crop rectangle stay inside only because the
values they are given cannot leave the document.

**Why not a quick fix.** `clip: true` on the wrong ancestor would also clip the
canvas item itself, which is the `QQuickRhiItem` the whole zero-copy present
goes through — the one thing in the shell that must not be re-parented or
clipped casually (handbook 17). The overlay layer wants its own clipping
container sized to the viewport, with the canvas item outside it, and that is a
change to the canvas scene graph rather than a property flip. It is also worth
doing once for every overlay rather than for this one.

## QA-011 — A freshly opened document is already marked modified, and the tab strip reorders itself

| | |
|---|---|
| **Severity** | medium |
| **Area** | `phototux_ui` — `lib.rs`, `phototux_engine` — `document_registry.rs` |
| **Checklist item** | [H-05](QA_CHECKLIST.md), [H-12](QA_CHECKLIST.md) |
| **Status** | **fixed** |

Three things go wrong around opening a document. They are filed together
because they are all the document-session model and would be fixed in one
sitting.

**Observed — 1. Opening marks the document modified.** Launch with a `.ptx`
path, touch nothing: the tab reads `* qa-doc.ptx`. Nothing has been edited. The
consequences follow from that flag — closing prompts to discard, and the
autosave timer, which runs `while AppSession.dirty`, starts writing recovery
snapshots for a document identical to the file on disk.

The cause is in the source and is not a guess: `finish_opened_ptx` ends with
`self.dirty = true`, and the PSD path does the same. The raster path
(`.png`, `.jpg`, …) does not, and a PNG opened beside it is correctly clean.

**Observed — 2. The window title disagrees with the tab.** In the same state
the title bar reads `qa-doc.ptx — PhotoTux`, with no asterisk, while the tab
shows one. Both are supposed to come from the same `AppSession.dirty` — the
title binds it directly and the tab reads the `dirty` field of the tabs JSON,
which for the active tab is that same value — and `emit_doc_fields` emits
`dirty_changed` after the flag is set. **The mechanism was not established.**
It is recorded here as observed rather than explained, because guessing at it
would be worse than saying so.

**Observed — 3. The tab strip reorders itself when you switch tabs.**
`DocumentRegistry::tabs_json` builds the list as the active document first and
then the parked ones, so a tab moves to position 0 the moment it is clicked and
the others shuffle down. Nothing else in the shell moves under the pointer like
that, and it makes a strip of three or more unreadable: the tab you just left
is not where you left it.

**Observed — 4. Opening a file that is already open opens it twice.** There is
no de-duplication by path anywhere in the open path, so the same document ends
up in two tabs with two independent histories, and a save from one silently
loses the other's edits. Photoshop raises the existing tab.

**Steps to reproduce.**

1. Save any document as `qa-doc.ptx`.
2. Relaunch with that path as the argument, or `Ctrl+O` it. The tab shows `*`
   and the title does not.
3. `Ctrl+O` the same file again → a second tab for the same path.
4. Click between the two tabs → the active one jumps to the left each time.

**Why not a quick fix.** Dropping the two `self.dirty = true` lines is a
one-line change each, but it needs a test that a freshly opened document is
clean, and (2) has to be understood first — a document that is genuinely dirty
and *shows* clean in the title is the more dangerous direction of the same bug,
and it may be the same root cause. (3) changes the published order, which the
QML tab strip and `document_tabs_json` guards both read. (4) is a new lookup and
a decision about what "already open" means for a document that has been
`Save As`-ed since. Handbook [DR-024](internal_docs/Appendix/Decision-Register.md#dr-024--document-session-model)
owns the session model these all sit in.

**Resolution.** All four.

(1) and (2) went together and were fixed earlier, in
[T-040](internal_docs/Appendix/Interactive-Stability-Checklist.md): `dirty` was
published twice — as a property and inside `documentTabsJson` — and three
writers set the field while emitting only the property, so the *tab* was the one
lying, not the title. Every write now goes through `set_dirty`, which publishes
both, and `finish_opened_ptx` no longer marks an opened document modified at
all. A PSD import still does, deliberately: it has no `.ptx` of its own yet.

(3) The strip's order is now the order tabs were opened, held in
`DocumentRegistry::order` and independent of which tab is active. Activating
moves a document between the parked vector and the host's active slot without
touching it; closing removes it, through the new `forget`.

(4) "Already open" is answered by a new `SessionState::source_path` — the file a
document was *loaded from*, whatever its format. It is deliberately not
`document_path`, which is where `Save` writes: the two agree only for a `.ptx`,
because a raster or PSD import has no `.ptx` yet and writing one over the file it
came from would destroy it. That distinction is what the issue's "what does
already open mean" was asking for, and conflating them is why the naive lookup
would have missed every imported PNG.

`source_path` also turned out to be what the file dialogs should have been
starting from: an imported PNG's `documentPath` is empty, so the browser opened
wherever it was last used rather than in the folder the image came from.

Verified live: three tabs keep their order when the leftmost is clicked, and
re-opening the launch file raises the existing tab with "/tmp/qa_a.png is
already open" rather than making a second. Two registry tests, both watched
failing.

## QA-012 — A torn-off panel is a window with no panel in it

| | |
|---|---|
| **Severity** | low |
| **Area** | `qml/Main.qml` — floating panel windows |
| **Checklist item** | [H-42](QA_CHECKLIST.md) |
| **Status** | open |

**Observed.** Tearing off the Navigator opens a window titled "Navigator"
containing two lines of prose — "Navigator (floating) · Close window or Dock to
return this panel to the right stack." — and a Dock button. The panel's actual
content is not in it. The thumbnail, the zoom controls, everything the panel is
for stays behind in the dock's place, which now shows the panel's *neighbour*
grown to fill the space.

Docking it back works, and the panel returns with its content intact — but it
returns as its own group at the bottom of the stack rather than to the group it
was torn from, so a tear-off and re-dock is not a round trip.

**Steps to reproduce.**

1. Open any document.
2. Navigator panel header → the tear-off button (the rightmost icon).
3. The floating window has the text and the button, and no Navigator.
4. Press Dock. The panel is back in the right stack, in a new group of its own.

**Root cause.** The floating window is a placeholder by construction: the
`Instantiator` in `Main.qml` builds a `Window` whose content is the explanatory
label and the Dock button, and never reparents or re-instantiates the panel
into it. Nothing is broken — the window does exactly what it was written to do.

**Why not a quick fix.** Moving a live panel into another window means either
reparenting the item into a second `QQuickWindow`, which is the thing
[T-027](internal_docs/Appendix/Interactive-Stability-Checklist.md) got a
process abort out of and would want its geometry write-back re-examined, or
instantiating a second copy of the panel bound to the same host properties,
which is cheap for the Navigator and not obviously right for panels that hold
their own view state. The re-dock grouping is a smaller, separable question:
`park`/`pin` do not record which group a panel left.

## QA-013 — One seam drag can evict every panel below it

| | |
|---|---|
| **Severity** | medium |
| **Area** | `qml/PanelResizeGrip.qml`, `phototux_engine` — `dock.rs` |
| **Checklist item** | [U-11](QA_CHECKLIST.md) |
| **Status** | open |

**Observed.** Dragging a dock seam downward past the bottom of the window makes
the panel above it fill the entire dock, and Navigator, Swatches, Layers and
History disappear — not collapsed to a header, not reachable by scrolling the
dock, simply gone. The Window menu still shows all four as visible, and they
come back with Window ▸ Reset Workspace, but nothing on screen says where they
went or how to get them back.

The other extreme is fine: dragging the same seam upward clamps at the minimum,
the panel keeps its header and its scroll bar, and nothing is lost.

**Steps to reproduce.**

1. Open any document; the right dock holds Properties, Navigator/Swatches and
   Layers/History.
2. Press on the seam above the Navigator header and drag to the bottom of the
   screen.
3. Properties fills the dock. The other three panels are not on screen.

**Root cause.** The clamp has no upper bound that depends on the dock.
`PanelResizeGrip.maximumHeight` is the constant 2000, mirroring
`DockTopology::MAX_PANEL_HEIGHT`, and neither side subtracts what the panels
below need. Handbook [04](internal_docs/04-Docking-System.md) asks for exactly
that — "Solver clamps against minimum/maximum constraints **and available
logical size**" — and the available-logical-size half is the part that is
missing.

**Why not a quick fix.** The ceiling a drag should respect is the dock's height
minus the sum of the minimums of every panel below the seam, which the grip
does not know: it is handed one panel's `currentHeight` and nothing about its
siblings. Either the grip learns the remaining budget — a new published value,
recomputed as the group changes — or the engine clamps on commit and the
preview follows what it returns. The second is the smaller change and matches
where the other constraint already lives, but it makes the drag feel like it
runs past the limit and snaps back, which is the thing the current comment in
the grip says it was written to avoid.

## QA-014 — Convert to Profile rewrites every pixel with nothing to undo it

| | |
|---|---|
| **Severity** | medium |
| **Area** | `phototux_engine` — `commands.rs`, `phototux_ui` — `apply_convert_pixels` |
| **Checklist item** | [H-39](QA_CHECKLIST.md) |
| **Status** | open |

**Observed.** Image ▸ Color ▸ Convert to Display-P3 rewrites every layer's
pixels on the GPU and records no history entry at all. Ctrl+Z afterwards walks
past the conversion to whatever came before it, and there is no way back to the
pixels the document had. The user is warned — the toast says "this rewrote
layer data" — so the destruction is disclosed, but disclosure is not undo.

Its two neighbours in the same submenu are now undoable: Assign Profile and
Embed/Clear ICC both record a `GraphCommand::SetColorState` entry.

**Steps to reproduce.**

1. New document, paint a stroke so there are pixels worth keeping.
2. Image ▸ Color ▸ Convert to Display-P3. The toast reports the rewrite.
3. Ctrl+Z. The stroke is undone; the conversion is not.

**Root cause.** The edit has two halves in two different places and the history
model has no entry kind that covers both. `cmd_document_convert_profile` moves
the graph's colour state, and `HostFollowUp::ConvertPixels` has the host read
every layer back, convert it and write it again — without taking a snapshot
first. `HistoryKind::Graph` reverses the graph and never asks the host;
`HistoryKind::Transform` asks the host and never touches the graph. There is no
kind that does both, so a single conversion cannot be recorded as one step.

**Why not a quick fix.** The pieces exist — `transform_snapshot_now` already
captures every layer's pixels, which is exactly what a conversion overwrites —
but wiring them up means either a new history kind that carries both halves, or
splitting the conversion into two entries that undo must always cross together.
The first changes `HistoryKind`, which the timeline, the History panel and the
host's two stacks all read; the second reintroduces the coupling
`HistoryService::undo_next` was written to prevent. It is a Decision Register
question about what a history entry is, not a correction.

Until then the exclusion is explicit rather than silent:
`every_action_that_edits_the_document_undoes_back_to_where_it_started` names
`DOCUMENT_CONVERT_PROFILE` in `NOT_UNDOABLE` and points here, so a command that
newly forgets its undo entry still fails rather than joining a quiet list.
