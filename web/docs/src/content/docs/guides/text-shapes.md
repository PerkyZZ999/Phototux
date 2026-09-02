---
title: Text, shapes and smart objects
description: >-
  The three kinds of layer that hold something other than pixels — words, an
  editable path, and wrapped source pixels — and when to turn each into pixels.
---

Three layer kinds carry structure rather than a bitmap. All three stay
editable indefinitely, and all three convert to pixels when you ask them to.

## Text

Pick the **Text** tool (<kbd>T</kbd>) and click on the canvas. A text layer
appears with a **T** badge, and the Properties panel shows the Character
controls.

| Control | What it sets |
|---|---|
| **Text** | The words. Editing this field is what changes the layer. |
| **Font** | Any font installed on your system. |
| **Size** | Point size. |
| **Tracking** | Space added between every pair of letters. |
| **Leading** | Line spacing, as a multiple of the size. |
| **Alignment** | Left, centre or right. |
| **Color** | A hex value. |
| **Frame W / Frame H** | The text frame. Leave at 0 to let the text set its own extent. |
| **Wrap within frame** | Wraps the text to the frame width instead of running on. |

<div class="callout callout-note">

**A text layer renders as an overlay while it is selected.** It composites
into the document like any other layer, but the live editing frame is drawn
only while the layer is active — so switching to another layer is not making
the text disappear.

</div>

### Baking text

**Layer ▸ Bake Text** turns the layer into pixels. Do it when you want to
paint on the letters, apply a filter to them, or hand the document to
something that does not have your font.

Baking is undoable — undo brings the words back — but only while the document
is open. Save a `.ptx` before baking if you might want to re-edit the words
later.

## Shapes

Pick the **Shape** tool (<kbd>U</kbd>) or use **Layer ▸ Shape** to insert one
of the built-in presets. A shape layer holds an editable path with a fill and
a stroke, both changeable from the Properties panel without touching the
geometry.

### Editing a path

The **Path Edit** tool (<kbd>A</kbd>) works on the anchors:

- Drag an anchor to move it.
- Click on a segment to add an anchor.
- Select an anchor and delete it to remove it.
- Close or open the path from the Properties panel.

### Combining shapes

**Layer ▸ Combine Shapes** applies a boolean operation between two shape
layers — union, subtract, intersect and exclude — producing one path.

### Turning a shape into pixels

- **Layer ▸ Rasterize Shape** replaces the path with pixels.
- **Layer ▸ Stroke Path to Layer** paints the path's outline onto a raster
  layer with the current brush, leaving the shape intact.

Rasterizing is undoable, and the undo restores the editable path.

## Smart objects

A smart object wraps a layer's pixels so that transforms are re-applied to the
**source** rather than accumulated on the result.

The difference shows up the second time you transform something. On an
ordinary raster layer, scaling to 50% and back to 100% leaves you with half
the detail, permanently. On a smart object, the placement is stored — a scale
factor, a rotation, an offset — and applied to the pristine source each time,
so the same round trip comes back at full quality.

### Using one

- **Layer ▸ Smart Objects ▸ Convert to Smart Object** wraps the active layer.
- Transform it as usual with **Free Transform** (<kbd>Ctrl</kbd> <kbd>T</kbd>).
  Each commit folds into the placement: rotations add, scales multiply,
  offsets sum.
- **Layer ▸ Smart Objects ▸ Reset Placement** puts the source back at its
  original size and position.
- **Layer ▸ Smart Objects ▸ Rasterize Smart Object** bakes it down to ordinary
  pixels and drops the source.

<div class="callout callout-tip">

**Convert before you scale, not after.** A smart object cannot recover detail
that was already thrown away by an earlier transform on a raster layer.

</div>

### Sources and documents

A smart object's source travels with its document: it is written into the
`.ptx` file, restored when you reopen it, and dropped when you close the tab.
Opening two documents that both contain smart objects keeps their sources
apart.

Rasterizing is undoable, and the undo restores the source — the source lives
as long as its document, not as long as the layer.
