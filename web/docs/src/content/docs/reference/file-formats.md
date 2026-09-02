---
title: File formats
description: >-
  What PhotoTux reads and writes, what each format carries, and the limits the
  file boundary enforces on anything it opens.
---

## Support matrix

| Format | Extensions | Open | Save | Layers |
|---|---|---|---|---|
| **PhotoTux document** | `.ptx` | Yes | Yes | Yes |
| **Photoshop** | `.psd` | Yes | Yes | Subset |
| **PNG** | `.png` | Yes | Yes | Flattened |
| **JPEG** | `.jpg`, `.jpeg` | Yes | Yes | Flattened |
| **WebP** | `.webp` | Yes | Yes | Flattened |
| **TIFF** | `.tif`, `.tiff` | Yes | Yes | Flattened |
| **BMP** | `.bmp` | Yes | Yes | Flattened |
| **GIF** | `.gif` | Yes | Yes | Flattened, first frame |

The export format is chosen by the extension you type in the Save or Export
dialog. An extension PhotoTux does not recognise is refused rather than
guessed at.

## `.ptx` — the native document

A chunked container holding:

- the **layer graph** — every layer's kind, name, opacity, blend mode,
  visibility, locks, clipping, blend-if ranges, styles and filter plan;
- a **raster** per raster layer;
- a **mask** per masked layer;
- the **source pixels** of every smart object, so a placement can be
  re-applied rather than re-composed;
- the document's embedded ICC profile, if it has one.

Layer rasters are stored as PNG inside the container and the whole thing is
deflate-compressed.

<div class="callout callout-warning">

**The format is pre-1.0.** The current container version is 2; version 1
documents still open. Newer builds read older documents, but an older build
will not read a newer document. While PhotoTux is pre-release, export anything
important to PNG or PSD as well.

</div>

## PSD

PhotoTux reads and writes a deliberate subset:

**Carried:** RGB colour, 8 bits per channel, layers with names, opacity,
visibility and blend modes, and the flattened composite.

**Not carried:** CMYK, Lab and indexed colour; 16- and 32-bit files;
adjustment layers, layer styles, smart objects and text as PSD understands
them; layer comps; paths.

On import, everything the subset could not carry is listed in a
**compatibility report** rather than dropped in silence. A report is not an
error — the file has opened, and the report says what is different about it.

On export, features with no PSD equivalent are flattened into the layer they
belong to.

<div class="callout callout-note">

**PSD is an interchange format here, not a working format.** Round-tripping a
document through PSD loses adjustment layers and filter plans. Keep the `.ptx`
as your master.

</div>

## Flat formats

| Format | Alpha | Compression | Notes |
|---|---|---|---|
| **PNG** | Yes | Lossless | The safe default for exporting. |
| **JPEG** | No | Lossy | Written at quality 92 — visually lossless for most images. Transparent areas are composited against the layers below. |
| **WebP** | Yes | Lossy or lossless | Smaller than PNG at similar quality. |
| **TIFF** | Yes | Lossless | For handing to print or to other editors. |
| **BMP** | No | None | Large. Included because some tools still want it. |
| **GIF** | 1-bit | Lossless, 256 colours | First frame only. PhotoTux is not an animation editor. |

## Limits at the file boundary

Every file PhotoTux opens is somebody else's bytes, so the parsers work behind
fixed limits:

| Limit | Value |
|---|---|
| Maximum width or height | 32,768 pixels |
| Maximum decoded RGBA buffer | 512 MB |

A file that exceeds either is refused with a message naming the reason, rather
than being loaded until something runs out of memory.

## Colour profiles

ICC profiles can be **embedded** in a document, **assigned** to it without
changing its numbers, or **converted to** so the appearance is preserved
across spaces. sRGB and Display-P3 are built in, and an arbitrary profile can
be embedded from a file.

Profile bytes are validated before they are written into a document, so a
truncated or malformed `.icc` is rejected at the point you choose it rather
than at the point something tries to use it.

Soft-proofing shows what a document will look like through a chosen output
profile without changing the document.

## Recovery files

Documents open when a session ends badly are written to a recovery store and
offered back at the next launch. They are not a format you open by hand — see
[opening, saving and exporting](/guides/files/#autosave-and-recovery).
