# phototux_engine

Portable document, commands, history, and session semantics.

- No Qt, no wgpu, no filesystem dialogs.
- Library paths: `Result` + typed errors (`thiserror`).
- Tests: `cargo test -p phototux_engine`.
- Mutation enters `SessionState::invoke` / the command spine ([08](../../internal_docs/08-Command-System.md)).
