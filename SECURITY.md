# Security Policy

## Supported versions

PhotoTux is pre-1.0 and ships from `main`. Only the current `main` branch
receives fixes; there are no maintained release branches yet.

| Version | Supported |
|---|---|
| `main` | Yes |
| Anything older | No |

## Reporting a vulnerability

**Please do not open a public issue for a security problem.**

Report it privately through GitHub's
[security advisory form](https://github.com/PerkyZZ999/Phototux/security/advisories/new).
That opens a private channel with the maintainer and lets a fix be prepared
before the details are public.

A useful report includes:

- what the problem is, and what an attacker gets out of it;
- the file or crate involved, if you know it;
- a reproduction — for a file-parsing issue, the smallest input that triggers
  it;
- the commit you tested, and your environment (distribution, Qt version, GPU
  driver).

You can expect an acknowledgement within a week. PhotoTux is maintained by one
person in their own time, so a fix may take longer than that; you will be told
where it stands rather than left waiting.

## What counts

PhotoTux is a local desktop application with no network features, no accounts
and no telemetry. That narrows the interesting surface considerably. The parts
worth attention are the ones that read bytes somebody else produced:

- **`phototux_io`** — the `.ptx` container, the PSD subset, and every raster
  codec path. These parse untrusted input behind size and dimension limits
  (`MAX_DIMENSION`, `MAX_RASTER_BYTES`). A crafted file that reads out of
  bounds, allocates without limit, or escapes those limits is in scope.
- **ICC profile handling** — profile bytes are validated before being embedded
  or applied.
- **Recovery files** — documents written by a session that ended badly are
  read back at next launch.
- **`unsafe` and FFI in `phototux_canvas`** — the Qt ↔ wgpu interop is the only
  place in the workspace where `unsafe` is permitted.

## What does not count

- Crashes caused by a broken or missing GPU driver, or by a Vulkan
  implementation that does not meet the requirements. Report those as
  ordinary bugs.
- Denial of service from opening a genuinely enormous document. The limits
  are documented; a document inside them that is merely slow is a performance
  issue.
- Vulnerabilities in Qt, Mesa, or another dependency, unless PhotoTux uses
  them in a way that makes the problem worse. Report those upstream —
  `cargo-deny` tracks Rust advisories in this repository.

## Disclosure

Once a fix is available it lands on `main`, the advisory is published with
credit to the reporter unless they prefer otherwise, and the
[changelog](CHANGELOG.md) records it.
