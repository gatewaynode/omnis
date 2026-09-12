#!/usr/bin/env bash
# Simulation crate lint (CLAUDE.md "Simulation Crate Lints", ARCHITECTURE.md §11).
# Fails with file:line on any banned construct in the simulation crates. Run from anywhere.
#   scripts/lint-sim.sh              lint the workspace
#   scripts/lint-sim.sh --self-test  plant a violation of every kind and require the lint to catch it
set -uo pipefail
cd "$(dirname "$0")/.."

# Every simulation crate. A crate directory that does not exist yet is skipped; when a crate is
# created it must already be in this list (CLAUDE.md).
SIM_CRATES=(omnis-core omnis-expr omnis-data omnis-rules omnis-gen omnis-eco omnis-story omnis-sim)
# Crates that must declare #![no_std]. omnis-data (file I/O) and omnis-expr (Rhai) are std.
NO_STD_CRATES=(omnis-core omnis-rules omnis-gen omnis-eco omnis-story omnis-sim)
# Marker that exempts one line, for the rare documented case (e.g. a comment quoting a banned name).
ALLOW='lint-sim: allow'

fail=0
say() { echo "lint-sim: $*" >&2; fail=1; }

# grep_src PATTERN DIR LABEL — report matching Rust lines not carrying the allow marker.
grep_src() {
  local pat="$1" dir="$2" label="$3" hits
  hits=$(grep -rnE --include='*.rs' -- "$pat" "$dir" | grep -v -- "$ALLOW" || true)
  if [ -n "$hits" ]; then
    say "$label in $dir:"; echo "$hits" >&2
  fi
}

for crate in "${SIM_CRATES[@]}"; do
  dir="crates/$crate"
  [ -d "$dir" ] || continue

  # Floats: the types, suffixed literals, and `as` casts.
  grep_src '(^|[^A-Za-z0-9_])f(32|64)([^A-Za-z0-9_]|$)|[0-9_]f(32|64)([^A-Za-z0-9_]|$)' "$dir" "float type or literal"
  # Hashed collections and hashers: iteration order is unspecified; output differs across runs.
  grep_src 'HashMap|HashSet|DefaultHasher|RandomState|hash_map|hash_set|BuildHasher' "$dir" "hashed collection or hasher"
  # Wall clock, threads, network.
  grep_src '(std|core)::time|std::thread|std::net|SystemTime|Instant::now' "$dir" "time, thread, or network use"
  # File I/O anywhere but the pack loader.
  if [ "$crate" != "omnis-data" ]; then
    grep_src 'std::fs|std::io|std::process|std::env|File::open|File::create' "$dir" "file or process I/O"
  fi
  # Float arithmetic must also be denied by clippy inside the crate itself.
  if ! grep -q 'deny(clippy::float_arithmetic)' "$dir/src/lib.rs" 2>/dev/null; then
    say "$dir/src/lib.rs must carry #![deny(clippy::float_arithmetic)]"
  fi
done

for crate in "${NO_STD_CRATES[@]}"; do
  dir="crates/$crate"
  [ -d "$dir" ] || continue
  if ! grep -q '^#!\[no_std\]' "$dir/src/lib.rs" 2>/dev/null; then
    say "$dir/src/lib.rs must declare #![no_std]"
  fi
done

# No Bevy edge below omnis-app: check the resolved dependency graph, not just source text.
if command -v cargo >/dev/null; then
  for crate in "${SIM_CRATES[@]}"; do
    [ -d "crates/$crate" ] || continue
    edges=$(cargo tree -p "$crate" -e normal,build --prefix none --locked 2>/dev/null | grep -E '^bevy' || true)
    [ -n "$edges" ] && say "$crate depends on Bevy:" && echo "$edges" >&2
  done
  # Rhai must be built with no floats, one integer type, and never unchecked.
  if [ -d crates/omnis-expr ]; then
    feats=$(cargo tree -p omnis-expr -e features -i rhai --prefix none --locked 2>/dev/null | grep -E '^rhai feature' || true)
    echo "$feats" | grep -q 'feature "no_float"' || say "omnis-expr: rhai must enable no_float"
    echo "$feats" | grep -q 'feature "only_i64"' || say "omnis-expr: rhai must enable only_i64"
    echo "$feats" | grep -q 'feature "unchecked"' && say "omnis-expr: rhai must not enable unchecked"
  fi
fi

if [ "${1:-}" = "--self-test" ]; then
  # Plant one violation of each textual kind in a scratch file inside omnis-core and require a
  # failure for each. The scratch file is removed on exit no matter what.
  scratch="crates/omnis-core/src/__lint_self_test.rs"
  trap 'rm -f "$scratch"' EXIT
  cat > "$scratch" <<'PLANT'
fn a() -> f64 { 1.0f64 }
fn b() { let _ = 1.5f32 as f64; }
fn c() { let _ = HashMap::<u8, u8>::new(); }
fn d() { let _ = SystemTime::now(); }
fn e() { let _ = File::open("x"); }
PLANT
  out=$(bash "$0" 2>&1 >/dev/null || true)
  rm -f "$scratch"; trap - EXIT
  for want in "float type or literal" "hashed collection or hasher" "time, thread, or network use" "file or process I/O"; do
    echo "$out" | grep -q "$want" || { echo "lint-sim self-test: did not catch '$want'" >&2; fail=1; }
  done
  [ "$fail" = 0 ] && echo "lint-sim self-test: all planted violations caught"
  exit "$fail"
fi

if [ "$fail" = 0 ]; then echo "lint-sim: clean"; fi
exit "$fail"
