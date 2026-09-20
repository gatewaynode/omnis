# Verification

## The gate
`scripts/verify.sh` runs `cargo fmt --check`, clippy on every target and on the app's release
configuration (`--no-default-features --lib`), every test with `--no-fail-fast` (a failed test
fails the script), `scripts/lint-sim.sh --self-test` and the lint, `scripts/check-duplicates.sh`,
and `omnis-cli validate packs/base packs/test`; it prints `VERIFY-GREEN` and exits zero only when
every step passed. Run it unpiped and read the status. CI (`.github/workflows/ci.yml`) runs the
same steps without the release clippy, plus both golden replays through the CLI with
`--pack packs/base --pack packs/test`.

Test count at the gate: 349 passed, 6 ignored (2026-09-20).

## Sentrux
`rescan` then `check_rules` before every commit. Rules: `max_fn_lines 100` including tests,
`max_cc 25`, `max_cycles 0`. Near the caps (leave them alone or split first):
`plan::viewport` 100, `debug_menu::adjust` 93, `debug_screen::row_text` 91, `combat_text::wound_line`
89, `dump_screens` about 92, `tests/inventory.rs` first test 94; `screen.rs` 985 lines (the
screen-dump test is the piece to move out next); `game_tools` 99 (new MCP tools go in
`party_tools`); `loader::load_one` 92; `character::create` 84.

## Pinned numbers
- Base pack tuple in `omnis-data/tests/load_base_pack.rs`: `(races 4, classes 4, backgrounds 3,
  items 24, conditions 16, spells 11, monsters 3, rule slots 19)`; `chain_mail`'s shape and the
  bad-pack error wording are pinned in the same directory.
- Golden replays `crates/omnis-sim/tests/replays/{walk,fight}.ron`: walk `238033710167572364`,
  fight `7317777168019603128`. Rebaseline with `cargo test -p omnis-sim rebaseline -- --ignored`
  in the same commit as any `packs/` or serialized-`World` change; none since save schema 4.
- `SAVE_SCHEMA 4`; `migrate.rs` holds `v2_to_v3` and the data-aware `v3_to_v4`; fixture
  `tests/saves/v3.ron`.
- MCP: 18 tools (asserted in `omnis-mcp/src/tools.rs` and `tests/bridge.rs`); the hand-written
  `Command` schema has `oneOf` 9, combat arms 5, `item_schema` 6, `dev_schema` 12.
- The headless driver is a devtools world, so its fingerprints differ from a default replay of the
  same commands by design.

## Tools and commands
- Screen dumps as PNGs: `OMNIS_DUMP_SCREENS=<dir> cargo test -p omnis-app --lib dump_screens -- --ignored`
  (entries include `explore`, `pause`, `inventory`, `combat_use`, the sheet pages).
- Encounter measurement over seeds: `cargo test -p omnis-sim --test measure -- --ignored --nocapture`
  (300 seeds, parties of 2 and 6, attack-only vs cast-every-turn; integers, tenths and hundredths).
- MCP: `.mcp.json` runs `target/debug/omnis-mcp` against the game's `.omnis/dev.addr`; restart the
  server after a new build. `omnis-mcp --headless --pack … --seed n` hosts its own world.
- Game flags: `--pack`, `--seed`, `--save` (default `.omnis/quick.ron`), `--autostart`, `--window
  small|medium|large|huge`; with devtools `--script`, `--screenshot`, `--settle`, `--dev-socket`,
  `--no-dev-socket`.
- CLI: `validate`, `schema dump`, `map text`, `play --script`, `replay`, `tileset bake`.
