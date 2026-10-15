# Continuity notes

Written 2026-10-15, after M8.5 step 2a was built and before 2b starts, ahead of a compact. Rewrite this file
every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes). Untracked files under `assets/` are the owner's
  (four now: `Resdie_graph_map.jpg`, `Resdie_map_cleaned_up.png`, `extrude-stairs-down.svg.kra`, `window.kra`);
  never stage them.
- **M8.5 build order:** 1a, 4, 4b, 1b, 3, 1c (all accepted), **2 (in progress: 2a built, 2b next)**, 7 (saves by
  name, planned), editor (M5), 5, 6.
- **Step 2 plan:** `tasks/plans/m8.5-objects.md` (owner answers 2026-10-14; design, items, "Decided by me",
  mutation targets). TODO items under M8.5 step 2.
- **2a is built** (items 0–16, each its own commit, every gate VERIFY-GREEN): the last are views `19aacfb`, docs and
  acceptance `35fa6e7`. The owner moved straight to 2b (2026-10-15) without walking
  `tasks/acceptance/m8.5-2a.md`; it stays open in TODO ("awaiting the owner's walk") and can be walked any time.
  The owner already watched drop, pick-up and the chest round trip run live in the window through MCP.
- **Gate:** `tests passed 791 failed 0 ignored 12`, VERIFY-GREEN at `35fa6e7`. Sentrux rules pass, quality 8973.

## Next: 2b, the client (`omnis-vector`), so the owner can test by hand
The owner's aim (2026-10-15): "start putting the things in the game that I can test manually". Each 2b item
should end with something visible or clickable in the window, and the report should say how to try it.
Plan items (`tasks/plans/m8.5-objects.md`, "2b — client"):
1. **Shapes:** new `shapes.rs` (segments and fills from the integer `Look`, turned by facing, set on
   `ground::height`); `SegKind::Object`, provisional colour; `Scene` draws `ViewportModel.objects` and `.piles`;
   reshape on object, ground and `CombatEnded` events. Tests: a shape's base equals the engine's footprint
   (`omnis_data::footprint`); hidden and removed objects not drawn; fills present. Captures to
   `.omnis/m85/s2b-*`.
2. **Mirror:** `collide::blocks` checks footprints (`api::object_blocks`); `tests/agreement.rs` on the yard,
   also after the gate is pushed open.
3. **Verbs screen:** `ScreenId::Verbs` (`screen/verbs.rs`) from Use in Actions (button first, key as shortcut):
   one button per `VerbView` from `objects.get`, dimmed with its refusal; a `ChooseVerb` refusal opens it;
   "Search here" in Actions.
4. **Inventory:** Drop; an "On the ground" list with Take (PickUp); an open container's contents with Take out
   and Put in.
5. **Text, docs, acceptance:** HUD lines for the object events, `Noticed`, `Searched` (names now available
   through `ObjectView.name` / kind names); presentation-ARCHITECTURE §5.4, §6, §10.1, new §8.7; presentation-PRD
   §4.2, §4.3, §6.2, §12.5; `tasks/acceptance/m8.5-2b.md`.
- Before item 1, read presentation-ARCHITECTURE §8 (the 3D view, segments, fills, `Scene`) and the client part
  of `tasks/knowledge/code-map.md`. Known plan notes for 2b: the mirror ignores an object's state and a shape's
  base not turned by facing were listed as mutation targets; the client does not push the pose out of a
  footprint in its own cell (visual only, later).
- Manual testing setup: `just run --windowed --pack packs/base --pack packs/test --pack
  crates/omnis-sim/tests/packs/objects`, New game, devtools on; the debug panel (Esc, Debug, "World" page)
  teleports to `objects:map:yard`.

## Engine facts 2b builds on (2a)
- **Views:** `objects.get` → `Reply::Objects { objects: [ObjectView], ground: [ItemCount] }`; `ObjectView
  { object, object_kind, name, state, capacity, contents, verbs: [VerbView { verb, option, label, checks:
  [CheckView { kind, dc }], minutes, refusal }] }`. `ViewportModel.objects: [SeenObject { object, object_kind,
  x, y, pos, facing, state, look }]` and `.piles: [PileView { x, y, pos, items }]`, both skip-if-empty. Tier 1:
  `omnis_sim::objects_view`, `query::viewport`.
- **Refusals** come from `objects::admit`, shared by `Act` and the view (agreement proven across the yard).
- **Commands:** `Act { object, verb, option }`, `Door`, `Search`, `Interact` (→ `ChooseVerb { choices }` when
  several), `ItemCommand::{Drop, PickUp, TakeOut, PutIn}` (exploring only, no time; pick-up own cell; a new pile
  lies 2 eighths ahead).
- **Events:** `ObjectUsed`, `ObjectState`, `ObjectRemoved`, `ObjectMessage`, `GoldFound`, `ItemFound`,
  `ItemSpent`, `Noticed { object, how }`, `Searched { member, roll }`, `ItemMoved` with `ItemPlace::{Ground,
  Object}`, `Death.loot`, `CombatEnded.loot`. The client already has HUD lines for the four item moves and the
  loot lines; object events and Noticed/Searched have none yet.
- **Yard fixture** (`objects:map:yard`, 7×5, hedge along the south of row 2 with a gap at x 3, loft on level 1
  over (5, 0)–(6, 0), ladder at (6, 1)): crate (1, 1); statue (3, 0), named "The knight"; hidden lever (0, 0),
  DC 12; stone (0, 2); plaque on the edge of (4, 1)/(4, 2); gate across (3, 2)/(3, 3), opened by the lever's
  flag; fountain (1, 3); chest (5, 3) with a dagger; hidden rune (2, 4) with an Abjuration aura, DC 25; torch
  (6, 4) behind a door on its north edge; loft crate (5, 0) level 1; arrows pile (2, 2).
- **Finding to remember:** sight passes through the corner of a gap in a wall (from (2, 1) facing South the party
  sees (2, 3), (4, 3), (5, 3), (5, 4) behind the hedge). Recorded in `horizons.md`; not changed.

## Step 7, planned (saves by name)
`tasks/plans/m8.5-saves-by-name.md` (`31576c3`, owner answers `4d80c52`): after step 2 (2b, 2c), before the
editor. A name table in the save and a remap on load; `save.check` and a `LoadReport`; DB-style numbered
migrations per pack, offered, nothing dropped without one; no refunds; version and hash in every `pack.ron`
(`omnis-cli pack seal`); no saving in an encounter or a fight (its first item); `pack.reload` by name (it
renumbers silently today). Saves still keep interned numbers until then: a new base-pack id renumbers later ids
(item 14 renumbered the v5/v6 fixtures' spells).

## Pins
- Replays: walk `8d880647a25097b2`, fight `d06fa80460a947ef` (rebaselined at item 14 for the pack hash and the
  wizard's Detect Magic, proven). `SAVE_SCHEMA` 7, `PROTOCOL` 2.
- MCP: command `oneOf` 13, item `oneOf` 10, schema proof 115/165, tools 26, ops 26. `Command` 15, `Event` 71,
  `Rejection` 70. Base pack: 19 spells (Detect Magic appended to the cleric and wizard lists).

## Open, carried over
- `assets/README.md` font rows: the owner's file; report only, never edit.
- P6 colours unconfirmed; step 5 revisits colour roles (joins, fills, crest, objects).
- The automap still records the forward cone (§12.5); no objects on it (owner, 2026-10-14).
- Dependencies: the rhai Socket re-audit (ask when); `smartstring` RUSTSEC-2026-0249; `ttf-parser`
  RUSTSEC-2026-0192; Bevy 0.20 not before 2026-11-07.

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first and again after any
  appended test. Workspace clippy with `--all-targets --features omnis-vector/test-fixtures -- -D warnings`
  before every commit. Sim tests: `cargo test -p omnis-sim --test integration <filter>`; MCP the same with
  `-p omnis-mcp`. A `cargo test -p a -p b` run stops at the first failing package, so read every result.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<item>.txt 2>&1`, VERIFY-GREEN, no edits while it runs. Then
  Sentrux `rescan` and `check_rules` (load with ToolSearch `select:mcp__plugin_sentrux_sentrux__rescan,
  mcp__plugin_sentrux_sentrux__check_rules`).
- **Tests red first; mutations** with a Python runner in the scratchpad (anchor checked unique, source restored
  in `finally`, `python3 -u` to a file), then `git status` to confirm the sources are clean.
- **Each new command, event or rejection** also moves: `docs/api.md` and its count lines, `api_doc.rs` pins,
  the MCP schema and proof, `common/commands.rs`, `schema.rs` unit test, client `shell/text.rs` and `rolllog.rs`.
  A new op also moves `replies.rs` (`EVERY`), `api_doc.rs` (`ops!`, `every_op`, "N ops."), `tools.rs` (tool,
  example, count), `bridge.rs` counts, `omnis-cli/src/schema.rs` and `tests/headless.rs`, and the vocabulary's
  `VIEWS`.
- **Commits:** stage new files by name; build the path list from `git status --short | grep -v "^??"` without
  `assets/` or `.claude/`, write it to a scratchpad file, `git commit --pathspec-from-file=<file> -F -` (zsh does
  not split `$VAR`). Messages end with the Co-Authored-By and Claude-Session lines. Never push.
- **MCP from this session:** the omnis MCP tools are not attached here; drive `target/debug/omnis-mcp` with the
  scratchpad scripts (`mcp_drive.py '[["tool",{args}]]'` for the window, `mcp_accept.py` headless). Never pipe a
  live run through `head`. A headless `save_write` lands in the project root: delete it.
- **Context:** pause and recommend a compact past 350K.
