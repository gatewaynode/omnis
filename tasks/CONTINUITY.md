# Continuity notes

Written 2026-10-31, after step 3b of the editor's column as a drawing GUI (a stamp's detail chosen in a modal
picker, with a properties modal; `35b3e1ac`, **accepted by the owner the same day**: "Accepted, let's plan step
4"; `tasks/acceptance/editor-column-3b.md`). Step 3 (`48e73b74`) was hand-tested the same day and sent back with
the direction that became 3b; its acceptance is 3b's. Rewrite this file every time it is used.
Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `W1-authoring-tools`**, not pushed (the owner pushes). **M5 Editor v1 is closed** (M5.0 to M5.10,
  every step accepted; `tasks/acceptance/m5-10.md`); the **boundary tools** followed and were accepted
  (`9486183b`, `tasks/acceptance/boundary-tools.md`). Untracked and never staged: `assets/` and
  `docs/dnd_supplemental/` (the owner's). Owner world docs under `docs/background` and `docs/locations` that
  appear modified or new are committed at once, unedited, in their own commit.
- **The editor's column as a drawing GUI** (the owner's direction captured in TODO and memory
  `editor-column-as-drawing-gui`): step 1 (tool grid times palette grid, `96237cbf` and the next) and step 2
  (Fill and Move, `54161815`, `40863b52`) accepted 2026-10-25 (`c5e6dd8e`). **One open report kept past step 2's
  acceptance: Move does not carry the start marker in the owner's window** (not reproduced headless or through
  the real UI path; when taken up, ask whether the plan pans or the cursor follows and nothing lands, or read
  `editor_status` and `screen_text` over the dev socket while the owner holds the game open).
- **Step 3, placements as palette stamps (2026-10-25, `48e73b74`; `tasks/acceptance/editor-column-3.md`):**
  `Entry::Stamp(Placing)` last in the palette (icons door-open, swords, face-grinning, package, layers,
  scroll-text); a click with the brush places through the kind's own form, valid as it lands, refused where its
  kind already is, one undo step; a drag places once; the inspector's Add row gone; a right click picks a
  placement's stamp. Every placing form starts valid; `Form::keys` refuses a second portal, site or pile on a
  new form's tile; an edit under a new id goes `[Remove(old), new]` (`Place::key`). The owner's hand test
  (2026-10-31) rejected the detail's left/right choice: step 3b.
- **Step 3b, the detail as a modal picker with properties (2026-10-31, `35b3e1ac`, accepted;
  `tasks/acceptance/editor-column-3b.md`).** `editor/stamp.rs`: `Stamps([Vec<Place>; 6])`, a stamp holds what
  it drops as the data model's own places (`limit`, `details`, `short`, `rebuild`, `of_here` whole,
  `Editor::stamp_places` held-or-first, `Editor::stamp` through `Form::for_place` + `request` + `place_many`).
  `editor/pick.rs`: `Picking { kind, filter, chosen, props: Vec<Form> }` on `Editor.picking`;
  `pick_open/pick_filter/pick_toggle/pick_finish/props_open/prop/props_finish/typed_in/set_in`.
  `forms/props.rs`: `Form::for_place` (the builder with `was`, a fresh id), `Form::props_rows(n)`; `forms.rs`:
  `request` = `keys` + `value()`, `id_taken`. `acts.rs`: `EditAct::{Stamp (the icon), Pick, PickFilter,
  PickToggle(u16), PickFinish, Props, Prop(u8, u8), PropsFinish}`. `Asked::{PickDetail, Props}`;
  `ScreenId::{EditorPick, EditorProps}` (`screen/editor_pick.rs`, `screen/editor_props.rs`); `EditingCtx.pick`;
  **`Effect::Editor(act, payload)`** (text and set numbers reach the model); `shell/editor.rs::answer(nav, asked)`
  the one place the Asked's open and close overlays. Under the palette: the stamp's icon (`EditAct::Stamp`) and
  Select... (`Pick`) both open the picker (two acts: the layer syncs words by control id, so one id for two
  controls gave the button the icon's words), the sliders icon (`Props`) for Encounter/Object/Ground, a summary
  line ("giant_rat x3, goblin"; "nothing chosen, the first: <id>"). Owner's choices: the properties open after
  every Finish; piles get them too. Not offered: a per-placement loot (not in the data model). Verified: the
  search field keeps the keyboard and the caret through the rows' rebuild (`editor_pick.rs`, the keyboard test).
- **Observed, not fixed (told the owner 2026-10-25):** with the map's own file open as text and typed into, a
  paint and Save from the plan leave the text view older than the file; Save file would then write the old text
  over the paint. Narrow; a rule if the owner wants one.

## Next
1. **Step 4 (Lasso and moving a selection; runs that take a terrain entry), asked for by the owner on
   accepting 3b: plan mode first.** Or the entrances and exits enhancement where the start marker's move is
   taken up again. The owner's order of 2026-10-24 otherwise: the activities schema when
   a second kind has engine rules; then the plan in `tasks/TODO.md`: the look in data, M9.
2. Later: a quantity for an object kind in the picker (several of one kind in a click); a smooth (sub-cell)
   pan; a fill's region and a stamp's mark previewed on hover; the map's own fields (`Edit::Map`), Resize and
   New map as forms; a random entry's creatures as rows; the slider's thumb re-synced after a refused value;
   plan edges one pixel at every zoom; the stale-text-over-paint rule above; the 4 MiB file cap stays soft
   (owner); an `editor.run` op (and a fill, a move, a stamp); the icons' keys shown somewhere; `grab.rs`'s
   `moved_place` could reuse `Form::for_place` (a tidy).

## The owner's way of working (keep)
- Each item ends with something visible or clickable, and the report says how to try it (numbered: where to
  stand, what to press, what appears). The owner replies "tested" or "accepted" and asks for the next; a hand
  test can also come back as a direction (step 3 → 3b), which gets plan mode and its own commit.
- Hand-test launch: `just run --windowed --pack packs/base --pack packs/test --pack
  crates/omnis-sim/tests/packs/objects`, New game, devtools on; teleport with Esc, Debug, the World page, to
  `objects:map:yard`. The editor: Title → Editor, or `--editor test:map:town`. A Save reseals the pack on disk:
  hand tests of the editor run on a scratch copy (`cp -R packs/test /tmp/omnis-scratch`, named as the last pack).
- Offscreen captures: `scratchpad/yardshot.sh name x y facing walk [--screen ...]` (`cargo build -p omnis-vector`
  first). A capture cannot run a command first.
- Bug reports: fix directly, with a red test, a lesson in `tasks/LESSONS.md`, then report. Choices with real
  alternatives: AskUserQuestion with a recommended option first; larger additions get plan mode. A study's
  recommended answers become the owner's choices when the owner says "settle" (2026-10-21).
- Owner UI choices (memories `editor-pickers-are-modals`, `editor-column-as-drawing-gui`): pickers as modals
  over the editor, never a choice box over pack content (2026-10-31); a decision asked at the action, not spread
  as dim reasons; wheel zoom about the pointer to room scale; the column is a tool grid times a palette grid,
  never another row of text buttons; icons where they beat the word, words where the reason matters.
- The owner edits and saves through the editor and commits checkpoints of their own: tests never pin what those
  saves move (the pack's version, its notes); a content change the owner did not intend is asked about, not fixed.

## Pins
- **Replays:** walk `c5ce78d4a5d9655a`, fight `f90c92e47e6b2777` (rebaselined in M5.10a, `packs/test` 0.2.0, hash
  2508070832228001662). `SAVE_SCHEMA` 9, `PROTOCOL` 2, **ops 44, MCP tools 44, `Reply` 27 (`EVERY`), `OpError` 11**
  (since M5.3), `MAX_MIGRATIONS` 1024, `MAX_STEPS` 4096 (pinned to the docs by `omnis-mcp/tests/api_doc.rs`, run as
  `--test integration api_doc`; the tool counts also in `tools.rs`, `bridge.rs` twice, `omnis-cli/tests/headless.rs`,
  README, code-map).
- **Counts:** `Command` 16, `DevCommand` 14, `Event` 73, `Rejection` 71, `LoadError` 12. The palette is the
  terrains plus 10 (`brush::palette`: Wall, Door, Ramp, Height, then `Placing::ALL`'s six stamps; `tests/
  editor_fill.rs` pins WALL 9, DOOR 10, HEIGHT 12 on the town's nine terrains); `Tool::ALL` 7; `Placing::ALL` 6;
  the kit's `Kind` has 7 members; `Asked` 8; `ScreenId` has EditorMenu, EditorFiles, EditorPick, EditorProps as
  the editor's overlays (`nav::allowed` and `paused`). A stamp's picker limits: 1, `MAX_STACKS` 4,
  `MAX_OBJECTS_PER_CELL` 16, `MAX_PILE_STACKS` 32, `MAX_LORE_PER_CELL` 16. Base + test packs: 7 monsters, 24 items.
- **Fixtures:** `crates/omnis-sim/tests/packs/{ahead,chain/v1,chain/v3,heights,levels,objects}` (the shipped
  packs have no object kinds: a test that places an object opens `objects:map:yard` with the objects pack); a
  test that edits a pack works on a scratch copy (`omnis-data/tests/common::copy_pack`,
  `omnis-cli/tests/headless.rs::scratch_pack`, `omnis-vector/tests/common/editor.rs::in_editor_scratch`) and
  reseals it; the bad-pack corpus `broken` carries `notes/bad.ron`. `packs/test` ships notes on all four maps and
  on the town's cell (11,2), which also carries the portal; the town's sites sit on (1,1), (4,1), (7,1), (10,1),
  (1,3), (4,3), (7,3): a test that wants bare ground uses the street, row 2. The dungeon's encounters carry
  dispositions and counts above one (a right click's copy is tested there).
- **Limits the editor uses:** `MAX_STRING_BYTES` 4 KiB, `MAX_FILE_BYTES` 4 MiB (soft cap by the owner's decision),
  `MAX_SAVE_BYTES` 8 MiB, `MAX_COLLECTION` 65,536, `MAX_MAP_SIDE` 256, `MAX_LEVELS` 16, `UNDO_CAP` 100.
  `Session::files()` lists `pack.ron` and `.ron` under `data`, `text`, `notes`, `migrations` only.
- **Client test helpers:** `tests/common/app.rs` (`keyboard_on`, `keyboard`, `tap`, `set`, `press_ui`, `top` (needs
  a `Session`: the editor tests use `common::editor::top_screen`), `churn_fields`, `wheel`, `ui_editor_app`),
  `tests/common/editor.rs` (`in_editor`, `in_editor_on`, `in_editor_scratch`, `press_edit`, `arrow`, `point_at`,
  `point_frac`, `leave_plan`, `mouse`, `type_into` (an `EditorPress` field), `type_ui` (a `Pressable` field of
  the top screen), `top_screen`, `hold(app, kind, ids)` (a stamp's held places as Finish would make them, the
  revision bumped), `held`, `field_entity`, `field_of`, `panel`); `editor_pick.rs` has `pick(app, kind, ids)`
  (the UI path through the picker), `props`, `count_of`, `prop_of`, `pressable` (a dim control keeps its
  `Pressable` but loses `Button`), `bare_cells`; `editor_stamp.rs` has `choose(entry)`, `detail_to` (now
  `hold`), `first`, `click`, `right_click`. A choice control has no press entity: step it by `arrow`, never
  `press_edit`/`press_ui`. A written `RelativeCursorPosition` reaches the model on the next `app.update()`.
  Under Bevy picking a faked `Interaction::Pressed` is reset before the route reads it: drive the model through
  `shell::editor::act` in `ui_editor_app` tests. `screen_text` prints an icon as `[button] [icon] "name"` and a
  swatch as its glyph; an overlay's rows print after the editor's.

## Open, carried over
- `assets/README.md` font rows: the owner's file; report only, never edit.
- `docs/dnd_supplemental/`: the owner's, in progress; ask before committing anything there.
- P6 colours unconfirmed; step 5 revisits colour roles (joins, fills, crest, objects, beings).
- The automap still records the forward cone (§12.5); no objects or beings on it (owner, 2026-10-14).
- Dependencies: the rhai Socket re-audit (ask when); `smartstring` RUSTSEC-2026-0249; `ttf-parser`
  RUSTSEC-2026-0192 (informational); Bevy 0.20 not before 2026-11-07.

## Process
- **Cargo and git:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools` for both. `cargo fmt --all` first and
  again after any appended test. Workspace clippy with `--all-targets --features omnis-vector/test-fixtures --
  -D warnings` before every commit. Sim tests: `cargo test -p omnis-sim --test integration <filter>`; data `-p
  omnis-data --test integration <filter>`; client `-p omnis-vector --features test-fixtures --test integration
  <filter>` (one filter per invocation); client unit tests `--lib`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<item>.txt 2>&1`, VERIFY-GREEN, no source or pinned-document
  edits while it runs (about seven minutes; it fails fast on a clippy error, so run the workspace clippy first).
  Then `git add <paths by name>`, `git commit -F <msg>`, Sentrux `rescan` (first `scan` on
  `/Users/john/code/omnis/crates` in a new session) and `check_rules`. The scratchpad is
  `/private/tmp/claude-501/-Users-john-code-omnis/787890da-120b-4905-befb-f4247b7d8f51/scratchpad` (absolute).
- **Tests red first; mutations** by a Python runner in the scratchpad (anchor checked unique, source restored by
  copying the backup back, then `diff -q`; `mut3b/run.py` is the pattern). No cargo command and no fixture edit
  while a mutation run or the gate runs.
- **Moves and edits** by an anchored Python script in the scratchpad (every anchor asserted unique before
  writing). After `cargo fmt` the anchors move, so patch the file as it is. zsh: quote `echo "===="` (an
  unquoted `====` is a command lookup) and never pass `--include=*.rs` unquoted to grep.
- **Each new op** moves: `ops.rs`, `api_doc.rs`, `replies.rs`, `tools.rs`, `bridge.rs`, `omnis-cli/src/schema.rs`,
  `tests/headless.rs`, `docs/api.md`, README, `code-map.md`, ARCHITECTURE §9.3. Each new command, event or
  rejection also moves ARCHITECTURE §4.2's counts, `code-map.md`, `variants!` lists, the MCP `schema.rs` test and
  `schema_proof.rs`, `common/commands.rs`, `word.rs`, client `labels::refused`.
- **A change to `packs/` or a serialized `World`** rebaselines the replays in the same commit and records the
  fingerprints in `docs/api.md`'s changelog, `tasks/knowledge/verification.md` and the pins above.
- **Commits:** stage the changed files by name; never `git add -A` on `docs/`, `assets/` or the root; read
  `git status --short` before `git commit`. Messages end with the Co-Authored-By line and the Claude-Session line
  the system gives. Never push. Owner world docs (`docs/background`, `docs/locations`) go in their own commit,
  unedited. Ask before changing `PRD.md` or `ARCHITECTURE.md`; an owner's direction counts as the ask, flagged in
  the commit. Interface strings are ASCII only; every action has a mouse entry point.
- **Context:** pause and recommend a compact past 350K.
