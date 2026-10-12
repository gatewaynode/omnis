# Verification

## The gate
`scripts/verify.sh` runs `cargo fmt --check`, clippy on every target and on the app's release
configuration (`--no-default-features --lib`), every test with `--no-fail-fast` (a failed test
fails the script), `scripts/lint-sim.sh --self-test` and the lint, `scripts/check-duplicates.sh`,
`scripts/check-test-modules.sh --self-test` and the check (every test file is in a target),
and `omnis-cli validate packs/base packs/test`; it prints `VERIFY-GREEN` and exits zero only when
every step passed. Run it unpiped and read the status. CI (`.github/workflows/ci.yml`) runs the
same steps, the release clippy included since M7 step 1, plus both golden replays through the CLI
with `--pack packs/base --pack packs/test`. The shipped configuration (`--no-default-features`)
carries `bevy_ui` like every other build; only `devtools` is a feature. It runs on a push to `main` and on a pull request only: a
push to a work branch with no PR open shows the Socket scans alone, which is not a green build.

Test count at the gate: 697 passed, 12 ignored (2026-10-12, after M8.5 step 3; 678 after 1b; 715 and 13
ignored after parity P3, before `omnis-app` retired; 583 before `omnis-vector` joined). The gate's log says it
on one line: `tests passed 697 failed 0 ignored 12`. Write each gate run to a new file name: `verify.sh` stops at the
first failing step, so `cargo fmt --check && verify.sh > gate.txt` that fails at fmt leaves an older
`gate.txt` in place, and reading it reports an old count as this run's (P2, 2026-10-10).

The vector client: `cargo test -p omnis-vector` (seconds once built); `tests/agreement.rs` holds its
collision mirror to the simulation on every map. An offscreen capture:
`cargo run -p omnis-vector -- --screenshot <relative path>.png --size 1600x900 [--walk N]
[--screen title|new-game|pause|actions]` (paths must be relative, without `..`). The window starts at the
title; tests that save name their own file under the crate's `.omnis/test-saves/` (the test config's
quick save is `.omnis/test-saves/none.ron`, never the player's). The dev socket: `just run` writes `.omnis/dev.addr`; the loopback test is
`tests/socket.rs` (headless, no renderer, so `screenshot` there answers "no window"). A window launched
from an agent's shell gets no window-server connection ("Connection invalid … hiservices" in its log, in
the sandbox and out of it on 2026-10-10) and runs no frames, so a live `screenshot` needs
the owner's session.

The API reference `docs/api.md` is held to the code by `omnis-mcp/tests/api_doc.rs`: a new op, reply,
`OpError` kind, command, dev command, event or rejection fails the build until it is named there and in
the document.

Linker (2026-10-02): `cc` finds clang through `xcodebuild -find clang`, which reads
`/Library/Preferences/com.apple.dt.Xcode.plist`. The session's sandbox cannot read that file, so
inside it every fresh link fails ("xcodebuild -find clang ... exit code 17664", `xcodebuild
-checkFirstLaunchStatus` exits 69) even with the license accepted (it was, for Xcode 26.5, on
2026-10-02). Prefix cargo and the gate with `DEVELOPER_DIR=/Library/Developer/CommandLineTools`
(process-local) inside the sandbox. A gate with nothing new to link proves nothing about the
linker: test it with a fresh target directory or a scratch worktree.

Test binaries (2026-10-10): every crate with several test files builds them as one target,
`integration` (`autotests = false`, `tests/main.rs` declares the files as modules; a file reaches
the shared helpers with `use crate::common;`). Run one file's tests with
`cargo test -p omnis-mcp --test integration vocabulary::`. Kept as their own binaries: `omnis-cli`
`headless` (it changes the working directory, which is per process) and `omnis-sim` `measure`
(`omnis-app`'s `socket` went with the crate at P9). A new test file is added to its crate's `tests/main.rs`;
`scripts/check-test-modules.sh` fails the gate on a file no target runs. Why: macOS holds every newly linked executable about 20 s on its first launch (the
owner's log: the kernel refuses the linker's ad hoc signature, about 10 s pass before Gatekeeper's
scan, then XProtect never answers and Gatekeeper cancels it after 10 s), and the Developer Tools
exemption did not lift it for iTerm2 at first. 72 test binaries became 23; the test step went from about
38 min to 549 s on a full rebuild. Lifted 2026-10-10 after the owner restarted iTerm2 with it listed
under Developer Tools: a freshly relinked `omnis-bus` test binary and a never-seen scratch binary both
launch in 0.00 s the first time, from this session too. Measure a first launch with a binary whose
bytes are new (a touch can relink to identical bytes); if the wait returns, check that entry first.

## Sentrux
`rescan` then `check_rules` before every commit. A `scan` does not count untracked files (seen
2026-09-27: two new files showed only after the commit), so a commit that adds files gets a
`rescan` and `check_rules` again right after it. Rules: `max_fn_lines 100` including tests,
`max_cc 25`, `max_cycles 0`. Near the caps (leave them alone or split first):
`service::settle` 81 (a new deal arm goes in a helper); `command.rs` 761, vector's `combat_menu.rs` 719; `game_tools` near the cap (`screenshot_tool` was moved out of it; new MCP tools go in
`party_tools`); `loader::load_one` 92; `character::create` 84;
`omnis-sim/src/ops.rs` 606 lines since M7c step 6a moved the party view to `party_view.rs` (a new view goes in its own file);
(The `omnis-app` entries left with the crate at P9.)
Sentrux's rules are `crates/.sentrux/rules.toml`: scan `/Users/john/code/omnis/crates`, not the
repository root (the root has no rules file).

## Pinned numbers
- Base pack tuple in `omnis-data/tests/load_base_pack.rs`: `(races 4, classes 4, backgrounds 3,
  items 24, conditions 16, spells 18, monsters 3, rule slots 36, services 7)` (slots 31 → 34 in M7c step 2: `turn.*`; 34 → 36 in M8 step 1: `time.settled`, `time.wild`)`; `chain_mail`'s
  shape and the bad-pack error wording are pinned in the same directory.
- Golden replays `crates/omnis-sim/tests/replays/{walk,fight}.ron`: walk `8711507745385976768`
  under its own seed `WALK_SEED = 2` (the smallest that meets the random table on the way),
  fight `5247080599556612730` under the golden seed (M8 step 4: the base pack hash, the rumors' minutes; step 3: contacts, party time and the bus in the save, and `Reconciled` at each region entry, every other event unchanged; step 1: the pack hash; before that M7c acceptance c, B1: the fighter's `EndTurn` went; both leave town through the
  gate and walk up the road to the meadow's start first). Rebaseline
  with `cargo test -p omnis-sim rebaseline -- --ignored` in the same commit as any `packs/` or
  serialized-`World` change.
- `SAVE_SCHEMA 7`; `migrate.rs` holds `v2_to_v3`, the data-aware `v3_to_v4`, `v4_to_v5`
  (gold ×100, a saved fight's loot too), `v5_to_v6` (M7c: auto-cast spells become declared sets; a
  saved fight gets its budget and reactions) and `v6_to_v7` (M8: the past counted in full as shared
  time and date; the bus's default subscriptions, and `Battle → Reactions` for a fight saved mid-way); fixtures `tests/saves/v1..v6.ron` (`v6.ron` captured late, in M8 step 8c, from a worktree of the step 2 build `0bcb02f`), each captured by an
  ignored `capture_schema_N_fixture` before the schema moved on. Content ids are interned in
  file order, so a new map or spell file renumbers those after it: the fixtures (loaded with
  `force`) name the dungeon by their own id, `FIXTURE_DUNGEON` (M7b).
- The API (ARCHITECTURE §4.9): `ops::PROTOCOL` 2 (`omnis-sim/src/ops.rs`), reported by `game.status`; every `Reply` tagged `reply`,
  each variant round-tripped through JSON from a live headless world in `omnis-mcp/tests/replies.rs` (17).
  Protocol 2's names are held by `omnis-mcp/tests/vocabulary.rs`: `every_key_on_the_wire_has_one_meaning`
  reads every key from the schema proof's command instances, a town service and both golden replays (with
  every view after each command); `the_vocabulary_catches_a_name_that_drifted` plants breaks and asserts
  each finding by message. Seconds to run: run it after any wire change, before the gate.
- MCP: 25 tools (asserted in `omnis-mcp/src/tools.rs` and twice in `tests/bridge.rs`; the op list in
  `omnis-cli`'s schema dump, 25, in `omnis-cli/tests/headless.rs`: a hand list, so a new op is added to
  `omnis-cli/src/schema.rs` by hand; the gate does not catch a missing one); the hand-written
  `Command` schema has `oneOf` 12 (M8.5 1b: `Climb`), combat arms 6 (M7c: `Feature`; `EndTurn` in the string enum), `item_schema` 6, `service_schema` 12,
  `rest_schema` 2, `dev_schema` 13 (M8: `Reconcile`).
- The schema proof (`omnis-mcp/tests/schema_proof.rs`): 96 instances from the `next` chain of
  exhaustive matches (M8.5 1b: `Climb`), 145 offered `oneOf` branches and `enum` values, all used; the drift test's
  padded branch is `/oneOf/12`. A new `Command`
  variant needs a successor arm there and a schema branch; both numbers move with it.
- The headless driver is a devtools world, so its fingerprints differ from a default replay of the
  same commands by design.

## Tools and commands
- Client screens (`omnis-vector`): a screen is a Bevy-free description tested on the real packs
  (`tests/common/mod.rs::ctx`, `views`); the shell runs under `MinimalPlugins` (`tests/common/app.rs`:
  `shell`, `press_ui`, `tap`, `click`, `top`). A picture is the offscreen capture
  (`cargo run -p omnis-vector -- --screenshot <png> [--screen title|new-game|create-party|pause|actions]
  [--walk N] [--size WxH]`), or the `screenshot` op in the owner's session. The `omnis-app` harness
  (`feathers_app`, `OMNIS_DUMP_SCREENS`) went with the crate at P9.
- Encounter measurement over seeds: `cargo test -p omnis-sim --test measure -- --ignored --nocapture`
  (300 seeds, parties of 2 and 6, attack-only vs cast-every-turn; integers, tenths and hundredths).
  `ambush_over_seeds` measures resting in the dungeon: wipes of a spent party per ambush, per
  100 rests at a few chances, and the ambush rate the slots give over 3000 rests.
  `clear_over_seeds` (M7b) measures one clear of the dungeon and the depths by a new party of 2,
  4 or 6 that goes back to town to raise, rest, train and pick when spent: wipe %, trips,
  fights, XP a member, gold, the level reached, raises, levels unpaid, gold left. It never
  flees and does not model the walk; tuned for four (owner, 2026-10-03). The harness is
  `tests/measure/` since M7c step 5: `budget_over_seeds` (the clear played one command a turn, with
  the whole budget, and with Shield declared; 3,000 seeds) and `boss_over_seeds` (Bob's group alone
  against parties of 2, 4, 6 at levels 1–3). Run one with `--release` and its name as the filter.
- MCP: `.mcp.json` runs `target/debug/omnis-mcp` against the game's `.omnis/dev.addr`; restart the
  server after a new build. `omnis-mcp --headless --pack … --seed n` hosts its own world.
- Game flags (`omnis-vector/src/shell/session.rs::parse`): `--pack`, `--seed`, `--save` (default
  `.omnis/quick.ron`), `--settings` (default `.omnis/settings.ron`), `--screen`, `--windowed`,
  `--no-vsync`, `--frame-stats` (log the frame times, vsync off), `--monitor`, `--log`, `--screenshot`,
  `--walk`, `--size`; with devtools `--dev-socket`, `--no-dev-socket`.
- CLI: `validate`, `schema dump`, `map text`, `play --script`, `replay`, `tileset bake`.
