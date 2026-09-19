#!/bin/sh
# The verification chain a commit is gated on: every step must pass, and a failing test fails
# the chain even though the summary is filtered. Run after the last change to the tree.
set -e
cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy -p omnis-app --no-default-features --lib --locked -- -D warnings
cargo test --workspace --locked --no-fail-fast 2>&1 \
  | grep -E "^test result|FAILED|panicked" \
  | awk '/^test result/{p+=$4; f+=$6; i+=$8} !/^test result/{print} END {print "tests passed", p, "failed", f, "ignored", i; exit (f > 0)}'
scripts/lint-sim.sh --self-test
scripts/lint-sim.sh
scripts/check-duplicates.sh
cargo run -q -p omnis-cli -- validate packs/base packs/test
echo VERIFY-GREEN
