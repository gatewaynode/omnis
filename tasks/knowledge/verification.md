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
carries `bevy_ui` and Feathers like every other build; only `devtools` is a feature. It runs on a push to `main` and on a pull request only: a
push to a work branch with no PR open shows the Socket scans alone, which is not a green build.

Test count at the gate: 670 passed, 13 ignored (2026-10-10, after the vector adoption's V3; 583 of
them before `omnis-vector` joined), in 268 s with Bevy's `bevy_pbr` unified across the workspace (222 s
before). The gate's log says it on one line: `tests passed 670 failed 0 ignored 13`.

The vector client: `cargo test -p omnis-vector` (seconds once built); `tests/agreement.rs` holds its
collision mirror to the simulation on every map. An offscreen capture:
`cargo run -p omnis-vector -- --screenshot <relative path>.png --size 1600x900 [--walk N]` (paths must be
relative, without `..`).

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
`cargo test -p omnis-mcp --test integration vocabulary::`. Kept as their own binaries: `omnis-app`
`socket` and `omnis-cli` `headless` (each changes the working directory, which is per process) and
`omnis-sim` `measure`. A new test file is added to its crate's `tests/main.rs`;
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
`debug_menu::adjust` 93, `service::settle` 81 (a new deal arm goes in a helper), `debug_screen::row_text` 91, `combat_text::wound_line`
89, `dump_screens` about 92, `tests/inventory.rs` first test 94; `screen.rs` 935 lines (the
screen-dump test is the piece to move out next); `tactics_panel.rs` 771, `command.rs` 761, `combat_text.rs` 708, `combat_menu.rs` 707 (M7c); `game_tools` near the cap (`screenshot_tool` was moved out of it; new MCP tools go in
`party_tools`); `main.rs::parse_args` 100 (a new flag goes into `Look::take` or a helper); `loader::load_one` 92; `character::create` 84;
`omnis-sim/src/ops.rs` 606 lines since M7c step 6a moved the party view to `party_view.rs` (a new view goes in its own file);
`tests/service.rs::the_panels_lie_inside_the_map_at_both_window_sizes` 89; `ui.rs` 593 lines (after 8a moved the tools out).
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
- MCP: 24 tools (asserted in `omnis-mcp/src/tools.rs` and twice in `tests/bridge.rs`; the op list in
  `omnis-cli`'s schema dump, 24, in `omnis-cli/tests/headless.rs`); the hand-written
  `Command` schema has `oneOf` 11, combat arms 6 (M7c: `Feature`; `EndTurn` in the string enum), `item_schema` 6, `service_schema` 12,
  `rest_schema` 2, `dev_schema` 13 (M8: `Reconcile`).
- The schema proof (`omnis-mcp/tests/schema_proof.rs`): 94 instances from the `next` chain of
  exhaustive matches (M8: `Reconcile`), 142 offered `oneOf` branches and `enum` values, all used; the drift test's
  padded branch is `/oneOf/11`. A new `Command`
  variant needs a successor arm there and a schema branch; both numbers move with it.
- The headless driver is a devtools world, so its fingerprints differ from a default replay of the
  same commands by design.

## Tools and commands
- `bevy_ui` screens (every build): the headless harness is `tests/common/mod.rs::feathers_app`
  (`DefaultPlugins` without winit and logging, no render backend: real layout, picking, focus, no
  GPU); helpers in `tests/common/feathers.rs` (`layout_faults` over the one `PanelRoot`,
  `bar_faults` over the tool bar, `controls` (the panels' only), `text_tree`, `click_node`,
  `drag_node`, `keys`, `tab`, `resize`, `ultrawide`; ids are `impl Into<UiId>`, and the layout check
  is structural: menu items under a `MenuPopup` are left out, what sits under a `ScrollArea` counts
  as far as its clip shows it). An app under `MinimalPlugins` has no panel:
  its tests get their party from `tests/common/mod.rs::party_by_command` (`CreationAsk`, what the
  panel itself sends), and press the tool bar with `common::tool` (a `ToolPressed`, gated as the
  bar's own) and read it with `tool_live` (the `ToolStates` resource); real pointer clicks on the
  bar are `tests/tool_bar.rs`'s, as is the bar's fit: every word on one line inside its button
  at both window sizes, at the fitted scale and the cap, in all three fonts (2026-10-02: the
  widest, "Spells", 50 of 70 px at 1x and the cap, 100 of 141 at 2x and the cap). A PPM dump cannot show them; the text tree
  (`creation_feathers.txt` under `OMNIS_DUMP_SCREENS`) stands in. A picture needs the running
  game: `--script "party,create" --settle 60 --screenshot-composed <file.png>`, or the
  `screenshot` op with `target: "window"`. An agent-launched window is on no screen: plain window
  captures are black, frame times mean nothing, and `--window medium` is clamped to 2560×1378
  (canvas 1×), so such a capture never shows the ultrawide's 2× layout.
- Screen dumps as PNGs: `OMNIS_DUMP_SCREENS=<dir> cargo test -p omnis-app --lib dump_screens -- --ignored`
  (entries include `explore`, `pause`, `inventory`, `combat_use`, the sheet pages). Since M7 step
  8a the tool bar is `bevy_ui`, so the dumps show its strip empty; `tool_bar.txt` (the bar's text
  tree) is written by `tests/tool_bar.rs` under `OMNIS_DUMP_SCREENS`.
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
- Game flags: `--pack`, `--seed`, `--save` (default `.omnis/quick.ron`), `--autostart`, `--window
  small|medium|large|huge`; with devtools `--script`, `--screenshot`, `--settle`, `--dev-socket`,
  `--no-dev-socket`, `--screenshot-canvas`, `--screenshot-composed`; in every build `--font 0|1|2`,
  `--ui-scale <hundredths>` (held to what the window holds: 1.25 per canvas pixel,
  `creation_panel::scale_cap`), `--frame-stats` (vertical sync off).
- CLI: `validate`, `schema dump`, `map text`, `play --script`, `replay`, `tileset bake`.
