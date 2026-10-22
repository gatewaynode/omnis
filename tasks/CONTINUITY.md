# Continuity notes

Written 2026-10-21, at the close of the session that planned W1 and built M5.0 to M5.3. Rewrite this file every
time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `W1-authoring-tools`**, eight commits ahead of `main`, not pushed (the owner pushes): `5c65ee0` M5.0,
  `4704230` M5.1, `ef3c12b` M5.2, `a303fa8` M5.3 (accepted by the owner 2026-10-22), `9ac9468` B8 (the camera's
  pitch clamped a degree short of vertical, the owner's choice), `faa797b` continuity, `c8bb62e` **M5.4** (2026-10-22,
  VERIFY-GREEN, 1067 tests), then this continuity commit. Tree clean. Untracked and never staged: `assets/` and
  `docs/dnd_supplemental/` (the owner's).
- **The plan:** `tasks/plans/m5-editor-v1.md`, ten steps ticked in `tasks/TODO.md` under M5; M5.0 to M5.4 done.
- **M5.4 as built** (`tasks/acceptance/m5-4.md`, not yet reported on by the owner): `crates/omnis-vector/src/editor/`
  (Bevy-free `Editor` over `edit::Session`: `maps`, `chosen`, `level`, `cursor`, `facing`, `view: plan::View`,
  `pane`, `resolved` (last valid `MapData::resolve`), `revision`; `open`, `changed`, `point`, `step`, `show_level`,
  `laid_out`, `apply(EditAct, step) -> Asked`; `plan.rs` `View { origin, zoom }`, `ZOOMS` 4..32, `fitted`,
  `centred_on`, `zoomed`, `hit`, `cell_rect`, `paint(Source, view, pane)`, `fill`; `view.rs` `tools`/`inspector`
  kit screens with ids `EditorTools`/`EditorInspector`), `shell/editor.rs` (`Editing(Editor)` resource, `EditorPlugin`
  in `VectorSet::Screens`: `enter`/`leave` on the resource, `route`, `refresh`; `Column::{Tools, Plan, Inspector}`,
  `PlanImage`, `PlanCursor`; headless-safe, the image only with `Assets<Image>`), `screen/editor_menu.rs`,
  `Nav.editing` + `edit()`, `ScreenId::{EditorMenu, EditorTools, EditorInspector}`, `ControlId::{Editor, Edit(EditAct)}`,
  `Effect::OpenEditor`, `Mark::Editor` + `EditorPress`, `Config.editor` (`--editor [map-id]`, refused with a game),
  `grab_cursor` never locks over the editor, the socket answers `Op::editor()` on `Editing` (`NO_EDITOR` otherwise)
  and calls `changed` so the layer redraws. `raster.rs` now holds `EdgeKey`, `edge_key`, `tick_pixels`, `line_pixels`,
  `join_pixels`, `edge_pixels` (the minimap uses them). Tests: `tests/editor.rs` (9) + plan unit tests (4).
- **Lessons today** (`tasks/LESSONS.md` 2026-10-22): a control chosen against the convention is accepted by stumbling
  into it; the mutation-restore rule broken again by an abbreviated note (restore with a *fresh* mtime, never
  `os.utime` back).

## Next
1. **The owner's acceptance of M5.4**: `tasks/acceptance/m5-4.md` (13 numbered steps, the levels fixture and the
   test pack, three over MCP). Bugs: fix with a red test and a lesson first.
2. **M5.5, brushes and painting** (plan §Client side "Input", steps table row 5): `Brush` rows in the tools column
   with the current one marked (digits 1-9 pick), left click paints at the cursor's target with the brush, a drag is
   one stroke (`Document::paint(.., stroke: true)` while the button is held; `Editor` gets `brush`, `stroke_open`),
   right click picks (never destructive: the brush becomes what is under the pointer), edges: `plan::hit` grows a
   `Target { Cell | Edge(side) }` by the pixel's distance to the cell's border (≤ scale/4), Undo/Redo buttons and
   Cmd/Ctrl+Z/Y, the dirty `*` in the title (already shown), Save in the editor's menu dim with "N errors" and the
   first listed, Revert; after a Save `Packs.data` is reloaded from disk (`Packs::load(&config)` and the `Editing`'s
   `changed`). Everything M5.4 built is read in `tasks/knowledge/code-map.md` (the editor paragraph) so nothing is
   re-read; `edit::Document::paint/undo/redo` and `Session::save` are in `crates/omnis-data/src/edit/`.
3. Then M5.6 to M5.10 per the plan's steps table.

## The owner's way of working (keep)
- Each item ends with something visible or clickable, and the report says how to try it (numbered: where to
  stand, what to press, what appears). The owner replies "Manually tested" and asks for the next.
- Hand-test launch: `just run --windowed --pack packs/base --pack packs/test --pack
  crates/omnis-sim/tests/packs/objects`, New game, devtools on; teleport with Esc, Debug, the World page, to
  `objects:map:yard`.
- Offscreen captures: `scratchpad/yardshot.sh name x y facing walk [--screen title|new-game|create-party|pause|
  actions|verbs|inventory]` (`cargo build -p omnis-vector` first). A capture cannot run a command first.
- Bug reports: fix directly, with a red test, a lesson in `tasks/LESSONS.md`, then report. Choices with real
  alternatives: AskUserQuestion with a recommended option first; larger additions get plan mode. A study's
  recommended answers become the owner's choices when the owner says "settle" (2026-10-21).

## Pins
- **Replays:** walk `8eab9a1dd95b472d`, fight `79ca2704c92bc312` (rebaselined in 7h2). `SAVE_SCHEMA` 9,
  `PROTOCOL` 2, **ops 44, MCP tools 44, `Reply` 27 (`EVERY`), `OpError` 11** (since M5.3), `MAX_MIGRATIONS` 1024,
  `MAX_STEPS` 4096 (pinned to the docs by `omnis-mcp/tests/api_doc.rs`, run as `--test integration api_doc`;
  the tool counts also in `tools.rs`, `bridge.rs` twice, `omnis-cli/tests/headless.rs`, README, code-map).
- **Counts:** `Command` 16, `DevCommand` 14, `Event` 73, `Rejection` 71, `LoadError` 12.
- **Fixtures:** `crates/omnis-sim/tests/packs/{ahead,chain/v1,chain/v3,heights,levels,objects}`; a test that edits a
  pack works on a scratch copy (`omnis-data/tests/common::copy_pack`, `omnis-cli/tests/headless.rs::scratch_pack`)
  and reseals it; the bad-pack corpus `broken` carries `notes/bad.ron` (not hashed: no reseal was needed).
- **Limits the editor uses:** `MAX_STRING_BYTES` 4 KiB (a note, a key), `MAX_FILE_BYTES` 4 MiB (`bounded_file`, a
  check or write text), `MAX_COLLECTION` 65,536, `MAX_MAP_SIDE` 256, `MAX_LEVELS` 16, `UNDO_CAP` 100.

## Open, carried over
- `assets/README.md` font rows: the owner's file; report only, never edit.
- `docs/dnd_supplemental/`: the owner's, in progress; ask before committing anything there.
- P6 colours unconfirmed; step 5 revisits colour roles (joins, fills, crest, objects, beings).
- The automap still records the forward cone (§12.5); no objects or beings on it (owner, 2026-10-14).
- Dependencies: the rhai Socket re-audit (ask when); `smartstring` RUSTSEC-2026-0249; `ttf-parser`
  RUSTSEC-2026-0192 (built by the advisory Linux CI job; informational); Bevy 0.20 not before 2026-11-07.
- The first CI run with the advisory Linux job passed (owner, 2026-10-21).

## Process
- **Cargo and git:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools` for both (git refuses without it:
  the Xcode licence). `cargo fmt --all` first and again after any appended test. Workspace clippy with
  `--all-targets --features omnis-vector/test-fixtures -- -D warnings` before every commit. Sim tests:
  `cargo test -p omnis-sim --test integration <filter>`; data `-p omnis-data --test integration <filter>`; client
  `-p omnis-vector --features test-fixtures --test integration <filter>`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<item>.txt 2>&1`, VERIFY-GREEN, no source or pinned-document
  edits while it runs. In the background: one chain, `verify.sh > log; grep -q VERIFY-GREEN log && git add <paths>
  && git commit -F -` (lesson 2026-10-21); the runner's exit code is the wrapper's. Then Sentrux `rescan` (first
  `scan` on `/Users/john/code/omnis/crates` in a new session) and `check_rules`.
- **Tests red first; mutations** with a Python runner in the scratchpad (anchor checked unique, source restored
  in `finally` by rewriting its text with a *fresh* mtime, never `os.utime` back to the old one and never by
  moving a backup back: lesson 2026-10-18, broken 2026-10-22), then `git status` to confirm
  the sources are clean. No cargo command and no fixture edit while a mutation run or the gate runs.
- **Each new op** moves: `ops.rs` (`Op`, `is_host`, the `dispatch` HostOnly arm), `api_doc.rs` (`ops!`, `every_op`,
  the lists and literal counts), `replies.rs` (`tag`, `EVERY`, an ask), `tools.rs` (tool + example + count),
  `bridge.rs` (two counts), `omnis-cli/src/schema.rs` (op dump) and `tests/headless.rs` (count), `docs/api.md`
  (§5 table and "N ops.", §6, §10, changelog), README (count and names), `code-map.md` (count), ARCHITECTURE §9.3.
  Each new command, event or rejection also moves ARCHITECTURE §4.2's counts, `code-map.md`'s command list,
  `variants!` lists, the MCP `schema.rs` `oneOf` count test and `schema_proof.rs`, `common/commands.rs`, `word.rs`,
  client `labels::refused`.
- **Edits** by an anchored Python script in the scratchpad (every anchor asserted unique before writing); after
  `cargo fmt` the anchors move, so patch the file as it is (lesson 2026-09-12).
- **Commits:** stage the changed files by name (or `git add -u` plus each new file); never `git add -A` on `docs/`,
  `assets/` or the root; read `git status --short` before `git commit`. Messages end with the Co-Authored-By line
  and the Claude-Session line the system gives. Never push. Owner world docs (`docs/background`, `docs/locations`)
  go in their own commit, unedited. Ask before changing `PRD.md` or `ARCHITECTURE.md`; an owner's direction counts
  as the ask, flagged in the commit.
- **Context:** pause and recommend a compact past 350K.
