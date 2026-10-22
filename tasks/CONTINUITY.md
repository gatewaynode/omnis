# Continuity notes

Written 2026-10-21, at the close of the session that planned W1 and built M5.0 to M5.3. Rewrite this file every
time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `W1-authoring-tools`**, six commits ahead of `main`, not pushed (the owner pushes): `5c65ee0` M5.0 (the
  plan), `4704230` M5.1, `ef3c12b` M5.2, `a303fa8` M5.3, `fef5fe1` continuity, `9ac9468` B8 (2026-10-22: the
  camera's pitch clamped a degree short of vertical, the owner's choice after the free head of M8.5 4b left the
  view upside down during M5.3 acceptance; `tasks/BUGS.md` B8; VERIFY-GREEN, 1054 tests). Tree clean. Untracked and never staged: the files under
  `assets/` and `docs/dnd_supplemental/` (the owner's).
- **The plan:** `tasks/plans/m5-editor-v1.md` (approved 2026-10-21), ten steps ticked in `tasks/TODO.md` under M5.
  Owner's choices in it: a save reseals and bumps the patch version; the four maps' comments move into notes;
  Playtest uses the fixed party. Owner's direction during M5.1: notes are a keyed store `notes/*.ron` (never
  sealed, dangling keys allowed, the editor marks orphans), not a field; PRD §10, ARCHITECTURE §6.1 and §8.2 say so.
- **Built:** `MapData::resolve` (the loader's per-map pass shared, `map_data.rs`); `omnis_data::edit` (`Grid`,
  `Document`, `Session`, `Notes`, `check_text`, `Request`/`Answer`/`answer`); 17 `editor.*` ops with MCP tools on
  `Headless`. Acceptance scripts: `tasks/acceptance/m5-1.md`, `m5-2.md`, `m5-3.md`.
- **Gate:** VERIFY-GREEN at `a303fa8`; the data crate alone has 83 integration tests (30 the editor's).
- Two lessons added today (`tasks/LESSONS.md`, 2026-10-21): an author's text is sized to how authors work and
  keyed to what it is about; a background gate's verdict is read by `grep -q VERIFY-GREEN` in the commit chain.

## Next (tomorrow)
1. **The owner's acceptance of M5.3** (begun 2026-10-22; B8 came out of it, the editor ops themselves not yet
   reported on): `tasks/acceptance/m5-3.md`, an agent against `omnis-mcp --headless packs/base
   /tmp/omnis-edit` on a copy of `packs/test` (the shipped pack stays sealed). The owner reports; a bug is fixed
   with a red test and a lesson before anything else.
2. **M5.4, the client's editor layer** (plan §Client side; the design's C1 and C2). Everything it touches was read
   on 2026-10-21; the facts, so nothing is re-read:
   - `screen/nav.rs`: `Nav { stack, creating }`; `base(None)` is `Some(Title)`; add `editing: bool` with `edit()`, make
     `base(None)` `None` while editing, arms in `allowed` (`EditorMenu` with no game; `Settings` stays), `paused`
     (`EditorMenu`), `escape` (push `EditorMenu` when editing and nothing is open). `ScreenId` has 19 variants;
     add `EditorMenu` (+ `EditorTools`, `EditorInspector` as ids for the kit `Screen`s the layer builds).
   - `screen/mod.rs`: `Effect` enum (`None, Command, Host, Back, Open, ToTitle, ToNewGame, Quit, Settings`); add
     `OpenEditor`, `ToEditor`. `Ctx { data, game, save, prefs, fits }` built in `shell/screen.rs::ctx/ctx_with`
     and `tests/common`. `content(id, ctx, forms)`/`apply` are exhaustive matches.
   - `screen/title.rs`: three buttons in one row + Quit; add `ControlId::Editor` "Editor".
   - `shell/screen.rs`: `spawn` (the `ScreenLayer`/`ScreenRoot`), `open_start` (`StartScreen`), `route` (Esc via
     `Nav::escape`, backquote Debug), `carry_out` (`Effect` arms; `ToTitle` removes `Session`, `screens.reset(None)`),
     `chrome` (hides HUD when not `played`), `playing` (needs a `Session`).
   - The layer pattern: `shell/combat.rs` (`FightScreen` resource, `FightRoot`, `enter`/`leave` on the resource,
     `route`, `refresh` rebuilding only when the `Screen` changed, `screen_draw::build(parent, &screen, (height,
     Mark::Fight), keep)`, `FightPress(ControlId)`). `screen_draw.rs`: `Mark { Screen, Side, Fight }` is
     `pub(super)`; add `Mark::Editor` + `EditorPress`; helpers `colour(Role)`, `shade(i)`, `text_field(value, max)`,
     `cursor_style()`, `words(..)`.
   - The picture path: `shell/minimap.rs::image(&Raster) -> Image` (pub), `ImageNode` + `Node` sized to the raster,
     repaint keyed on a `Shown` tuple. `minimap.rs` constants (`BACKGROUND`, `WALL`, `DOOR`, `CLIFF`, `RAMP`, `JOIN`,
     `HOLE`, `MARKER`) and private `tick_pixels/line_pixels/join_pixels/edge_pixels` to move to `raster.rs` as pub.
     `raster.rs`: `Raster { width, height, scale, rgba }`, `filled`, `pixel`, `put`, `lighten`, `stroke`.
   - `shell/movement.rs::grab_cursor` (lines ~107-139): the `panel:` condition is `showing || fight`; add the editor.
   - `shell/session.rs`: `Config` (+ `editor: Option<Option<String>>` for `--editor [map-id]`), `parse` (a `match
     arg.as_str()` with `value(name)`), `StartScreen` resource, `Packs { data: Arc<Data>, config }` always present.
     `main.rs` inserts `Packs`, `Current(prefs)`, the `StartScreen`, then the plugin list (add `EditorPlugin`).
   - `shell/socket.rs::serve`: the `_ =>` arm answers `host::handle` with a `Session` else `NO_GAME`; answer
     `op.editor()` first through an `Option<ResMut<Editor>>` resource with `edit::answer(&mut editor.session,
     &packs.data, request?)` (`Op::editor()` returns `Option<Result<Request, OpError>>`).
   - Tests: `tests/common/app.rs` (`shell(config, in_game)`, `app_at_title`, `click`, `tap`, `top`, `press_ui`,
     `set`), `screen_text(&World)`; one binary `tests/main.rs`; every test file in a target
     (`scripts/check-test-modules.sh`).
   - Style: `style::Role { Interface, Title, Dim, Caution, Alert }`, `SHADES`, `Size::{Title, Body, Small}.px(h)`,
     `scaled`. Interface text ASCII only.
   - Sub-steps for M5.4: (a) entry: Title "Editor", `--editor`, `Nav.editing`, `EditorMenu`, the empty layer with
     three columns and a map choice + Open, `grab_cursor`; (b) `editor/plan.rs` raster + `hit` + cursor node,
     level choice, zoom/pan/centre, inspector cell lines; (c) the socket arm. Try: `just run --windowed --editor
     test:map:dungeon`; levels on `--pack crates/omnis-sim/tests/packs/levels`.
3. Then M5.5 to M5.10 per the plan's steps table.

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
