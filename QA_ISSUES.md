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
