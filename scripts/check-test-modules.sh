#!/usr/bin/env bash
# A crate whose tests build as one binary (`autotests = false`, `tests/main.rs`) runs only the
# files that `tests/main.rs` declares or that `Cargo.toml` names as their own target: a test file
# added to the directory and to neither would never run. This check fails on such a file, and on
# a crate with test files but `autotests = false` and no targets at all.
#   scripts/check-test-modules.sh              check every crate
#   scripts/check-test-modules.sh --self-test  prove the check catches a stray file
set -euo pipefail
cd "$(dirname "$0")/.."

# Prints one line per test file in crate directory $1 that no target reaches.
strays() {
  local crate=$1 file name
  grep -q '^autotests = false' "$crate/Cargo.toml" || return 0
  for file in "$crate"/tests/*.rs; do
    [ -e "$file" ] || continue
    name=$(basename "$file" .rs)
    [ "$name" = main ] && continue
    grep -q "^mod $name;" "$crate/tests/main.rs" 2>/dev/null && continue
    grep -q "^path = \"tests/$name.rs\"" "$crate/Cargo.toml" && continue
    echo "$file"
  done
}

if [ "${1:-}" = "--self-test" ]; then
  scratch=$(mktemp -d)
  trap 'rm -rf "$scratch"' EXIT
  mkdir -p "$scratch/tests"
  printf '[package]\nautotests = false\n\n[[test]]\nname = "integration"\npath = "tests/main.rs"\n' \
    > "$scratch/Cargo.toml"
  printf 'mod listed;\n' > "$scratch/tests/main.rs"
  touch "$scratch/tests/listed.rs" "$scratch/tests/stray.rs"
  [ "$(strays "$scratch")" = "$scratch/tests/stray.rs" ] || {
    echo "check-test-modules: self-test failed: the stray file was not caught" >&2
    exit 1
  }
  echo "check-test-modules: self-test passed"
  exit 0
fi

found=$(for crate in crates/*/; do strays "${crate%/}"; done)
if [ -n "$found" ]; then
  echo "check-test-modules: test files no target runs (add them to tests/main.rs):" >&2
  printf '  %s\n' $found >&2
  exit 1
fi
echo "check-test-modules: every test file is in a target"
