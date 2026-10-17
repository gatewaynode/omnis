# Continuity notes

Written 2026-10-17, after M8.5 step 2b was built and refined by the owner's hand tests, ahead of a compact and
before 2c. Rewrite this file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, HEAD `b9d250d`, not pushed (the owner pushes). Untracked and never staged: the four
  files under `assets/` and `docs/dnd_supplemental/`.
- **M8.5 build order:** 1a, 4, 4b, 1b, 3, 1c (all accepted), **2 (2a and 2b built; 2c next)**, 7 (saves by
  name, planned), editor (M5), 5, 6.
- **Step 2 plan:** `tasks/plans/m8.5-objects.md` (design, items, "Decided by me", mutation targets). TODO under
  M8.5 step 2.
- **2b commits**, each gate VERIFY-GREEN, each hand-tested by the owner as it landed:
  shapes `22b2ce8`, mirror `a2e7677`, verbs `1ab07ea`, outcome notice `fb2a757`, inventory `e5dcf1e`,
  lines/docs/acceptance `f1c37d0`, search answers `a85de19`, lore 1–3 `0962023` `fe01e5a` `6e201e4`
  (data, engine, client), the log apart from the notice `b9d250d`.
- **Open acceptance walks:** `tasks/acceptance/m8.5-2a.md` (never walked) and `tasks/acceptance/m8.5-2b.md`
  (Parts 1–5, includes searching and lore). Both stay open in TODO.
- **Gate:** `tests passed 842 failed 0 ignored 12`, VERIFY-GREEN at `b9d250d`. Sentrux rules pass, quality 8993.
- **The owner was hand-testing visuals** when this was written (2026-10-17: found the log under the notice; it is
  fixed). Expect more visual notes before 2c; ask "anything else from the hand test?" if the next message is
  only "continue".

## The owner's way of working on 2b (keep for 2c)
- Each item ends with something visible or clickable, and the report says how to try it (numbered: where to
  stand, what to press, what appears). The owner replies "Manually tested" and asks for the next.
- Hand-test launch: `just run --windowed --pack packs/base --pack packs/test --pack
  crates/omnis-sim/tests/packs/objects`, New game, devtools on, a party with a rogue and a wizard; teleport with
  Esc, Debug, the World page, to `objects:map:yard`.
- Offscreen captures: `scratchpad/yardshot.sh name x y facing walk [--screen title|new-game|create-party|pause|
  actions|verbs|inventory]` (copies the fixture pack with the yard as entry; `cargo build -p omnis-vector`
  first). A capture cannot run a command first: an opened chest, a notice or a search needs the owner's eye.
- Bug reports from the owner: fix directly, with a red test, a lesson in `tasks/LESSONS.md`, then report. Design
  choices with real alternatives: AskUserQuestion with a recommended option first; larger additions get plan
  mode (lore did).

## Next: 2c, visible beings (plan "2c — visible beings")
1. **Data:** `FixedEncounter.stand: Option<{pos, facing}>`; `Monster.look`, defaulting by size; `Shape::Figure`;
   checks and bad-pack rows.
2. **Engine:** without `stand`, encounters behave as today; `ViewportModel.beings`; entering the cell starts
   it as today; Use on a being in the faced cell starts it with the retreat at the party's own cell; a cleared
   `once` group is no longer shown.
3. **Client:** figures drawn; reshape on encounter and combat events; captures.
4. **Docs and acceptance** (`tasks/acceptance/m8.5-2c.md`).
Plan 2c in plan mode first; expect owner questions (a figure's look, Use on a being among `ChooseVerb`'s
choices, a standing pack rat in the yard fixture, whether "Within reach" names a being).

## Client and engine facts from 2b
- **Screens:** `ScreenId::Verbs` (`screen/verbs.rs`) over Actions; Actions offer "Use..." (`Does::Choose`) when a
  trial of `Interact` answers `ChooseVerb` or `VerbRefused`, and "Search here". Inventory: `ItemAct::{Drop,
  PickUp, TakeOut(k), PutIn(k)}`, `k` an open container's place in `Views.objects.objects`; the pile and open
  containers under the stores. `labels::refused` words any refusal short. `SimWorld::faced_door`.
- **Notice:** `crate::notice` (Bevy-free fade: whole within 0.5 cells, gone at 1.5) and `shell/notice.rs`
  (`NOTICE_WIDTH` 60 %); `Session.notice` set in `Session::ordered` from a chosen command's non-move lines (at
  most `NOTICE_LINES` 6) or its refusal (alert role); shown only while exploring with no screen up.
- **HUD:** `shell/hud.rs`: a `Hud` column (status text, then `LogColumn` of `LOG_LINES` 8 `LogLine` texts, at
  most `log_width()` 18 vw, dim role, `log_alpha` fading by age); systems `arrange`, `status`, `log`.
- **Lines:** `text::objects` (checks, failed use, states, removals, finds, spent items, searches, notices, lore);
  `text::searched` after a search ("Within reach: …", "On the ground: …", "Nothing hidden here" only with no
  `Noticed` and no `Lore`); `Names::object` / `Names::locate` in `Session::take_in`.
- **Lore** (`omnis-data/src/lore.rs`): `Lore { text, dc, once }` on `ObjectKind` and `Placement` (placement
  wins, replacing the kind's), `CellLore` in `MapDef.lore` (id, level, x, y), `MapData.lore` by cell;
  `Event::Lore { key, object }` from `objects/notice.rs` `tell`, after the finds; once-state
  `MapState.lore_told`, `ObjectState.told`. Named `lore` because `story` is M11's.
- **Yard fixture** (`objects:map:yard`, 7×5, hedge south of row 2 with a gap at x 3, loft over (5,0)–(6,0),
  ladder (6,1)): crate (1,1) with its kind's stencil lore; statue (3,0) "The knight", kind lore moss, placement
  lore DC 12 once; hidden lever (0,0) DC 12 with rust lore; stone (0,2); plaque edge (4,1)/(4,2); gate
  (3,2)/(3,3); fountain (1,3); chest (5,3) with a dagger; hidden rune (2,4) DC 25; torch (6,4) behind a door;
  loft crate (5,0) level 1; arrows (2,2); cell lore bootprints (2,2), carving (4,3) DC 10 once.

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
- MCP: command `oneOf` 13, item `oneOf` 10, schema proof 115/165, tools 26, ops 26. `Command` 15, `Event` 72 (`Lore`),
  `Rejection` 70. Base pack: 19 spells (Detect Magic appended to the cleric and wizard lists).

## Open, carried over
- `assets/README.md` font rows: the owner's file; report only, never edit.
- `docs/dnd_supplemental/dnd_5e_combat_flow.md` appeared empty on 2026-10-17: the owner's, in progress; not
  committed. Ask before committing it once it has content (the commit-owner-docs rule names `docs/background`).
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
- **Commits:** stage new files by name; build the path list from `git status --short` without `assets/`,
  `.claude/` or `docs/dnd_supplemental/` (the owner's, in progress), write it to a scratchpad file, `git commit --pathspec-from-file=<file> -F -` (zsh does
  not split `$VAR`). Messages end with the Co-Authored-By and Claude-Session lines. Never push.
- **MCP from this session:** the omnis MCP tools are not attached here; drive `target/debug/omnis-mcp` with the
  scratchpad scripts (`mcp_drive.py '[["tool",{args}]]'` for the window, `mcp_accept.py` headless). Never pipe a
  live run through `head`. A headless `save_write` lands in the project root: delete it.
- **Context:** pause and recommend a compact past 350K.
