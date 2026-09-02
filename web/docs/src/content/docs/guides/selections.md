---
title: Selections and masks
description: >-
  Drawing a selection, combining several, modifying an edge, and turning a
  selection into a layer mask and back again.
---

A selection limits where an edit lands. Paint, fill, adjust or delete with a
selection active and only the selected pixels change.

PhotoTux's selection is a **pixel selection** — a greyscale mask rather than a
path — so it can be feathered, blurred and painted, and it converts cleanly
into a layer mask.

## Drawing a selection

<div class="callout callout-note">

Every selection tool is on the shelf, in the second band from the top.

</div>

| Tool | Shortcut | Draws |
|---|---|---|
| Rectangular Marquee | <kbd>M</kbd> | A rectangle. |
| Elliptical Marquee | <kbd>Shift</kbd> <kbd>M</kbd> | An ellipse. |
| Lasso | <kbd>L</kbd> | A freehand outline; releasing closes it. |
| Polygonal Lasso | <kbd>Shift</kbd> <kbd>L</kbd> | A polygon, one click per corner. |
| Magic Wand | <kbd>W</kbd> | Everything contiguous with the clicked pixel and close to it in colour. |
| Color Range | <kbd>Shift</kbd> <kbd>W</kbd> | Everything in the layer close to the clicked colour, contiguous or not. |

For the wand and Color Range, the options bar carries a **tolerance** — how
far from the seed colour still counts. Low tolerance picks a narrow band of
colour; high tolerance takes most of the image.

## Combining selections

The options bar has four modes, and they are the standard ones:

- **New** replaces whatever was selected.
- **Add** unions the new shape with the existing selection.
- **Subtract** removes it.
- **Intersect** keeps only the overlap.

## The whole-selection commands

| Command | Shortcut |
|---|---|
| **Select ▸ Select All** | <kbd>Ctrl</kbd> <kbd>A</kbd> |
| **Select ▸ Deselect** | <kbd>Ctrl</kbd> <kbd>D</kbd> |
| **Select ▸ Invert Selection** | <kbd>Ctrl</kbd> <kbd>Shift</kbd> <kbd>I</kbd> |

## Modifying an edge

**Select ▸ Modify** holds five operations. Each opens a prompt asking how far,
because for feathering especially the radius *is* the operation.

| Operation | What it does |
|---|---|
| **Feather…** | Softens the edge over the given radius, so an edit fades in rather than stopping abruptly. |
| **Expand…** | Grows the selection outwards. |
| **Contract…** | Shrinks it inwards. |
| **Smooth…** | Rounds off jagged edges and drops stray specks — useful after a wand selection on a noisy photograph. |
| **Border…** | Replaces the selection with a band straddling its edge, for outlining. |

<div class="callout callout-tip">

**Contract then feather** is the usual recipe for a clean composite: pull the
edge in by a pixel or two so you are not carrying a halo of background, then
soften what is left.

</div>

## Selections and masks

The two are the same kind of thing — a greyscale coverage map — seen from two
places. A selection belongs to the document; a mask belongs to a layer.

- **Select ▸ Selection to Mask** writes the current selection into the active
  layer's mask, so what you selected is what stays visible.
- **Select ▸ Mask to Selection** loads the active layer's mask back into the
  selection, so you can modify it with the tools above and write it back.

That round trip is how you edit a mask with selection tools instead of with a
brush.

## Painting a mask

A mask can also be painted directly. Select the layer's mask in the Properties
panel and paint: black hides, white shows, grey is partial. The Brush,
Gradient and Paint Bucket all work.

A gradient painted on a mask is the standard way to fade one layer into
another — a linear black-to-white gradient across a mask hides one end of the
layer and shows the other.

## Copying and pasting through the clipboard

The clipboard carries three different things, and the Edit menu names each one:

- **Copy** (<kbd>Ctrl</kbd> <kbd>C</kbd>) — the selected pixels.
- **Copy Selection Mask** — the selection itself, as greyscale.
- **Copy Layer Mask** — the active layer's mask, as greyscale.

And three ways back in:

- **Paste as New Layer** (<kbd>Ctrl</kbd> <kbd>V</kbd>)
- **Paste as Selection** — treats what is on the clipboard as a selection.
- **Paste as Layer Mask** — writes it into the active layer's mask.

That is more explicit than a single Paste that guesses, and it is why pasting
a screenshot into a mask does something predictable.

## What a selection affects

- **Painting** is clipped to it.
- **Fill** and **Gradient** apply within it.
- **Delete** clears the selected pixels of the active layer.
- **Filters** apply within it.
- **Adjustment layers** ignore it — they apply to everything below them.
  Confine one with a [clipping mask](/guides/layers/#clipping-masks) or by
  writing the selection into the adjustment's own mask.
