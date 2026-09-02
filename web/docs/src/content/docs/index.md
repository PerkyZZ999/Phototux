---
title: PhotoTux documentation
description: >-
  PhotoTux is a GPU-accelerated image editor for Linux and Wayland. This is the
  guide to using it — installing it, finding your way around, and getting work
  done.
---

PhotoTux is a desktop image editor built with Rust and Qt 6 QML, compositing
entirely on the GPU through `wgpu` and Vulkan. It is designed for Linux, and
specifically for a Wayland session on KDE Plasma 6, though it runs on other
desktops and on X11.

These pages are for people **using** PhotoTux. If you want to work on it
instead, the [engineering handbook](https://github.com/PerkyZZ999/Phototux/tree/main/internal_docs)
in the repository describes how it is built.

<div class="callout callout-warning">

**PhotoTux is pre-release software.** Version 0.1.0 has never been tagged as a
release. The editor opens, paints, composites, saves and reopens documents, and
it is used daily by its author — but it has not been through a public beta, and
the `.ptx` format still moves between versions. Keep backups of anything you
care about.

</div>

## Where to start

If you have not installed it yet, start with
[Installing PhotoTux](/guides/installation/). It builds from source; there are
no distribution packages yet.

Once it runs, [a tour of the workspace](/guides/tour/) explains what each part
of the window is for, and [your first edit](/guides/first-edit/) walks through
opening a photograph, adjusting it non-destructively and exporting the result.

After that the guides go deeper — [layers](/guides/layers/),
[selections and masks](/guides/selections/),
[adjustments and filters](/guides/adjustments/) — and the reference section
has the flat lists: every [tool](/reference/tools/), every
[keyboard shortcut](/reference/shortcuts/), every [blend mode](/reference/blend-modes/)
and every [file format](/reference/file-formats/).

## If you are coming from Photoshop

Most of what you know transfers. Panels, tools and menu entries are where
Adobe puts them, on purpose: the tool shelf runs down the left, the options
bar sits under the menu bar, and Layers, Properties and History are docked on
the right. `Ctrl+J` duplicates a layer, `Ctrl+E` merges down, `Ctrl+Shift+E`
merges visible, `Ctrl+0` fits the document on screen.

Two differences worth knowing before you start:

- **Export is `Ctrl+Alt+Shift+W`**, matching Photoshop's Export As rather than
  its older Save For Web chord. `Ctrl+Shift+E` is Merge Visible here, as it is
  in Photoshop.
- **Documents are tabs, not windows.** Multiple windows are out of scope; a
  session holds however many documents you open, each as a tab under the
  options bar.

## If you are coming from GIMP or Krita

The vocabulary is Photoshop's rather than GIMP's — *layer mask* not *layer
mask channel*, *Levels* not *Curves*, *Canvas Size* for extending the page
and *Image Size* for resampling it. Selections are pixel selections with
feather, expand, contract, smooth and border under **Select ▸ Modify**.

The `.ptx` format is PhotoTux's own layered document. PSD import and export
work for a useful subset, and PhotoTux tells you what it could not carry.

## What PhotoTux will not do

Some things are deliberately out of scope, and knowing them now saves you
looking:

- No cloud storage, accounts, sign-in or telemetry. Nothing leaves your
  machine.
- No AI or generative features.
- No multiple windows — documents are tabs.
- No command-line or terminal interface. PhotoTux is a GUI application.
- No Windows or macOS build for version 1.

## Getting help

- Something behaving wrongly? Check [troubleshooting](/troubleshooting/)
  first, then [open an issue](https://github.com/PerkyZZ999/Phototux/issues).
- A question rather than a bug?
  [Discussions](https://github.com/PerkyZZ999/Phototux/discussions).
- A security problem? Report it
  [privately](https://github.com/PerkyZZ999/Phototux/security/advisories/new),
  not in the public tracker.
