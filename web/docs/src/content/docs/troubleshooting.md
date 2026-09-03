---
title: Troubleshooting
description: >-
  The problems people actually hit — a black canvas, a missing Qt, a slow
  viewport, a lost document — and what to do about each.
---

## Before anything else

Run PhotoTux from a terminal. Almost every problem here prints something, and
the something is usually the answer:

```bash
RUST_LOG=debug cargo run --release -p phototux
```

And collect the environment, because Linux graphics problems are environment
problems until proven otherwise:

```bash
# Distribution and kernel
cat /etc/os-release | head -2; uname -r
# Session
echo "$XDG_SESSION_TYPE $XDG_CURRENT_DESKTOP"
# GPU and driver
vulkaninfo --summary 2>/dev/null | head -30
# Qt
qmake6 -query QT_VERSION
```

## The build

### `qmake` reports Qt 5, or linking fails with missing Qt symbols

`PATH` and `QMAKE` are pointing at Qt 5. Set them, then force a relink — a
stale object file linked against the wrong Qt will not fix itself:

```bash
export PATH=/usr/lib/qt6/bin:$PATH
export QMAKE=/usr/lib/qt6/bin/qmake
cargo clean -p phototux_ui -p phototux_canvas
cargo run --release -p phototux
```

If `/usr/lib/qt6/bin` is not where your distribution keeps Qt 6, ask it:
`qmake6 -query QT_INSTALL_BINS`.

### `Could not find Qt6Config.cmake`

The Qt 6 **development** packages are missing, not just the runtime. On Debian
and Ubuntu those are the `-dev` packages; on Fedora and openSUSE, `-devel`.
See [installation](/guides/installation/#install-the-dependencies).

### The Qt version is too old

PhotoTux needs Qt 6.10 or newer. Check with `qmake6 -query QT_VERSION`. If
your distribution ships something older, install Qt 6.10 from the
[Qt online installer](https://www.qt.io/download-qt-installer) and point
`PATH` and `QMAKE` at its `bin` directory.

### `error: linker 'cc' not found`

No C++ toolchain. Install `build-essential` (Debian, Ubuntu), `gcc-c++`
(Fedora, openSUSE) or `base-devel` (Arch).

## Starting up

### The canvas is black, or the window never appears

Vulkan is not working. In order:

1. **Is there a Vulkan device at all?** `vulkaninfo --summary`. If it lists
   nothing, install the driver for your hardware — `vulkan-radeon`,
   `vulkan-intel` or `nvidia-utils` on Arch; `mesa-vulkan-drivers` on Debian
   and Ubuntu.
2. **Is the loader installed?** `vulkan-icd-loader` on Arch, `libvulkan1` on
   Debian and Ubuntu.
3. **Two GPUs?** A laptop with switchable graphics may be offering the wrong
   one. Try forcing the discrete card with `DRI_PRIME=1`, or the integrated
   one with `DRI_PRIME=0`.
4. **NVIDIA on Wayland?** Check that `nvidia_drm.modeset=1` is set. Without
   it, Wayland sessions on NVIDIA misbehave in ways that are not specific to
   PhotoTux.

The log line naming the adapter PhotoTux selected is printed at startup with
`RUST_LOG=debug`.

### The status bar does not say GPU ACCELERATED

PhotoTux fell back to the CPU compositor. That path exists for tests and for
degraded operation; it is not meant to be what you edit on, and it will be
slow on anything large. The cause is the same as a black canvas above — work
through that list.

### It starts, then exits immediately

Run it from a terminal and read the last lines. The common causes are a Vulkan
device that disappeared mid-initialisation, and a saved workspace referring to
something that no longer exists. For the latter, tick **Safe start next
launch** in Preferences — or, if you cannot get that far, delete the saved
workspace from `~/.config/phototux/` and start again.

## While you work

### Everything is slow, or the frame counter is low

- Check the status bar says **GPU ACCELERATED**. If it does not, see above.
- Check the **composite** number in the status bar. A large stack of layers
  with several filters is genuinely more work; try hiding layers to find the
  expensive one.
- Software rendering will be forced if `LIBGL_ALWAYS_SOFTWARE` or
  `WLR_RENDERER=pixman` is set in your environment. Unset them.

### The tablet does not draw, or the pressure is ignored

- Confirm the tablet works elsewhere in the session first.
- On Wayland, the compositor mediates tablet input. If it works in another
  application and not in PhotoTux, that is worth
  [reporting](https://github.com/PerkyZZ999/Phototux/issues) with your
  compositor and tablet model.

### The file dialog opens in the wrong folder

It opens in the folder of the document you have open, and otherwise in
whichever folder a dialog was last accepted in. If it is somewhere unexpected,
that is the last folder you confirmed a file in — not a default.

### Text disappeared from the canvas

A text layer draws its live editing frame only while it is the active layer.
Selecting a different layer removes the frame, not the text. If the text
itself is gone, check its colour against what is underneath it — and check the
layer's eye icon.

### A layer looks like it did nothing

- Is its **eye** on?
- Is its **opacity** above zero?
- Is its **blend mode** one that cannot affect this backdrop? Multiply over
  black does nothing; Screen over white does nothing.
- Is there an active **selection** limiting the edit? <kbd>Ctrl</kbd>
  <kbd>D</kbd> deselects.
- Is a **lock** on?

### Undo does not go back far enough

History retention defaults to 128 steps and is set in
[Preferences](/guides/workspace/#appearance-and-accessibility).

## Files

### A PSD opened with a compatibility report

That is not an error. The report lists what PhotoTux's PSD subset could not
carry — see [file formats](/reference/file-formats/#psd). The document has
opened; the report says what is different about it.

### A file is refused as too large

The file boundary caps any dimension at 32,768 pixels and any decoded RGBA
buffer at 512 MB. A file over either is refused rather than loaded until
memory runs out.

### A `.ptx` from a newer build will not open

Older builds cannot read newer documents while the format is pre-1.0. Update
PhotoTux.

### The application crashed and I lost work

Restart it. Documents open when a session ends badly are offered back from the
recovery store at the next launch. Recovery holds what autosave last managed
to write, which may be a few minutes behind — see
[autosave and recovery](/guides/files/#autosave-and-recovery).

## Reporting a problem

[Open an issue](https://github.com/PerkyZZ999/Phototux/issues) with:

- what you did, what happened, and what you expected;
- the PhotoTux version or commit;
- distribution and kernel;
- session type (Wayland or X11), desktop and compositor;
- display scaling;
- GPU, driver and version;
- Qt version;
- the console output, ideally with `RUST_LOG=debug`.

A security problem goes
[privately](https://github.com/PerkyZZ999/Phototux/security/advisories/new)
instead, not into the public tracker.

## "8192 px is the limit on each edge"

Every layer is a full-size image on the GPU, and the largest texture PhotoTux
asks for is 8192 × 8192. A document bigger than that on either edge cannot be
composited at all, so New Document, Image Size, Canvas Size and Open all refuse
it rather than opening something that would draw nothing.

If you are trying to open a photograph larger than this — a scan, or a stitched
panorama — resize it first in another tool. Tiled compositing, which is what
would lift the limit, is not implemented.
