# phototux_io

`.ptx`, raster codecs, PSD subset. No Qt, no wgpu.

- Untrusted bytes: validate dimensions, counts, and compression before graph commit.
- Native editable format is `.ptx` ([DR-026](../../internal_docs/Appendix/Decision-Register.md#dr-026--native-ptx-container-v1)).
- Tests: `cargo test -p phototux_io`.
