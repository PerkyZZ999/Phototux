# Contributing to PhotoTux

Thanks for looking. PhotoTux is a Rust + Qt 6 QML image editor for Linux and
Wayland, and it is small enough that one well-aimed patch makes a visible
difference.

This file is the short version. The long version — architecture, subsystem
contracts, the reasoning behind decisions — lives in the
[Engineering Handbook](internal_docs/README.md), and the handbook is
authoritative wherever the two disagree.

## Ground rules

1. **The handbook is the authority.** If the code and
   [`internal_docs/`](internal_docs/README.md) disagree, that is a finding —
   say so in the issue or pull request rather than quietly following one of
   them. Architectural changes are recorded in the
   [Decision Register](internal_docs/Appendix/Decision-Register.md).
2. **Update the docs with the change.** A change to behaviour updates the
   handbook chapter that describes that behaviour, in the same commit.
3. **The gate is local.** Every required check runs on your machine without a
   CI vendor. Run it before you push.

## Setting up

You need Qt 6.10+, a Rust toolchain matching
[`rust-toolchain.toml`](rust-toolchain.toml), a working Vulkan driver, and
CMake.

On Arch Linux and derivatives:

```bash
sudo pacman -S --needed rust qt6-base qt6-declarative qt6-svg vulkan-icd-loader cmake
```

On Debian and Ubuntu:

```bash
sudo apt install qt6-base-dev qt6-declarative-dev qt6-svg-dev libvulkan-dev cmake build-essential
```

On Fedora:

```bash
sudo dnf install qt6-qtbase-devel qt6-qtdeclarative-devel qt6-qtsvg-devel vulkan-loader-devel cmake
```

Then, in every shell you build from:

```bash
export PATH=/usr/lib/qt6/bin:$PATH
export QMAKE=/usr/lib/qt6/bin/qmake
```

Most distributions put Qt 5's `qmake` first on `PATH`. The build links against
whichever Qt that `qmake` reports, so getting this wrong produces a link
failure that looks like a missing symbol rather than a wrong Qt.

Install the git hooks once per clone:

```bash
./scripts/install-git-hooks.sh
```

## Building and running

```bash
cargo run -p phototux            # the editor
cargo run --release -p phototux  # the editor, at shipping speed
```

Debug builds already optimize dependencies (`opt-level = 2` for third-party
crates), because codec and compression work dominates open and save. Your own
crates stay debuggable.

## The quality gate

The public quality CLI is **`rust-tc`**. Do not call `just` directly.

| Command | When |
|---|---|
| `rust-tc check` | Fastest compiler-only feedback while iterating. |
| `rust-tc quick` | fmt + check + clippy + tests + doctests. **Before every push.** |
| `rust-tc doctor` | The full local gate: fmt, clippy, nextest, doctests, `cargo-deny`, `cargo-shear`, `cargo-hack`. **Before opening a pull request.** |
| `./scripts/check-rust.sh` | What the pre-commit hook runs (fmt + clippy). |
| `./scripts/check-sonar.sh` | Optional SonarQube pass, if you run one locally. |

Device-backed GPU tests are opt-in and need a Vulkan device:

```bash
cargo test -p phototux_gpu --features gpu-tests
```

Do not run `rust-tc mutants`, `rust-tc miri`, `rust-tc fuzz` or
`rust-tc features-deep` on every edit — they exist for deliberate deep passes.

## The workspace

Six crates, each with a boundary that is enforced rather than suggested.

| Package | Owns | Must stay free of |
|---|---|---|
| `phototux_engine` | Document, commands, history, session semantics | Qt, wgpu, filesystem dialogs |
| `phototux_ui` | qtbridge `QObject`s, the host side of the shell | wgpu |
| `phototux_gpu` | `wgpu` pipelines, shaders, compositing | Qt |
| `phototux_canvas` | Qt ↔ wgpu interop, the thin C++ shim | Handwritten C++ beyond that shim |
| `phototux_io` | `.ptx`, raster codecs, the PSD subset | Qt, wgpu |
| `phototux` | Binary, QML AOT module | Business logic |

Directories are kebab-case (`crates/phototux-engine`); package names are
`phototux_*`. The full rationale is
[DR-025](internal_docs/Appendix/Decision-Register.md#dr-025--crate-topology-coarse-workspace).

## Rust conventions

- Edition 2024. Clippy runs with `-D warnings`.
- Library paths return `Result` with typed errors (`thiserror`). `unwrap` and
  `expect` belong in tests, or behind a documented invariant.
- Lint overrides are `#[expect(..., reason = "...")]`, never a silent
  `#[allow]`.
- `unsafe` lives only in `phototux_canvas` and FFI, and every block carries a
  one-line `// SAFETY:` invariant.
- Tracing uses structured fields on hot paths, not `format!` in a loop.
- Clippy's cognitive-complexity threshold is 30; SonarQube's `S3776` is 15.
  Split a helper out rather than raising either.
- Add tests in the crate you changed. Engine logic gets engine tests — the
  core is headless by design
  ([DR-022](internal_docs/Appendix/Decision-Register.md#dr-022--headless-testability-of-core)).

## QML conventions

QML lives in [`qml/`](qml/) and ships through the AOT module.

- Colours, spacing and type come from [`qml/Theme.qml`](qml/Theme.qml). Do not
  start a second palette or spacing scale.
- A translucent colour is **`#AARRGGBB`** — Qt's order, alpha first. Writing
  CSS's `#RRGGBBAA` compiles and renders the wrong colour.
- Icons come from `assets/icons/phosphor/` through
  [`assets/icons/ICON_MAP.md`](assets/icons/ICON_MAP.md), and any new icon key
  must be added to `crates/phototux/qml-aot/CMakeLists.txt` or a guard test
  fails the build.
- User-facing strings go through `qsTr(...)`.
- Use the themed controls (`ThemedButton`, `ThemedComboBox`, `ThemedMenu`, …)
  rather than the bare Qt Quick Controls. The shell runs the Basic style,
  which hardcodes a light palette; a guard test fails the build on an
  unstyled control.
- Size a layout child with `implicitWidth` / `implicitHeight` or
  `Layout.preferredWidth`, never `width` / `height` / `anchors`. `qmllint`'s
  `layout-positioning` and `property-override` must both stay at zero.
- **Never call an `AppSession` slot synchronously from a handler reacting to
  an `AppSession` signal**, and never read `AppSession` from a binding inside
  a model-driven delegate. Both re-enter a borrowed session and abort the
  process. Route through `root.afterHostSlot(fn)`. See
  [32 — Host Slot Re-entrancy](internal_docs/32-Developer-Guide.md#host-slot-re-entrancy).

## Design direction

Two rules decide most UI questions.

**Photoshop decides where.** Panels, tools and menu entries go where Adobe
Photoshop puts them, so that someone moving across does not have to relearn
placements — tool shelf on the left, options bar under the menu bar, Layers
and Properties and History on the right, and each menu item under the menu it
belongs to.

**KDE Plasma 6 decides how it looks.** Spacing, control shapes, focus and
hover treatment follow Plasma 6, rendered through `Theme.qml`.

Surfaces open on the controls most people need and reveal depth on demand.
Group advanced parameters behind a disclosure rather than showing forty at
once.

## Commits and pull requests

- Atomic and conventional-ish: `feat:`, `fix:`, `docs:`, `chore:`, with the
  touched area in parentheses where it helps (`fix(qml): …`).
- One coherent unit of work per commit. If an investigation turns up two
  separate concerns, that is two commits.
- Reference a DR when the change is architectural.
- Never commit secrets, `target/`, `.sonar/`, or `node_modules/`.
- Say in the pull request what you ran. `rust-tc doctor` passing is the bar
  for anything non-trivial; for GUI changes, say what you clicked.

## Reporting a bug

Linux graphics bugs are environment bugs until proven otherwise, so a useful
report names the environment. Please include:

- distribution and kernel;
- session type (Wayland or X11) and compositor;
- desktop, and display scaling;
- GPU, driver and version;
- PhotoTux version or commit;
- what you did, what happened, and what you expected.

The [interactive stability checklist](internal_docs/Appendix/Interactive-Stability-Checklist.md)
lists the GUI edge cases that are already known and covered.

## What is out of scope

So nobody spends a weekend on something that cannot be merged:

- Cloud storage, accounts, remote services, telemetry
- AI or generative features
- Multi-window sessions (documents are tabs —
  [DR-024](internal_docs/Appendix/Decision-Register.md#dr-024--document-session-model))
- A CLI or TUI product
- A Windows or macOS port, for v1
- Replacing the UI toolkit, the FFI, or the zero-copy present path — each of
  these needs a Decision Register entry before code

## Licence

Contributions are accepted under the
[GNU General Public License v3.0 or later](LICENSE), the licence PhotoTux
ships under. By opening a pull request you agree your work is licensed that
way.
