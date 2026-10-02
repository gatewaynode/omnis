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

Test count at the gate: 444 passed, 8 ignored (2026-10-02, after M7 step 7). The gate's log says
it on one line: `tests passed 444 failed 0 ignored 8`.

Linker (2026-10-02): an Xcode update whose license is not yet accepted makes `cc` fail with
"xcodebuild -find clang ... exit code 17664". Until the owner runs `sudo xcodebuild -license`,
prefix cargo and the gate with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` (process-local).

## Sentrux
`rescan` then `check_rules` before every commit. A `scan` does not count untracked files (seen
2026-09-27: two new files showed only after the commit), so a commit that adds files gets a
`rescan` and `check_rules` again right after it. Rules: `max_fn_lines 100` including tests,
`max_cc 25`, `max_cycles 0`. Near the caps (leave them alone or split first):
`debug_menu::adjust` 93, `service::settle` 81 (a new deal arm goes in a helper), `debug_screen::row_text` 91, `combat_text::wound_line`
89, `dump_screens` about 92, `tests/inventory.rs` first test 94; `screen.rs` 985 lines (the
screen-dump test is the piece to move out next); `game_tools` near the cap (`screenshot_tool` was moved out of it; new MCP tools go in
`party_tools`); `main.rs::parse_args` 100 (a new flag goes into `Look::take` or a helper); `loader::load_one` 92; `character::create` 84;
`omnis-sim/src/ops.rs` 817 lines (a new view goes in its own file, as `service_view.rs`);
`tests/service.rs::the_panels_lie_inside_the_map_at_both_window_sizes` 89; `ui.rs` 702 lines.
Sentrux's rules are `crates/.sentrux/rules.toml`: scan `/Users/john/code/omnis/crates`, not the
repository root (the root has no rules file).

## Pinned numbers
- Base pack tuple in `omnis-data/tests/load_base_pack.rs`: `(races 4, classes 4, backgrounds 3,
  items 24, conditions 16, spells 11, monsters 3, rule slots 31, services 7)`; `chain_mail`'s
  shape and the bad-pack error wording are pinned in the same directory.
- Golden replays `crates/omnis-sim/tests/replays/{walk,fight}.ron`: walk `9901411989274517557`
  under its own seed `WALK_SEED = 2` (the smallest that meets the random table on the way),
  fight `15728260309841309156` under the golden seed (M7 step 5; both leave town through the
  gate and walk up the road to the meadow's start first). Rebaseline
  with `cargo test -p omnis-sim rebaseline -- --ignored` in the same commit as any `packs/` or
  serialized-`World` change.
- `SAVE_SCHEMA 5`; `migrate.rs` holds `v2_to_v3`, the data-aware `v3_to_v4` and `v4_to_v5`
  (gold ×100, a saved fight's loot too); fixtures `tests/saves/v1..v4.ron`, each captured by an
  ignored `capture_schema_N_fixture` before the schema moved on.
- MCP: 20 tools (asserted in `omnis-mcp/src/tools.rs` and `tests/bridge.rs`; the op list in
  `omnis-cli`'s schema dump, 20, in `omnis-cli/tests/headless.rs`); the hand-written
  `Command` schema has `oneOf` 11, combat arms 5, `item_schema` 6, `service_schema` 9,
  `rest_schema` 2, `dev_schema` 12.
- The schema proof (`omnis-mcp/tests/schema_proof.rs`): 77 instances from the `next` chain of
  exhaustive matches, 111 offered `oneOf` branches and `enum` values, all used; the drift test's
  padded branch is `/oneOf/11`. A new `Command`
  variant needs a successor arm there and a schema branch; both numbers move with it.
- The headless driver is a devtools world, so its fingerprints differ from a default replay of the
  same commands by design.

## Tools and commands
- `bevy_ui` screens (every build): the headless harness is `tests/common/mod.rs::feathers_app`
  (`DefaultPlugins` without winit and logging, no render backend: real layout, picking, focus, no
  GPU); helpers in `tests/common/feathers.rs` (`layout_faults`, `text_tree`, `click_node`,
  `drag_node`, `keys`, `tab`, `resize`, `ultrawide`; ids are `impl Into<UiId>`, and the layout check
  is structural: menu items under a `MenuPopup` are left out, what sits under a `ScrollArea` counts
  as far as its clip shows it). An app under `MinimalPlugins` has no panel:
  its tests get their party from `tests/common/mod.rs::party_by_command` (`CreationAsk`, what the
  panel itself sends). A PPM dump cannot show them; the text tree
  (`creation_feathers.txt` under `OMNIS_DUMP_SCREENS`) stands in. A picture needs the running
  game: `--script "party,create" --settle 60 --screenshot-composed <file.png>`, or the
  `screenshot` op with `target: "window"`. An agent-launched window is on no screen: plain window
  captures are black, frame times mean nothing, and `--window medium` is clamped to 2560×1378
  (canvas 1×), so such a capture never shows the ultrawide's 2× layout.
- Screen dumps as PNGs: `OMNIS_DUMP_SCREENS=<dir> cargo test -p omnis-app --lib dump_screens -- --ignored`
  (entries include `explore`, `pause`, `inventory`, `combat_use`, the sheet pages).
- Encounter measurement over seeds: `cargo test -p omnis-sim --test measure -- --ignored --nocapture`
  (300 seeds, parties of 2 and 6, attack-only vs cast-every-turn; integers, tenths and hundredths).
  `ambush_over_seeds` measures resting in the dungeon: wipes of a spent party per ambush, per
  100 rests at a few chances, and the ambush rate the slots give over 3000 rests.
- MCP: `.mcp.json` runs `target/debug/omnis-mcp` against the game's `.omnis/dev.addr`; restart the
  server after a new build. `omnis-mcp --headless --pack … --seed n` hosts its own world.
- Game flags: `--pack`, `--seed`, `--save` (default `.omnis/quick.ron`), `--autostart`, `--window
  small|medium|large|huge`; with devtools `--script`, `--screenshot`, `--settle`, `--dev-socket`,
  `--no-dev-socket`, `--screenshot-canvas`, `--screenshot-composed`; in every build `--font 0|1|2`,
  `--ui-scale <hundredths>` (held to what the window holds: 1.25 per canvas pixel,
  `creation_panel::scale_cap`), `--frame-stats` (vertical sync off).
- CLI: `validate`, `schema dump`, `map text`, `play --script`, `replay`, `tileset bake`.
