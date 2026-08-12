set shell := ["bash", "-euo", "pipefail", "-c"]

# Qt 6 must win over host Qt 5 `qmake`. rust-tc / cargo / nextest live in
# ~/.local/bin and ~/.cargo/bin, which git hooks often omit from PATH.
export PATH := "/usr/lib/qt6/bin:" + env("HOME") + "/.local/bin:" + env("HOME") + "/.cargo/bin:" + env("PATH")
export QMAKE := env("QMAKE", "/usr/lib/qt6/bin/qmake")

# ---------------------------------------------------------
# Fast developer workflow
# ---------------------------------------------------------

fmt:
    cargo fmt --all -- --check


check:
    cargo check --workspace --all-targets --all-features


clippy:
    cargo clippy \
        --workspace \
        --all-targets \
        --all-features \
        -- \
        -D warnings


# Device-backed GPU tests (`gpu-tests`) stay opt-in:
#   cargo test -p phototux_gpu --features gpu-tests
# Clippy/check still compile that cfg via --all-features.
test:
    cargo nextest run \
        --workspace

    cargo test \
        --doc \
        --workspace \
        --all-features


# Fast developer validation.
quick: fmt check clippy test


# Pre-commit / `./scripts/check-rust.sh` default: fmt + clippy only.
precommit: fmt clippy


# Alias developers/agents are expected to use most often.
doctor-check: quick


# ---------------------------------------------------------
# Dependency verification
# ---------------------------------------------------------

deps:
    cargo deny check

    cargo shear --deny-warnings


# ---------------------------------------------------------
# Cargo feature validation
# ---------------------------------------------------------

features:
    cargo hack check \
        --workspace \
        --each-feature \
        --no-dev-deps


features-deep:
    cargo hack check \
        --workspace \
        --feature-powerset \
        --depth 2 \
        --no-dev-deps


# ---------------------------------------------------------
# Comprehensive LOCAL Rust-Toolchain
# ---------------------------------------------------------
#
# NOTE:
# Plain cargo check is intentionally omitted here because
# Clippy already performs compilation/checking as part of its
# analysis.
#
# `rust-tc check` remains useful during normal development because
# it provides the fastest compiler-only feedback path.
#
# SonarQube is a separate PhotoTux gate (`./scripts/check-sonar.sh`).
# `rust-tc doctor` must not call it.
#

doctor: fmt clippy test deps features
    @echo
    @echo "Rust-Toolchain: PASS"


# Nextest does not currently execute doctests, so this is a
# distinct/non-duplicated test category.
doctest:
    cargo test \
        --doc \
        --workspace \
        --all-features


# Optional local coverage. Not part of `rust-tc doctor`.
coverage:
    mkdir -p target/rust-toolchain
    cargo llvm-cov \
        --lcov \
        --output-path target/rust-toolchain/lcov.info \
        nextest \
        --workspace


# ---------------------------------------------------------
# SemVer validation
# ---------------------------------------------------------
#
# Intended primarily for library/public API crates.
# Skip ordinary application binaries (`phototux`).
#

semver package baseline="origin/main":
    cargo semver-checks \
        --package "{{package}}" \
        --baseline-rev "{{baseline}}"


# ---------------------------------------------------------
# Deep verification
# ---------------------------------------------------------

mutants:
    cargo mutants


miri:
    cargo +nightly miri test


fuzz target:
    cargo +nightly fuzz run "{{target}}"


deep: features-deep
    @echo
    @echo "Feature powerset validation complete."
    @echo "Run mutation, Miri and fuzz checks selectively:"
    @echo "  rust-tc mutants"
    @echo "  rust-tc miri"
    @echo "  rust-tc fuzz <target>"


# ---------------------------------------------------------
# Convenience
# ---------------------------------------------------------

clean-toolchain:
    rm -rf target/rust-toolchain


toolchain-help:
    @echo "Rust-Toolchain"
    @echo
    @echo "  rust-tc check         Fast compiler check"
    @echo "  rust-tc quick         Fast developer quality gate"
    @echo "  rust-tc doctor        Full local Rust-Toolchain validation"
    @echo "  rust-tc precommit     fmt + clippy (git hook / check-rust.sh)"
    @echo "  rust-tc features-deep Deeper Cargo feature combinations"
    @echo "  rust-tc semver PKG    Public API compatibility"
    @echo "  rust-tc mutants       Mutation testing"
    @echo "  rust-tc miri          Undefined-behavior checking"
    @echo "  rust-tc fuzz TARGET   Targeted fuzzing"
    @echo "  rust-tc coverage      Optional local LCOV report"
    @echo
    @echo "SonarQube is separate: ./scripts/check-sonar.sh"
