# Continuity notes

Written 2026-10-16, after M8.5 step 2b was built (all five items and the outcome notice), ahead of the owner's
2b walk and 2c. Rewrite this file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes). Untracked files under `assets/` are the owner's
  (`Resdie_graph_map.jpg`, `Resdie_map_cleaned_up.png`, `extrude-stairs-down.svg.kra`, `window.kra`); never
  stage them.
- **M8.5 build order:** 1a, 4, 4b, 1b, 3, 1c (all accepted), **2 (2a and 2b built; 2c next)**, 7 (saves by
  name, planned), editor (M5), 5, 6.
- **Step 2 plan:** `tasks/plans/m8.5-objects.md` (design, items, "Decided by me", mutation targets).
- **2b is built**, one commit per item, the owner hand-testing each as it landed ("Manually tested" for every
  item): shapes `22b2ce8`, mirror `a2e7677`, verbs `1ab07ea`, the outcome notice `fb2a757` (owner's request,
  replaced the bottom message line), inventory `e5dcf1e`, lines/docs/acceptance `f1c37d0`.
- **Open acceptance walks:** `tasks/acceptance/m8.5-2a.md` (never walked; the owner moved on) and
  `tasks/acceptance/m8.5-2b.md` (written at `f1c37d0`, offered to the owner). Both stay open in TODO.
- **Gate:** `tests passed 830 failed 0 ignored 12`, VERIFY-GREEN at `f1c37d0`. Sentrux rules pass, quality 8988.

## The owner's way of working on 2b (keep for 2c)
- Each item ends with something visible or clickable in the window, and the report says how to try it
  (numbered steps: where to stand, what to press, what appears). The owner tests by hand and replies
  "Manually tested", then asks for the next item.
- Launch for hand tests: `just run --windowed --pack packs/base --pack packs/test --pack
  crates/omnis-sim/tests/packs/objects`, New game, devtools on, a party with a rogue and a wizard; teleport
  with Esc, Debug, the World page, to `objects:map:yard`.
- Offscreen captures: `scratchpad/yardshot.sh name x y facing walk [--screen title|new-game|create-party|pause|
  actions|verbs|inventory]` (copies the fixture pack with the yard as entry). A capture cannot run a command
  first, so an opened chest or a notice needs the owner's eye.

## Next: 2c, visible beings (plan "2c — visible beings")
1. **Data:** `FixedEncounter.stand: Option<{pos, facing}>`; `Monster.look`, defaulting by size; `Shape::Figure`;
   checks and bad-pack rows.
2. **Engine:** without `stand`, encounters behave as today; `ViewportModel.beings`; entering the cell starts
   it as today; Use on a being in the faced cell starts it with the retreat at the party's own cell; a cleared
   `once` group is no longer shown.
3. **Client:** figures drawn; reshape on encounter and combat events; captures.
4. **Docs and acceptance** (`tasks/acceptance/m8.5-2c.md`).
Plan 2c in plan mode first if the owner asks; the plan's 2c section is short, so expect owner questions
(what a figure looks like, Use on a being vs Interact routing with `ChooseVerb`, whether the fixture gets a
standing pack rat).

## Client facts from 2b
- **Screens:** `ScreenId::Verbs` (`screen/verbs.rs`), over Actions; Actions offer "Use..." (`Does::Choose`)
  when a trial of `Interact` answers `ChooseVerb` or `VerbRefused`, and "Search here". The inventory has
  `ItemAct::{Drop, PickUp, TakeOut(k), PutIn(k)}`, `k` an open container's place in `Views.objects.objects`;
  the pile and open containers stand under the stores. `labels::refused` words any refusal short.
- **Notice:** `crate::notice` (Bevy-free fade: whole within 0.5 cells, gone at 1.5) and `shell/notice.rs`;
  `Session.notice`, set in `Session::ordered` from a chosen command's non-move lines (at most 6) or its
  refusal (alert role); shown only while exploring with no screen up.
- **Lines:** `text::objects` (checks, failed use, states, removals, finds, spent items, searches, notices);
  `Names::object` names a placement on the map `Names::locate` sets in `Session::take_in`.
- **Views:** `Views.objects` (`SimWorld::objects`, `api::objects_view`); `SimWorld::objects_on` for drawing;
  `SimWorld::object_blocks` for the mirror.

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
