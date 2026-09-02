---
title: Keyboard shortcuts
description: >-
  Every default binding, by menu. All of them are rebindable in Preferences,
  and the command palette shows the current chord for any action.
---

These are the defaults. Every one can be changed in
[Preferences ▸ Keyboard shortcuts](/guides/workspace/#keyboard-shortcuts), and
the [command palette](/guides/workspace/#the-command-palette)
(<kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>P</kbd>) always shows the chord that is
actually bound.

## File

| Action | Shortcut |
|---|---|
| New | <kbd>Ctrl</kbd> <kbd>N</kbd> |
| Open | <kbd>Ctrl</kbd> <kbd>O</kbd> |
| Save | <kbd>Ctrl</kbd> <kbd>S</kbd> |
| Save As | <kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>S</kbd> |
| Export | <kbd>Ctrl</kbd> <kbd>Alt</kbd> <kbd>Shift</kbd> <kbd>W</kbd> |
| Close | <kbd>Ctrl</kbd> <kbd>W</kbd> |
| Quit | <kbd>Ctrl</kbd> <kbd>Q</kbd> |

<div class="callout callout-note">

**Export is not <kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>E</kbd>.** That chord is
Merge Visible, here as in Photoshop, and someone pressing it expecting a merge
should not get a file dialog. Export takes Photoshop's Export As chord instead.

</div>

## Edit

| Action | Shortcut |
|---|---|
| Undo | <kbd>Ctrl</kbd> <kbd>Z</kbd> |
| Redo | <kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>Z</kbd> |
| Copy | <kbd>Ctrl</kbd> <kbd>C</kbd> |
| Paste as New Layer | <kbd>Ctrl</kbd> <kbd>V</kbd> |
| Command Palette | <kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>P</kbd> |
| Preferences | <kbd>Ctrl</kbd> <kbd>,</kbd> |

Copy Selection Mask, Copy Layer Mask, Paste as Selection and Paste as Layer
Mask have no default chord; they are in the Edit menu and the palette.

## Select

| Action | Shortcut |
|---|---|
| Select All | <kbd>Ctrl</kbd> <kbd>A</kbd> |
| Deselect | <kbd>Ctrl</kbd> <kbd>D</kbd> |
| Invert Selection | <kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>I</kbd> |

The five **Select ▸ Modify** entries — Feather, Expand, Contract, Smooth and
Border — have no default chord because each opens a prompt.

## Image

| Action | Shortcut |
|---|---|
| Image Size | <kbd>Ctrl</kbd> <kbd>Alt</kbd> <kbd>I</kbd> |
| Canvas Size | <kbd>Ctrl</kbd> <kbd>Alt</kbd> <kbd>C</kbd> |

## Layer

| Action | Shortcut |
|---|---|
| New Layer | <kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>N</kbd> |
| Duplicate Layer | <kbd>Ctrl</kbd> <kbd>J</kbd> |
| Merge Down | <kbd>Ctrl</kbd> <kbd>E</kbd> |
| Merge Visible | <kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>E</kbd> |
| Merge Group | — |

Merge Group has no default chord. Photoshop shares <kbd>Ctrl</kbd>
<kbd>E</kbd> between Merge Down and Merge Group and decides by what is
selected; here one chord binds one action, so Merge Down's refusal names Merge
Group instead.

## View

| Action | Shortcut |
|---|---|
| Zoom In | <kbd>Ctrl</kbd> <kbd>=</kbd> |
| Zoom Out | <kbd>Ctrl</kbd> <kbd>-</kbd> |
| Actual Pixels (100%) | <kbd>Ctrl</kbd> <kbd>1</kbd> |
| Fit on Screen | <kbd>Ctrl</kbd> <kbd>0</kbd> |

Zoom steps walk a fixed ladder rather than multiplying, so zooming in and out
again lands back where you started, and the ladder passes through 100%
exactly. The mouse wheel zooms too, anchored on the pointer rather than on the
centre of the view.

## Tools

| Tool | Shortcut |
|---|---|
| Move | <kbd>V</kbd> |
| Rectangular Marquee | <kbd>M</kbd> |
| Elliptical Marquee | <kbd>Shift</kbd> <kbd>M</kbd> |
| Lasso | <kbd>L</kbd> |
| Polygonal Lasso | <kbd>Shift</kbd> <kbd>L</kbd> |
| Magic Wand | <kbd>W</kbd> |
| Color Range | <kbd>Shift</kbd> <kbd>W</kbd> |
| Crop | <kbd>C</kbd> |
| Free Transform | <kbd>Ctrl</kbd> <kbd>T</kbd> |
| Eyedropper | <kbd>I</kbd> |
| Brush | <kbd>B</kbd> |
| Clone Stamp | <kbd>S</kbd> |
| Eraser | <kbd>E</kbd> |
| Gradient | <kbd>Shift</kbd> <kbd>G</kbd> |
| Paint Bucket | <kbd>G</kbd> |
| Blur | <kbd>R</kbd> |
| Sharpen | <kbd>Shift</kbd> <kbd>R</kbd> |
| Smudge | <kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>R</kbd> |
| Dodge | <kbd>O</kbd> |
| Burn | <kbd>Shift</kbd> <kbd>O</kbd> |
| Sponge | <kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>O</kbd> |
| Path Edit | <kbd>A</kbd> |
| Text | <kbd>T</kbd> |
| Shape | <kbd>U</kbd> |
| Hand | <kbd>H</kbd> |
| Zoom | <kbd>Z</kbd> |

## Modifiers on the canvas

| Held | Effect |
|---|---|
| <kbd>Space</kbd> | Pans, from any tool. |
| <kbd>Alt</kbd> with the Zoom tool | Zooms out. |
| Middle mouse button | Pans. |
| Mouse wheel | Zooms, anchored on the pointer. |

## Rebinding

Preferences lists every action with its current chord. Assigning a chord that
is already taken tells you which action currently owns it rather than silently
stealing it.

<kbd>+</kbd> is a valid key on its own: `Ctrl++` binds Ctrl plus the plus key
rather than collapsing to a bare modifier.
