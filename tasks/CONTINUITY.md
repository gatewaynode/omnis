# Continuity notes

Written 2026-10-10, mid P2d, before a terminal restart (the owner has added iTerm2 under Developer
Tools). M8 is closed (`8921c07`). Branch `m7a-b-tasks`. Rewrite this file every time it is used. Durable
knowledge lives in `tasks/knowledge/`.

## State of the tree
- `4ecb1d6` P2d WIP (renames, vocabulary test, docs), then `8eed1cc` "Tests: one integration binary
  per crate", then `3a13481` the vocabulary self-check fix. **The WIP has now passed the full gate**:
  `tests passed 583 failed 0 ignored 13`, VERIFY-GREEN, 127 s (binaries reused from a run before).
- The self-check (`the_vocabulary_catches_a_name_that_drifted`) was failing on the WIP commit: the
  last note's "vocabulary passed" was wrong. It read a numeric `row` and the `Row` predicate's string
  `row` into one vocabulary (3 findings, expected 2); now the planted breaks are asserted by message
  and the predicate has its own vocabulary that must report nothing.

## Test layout (new, `8eed1cc`; detail in `tasks/knowledge/verification.md` "Test binaries")
- Each crate with several test files builds one target, `integration` (`autotests = false`,
  `tests/main.rs`, `use crate::common;`). Own binaries: app `socket`, cli `headless` (they change the
  working directory), sim `measure`. 72 → 23 test binaries.
- One file's tests: `cargo test -p omnis-mcp --test integration vocabulary::`.
- A new test file goes into its crate's `tests/main.rs`; `scripts/check-test-modules.sh` (in the gate
  and CI) fails on a file no target runs.

## The first-launch wait (owner's log, 2026-10-10)
- Every newly linked binary waits ~20 s on its first launch; the second launch is 0.00 s. The log
  (`target/omnis-launch.log`, ignored): the kernel's AMFI refuses the ad hoc linker signature; ~10 s
  pass before `syspolicyd`'s `GK performScan`; its network check takes 71 ms ("Code did not match any
  currently allowed policy"); XProtect then never answers and Gatekeeper cancels it after 9.8 s.
- Developer Tools with iTerm2 "on" did not lift it before; the owner now thinks iTerm2 is properly in
  the list and is restarting the terminal.
- **After the restart, check first** (from the owner's iTerm2, and once from this session):
  `touch crates/omnis-bus/src/lib.rs && DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test
  -p omnis-bus --lib --no-run -q && /usr/bin/time -p "$(ls -t target/debug/deps/omnis_bus-* | grep
  -v '\.d$' | head -1)"`. Near 0 s: fixed for that launcher. Still ~20 s: the merge is the remedy;
  a session under Claude Code is not covered by the iTerm2 entry either way (unverified).
- In zsh, `log` is a builtin: use `/usr/bin/log`. It refuses to run in this session ("Cannot run
  while sandboxed", even with the sandbox off), and files at the top of `~` are not readable here:
  the owner captures and copies into `target/`.

## Left for P2d
1. Recapture `docs/api.md` §3's transcripts (still protocol 1: `"protocol":1`, `"map":3`, `"index"` in
   dice, `{"Party":0}`) from a `Headless` world on base+test, seed 1, with the same requests
   (game.status, party.create Wren, Turn Left + Step Forward, Encounter Attack rejected, combat.get
   NoEncounter, and an attack's `AttackResolved`/`Damage`). A small throwaway test or `omnis-mcp
   --headless` over stdio.
2. BUGS.md B6: party_view printed `?` for an unnamed id where everything else printed `#n`; fixed with
   `names::id_of`; red-first test `api_views.rs::a_number_no_pack_names_reads_the_same_in_the_party_view_as_everywhere`.
3. Knowledge: `verification.md` pins (test count 583, `PROTOCOL` 2 at line ~60, the vocabulary test,
   its mutation 3 of 3), `code-map.md` (`names.rs`, `word.rs`, `omnis-mcp/tests/vocabulary.rs`,
   `tests/common/`, each crate's `tests/main.rs`).
4. TODO: P2d and the parent protocol-2 item checked with detail; plan `tasks/plans/protocol-2.md`
   marked closed (the 12 renames beyond the plan, the as-built rule changes, the self-check fix).
   Also the TODO item "one test binary per crate" if one exists: done in `8eed1cc`.
5. `cargo fmt --all`, full gate, replays unchanged (walk `8711507745385976768`, fight
   `5247080599556612730`), Sentrux (`git add` new files, scan `/Users/john/code/omnis/crates`,
   `check_rules`), then one commit staged by name that completes P2d.
6. Raise with the owner: the Socket re-audit of `rhai` 1.26.1 (due 2026-10-10).

## Pins
- Gate `tests passed 583 failed 0 ignored 13` (580 before P2d + 2 vocabulary + 1 api_views).
- Walk replay `8711507745385976768`, fight replay `5247080599556612730` (not rerun since the merge;
  check in step 5).
- `SAVE_SCHEMA` 7, `PROTOCOL` 2, MCP 24 tools, schema proof 94/142, dev branches 13, 62 rejections.
- Base pack tuple `(4, 4, 3, 24, 16, 18, 3, 36, 7)`. Sentrux quality 9038 (before P2d), rules pass.

## Process
- Every cargo and gate run: `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first.
- Gate: `scripts/verify.sh > <scratchpad>/gate.txt 2>&1` in the background; no edits while it runs.
- Run `cargo test -p omnis-mcp --test integration vocabulary::` (seconds) after any wire change.
- Mutation: apply a break, run the named tests, restore in `finally`; check the sources are clean after.
- zsh: split a file list with `${=files}` when staging by name; avoid backticks inside double quotes.
- A file with hunks for two commits: write one hunk to a patch and `git apply --cached` it.

## Other open TODO items (unscheduled, owner's call)
- `DevCommand::Pass { minutes }`; the `data.*` ops (labels and definitions); `scripts/mcp-probe.py`.
- Editor v1 is held "after M8, before M9" (owner, 2026-09-20).

## Carry-over
- Not built: nothing answers `SpellCast`/`EnemyCasts`; `EnemyFlees`/`OwnTurn` have no source; region
  catch-up waits for M10.
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` 902, `save_and_replay.rs` 810,
  `tactics_panel.rs` 795.
- The command instances (`tests/common/commands.rs`) have no `Predicate::Row`, so the vocabulary
  never sees it on real data (the self-check covers it).
