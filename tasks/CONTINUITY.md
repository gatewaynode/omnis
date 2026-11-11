# Continuity notes

Written 2026-11-11 after the **`data.*` ops** were built (W1's first remaining item; plan mode, approved the
same day; `tasks/acceptance/data-ops.md`). **Built, awaiting the owner's test.** The owner's direction for the
plan: "we need forward thinking flexibility above all else, TTRPGs are filled with creative solutions and we
want to capture some of that magic" (memory `forward-flexibility-above-all`). Owner's choices: the definition as
its pack wrote it inside a stable envelope (`def` follows `omnis_data::SCHEMA`, not `PROTOCOL`); the author's
tier, a per-party gate for D30; generic filters now; the supplying pack recorded now.

What stands: `omnis-data/src/catalog.rs` (`KINDS`, 14 rows, every kind the loader gathers but the rules files;
`Definition` tagged `kind` beside `def`; `links`, `referrers`, `supplier`, `replaced`; `Kind::{ids, get, name}`),
`strings.rs` (a value's strings from its compact RON: a serializer of its own would name the float types, and
the lint's allow marker was not used), `Data.sources` from the loader; `omnis-sim/src/data_ops.rs` (`Op::data`,
`kinds`, `list`, `get`, `text`; `Listing`; `DefinitionRow`, `DefinitionView`, `KindCount`, `Words`; `LIST_DEFAULT`
100, `LIST_MAX` 500, `MAX_KEYS` 256; the page's field is `cursor` both ways); `OpError::UnknownId`; the socket
answers `data.*` with no game (`socket.rs` `serve`: the game's data when one runs, else `Packs.data`);
`omnis-cli op <name> [<args>]` (any op from a shell, arguments as RON with bare optionals). Tests:
`omnis-data/tests/catalog.rs`, `omnis-sim/tests/data_ops.rs`, `omnis-mcp/tests/{replies,vocabulary}.rs` (the
envelope under the vocabulary, `def` left out by name), `omnis-vector/tests/socket.rs` (the title),
`omnis-cli/tests/cli.rs`. Mutation runners: `mut-data1/run.py`, `mut-data2/run.py` in the scratchpad.

**The gate had a hole, closed the same day** (`fb69f172`): it passed with no test run when every test binary
failed to link (stale artifacts after I stopped a gate mid-build). `scripts/test-summary.sh` fails on a failed
test, a cargo `^error`, or zero passed, and self-tests. Read the count line as a number (1228 before step 2).
Never stop a gate part-way; if one was, `cargo clean -p <crate>` first (`tasks/LESSONS.md` 2026-11-11).

Before this (2026-11-10): Windows CI accepted, advisory (`tasks/acceptance/windows-ci.md`); the entrances and
exits enhancement closed 2026-11-08; the fixture and the canvas two packs since 2026-11-07. Rewrite this file
every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## When you resume
- **State**: branch `W1-authoring-tools`. Read `git status` and `git log --oneline -6` first: the `data.*`
  commits follow `2967303b`; the owner pushes, never me. Nothing of mine runs.
- **Read first**: this file, `tasks/knowledge/README.md`, `tasks/LESSONS.md` (2026-11-11 is the newest), the
  open part of `tasks/TODO.md`.
- **Waiting on the owner**: the hand test of `tasks/acceptance/data-ops.md` (nine steps from a shell). A report
  from it is fixed with a red test first. Two things told to the owner and open: whether to use the structural
  serializer with two `lint-sim: allow` lines instead of the RON text walk; rule slots override by rules-file id
  order, not pack load order (existing behaviour; theirs to decide).
- **Next after acceptance**, in the owner's order of 2026-11-08: the pass-time dev command
  (`DevCommand::Pass { minutes }`) and the MCP bridge probe, both "when needed"; then W1 closes. Otherwise the
  four Bevy ECS tasks (`tasks/TODO.md`, Open), Bevy 0.20 (W2's first item, its date has passed).
- **The machine** (checked 2026-11-11 after the system updates): command line tools in place, Rust 1.98.0 with
  the `x86_64-pc-windows-msvc` std, a fresh link of `omnis-cli` good. `gh` stays blocked in this sandbox.
- **Per commit, unchanged**: `cargo fmt --all`; clippy workspace all targets with `omnis-vector/test-fixtures`,
  `-D warnings`; the gate to a scratchpad file, the commit chained on `grep -q VERIFY-GREEN`; `git add` by name
  (never `-A`; `assets/` and `docs/dnd_supplemental/` are the owner's, never staged; owner world docs under
  `docs/background` and `docs/locations` commit at once, unedited, own commit); `git commit -F`; Sentrux `scan`
  on `crates/` once a session, then `rescan` + `check_rules`; the Co-Authored-By trailer the system gives;
  never push.

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
- **Step 5, Undo as a history list (2026-10-31, `d845eb4e`, accepted; `tasks/acceptance/editor-column-5.md`).** `edit/words.rs` names every undo step (a stroke counts its cells;
  `Document::name_step` takes the editor's richer words from `report`, fill, a run, a grab); `Document::{steps,
  at, goto}`; `EditAct::{History, HistoryTo}`; `screen/editor_history.rs` the modal (the map as opened, a row a
  step, the current one marked and dim, Undo, Redo, Close), `EditingCtx.history` from the editor's cached list
  (`read_history` in `changed`). Owner's choices: a History icon beside Undo and Redo; a modal, the plan beneath.
- **Step 4, the lasso, moving a selection, cell runs (2026-10-31, `f6537ddd` and the loop-preview fix
  `4e36a1e0`, accepted; `tasks/acceptance/editor-column-4.md`).** `editor/select.rs`: `Selection { level, cells }` on
  `Editor.selection`, the loop in `Editor.lasso` while drawn; `inside` (the loop's cells through `run::cells`
  plus the even-odd rule over the bounding box), `outline`, `moving(grid, def, sel, d)` (source paints first:
  Clear, Height 0, sides Open; then the destination's tile with its stair's rise, height, sides; boundary sides
  skipped; then the start and the placements by d; refuses off the map and a portal, site or pile on a
  destination tile not vacated), `Editor::{lasso_start, lasso_extend, lasso_end, selected, selection_outline
  (the ghost: displaced by cursor - grab.from while Held::Selection), move_selection → Document::batch}`.
  `grab.rs`: `Held::Selection` first in `under`; `grabbing` reads a loop in progress first and starts one on
  the lasso's Down. `run.rs`: `cells(a, b)`, `run_cells`, `Editor::cell_run()` = `entry.fills()`, `take_cells`,
  `Preview.cells`; a cell run's points are cells and the hover stays. `brush.rs`: `Tool::Lasso` (lasso-select,
  L), `Tool::ALL` 8. `acts.rs`: a run tool forces the wall for a stamp only; `Brush(i)` keeps a run tool for any
  non-stamp entry; `apply` drops the loop. `plan.rs`: `Source.selection`, `SELECT`, `bars`. `omnis-data`:
  `Document::batch(level, &paints, places)`. Owner's choices: a freehand loop; everything moves; the destination
  is replaced. The old run tests choose the wall first now (a terrain is a cell run); `editor_column.rs` flipped.
- **Observed, not fixed (told the owner 2026-10-25):** with the map's own file open as text and typed into, a
  paint and Save from the plan leave the text view older than the file; Save file would then write the old text
  over the paint. Narrow; a rule if the owner wants one.

## The entrances and exits enhancement (planned 2026-11-01; `tasks/TODO.md` W1; **closed, every step accepted, 2026-11-08**)
- Owner's scope (all four): loader rules on landings and starts; every map reached and every portal paired; pairs
  authored once in the editor; the start as a real entrance. Owner's choices: **the strict mirror** (the way back
  stands directly behind the landing; the party arrives facing away from it), **reachability for the maps of the
  pack that sets `entry`** (fixture packs exempt), **the landing picked on the far map's plan**. The
  Move-the-start report of 2026-10-25 is closed ("works now").
- **Step 1 built 2026-11-01 (the rules)**: `map_data.rs` `Target`, `enterable`, `standing_fault`, `arrival_errors`
  (no ground / cannot enter / a portal stands), `way_back`, `no_way_back`, `unreached`, `check_reached`;
  `Facing::{ahead_of, behind}`; the engine's `landing` and `standable` on `enterable`. A map with no portals is
  exempt from reachability (else New map could never be saved: the loader is all or nothing). The shipped stairs
  into the dungeon and the depths now land facing east (the pack 0.2.4); the replay scripts turn right after the
  stairs; `disk.rs`'s frozen-save comparisons guard on `report.fit == PackFit::Same` (the world's `packs` field
  records today's packs on a load by name, so the old guard was always true). **A portal stamped alone says "has
  no way back" until step 2**; `editor_stamp` and `editor_pick` pin that wording for now.
- **The owner's hand test of step 1 (2026-11-01)**: Save and Playtest dimmed with the loader's first error in
  full, read as "lost the test play option" and Save "overlapped by the red error text"; fixed: `view::error_
  count` gives both buttons the count alone ("1 error"), the words stay the inspector's and the message's
  (`tasks/LESSONS.md`). The test also showed that **no pair can be authored in the editor until step 2** (each
  half is refused until the other is on disk; hand-editing the files and `omnis-cli pack seal` is the only way).
- **Step 2 re-planned 2026-11-01** on the owner's direction ("an intermediary state for portals, 'unlinked'...";
  "a freehand portal_id text field... search through all available portals"); owner's choices: **one side is
  enough** (A's file says `link: Some("b")`, B's may say nothing; the editor never writes a second map), **the
  landing is derived** (ahead of the partner, facing its way; `to_*` gone from the file), **ids unique across the
  world**. The earlier plan's far-map document and `Kind::Picture` are withdrawn.
- **Step 2a built (the model)**: `map.rs` `Portal { level, x, y, id, link, facing, marker }`, `Portal::{ID_MAX,
  id_fault, landing}`, `validate` (id faults, ids shared within a map); `map_data.rs` `MapTargets::maps()`,
  `pairing` (partners: the named one and every claimant; errors: nothing loaded, itself, a third, two claimants),
  `landing` (ahead of the partner: outside, no ground, cannot enter, a portal stands), `resolve_portal` (a
  `ResolvedPortal` for linked portals only, so the sim, the floor mark, the minimap and `map.text` needed no
  change), `check_inbound` (other maps' pairings and landings here), `unreached` over linked pairs (maps without a
  linked portal exempt; **asked only of a sound world**, in the loader and in `resolve`: an erring link would
  otherwise list every map beyond it), `pub partner_of`, `unlinked_portals`, `unlinked_portals_in`. The editor:
  `forms::portal(def, data, at, was, detail, fresh)` (id, link, facing, marker; `portal_id`, `portal_id_taken`),
  `stamp::options(Portal)` = the unlinked portals of the loaded maps as the files hold them (the open map's
  unsaved ones come in 2b), `rebuild(Portal, [])` an unlinked portal, `summary` "unlinked", the choice cleared
  after a linked placement, `Mark::Portal { linked }`, the label "Portal 'id' to 'link'". Corpora: `bad-ways`
  rewritten (12-wide stand: ten rows), `bad-reach` new (one row), `broken` and `bad-objects` converted and resealed.
  Mutation checks red: the inbound claim ignored, the landing behind, reachability over every portal, the mark.
- **Step 2b built (linking in the editor)**: `pick.rs` `For::{Stamp, Field(u8)}`, `Picking { purpose, options,
  labels, .. }` built at open by `pick::options(kind, def, data, except)` (a portal's rows over
  `unlinked_portals(def, data)`, the open map's unsaved ones among them, labelled "id  -  Map (x, y)" through
  `labels::map`; `shown()` matches the label), `pick_open_for_link` (the form's Link...: the edited portal left
  out, its partner offered and shown chosen), `unlink`, `pick_finish` writing a field's choice through
  `Form::typed`; `EditAct::{LinkPick, Unlink}`; `Form::rows` adds the Link.../Unlink row for a portal form;
  `view::partner_line` under a portal's inspector row; `stamp::of_here` drops a copied portal's link. The
  picker screen reads `p.options`/`p.labels`, no longer `stamp::options`. `tests/editor_link.rs` (two flows).
  Mutation checks red: the edited portal offered, the choice kept, the field left empty.
- **The hand test of 2026-11-07**: the stamp's Finish leak (above), fixed in `route` with `Local<bool>` `stood`
  (the frame the cover lifts takes no `Down`; `Inputs` bundles the keys, the mouse and the wheel as one
  `SystemParam`, the routine being at Bevy's sixteen). Mutation check red: the guard dropped. The harness's
  `press_ui` sends no mouse button: a window report that resists a headless reproduction gets the window's input
  (`real_click`) before a second theory (`tasks/LESSONS.md`).
- **Step 2c (2026-11-07)**: `links.rs` `candidates` (= `pick::options(Portal, def, data, Some(at))` + the
  partner), `matches` → `Listed { shown, more, any }` (the whole id marks and leads; `SHOWN` 10), `with_list`,
  `Editor::{link, unlink}` through `fill_link` → `Form::set_text`; `EditAct::Link(u16)` in `LinkPick`'s place
  (the `EditAct` count unchanged); `pick.rs` without `For`/`purpose`/`pick_open_for_link`, `fits` shared.
- **Step 3 (2026-11-07, `76572568`, accepted 2026-11-08)**: `MapDef.start_level`, `Place::Start { level, .. }`,
  `validate`'s start row, `resolve_one`'s standing check on the start's level, `arrivals` under it; `World::new`
  and `places::party` on it; `SetStart` on the shown level, `grab::under`/`select` by level, `open` on the start's
  level, `Source.start` with its facing, `plan::start_pixels` (diamond + notch to the cell's side);
  `remove_level` returns a start on it to the ground. `tests/editor_start.rs`; `common::editor::scratch_of`.
  Follow-ons noted in the TODO review line: a start mark in the 3D preview, `omnis-cli`/`editor_new` starts,
  Playtest from the start.

## Next
1. The `data.*` ops are built and await the owner's test (`tasks/acceptance/data-ops.md`). Then, in the owner's
   order of 2026-11-08 if they keep it: the pass-time dev command and the MCP bridge probe when needed. Windows CI is closed and advisory (`tasks/acceptance/windows-ci.md`); a red Windows step on a
   later push is a fault for its own commit with a test (grep `.join(` against the 2026-11-08 lesson first). When the owner orders it: **the client's use of Bevy's ECS** (`tasks/TODO.md`, Open: four tasks, a
   press as a message first; plan mode, one step a commit). The frozen fixture carries the portals as they were at
   0.2.9 (the owner's strays `portal_23_0`, `portal_23_28` and `portal_bank_hidden` included): a test names them
   by content, never by count. The editor's later items otherwise (the eraser or Delete clearing a selection's
   cells, copy and paste, `editor.history` over the socket, a quantity for an object kind in the picker). The
   owner's order of 2026-10-24 otherwise: the activities schema when a second kind has engine rules; then the
   plan in `tasks/TODO.md`: the look in data, M9. W1's remaining items: Windows CI at W1's close (D31).
2. Later: the eraser or Delete clearing a selection's cells; copy and paste of a selection; Esc dropping a loop
   or a run (Esc is the menu's); a quantity for an object kind in the picker (several of one kind in a click); a smooth (sub-cell)
   pan; a fill's region and a stamp's mark previewed on hover; the map's own fields (`Edit::Map`), Resize and
   New map as forms; a random entry's creatures as rows; the slider's thumb re-synced after a refused value;
   plan edges one pixel at every zoom; the stale-text-over-paint rule above; the 4 MiB file cap stays soft
   (owner); an `editor.run` op (and a fill, a move, a stamp); the icons' keys shown somewhere; `grab.rs`'s
   `moved_place` could reuse `Form::for_place` (a tidy); a start mark in the 3D preview with the icon pass.

## The owner's way of working (keep)
- Each item ends with something visible or clickable, and the report says how to try it (numbered: where to
  stand, what to press, what appears). The owner replies "tested" or "accepted" and asks for the next; a hand
  test can also come back as a direction (step 3 → 3b), which gets plan mode and its own commit.
- Hand-test launch: `just run --windowed --pack packs/base --pack packs/the_lower_kingdoms` (the owner's world; with the
  objects fixture: `--pack packs/base --pack crates/omnis-sim/tests/packs/test --pack
  crates/omnis-sim/tests/packs/objects`, the fixture standing in for the world), New game, devtools on; teleport with Esc, Debug, the World page, to
  `objects:map:yard`. The editor: Title → Editor, or `--editor test:map:town`. A Save reseals the pack on disk:
  hand tests of the editor run on a scratch copy (`cp -R packs/the_lower_kingdoms /tmp/omnis-scratch`, named as the last pack).
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
- **Replays:** walk `0370ab07b628f193`, fight `c81c08bde3a070e9` (against the frozen fixture at 0.2.9, hash
  14523718773761627429, since 2026-11-07; the owner's canvas moved on to 0.2.18 and beyond and moves them no more;
  earlier that day 0.2.9 for the owner's pack, three unlinked portals and four cells; 2026-11-01 for step 2a, the portals linked by id,
  `packs/test` 0.2.5, hash 15167835935440741063; earlier that day the two east arrivals at 0.2.4; 2026-10-31 the owner's depths at
  0.2.3; M5.10a at 0.2.0). A rebaseline
  records the new fingerprints here, in `docs/api.md`'s changelog and in `tasks/knowledge/verification.md`. `SAVE_SCHEMA` 9, `PROTOCOL` 2, **ops 48, MCP tools 48, `Reply` 31 (`EVERY`), `OpError` 12**
  (since the `data.*` ops, 2026-11-11), `MAX_MIGRATIONS` 1024, `MAX_STEPS` 4096 (pinned to the docs by `omnis-mcp/tests/api_doc.rs`, run as
  `--test integration api_doc`; the tool counts also in `tools.rs`, `bridge.rs` twice, `omnis-cli/tests/headless.rs`,
  README, code-map).
- **Counts:** `Command` 16, `DevCommand` 14, `Event` 73, `Rejection` 71, `LoadError` 12. The palette is the
  terrains plus 10 (`brush::palette`: Wall, Door, Ramp, Height, then `Placing::ALL`'s six stamps; `tests/
  editor_fill.rs` pins WALL 9, DOOR 10, HEIGHT 12 on the town's nine terrains); `Tool::ALL` 8 (Select, Brush, Move, Lasso, Fill, Corners, Freehand, Eraser; keys V B M L F P N X);
  `Placing::ALL` 6;
  the kit's `Kind` has 7 members; `Asked` 9; `ScreenId` has EditorMenu, EditorFiles, EditorPick, EditorProps,
  EditorHistory as the editor's overlays (`nav::allowed` and `paused`). A stamp's picker limits: 1, `MAX_STACKS` 4,
  `MAX_OBJECTS_PER_CELL` 16, `MAX_PILE_STACKS` 32, `MAX_LORE_PER_CELL` 16. Base + test packs: 7 monsters, 24 items.
- **Fixtures:** `crates/omnis-sim/tests/packs/test` is **the frozen fixture** (id `test`, 0.2.9, hash
  14523718773761627429: `packs/test` as committed at `8d58c3e2`; owner 2026-11-07: never saved onto, and a test never
  loads the owner's own pack under `packs/`; `common::FIXTURE` in the vector harness);
  `crates/omnis-sim/tests/packs/{ahead,chain/v1,chain/v3,heights,levels,objects}` (the shipped
  packs have no object kinds: a test that places an object opens `objects:map:yard` with the objects pack); a
  test that edits a pack works on a scratch copy (`omnis-data/tests/common::copy_pack`,
  `omnis-cli/tests/headless.rs::scratch_pack`, `omnis-vector/tests/common/editor.rs::in_editor_scratch`) and
  reseals it; the bad-pack corpus `broken` carries `notes/bad.ron`. `packs/test` ships notes on all four maps and
  on the town's cell (11,2), which also carries the portal; the town's sites are the owner's to move (rows 0 and
  4 since 2026-11-07): a test that wants bare ground uses the street, row 2, and finds a site by its service. The dungeon's encounters carry
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
  `hold`), `first`, `click`, `right_click`; `editor_select.rs` has `drag(app, button, from, through)`, `lasso_the_inn`,
  `selected`, `site_at`; `editor_run.rs::choose(tool)` chooses the wall first for a run tool. A choice control has no press entity: step it by `arrow`, never
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
  `/private/tmp/claude-501/-Users-john-code-omnis/762beda3-a95d-40a0-9a67-8bf9680a2cb8/scratchpad` (absolute).
- **Tests red first; mutations** by a Python runner in the scratchpad (anchor checked unique, source restored by
  copying the backup back, then `diff -q`; `mut3b/run.py` is the pattern). No cargo command and no fixture edit
  while a mutation run or the gate runs. A mutant must undo the rule it targets, not restate it (a
  "destination before source" mutant that moved the source paints to the front was still the rule: `mut4/run.py`
  takes a list of replacements so the source paints can be moved to the end).
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
