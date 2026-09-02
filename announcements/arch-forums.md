# Arch Linux Forums announcement

**Where:** [Community Contributions](https://bbs.archlinux.org/viewforum.php?id=39)
— that is the subforum for announcing software you wrote yourself. Not Newbie
Corner, not Applications & Desktop Environments.

**Before posting:**

- Read the [forum etiquette](https://wiki.archlinux.org/title/General_troubleshooting/Forum_etiquette)
  once. The community is friendly to people who did their homework and short
  with people who did not.
- Post from the account you will keep answering from. An announcement thread
  is a support thread within a day.
- The board runs FluxBB and takes **BBCode**, not Markdown. The version below
  is BBCode; paste it as-is.
- Expect the first replies to be "why not GIMP/Krita", "why Qt", and "is this
  another AI-generated project". Answers to all three are in the post; be
  ready to give them again in your own words.

---

## The post

**Subject:** `PhotoTux — a GPU-accelerated image editor for Wayland (Rust + Qt 6 + wgpu)`

```bbcode
[b]PhotoTux[/b] is an image editor I have been building for Linux, and
specifically for a Wayland session on Plasma 6. It is Rust and Qt 6 QML with a
zero-copy wgpu/Vulkan canvas.

[url=https://phototux.xyz]phototux.xyz[/url] · [url=https://docs.phototux.xyz]docs.phototux.xyz[/url] · [url=https://github.com/PerkyZZ999/Phototux]github.com/PerkyZZ999/Phototux[/url] (GPL-3.0-or-later)

[b]Up front: this is pre-release.[/b] Version 0.1.0, never tagged, no public
beta, and the native document format still moves between versions. It opens,
paints, composites, saves and reopens documents and I use it daily, but keep
backups of anything you care about. I am posting it here because I would
rather find out now what breaks on hardware that is not mine.

[b]Why another one[/b]

Not because GIMP and Krita are bad — they are not, and Krita in particular is
excellent at what it is for. Two things I wanted that I could not get:

[list]
[*][b]Frame time treated as a feature.[/b] Compositing happens on the GPU and
stays there. Document pixels are written by wgpu and presented by Qt's Vulkan
RHI on the same device, so nothing crosses the language boundary while you
pan, zoom or paint. A 4K document with a stack of layers moves at the refresh
rate of the monitor, not the speed of a memcpy.
[*][b]Photoshop's layout, drawn in Plasma's idiom.[/b] Tool shelf on the left,
options bar under the menu bar, Layers/Properties/History on the right, menu
entries under the menu they belong to. Nobody moving across should have to
relearn where anything lives. The look is Breeze — same accent, same spacing,
same focus treatment, from one token file.
[/list]

[b]What is in it[/b]

[list]
[*]Raster, group, text, shape, fill, adjustment and smart-object layers
[*]28 blend modes, layer masks, clipping masks, Blend If, eight layer styles
[*]Ten non-destructive adjustment layers and a per-layer filter plan with 13
effects, previewed in a gallery that does not touch the document until you
commit
[*]Smart objects — a transform re-applies to the pristine source instead of
accumulating on the pixels
[*]Selections by rectangle, ellipse, freehand, polygon and colour, with
Expand/Contract/Feather/Smooth/Border, each asking for its radius
[*]Brush, clone stamp, eraser, gradient, paint bucket, dodge/burn/sponge,
blur/sharpen/smudge
[*]Native layered .ptx, layered PSD import and export with a compatibility
report, PNG/JPEG/WebP/TIFF/BMP/GIF, ICC embed/assign/convert/soft-proof
[*]Document tabs, dockable panels, workspace presets, a command palette, and
rebindable shortcuts with conflict detection
[*]Crash recovery for anything open when a session ends badly
[/list]

No cloud, no accounts, no telemetry, no AI features. It opens files on your
disk and writes them back.

[b]Building it[/b]

There is no AUR package yet — I would rather have a few people build it and
tell me what broke first. On Arch:

[code]sudo pacman -S --needed rust qt6-base qt6-declarative qt6-svg vulkan-icd-loader cmake

git clone https://github.com/PerkyZZ999/Phototux.git
cd Phototux
export PATH=/usr/lib/qt6/bin:$PATH
export QMAKE=/usr/lib/qt6/bin/qmake
cargo run --release -p phototux[/code]

The two exports matter: the host qmake is often Qt 5, and the build links
against whichever Qt that qmake reports. Needs Qt 6.10+, Rust 1.87+ and a
working Vulkan driver.

[b]How it is put together[/b]

Six crates, with boundaries enforced by test rather than by convention. The
engine — document, command spine, history — has no Qt and no wgpu in it, which
is why the core is testable headless. phototux_gpu has no Qt; phototux_io has
neither. The Qt↔wgpu interop is a thin C++ shim and the only place unsafe is
allowed.

[b]On the AI question[/b]

I will get asked, so: I wrote this with an AI assistant as a pair programmer,
and the repository says so — the authors are "Charles W. (PerkyZZ999)" and
"Claude/Cursor". Every line is reviewed, the architecture decisions are
written down in a decision register in the repo, and there is a local quality
gate (fmt, clippy at -D warnings, tests, cargo-deny, cargo-shear, cargo-hack)
that has to pass before anything lands. Judge it on whether it works and
whether the code reads well. If it does not, tell me where.

[b]What would help most[/b]

[list]
[*]Does it start on your hardware? Intel, AMD and NVIDIA all matter, as do
the various Mesa drivers.
[*]Anything wrong under fractional scaling.
[*]Tablet input on Wayland — I have one tablet and one compositor.
[*]PSD files that import badly. The compatibility report should tell you what
it could not carry; if it stays quiet and the file is still wrong, that is a
bug I want.
[/list]

Issues: [url=https://github.com/PerkyZZ999/Phototux/issues]github.com/PerkyZZ999/Phototux/issues[/url].
Distribution, kernel, session type, compositor, scaling, GPU and driver
version in the report, please — Linux graphics bugs are environment bugs until
proven otherwise.
```

---

## A shorter version

For places with a length limit, or for a comment rather than a thread.

```text
PhotoTux — a GPU-accelerated image editor for Linux and Wayland. Rust and
Qt 6 QML, zero-copy wgpu/Vulkan canvas, Photoshop's layout drawn in Plasma's
idiom. Layers, masks, 28 blend modes, non-destructive adjustments and filters,
smart objects, native .ptx plus layered PSD. No cloud, no accounts, no
telemetry.

Pre-release: 0.1.0, never tagged, format still moving. Builds from source;
needs Qt 6.10+, Rust 1.87+ and a Vulkan driver.

https://phototux.xyz · https://docs.phototux.xyz · https://github.com/PerkyZZ999/Phototux
```

---

## After posting

- Put the thread URL in the repository README under a "Community" heading so
  people arriving from GitHub can find the discussion.
- Watch for the first "does not build" reply. It is almost always the Qt 5
  `qmake` trap, and it is worth answering in the thread rather than only in
  the docs, because the next person searches the thread.
- If the reception is good, the natural next step is an AUR `-git` package.
  Do not post about it until it exists.
