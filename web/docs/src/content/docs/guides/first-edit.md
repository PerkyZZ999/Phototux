---
title: Your first edit
description: >-
  Open a photograph, adjust it without touching the pixels, add a caption, and
  export the result — in about five minutes.
---

This walks through one short edit end to end. It uses only things that are in
the build, and every step is undoable.

## 1. Open a photograph

**File ▸ Open** (`Ctrl+O`), then pick a JPEG or PNG. The file chooser opens in
whichever folder you were last working in, so the second time you do this it
will already be somewhere sensible.

The photograph arrives as a single layer, named after the file, and the
document is zoomed to fit.

## 2. Add an adjustment layer

Rather than changing the photograph's pixels, put an adjustment *above* it.

**Layer ▸ New Adjustment Layer ▸ Vibrance**. A new layer appears at the top of
the stack with an **A** badge, and the Properties panel switches to its single
control.

Drag **Amount** to the right. The canvas updates as you drag, and the
photograph underneath is untouched — hide the adjustment layer with its eye
icon and the original is exactly as it was.

<div class="callout callout-tip">

**Adjustment layers apply to everything below them.** Move one down the stack
and it stops affecting the layers now above it. To confine one to a single
layer, select it and use **Layer ▸ Create Clipping Mask**.

</div>

The other nine adjustments work the same way: Levels, Brightness/Contrast,
Exposure, Hue/Saturation, Black & White, White Balance, Threshold, Posterize
and Invert. See [adjustments and filters](/guides/adjustments/).

## 3. Add a caption

Pick the **Text** tool (`T`) and click on the canvas. A new text layer appears
with a **T** badge, and the Properties panel shows the Character controls.

Type the caption into the **Text** field at the top of that panel. Set the
size, pick an alignment, and set the colour by typing a hex value into the
**Color** field.

<div class="callout callout-note">

**Text stays editable until you bake it.** A text layer is re-editable
indefinitely; it becomes pixels only when you choose **Layer ▸ Bake Text**.
That is undoable too — undo brings the words back.

</div>

## 4. Move things around

Switch to the **Move** tool (`V`) and drag the caption where you want it. The
options bar fills with align and distribute buttons: with one layer selected
they align it to the canvas, and with more than one they align the layers to
each other.

## 5. Look at the history

Open the **History** tab beside Layers. Every step you have taken is listed —
opening the file, adding the adjustment, changing its amount, adding the text.

Click a step to go back to it. The steps after it stay in the list, dimmed,
and clicking one of *those* walks forward again. `Ctrl+Z` and `Ctrl+Shift+Z`
do the same thing one step at a time.

## 6. Save the document

**File ▸ Save** (`Ctrl+S`) writes a `.ptx` file — PhotoTux's own format, which
keeps every layer, mask and adjustment exactly as you left it. This is what
you reopen to keep working.

## 7. Export a picture

**File ▸ Export** (`Ctrl+Alt+Shift+W`) writes a flattened image in whatever
format the filename's extension names — `.png`, `.jpg`, `.webp`, `.tif`,
`.bmp` or `.gif`. This is what you send to somebody.

<div class="callout callout-warning">

**Save and Export are different things.** Save keeps your layers. Export
flattens them. Exporting a PNG and closing the document loses the layer stack,
so do both.

</div>

## What next

- [Working with layers](/guides/layers/) — the stack, blend modes, groups,
  masks and styles
- [Selections and masks](/guides/selections/) — editing part of a layer
- [Adjustments and filters](/guides/adjustments/) — the full non-destructive
  vocabulary
- [Keyboard shortcuts](/reference/shortcuts/) — the list worth learning early
