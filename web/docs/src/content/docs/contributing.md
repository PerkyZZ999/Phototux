---
title: Contributing
description: >-
  Where the developer documentation lives, how to build and check the project,
  and what is deliberately out of scope.
---

PhotoTux is free software under the
[GNU GPL v3 or later](https://www.gnu.org/licenses/gpl-3.0.html), and the
source is at
[github.com/PerkyZZ999/Phototux](https://github.com/PerkyZZ999/Phototux).

These documentation pages are for people **using** the editor. The developer
documentation lives in the repository:

| What | Where |
|---|---|
| How to get started contributing | [CONTRIBUTING.md](https://github.com/PerkyZZ999/Phototux/blob/main/CONTRIBUTING.md) |
| The engineering handbook — the authoritative description of the system | [`internal_docs/`](https://github.com/PerkyZZ999/Phototux/tree/main/internal_docs) |
| Why things are the way they are | [Decision Register](https://github.com/PerkyZZ999/Phototux/blob/main/internal_docs/Appendix/Decision-Register.md) |
| Workflow, crate map, quality gate | [Developer Guide](https://github.com/PerkyZZ999/Phototux/blob/main/internal_docs/32-Developer-Guide.md) |
| Reporting a vulnerability | [SECURITY.md](https://github.com/PerkyZZ999/Phototux/blob/main/SECURITY.md) |
| Community expectations | [Code of Conduct](https://github.com/PerkyZZ999/Phototux/blob/main/CODE_OF_CONDUCT.md) |

## The short version

```bash
git clone https://github.com/PerkyZZ999/Phototux.git
cd Phototux
export PATH=/usr/lib/qt6/bin:$PATH
export QMAKE=/usr/lib/qt6/bin/qmake
./scripts/install-git-hooks.sh
cargo run -p phototux
```

Before pushing, `rust-tc quick`. Before opening a pull request,
`rust-tc doctor` — the full local gate: formatting, clippy, tests, doctests,
licence and advisory checks, unused-dependency checks and feature-combination
checks. Everything required runs on your own machine; there is no CI vendor to
wait on.

## How it is put together

Six crates, with boundaries that are enforced by test rather than by
convention:

| Crate | Owns | Stays free of |
|---|---|---|
| `phototux_engine` | Document, commands, history, session semantics | Qt, wgpu |
| `phototux_ui` | The qtbridge QObjects the QML shell binds to | wgpu |
| `phototux_canvas` | Qt ↔ wgpu interop and the thin C++ shim | Handwritten C++ beyond that shim |
| `phototux_gpu` | wgpu pipelines, shaders, compositing | Qt |
| `phototux_io` | `.ptx`, raster codecs, the PSD subset | Qt, wgpu |
| `phototux` | The binary and the ahead-of-time compiled QML module | Business logic |

The core is headless by design, which is why the document, the command spine
and the history can be tested without a GPU or a window.

## Design direction

Two rules settle most interface questions.

**Photoshop decides where.** Panels, tools and menu entries go where Adobe
Photoshop puts them, so that someone moving across does not have to relearn
placements.

**KDE Plasma 6 decides how it looks.** Spacing, control shapes, focus and
hover treatment follow Plasma, drawn from one token file.

Surfaces open on the controls most people need and reveal depth on demand.

## Out of scope

So nobody spends a weekend on a patch that cannot be merged:

- Cloud storage, accounts, remote services, telemetry
- AI or generative features
- Multiple windows — documents are tabs
- A command-line or terminal interface
- A Windows or macOS port, for version 1
- Replacing the UI toolkit, the FFI, or the zero-copy present path — each
  needs a Decision Register entry before any code

## Helping without writing code

- **Report bugs**, with the environment detail listed under
  [troubleshooting](/troubleshooting/#reporting-a-problem). A well-described
  bug is worth more than a guess at a fix.
- **Improve these pages.** They live in
  [`web/docs/`](https://github.com/PerkyZZ999/Phototux/tree/main/web/docs) in
  the same repository. Anything wrong, missing or unclear is an issue worth
  opening.
- **Say what you tried to do and could not.** A description of the task tells
  us more than a description of the feature.
