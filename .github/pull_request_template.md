## What this changes

<!-- One paragraph. What behaviour is different after this lands? -->

## Why

<!-- The problem. Link an issue if there is one. Reference a DR if this is
     architectural: internal_docs/Appendix/Decision-Register.md -->

## How it was verified

<!-- Delete what does not apply. -->

- [ ] `rust-tc quick` passes
- [ ] `rust-tc doctor` passes
- [ ] Tests added in the crate that changed
- [ ] Ran the editor and exercised the change by hand — say what you clicked
- [ ] `cargo test -p phototux_gpu --features gpu-tests` (device-backed, if the
      change touches the GPU path)

## Documentation

- [ ] The handbook chapter describing this behaviour is updated in this same
      change
- [ ] `CHANGELOG.md` entry added, if this is user-visible

## Notes for the reviewer

<!-- Anything that would otherwise be a surprise: a trade-off you took, a
     follow-up you deliberately left, a false trail worth knowing about. -->
