#!/usr/bin/env bash
set -o nounset -o pipefail -o errexit -o xtrace

# This script tries to emulate a run of CI.yml. If you can run this script
# without errors you can be reasonably sure that CI will pass for real when you
# push the code.

# CI sets this, so we should too
export CARGO_TERM_COLOR=always

cargo fmt -- --check

RUSTDOCFLAGS='--deny warnings' cargo doc --locked --no-deps --document-private-items

cargo clippy --all-targets --all-features -- -D clippy::all

cargo build --locked

cargo test --locked
