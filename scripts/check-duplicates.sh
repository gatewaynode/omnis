#!/usr/bin/env bash
# Dependency policy (CLAUDE.md, ARCHITECTURE.md §12): keep the dependency tree single-copy.
# Bevy's own transitive tree already carries some duplicate crates; those are recorded in
# scripts/duplicates.allow. This check fails when a new duplicate appears, i.e. when one of our
# direct dependencies resolves to a different version than Bevy's.
#   scripts/check-duplicates.sh            check against the allow list
#   scripts/check-duplicates.sh --update   rewrite the allow list from the current lock file
set -euo pipefail
cd "$(dirname "$0")/.."
allow=scripts/duplicates.allow
actual=$(cargo tree --workspace --duplicates --depth 0 --prefix none --locked --target all -e normal,build 2>/dev/null \
  | awk 'NF {print $1}' | sort -u)
if [ "${1:-}" = "--update" ]; then
  printf '%s\n' "$actual" > "$allow"
  echo "check-duplicates: wrote $(wc -l < "$allow" | tr -d ' ') entries to $allow"
  exit 0
fi
touch "$allow"
new=$(comm -13 <(sort -u "$allow") <(printf '%s\n' "$actual"))
gone=$(comm -23 <(sort -u "$allow") <(printf '%s\n' "$actual"))
if [ -n "$gone" ]; then
  echo "check-duplicates: no longer duplicated, remove from $allow:" >&2; echo "$gone" >&2
fi
if [ -n "$new" ]; then
  echo "check-duplicates: new duplicate crates (align the version with Bevy's or record an owner exception):" >&2
  for c in $new; do cargo tree --workspace --duplicates --prefix none --locked --target all -e normal,build -i "$c" 2>/dev/null | head -20 >&2; done
  exit 1
fi
echo "check-duplicates: no new duplicates"
