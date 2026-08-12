#!/usr/bin/env bash
# PhotoTux Rust quality gate (wraps Rust-Toolchain `rust-tc`).
# Default (pre-commit): rust-tc precommit  (fmt + clippy)
# Full local gate:      rust-tc doctor     via --full / CHECK_RUST_FULL=1
# Skip Sonar on full:   CHECK_SONAR=0 ./scripts/check-rust.sh --full
# Sonar only:           CHECK_SONAR=1 ./scripts/check-rust.sh
#                       or ./scripts/check-sonar.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

FULL=0
SONAR_REQUESTED=0
SONAR_FORCED_OFF=0
if [[ "${CHECK_RUST_FULL:-0}" == "1" ]]; then
  FULL=1
fi
if [[ "${CHECK_SONAR:-}" == "0" ]]; then
  SONAR_FORCED_OFF=1
elif [[ "${CHECK_SONAR:-0}" == "1" ]]; then
  SONAR_REQUESTED=1
fi
for arg in "$@"; do
  case "$arg" in
    --full) FULL=1 ;;
    --sonar) SONAR_REQUESTED=1 ;;
  esac
done
SONAR=0
if [[ "$SONAR_FORCED_OFF" -eq 0 && ( "$SONAR_REQUESTED" -eq 1 || "$FULL" -eq 1 ) ]]; then
  SONAR=1
fi

if [[ -t 1 ]]; then
  C_RED=$'\033[0;31m'
  C_GRN=$'\033[0;32m'
  C_YLW=$'\033[0;33m'
  C_RST=$'\033[0m'
else
  C_RED="" C_GRN="" C_YLW="" C_RST=""
fi

die() { echo "${C_RED}error:${C_RST} $*" >&2; exit 1; }
info() { echo "${C_YLW}==>${C_RST} $*"; }
ok() { echo "${C_GRN}ok${C_RST} $*"; }

if [[ ! -f "$ROOT/Cargo.toml" ]]; then
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    if git diff --cached --name-only --diff-filter=ACM 2>/dev/null | grep -qE '\.rs$'; then
      die "Rust sources staged but no Cargo.toml workspace yet. Scaffold the workspace first (ADR-006)."
    fi
  fi
  info "No Cargo.toml — skipping Rust-Toolchain (docs-only tree)."
  exit 0
fi

export PATH="${HOME}/.local/bin:${HOME}/.cargo/bin:/usr/lib/qt6/bin:${PATH:-}"
if [[ -x /usr/lib/qt6/bin/qmake ]]; then
  export QMAKE="${QMAKE:-/usr/lib/qt6/bin/qmake}"
fi

command -v rust-tc >/dev/null || die "rust-tc not found. Install Rust-Toolchain's rust-tc onto PATH (just is required)."
command -v just >/dev/null || die "just not found (required by rust-tc). Install: pacman -S just  OR  cargo install just --locked"

if [[ "$FULL" -eq 1 ]]; then
  info "rust-tc doctor"
  rust-tc doctor
  ok "rust-tc doctor"
else
  info "rust-tc precommit (fmt + clippy)"
  rust-tc precommit
  ok "rust-tc precommit"
fi

if [[ "$SONAR" -eq 1 ]]; then
  info "+ SonarQube (Clippy JSON + scanner + quality gate)"
  "$ROOT/scripts/check-sonar.sh"
fi

ok "all Rust checks passed"
exit 0
