# Continuity notes

Written 2026-10-14, mid M8.5 step 2a, before a compact. Rewrite this file every time it is used. Durable
knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes). Untracked `assets/*.kra` are the owner's; never stage.
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
- **Gate:** `tests passed 736 failed 0 ignored 12`, VERIFY-GREEN at `9f0fd7b`. Sentrux rules pass, quality 8966.

## Next: 2a item 9, `Command::Act`
- New `omnis-sim/src/objects/act.rs` (eligibility, refusals) and `objects/outcome.rs` (requirements,
  effects). Read the plan file's "Save and protocol" and "Build" sections first.
- Checks roll for the best member who can act on a copy of an `object` RNG stream written back only on
  success; a failed check spends the minutes but not `once`; `Item{consume}` only on success; minutes
  from `Interaction.minutes`, else `INTERACT_MINUTES`.
- Wire names first: grep the wire types and run the vocabulary test (`cargo test -p omnis-mcp --test
  integration vocabulary::`) before naming (lesson 2026-10-10). Watch `kind` (use `object_kind`), `gold`,
  `state`, `loot`.
- Each new command, event or rejection also moves: `docs/api.md`, the MCP command schema `oneOf` 12 → 13 and
  the schema proof's numbers, `api_doc.rs`, and the client's exhaustive matches (`shell/text.rs`).
- Then items 10 to 16 as listed in the plan (Interact routing, ground and containers, loot on victory,
  hidden objects, Detect Magic, views and `objects.get`, docs and acceptance).

## Facts
- **Yard fixture** (`objects:map:yard`, 7×5, hedge along the south of row 2 with a gap at (3, 2), loft on level 1
  over (5, 0)–(6, 0), ladder at (6, 1)): crate (1, 1), statue (3, 0), hidden lever (0, 0), stone (0, 2), plaque
  on the edge of (4, 1)/(4, 2), gate across (3, 2)/(3, 3), fountain (1, 3), chest (5, 3) against its north side
  with a dagger, hidden rune with an aura (2, 4), torch (6, 4), loft crate (5, 0) level 1, arrows pile (2, 2).
- **Tests:** `crates/omnis-sim/tests/objects.rs` (helpers `objects()`, `in_yard`, `step_result`, `sees`);
  `crates/omnis-data/tests/footprint.rs`; bad packs in `crates/omnis-data/tests/bad_packs.rs`.
- **Pins:** replays unmoved (walk `b0220a6eefc5e6e3`, fight `f130927d1f09f19f`); `SAVE_SCHEMA` 7, `PROTOCOL` 2;
  MCP command schema `oneOf` 12; schema proof 96 instances, 145 offered; MCP tools 25.
- **Departure so far:** a spill across a cliff is refused too (commit `1e740f4`).

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
  paths). Never `.claude/` or the owner's assets. Unfinished work before a compact is committed as `WIP`.
