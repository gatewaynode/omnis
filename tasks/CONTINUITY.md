# Continuity notes

Written 2026-10-15, mid M8.5 step 2a, before a compact. Rewrite this file every time it is used. Durable
knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes). Untracked files under `assets/` are the owner's; never stage.
- **M8.5 build order:** 1a, 4, 4b, 1b, 3, 1c (all accepted), **2 (in progress)**, editor (M5), 5, 6.
- **Step 2 plan:** `tasks/plans/m8.5-objects.md` (owner answers 2026-10-14; the design, the item list, the
  mutation targets). TODO items under M8.5 step 2 (2a, 2b, 2c).
- **2a done (one commit each, every gate VERIFY-GREEN, replays unmoved):**
  - plan saved `d302e24`; B7 retreat keeps its level `7f6fe15`; `MapData` into `map_data.rs` `6c4722f`;
  - object kinds `93b2829` (`omnis-data/src/object.rs`, `object/check.rs`; fixture pack
    `crates/omnis-sim/tests/packs/objects`, ten kinds);
  - `footprint.rs` `9e86c9b` (sixteenths of a cell; `of`, `path`, `overlaps`, `cells`, `ray_span`);
  - placements and piles `1e740f4` (`placement.rs`; the yard map `objects:map:yard`; bad pack
    `crates/omnis-data/tests/packs-bad/bad-objects`, loaded over base and test);
  - loot tables `467be18` (`loot.rs`, `Monster.loot`; fixture `objects:monster:pack_rat`, `objects:loot:hoard`);
  - step blocking `468e134` (`omnis-sim/src/objects/mod.rs`: `ObjectState` in `MapState.objects`,
    skip-if-empty; `blocks_step` in `apply::landing` after `Cliff`; `BlockReason::Object`;
    `api::object_blocks`);
  - sight `9f0fd7b` (`visibility::Sheet::object_hides`; `line_of_sight`/`ray` take `visibility::Kinds`).
- **Gate:** `tests passed 791 failed 0 ignored 12`, VERIFY-GREEN at `35fa6e7`. Sentrux rules pass, quality 8973.

## Next: the owner walks `tasks/acceptance/m8.5-2a.md`, then 2b
- 2a is built: items 0-16, the last `35fa6e7` (docs, acceptance). Views `19aacfb`; Detect Magic `88faba2`;
  hidden objects `53d765a`; loot `c02fae0`; ground and containers `61f0904`; Use routing `01385b9`.
- The walk runs through MCP in the window (launch with `--pack crates/omnis-sim/tests/packs/objects`, devtools
  on, add a wizard with `party_create`). This session has no omnis MCP tools attached; drive the bridge with
  the scratchpad scripts (`mcp_drive.py` for single calls, `mcp_accept.py` for the headless walk) or after a
  restart. A headless save writes into the project root: delete it afterwards.
- 2b (client): shapes, the mirror, the verbs screen, inventory (Drop, the ground, containers), HUD lines for
  object events and `Noticed`/`Searched`, docs and acceptance. Plan items in `tasks/plans/m8.5-objects.md`.
- **Step 7 planned** (`tasks/plans/m8.5-saves-by-name.md`, `31576c3`, owner answers `4d80c52`): after 2b/2c,
  before the editor. Owner: DB-style numbered migrations per pack, nothing dropped without one, no refunds,
  version and hash in every `pack.ron`, no saving in an encounter or fight.
- **Saves keep definitions as interned numbers** (sorted ids per kind): any new base-pack id shifts later
  numbers. The v5/v6 save fixtures were renumbered in `88faba2` (spells from 4 up, by one). Step 7 ends it
  (a name table in the save, option A, chosen by the owner).
- Owner wants to test drop and pick-up by hand: through MCP now, or the 2a acceptance (item 16); in the window
  after 2b items 1 (piles drawn) and 4 (inventory buttons). Item 15's `objects.get` shows piles.
- Each new command, event or rejection also moves: `docs/api.md` (+ count pins in `api_doc.rs`, including the
  `Command` (N variants) line), MCP schema and proof numbers, `common/commands.rs` chain, `schema.rs` unit
  test, `rolllog.rs`, client `shell/text.rs`.
- Commit path lists: build them from `git status --short | grep -v "^??"`; the owner adds files under
  `assets/` at any time (a `.jpg` appeared during item 13).
- Mutation runners: a list of (name, file, anchor, break), each anchor checked unique, `cargo test -p
  omnis-sim --test integration <filter>`, source restored in `finally`, run with `python3 -u` to a file.

## Facts
- **Yard fixture** (`objects:map:yard`, 7×5, hedge along the south of row 2 with a gap at (3, 2), loft on level 1
  over (5, 0)–(6, 0), ladder at (6, 1)): crate (1, 1), statue (3, 0), hidden lever (0, 0), stone (0, 2), plaque
  on the edge of (4, 1)/(4, 2), gate across (3, 2)/(3, 3), fountain (1, 3), chest (5, 3) against its north side
  with a dagger, hidden rune with an aura (2, 4), torch (6, 4) behind a door on its north edge, loft crate (5, 0) level 1, arrows pile (2, 2).
- **Tests:** `crates/omnis-sim/tests/objects.rs` (helpers `objects()`, `in_yard`, `step_result`, `sees`);
  `crates/omnis-data/tests/footprint.rs`; bad packs in `crates/omnis-data/tests/bad_packs.rs`.
- **Pins:** replays walk `8d880647a25097b2`, fight `d06fa80460a947ef` (rebaselined in `88faba2`); `SAVE_SCHEMA` 7, `PROTOCOL` 2;
  MCP command schema `oneOf` 13; schema proof 115 instances, 165 offered; item `oneOf` 10; MCP tools 26, ops 26; `Event` 71,
  `Rejection` 70, `Command` 15.
- **Departures so far:** a spill across a cliff is refused too (`1e740f4`); `Act.option`, `ItemFound`,
  `ItemSpent` (`ef0b77c`); `Command::Door`, `ChooseVerb.choices` (`01385b9`); pick-up own cell only, ground and container moves free, `PileFull` (`61f0904`); loot on the `combat` stream, may exceed 32 kinds (`c02fae0`); Search an enum value (oneOf stays 13), reveal needs no sight (`53d765a`); Detect Magic on both class lists, a new wizard knows it (`88faba2`). Fixture: the fountain's wish and `data/rules/objects.ron` (`objects.crowd`).

## Open, carried over
- `assets/README.md` font rows: the owner's file; report only, never edit.
- P6 colours unconfirmed; step 5 revisits colour roles (joins, fills, crest, objects).
- The automap still records the forward cone (§12.5); no objects on it (owner, 2026-10-14).
- Dependencies: the rhai Socket re-audit (ask when); `smartstring` RUSTSEC-2026-0249; `ttf-parser`
  RUSTSEC-2026-0192; Bevy 0.20 not before 2026-11-07.

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, `cargo fmt --all` first and again after any
  appended test (the gate fails on format). Workspace clippy with `--all-targets --features
  omnis-vector/test-fixtures -- -D warnings` before every commit. Test targets are `--test integration`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<item>.txt 2>&1`, no edits while it runs.
- **Mutations:** a Python runner in the scratchpad (anchor checked, source restored in `finally`), run with
  `python3 -u`.
- **Commits:** stage new files by name, then `git commit -F - -- <paths>` (zsh does not split `$VAR` into
  paths; `--pathspec-from-file=<scratchpad file>` works). Never `.claude/` or the owner's assets. Unfinished work before a compact is committed as `WIP`.
