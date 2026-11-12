#!/bin/sh
# The verification chain a commit is gated on: every step must pass, and a failing test, a test
# binary that does not build, or a run with no test at all fails the chain even though the
# summary is filtered (`test-summary.sh`). Run after the last change to the tree.
set -e
cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy -p omnis-vector --lib --locked -- -D warnings
cargo clippy -p omnis-vector --no-default-features --lib --locked -- -D warnings
scripts/test-summary.sh --self-test
cargo test --workspace --locked --no-fail-fast 2>&1 | scripts/test-summary.sh
scripts/lint-sim.sh --self-test
scripts/lint-sim.sh
scripts/check-duplicates.sh
scripts/check-test-modules.sh --self-test
scripts/check-test-modules.sh
cargo run -q -p omnis-cli -- validate packs/base packs/the_lower_kingdoms
cargo run -q -p omnis-cli -- validate packs/base crates/omnis-sim/tests/packs/test
echo VERIFY-GREEN
