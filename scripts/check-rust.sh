#!/usr/bin/env bash
# PhotoTux Rust quality gate: rustfmt + clippy (fast default for pre-commit).
# Optional full gate: CHECK_RUST_FULL=1 ./scripts/check-rust.sh  (adds rust-doctor)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

FULL=0
if [[ "${CHECK_RUST_FULL:-0}" == "1" ]] || [[ "${1:-}" == "--full" ]]; then
  FULL=1
fi

# Colors only if TTY
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

# --- Presence of Rust project ---
if [[ ! -f "$ROOT/Cargo.toml" ]]; then
  # Staged .rs without workspace = broken state
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    if git diff --cached --name-only --diff-filter=ACM 2>/dev/null | grep -qE '\.rs$'; then
      die "Rust sources staged but no Cargo.toml workspace yet. Scaffold the workspace first (ADR-006)."
    fi
  fi
  info "No Cargo.toml — skipping rustfmt/clippy (docs-only tree)."
  exit 0
fi

export PATH="${HOME}/.cargo/bin:/usr/lib/qt6/bin:${PATH:-}"

command -v cargo >/dev/null || die "cargo not found"
command -v rustfmt >/dev/null || die "rustfmt not found (rustup component add rustfmt)"
command -v cargo-clippy >/dev/null 2>&1 || cargo clippy -V >/dev/null 2>&1 \
  || die "clippy not found (rustup component add clippy)"

# Prefer Qt 6 qmake when present (host often has Qt5 as default qmake)
if [[ -x /usr/lib/qt6/bin/qmake ]]; then
  export PATH="/usr/lib/qt6/bin:$PATH"
  export QMAKE="${QMAKE:-/usr/lib/qt6/bin/qmake}"
fi

info "1/2 rustfmt --check"
cargo fmt --all -- --check
ok "rustfmt"

info "2/2 clippy (-D warnings)"
cargo clippy --workspace --all-targets --all-features -- -D warnings
ok "clippy"

if [[ "$FULL" -eq 1 ]]; then
  RUST_DOCTOR_BIN=""
  if command -v rust-doctor >/dev/null 2>&1; then
    RUST_DOCTOR_BIN="rust-doctor"
  elif [[ -x "${HOME}/.bun/bin/rust-doctor" ]]; then
    RUST_DOCTOR_BIN="${HOME}/.bun/bin/rust-doctor"
  elif [[ -x "${HOME}/.cargo/bin/rust-doctor" ]]; then
    RUST_DOCTOR_BIN="${HOME}/.cargo/bin/rust-doctor"
  fi
  [[ -n "$RUST_DOCTOR_BIN" ]] || die "rust-doctor not found. Install: cargo install rust-doctor"

  info "+ rust-doctor (offline, fail on error) [CHECK_RUST_FULL]"
  set +e
  "$RUST_DOCTOR_BIN" "$ROOT" --offline --fail-on error -v
  rd_ec=$?
  set -e
  case "$rd_ec" in
    0) ok "rust-doctor" ;;
    3) die "rust-doctor quality gate failed (errors present)" ;;
    2) die "rust-doctor scan failed (compile/discovery). Fix build first." ;;
    *) die "rust-doctor exited $rd_ec" ;;
  esac
fi

ok "all Rust checks passed"
exit 0
