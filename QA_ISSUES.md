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
| [QA-001](#qa-001--lock-all-does-not-block-the-three-things-that-restyle-a-layer) | medium | `phototux_engine` / layer locks | Lock All permits opacity, blend mode and effects | open |
| [QA-002](#qa-002--the-transparency-lock-is-state-nothing-sets-and-nothing-reads) | low | `phototux_engine` / layer locks | `LockFlags::alpha` is persisted, unreachable and unread | open |
| [QA-003](#qa-003--canvas-overlay-colours-are-a-second-palette) | low | `qml/Main.qml` | Six canvas-overlay colours are literals, not tokens | open |
| [QA-004](#qa-004--an-adjustments-editor-range-and-its-clamp-disagree) | medium | `phototux_engine` / adjustments | Editor slider ranges are narrower than the values the engine keeps | open |
| [QA-005](#qa-005--a-selection-entirely-off-canvas-reports-itself-as-a-selection) | low | `phototux_engine` / selection | A marquee dragged beside the canvas reports a selection covering no pixels | open |

---

## Entries

## QA-001 — Lock All does not block the three things that restyle a layer

| | |
|---|---|
| **Severity** | medium |
| **Area** | `phototux_engine` — `commands.rs`, `layer.rs` |
| **Checklist item** | [E-29](QA_CHECKLIST.md) |
| **Status** | open |
| **Also logged as** | not yet — GUI-observable, so it warrants a `T-nnn` row when addressed |

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

**Resolution.** *(pending)*

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
| **Status** | open |

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

**Resolution.** *(pending)*

---

## QA-004 — An adjustment's editor range and its clamp disagree

| | |
|---|---|
| **Severity** | medium |
| **Area** | `phototux_engine` — `layer.rs` |
| **Checklist item** | [E-05](QA_CHECKLIST.md) |
| **Status** | open |

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

**Resolution.** *(pending)*

---

## QA-005 — A selection entirely off-canvas reports itself as a selection

| | |
|---|---|
| **Severity** | low |
| **Area** | `phototux_engine` — `commands.rs`; `qml/CanvasInput.qml` |
| **Checklist item** | [E-08](QA_CHECKLIST.md) |
| **Status** | open |

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

**Resolution.** *(pending)*
