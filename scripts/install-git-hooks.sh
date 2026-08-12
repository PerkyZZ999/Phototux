#!/usr/bin/env bash
# Point this repo at .githooks/ (portable; no global git config required).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

chmod +x "$ROOT/.githooks/pre-commit" "$ROOT/scripts/check-rust.sh" "$ROOT/scripts/check-sonar.sh" "$ROOT/scripts/install-git-hooks.sh"

git config core.hooksPath .githooks
echo "ok: core.hooksPath=.githooks"
echo "    pre-commit runs: scripts/check-rust.sh → rust-tc precommit (fmt + clippy)"
echo "    full local gate: rust-tc doctor"
echo "    full + Sonar:    CHECK_RUST_FULL=1 ./scripts/check-rust.sh"
