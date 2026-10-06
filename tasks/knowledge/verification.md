# Verification

## The gate
`scripts/verify.sh` runs `cargo fmt --check`, clippy on every target and on the app's release
configuration (`--no-default-features --lib`), every test with `--no-fail-fast` (a failed test
fails the script), `scripts/lint-sim.sh --self-test` and the lint, `scripts/check-duplicates.sh`,
and `omnis-cli validate packs/base packs/test`; it prints `VERIFY-GREEN` and exits zero only when
every step passed. Run it unpiped and read the status. CI (`.github/workflows/ci.yml`) runs the
same steps, the release clippy included since M7 step 1, plus both golden replays through the CLI
with `--pack packs/base --pack packs/test`. The shipped configuration (`--no-default-features`)
carries `bevy_ui` and Feathers like every other build; only `devtools` is a feature. It runs on a push to `main` and on a pull request only: a
push to a work branch with no PR open shows the Socket scans alone, which is not a green build.

Test count at the gate: 531 passed, 12 ignored (2026-10-04, after M7c step 8a). The gate's log says
it on one line: `tests passed 531 failed 0 ignored 12`.

Linker (2026-10-02): `cc` finds clang through `xcodebuild -find clang`, which reads
`/Library/Preferences/com.apple.dt.Xcode.plist`. The session's sandbox cannot read that file, so
inside it every fresh link fails ("xcodebuild -find clang ... exit code 17664", `xcodebuild
-checkFirstLaunchStatus` exits 69) even with the license accepted (it was, for Xcode 26.5, on
2026-10-02). Prefix cargo and the gate with `DEVELOPER_DIR=/Library/Developer/CommandLineTools`
(process-local) inside the sandbox. A gate with nothing new to link proves nothing about the
linker: test it with a fresh target directory or a scratch worktree.

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
- Golden replays `crates/omnis-sim/tests/replays/{walk,fight}.ron`: walk `9748142822655127576`
  under its own seed `WALK_SEED = 2` (the smallest that meets the random table on the way),
  fight `17071167585389510742` under the golden seed (M8 step 1: the pack hash moved with the region and time files; before that M7c acceptance c, B1: the fighter's `EndTurn` went; both leave town through the
  gate and walk up the road to the meadow's start first). Rebaseline
  with `cargo test -p omnis-sim rebaseline -- --ignored` in the same commit as any `packs/` or
  serialized-`World` change.
- `SAVE_SCHEMA 6`; `migrate.rs` holds `v2_to_v3`, the data-aware `v3_to_v4`, `v4_to_v5`
  (gold ×100, a saved fight's loot too) and `v5_to_v6` (M7c: auto-cast spells become declared sets; a
  saved fight gets its budget and reactions); fixtures `tests/saves/v1..v5.ron`, each captured by an
  ignored `capture_schema_N_fixture` before the schema moved on. Content ids are interned in
  file order, so a new map or spell file renumbers those after it: the fixtures (loaded with
  `force`) name the dungeon by their own id, `FIXTURE_DUNGEON` (M7b).
- MCP: 20 tools (asserted in `omnis-mcp/src/tools.rs` and `tests/bridge.rs`; the op list in
  `omnis-cli`'s schema dump, 20, in `omnis-cli/tests/headless.rs`); the hand-written
  `Command` schema has `oneOf` 11, combat arms 6 (M7c: `Feature`; `EndTurn` in the string enum), `item_schema` 6, `service_schema` 12,
  `rest_schema` 2, `dev_schema` 12.
- The schema proof (`omnis-mcp/tests/schema_proof.rs`): 93 instances from the `next` chain of
  exhaustive matches (M7c), 141 offered `oneOf` branches and `enum` values, all used; the drift test's
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
