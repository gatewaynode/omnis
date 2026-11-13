# TODO

The development execution plan (owner, 2026-10-21). Alignment flows from the vision documents, `PRD.md`,
`ARCHITECTURE.md`, `presentation-PRD.md` and `presentation-ARCHITECTURE.md`: every item names the section it
serves, and where the plan needs a document to change, the change is listed under W0 and the owner approves it
before it is made. What was built before 2026-10-20 (M0 to M8, protocol 2, vector adoption P0 to P9, M8.5 steps 1
to 4 and 7) is recorded in `tasks/archive/TODO-2026-10-20.md`. Small findings and unused assets: `tasks/BACKLOG.md`.
Bugs: `tasks/BUGS.md` (B1 to B7, all closed).

## The goal: ready for world building
The MVP is proven: a party is created, walks a 3D world drawn in lines, fights, levels, shops, rests, saves and
loads by name across changed packs, and every system is driven by data, by the CLI and by an agent over MCP. The
next goal is that the owner builds Toel's first locality, **the Three Kingdoms of the lower Blood Sea**
(`docs/locations/`: Karinguard, Shoentran and Resdie; Foloen and the Sunken Grove uncharted; the Old Dam, the
Great Goblin Head, the ruins and the catacombs; biospheres piedmont, coastline and mountains), with the shipped
tools and in pack data, nothing hard-coded (PRD goals 2, 3 and 6, D4, §11.3).

**Ready means (owner, 2026-10-21): the end of W1.** Every map and level is authored in the editor and
round-trips, and the owner starts on the first settlement, Resdie. The world's look as data a region names (W2)
and the locality's regions, settlements and wilds laid out on the continent (W3) are built while that content
grows, each against what the content needs (PRD R5). The living world (W4) makes what was authored move.

**The order and why.** The old order ran tools last (PRD §13 put the editor first, D4 moved it to close Phase 1,
and the milestones built the loop, the time model, the API, the presentation and the save model before it).
That was right for proving the loop; it leaves every map hand-written. From here the order is what world
building needs first:
1. **W0** clean-up and alignment, so the plan and the vision documents agree;
2. **W1** the authoring tools (M5), since nothing else the owner authors should be hand-written RON;
3. **W2** the look and the engine under it (Bevy 0.20, the rendering direction, M8.5 steps 5 and 6, set pieces),
   since the owner's regions each name biospheres and the lore rests on ambient mana;
4. **W3** the world's structure (M9), since the locality is regions on a continent with wilds to fill;
5. **W4** the living world (M10, M11), since factions, rumours and quests are the lore's second half;
6. **W5** release (M12).

Milestone numbers are kept inside the phases (M5, M8.5, M9 to M12); a milestone never changes number. Each phase
is planned in plan mode when it is reached, one sub-step at a time, each ending in something the owner can try.

## Standing rules (from the delivery plan, 2026-09-12)
Tests on real `packs/test` data, no mocks; sim-crate lint green (CLAUDE.md); Sentrux review; `LESSONS.md` updated
after any correction; one commit per checked item or small group; MCP tools and CLI subcommands ship with the
system they expose, never later; every player action, dev tools included, has a button before a key; the client
reads the engine through `omnis_sim::api`, never `World` fields; the owner pushes.

## W0 — Clean-up and alignment (now)
Done when the plan, the vision documents, the README and the notes all tell one story, and the three far
horizons have a study each.
- [x] W0.1 This plan (2026-10-21): `tasks/TODO.md` refactored (this commit); `tasks/CONTINUITY.md` and
  `tasks/knowledge/README.md` repointed.
- [x] W0.2 (2026-10-21) `tasks/alt-TODO.md` archived to `tasks/archive/alt-TODO-2026-10-21.md`: every item in it is done
  (Phases A and B, 2026-10-02 to 2026-10-10) or tracked here (Bevy 0.20). presentation-PRD §11's "Report in
  `tasks/alt-TODO.md`" follows the move (owner approves).
- [x] W0.3 (2026-10-21) `README.md`: the Status table stops at M6 and says "Next is M7"; "`tasks/TODO.md` is ... the record of
  what each milestone delivered" now names the archive.
- [x] W0.4 (2026-10-21, owner: all in one commit) The vision documents brought to this plan:
  - PRD §13's note of 2026-09-12 ("revisit this section once the milestones settle"): revisited; it names this
    plan's phases and which PRD phase each closes; §13 Later gains the three far horizons as the studies leave
    them.
  - ARCHITECTURE §16 phase mapping: a status column (0 and 1 built but the editor; 2 to 5 planned); the Phase 1
    row still names the retired `omnis-app`; the build-order note points at the archive. §4.7's "the tables are
    in `tasks/TODO.md`" and §8.3's "`tasks/TODO.md` M1 review" point at the archive. The Date row is the
    creation date.
  - presentation-PRD §11: steps 1 (the merge, 2026-10-10) and 4 (height and cliffs, M8.5 1a to 1c and 4) marked
    done; the list reordered to W2's order.
  - presentation-ARCHITECTURE: the "Vector adoption" pointer in its status line names the archive; §4.3's
    labels are built (`labels.rs`), not "to build"; §7's style system is partly built (the type scale and fonts).
- [x] W0.5 (2026-10-21; `tasks/plans/study-{accounts,multiplayer,multi-platform}.md`; the owner chose from them 2026-10-21: PRD D29 to D31, placed in the phases below) Three studies for the far horizons the MVP makes thinkable (owner, 2026-10-21), each a
  `tasks/plans/study-<name>.md` that reads only: what the documents say today, what the code constrains, the
  options with their costs, the proposed PRD rows (a decision, the §2 consumers, §5 non-goals, §13 Later) and
  the earliest phase the work could enter. Nothing is built until the owner chooses from a study.
  - **Cryptographically secure user accounts.** Today: no account, login, cloud or server exists in any
    document; PRD §7.7 says saves are the player's files with no cloud dependency; every hash the engine uses
    (the world id, base names, the pack seal) is FNV-1a 64, which is not cryptographic; the dev socket is
    loopback only with no auth. The study covers the four purposes the owner named (2026-10-21), since each is a
    different design and a different dependency audit: identity in multiplayer (a player keypair naming a party
    and signing its commands); signed saves and world files (tampering detectable); signed packs and mod
    provenance (the seal becomes a signature); a hosted service (login, profiles, cloud saves), which PRD §7.7's
    "no cloud dependency" admits only as optional, a row the study proposes.
  - **Multiplayer.** Today: a v1 non-goal (PRD §5) kept possible by goal 7 and D5 (co-op later, §13 Later);
    ARCHITECTURE §4.6 "share the command stream", §11 a `party:<id>` stream per party, §4.4 each party a holder
    of its own clock (D25); `World` holds one party and one position; no async runtime (`tokio` deliberately
    absent). The study maps co-op onto the character save and world folder of D28 (several character saves over
    one world file is already named in `tasks/plans/world-state.md`).
  - **Multi-platform.** Today: D5 names macOS, Linux and Windows; browser and mobile are non-goals, with §14
    asking the owner which are rejected outright; CI runs macOS only (Linux deferred by the owner 2026-09-12 until
    needed; Windows once Phase 1 is playable, ARCHITECTURE §14); packaging and the settings directory arrive
    with M12. The study costs Linux and Windows CI now against at M12, and names what a web or mobile build would
    give up (the lint's own rules make the simulation portable; the client is Bevy's reach).
- [x] W0.6 (2026-10-21) The Linux CI job, advisory (D31): `.github/workflows/ci.yml` gains a `linux` job with
  `continue-on-error` and the same steps as the macOS check; the golden replays recorded on macOS reproduce their
  fingerprints there (ARCHITECTURE §11 tier 3). It reports and never blocks, so no merge waits on a Linux build
  while no one plays on Linux (owner, 2026-10-21). Verified before the push only as far as a Mac can: the non-Bevy
  crates type-check for `x86_64-unknown-linux-gnu`; the first run on GitHub is the Bevy build's proof.

## W1 — The authoring tools (closes PRD §13 Phase 1, D4)
Done when the Phase 1 content (`packs/test`'s maps and levels) is re-authored in the editor and round-trips with
no loss, and every later map is built with it.
- [x] **M5 Editor v1** (closed 2026-10-25 at M5.10; deferred 2026-09-19; settled 2026-10-14: `bevy_ui` in the neon style, a top-down plan one
  level at a time with the 3D view as preview and Playtest, maps plus a RON text editor checked by the loader for
  every other file, a `notes` text on maps and levels; PRD §10, ARCHITECTURE §8.2; `tasks/plans/editor-v1.md` is
  superseded for planning and keeps §A, the document model, and §E's kinds and limits to reuse). Planned in
  plan mode first, against the format as built (heights, ramps, levels, joins, stairs, portals, encounters by
  id, sites, objects, the pack seal). **Planned 2026-10-21: `tasks/plans/m5-editor-v1.md`.** Owner's choices: a
  save reseals and bumps the patch version; the four maps' comments move into `notes`; Playtest uses the fixed
  party. The steps, one commit each, each with an acceptance script `tasks/acceptance/m5-N.md`:
  - [x] M5.0 (2026-10-21) The plan saved; this block.
  - [x] M5.1 (2026-10-21) The author's notes as a keyed store, `notes/*.ron`, checked on load and never sealed (owner's
    direction, replacing a 4 KiB field per map and level); the loader's per-map resolve extracted as
    `MapData::resolve` (in memory, with the other maps' portals checked both ways); `tasks/acceptance/m5-1.md`.
  - [x] M5.2 (2026-10-21) `omnis_data::edit`: `Grid`, `Document` (undo, redo, strokes), `Session` (open, save, seal,
    files, check, write, the notes store); byte-identical layouts on every fixture map; `tasks/acceptance/m5-2.md`.
  - [x] M5.3 (2026-10-21) `editor.*` ops (17, notes among them), seven replies, `OpError::Invalid`, `Headless.editor`,
    the MCP tools and schemas; counts 27 to 44; `tasks/acceptance/m5-3.md`. Accepted by the owner 2026-10-22 (B8 found
    on the way, fixed in `9ac9468`).
  - [x] M5.4 (2026-10-22) The client's editor layer: Title "Editor", `--editor [map-id]`, `Nav.editing`, `EditorMenu`, the
    three columns (tools, the plan raster, the inspector), cursor by click and arrows, levels, zoom, facing; the
    socket answers `editor.*` on the open editor; `tasks/acceptance/m5-4.md`. Accepted by the owner 2026-10-22.
  - [x] M5.5 (2026-10-22) Brushes (`editor/brush.rs`: the terrains, Wall, Door, Ramp, Open edge, Height, No ground;
    digits 1-9), a click paints the cell or the edge under the pointer, a drag is one stroke, a right click picks,
    Enter paints at the cursor, Undo/Redo (Cmd Z, Shift Z, Y), Save (button, Cmd S, the menu) dim in the loader's
    words while invalid and otherwise writing, resealing and reloading the packs, Revert; `tasks/acceptance/m5-5.md`.
    Accepted by the owner 2026-10-24 after three fixes from the hand tests (below).
    - [x] Fix (2026-10-24, the owner's first hand test): the plan's pointer read Bevy's `normalized` as 0..1
      (it is −0.5..0.5), so the mouse did nothing; WASD moves the cursor like the arrows. Red tests:
      `tests/editor_pointer.rs` (a click through Bevy's own UI layout and focus) and `wasd_moves_the_cursor`.
    - [x] Fix (2026-10-24, the owner's second report: doors direction-sensitive and unremovable, walls hard):
      an edge brush aims at the nearest side of the cell wherever the click lands (`Editor::aim`,
      `View::edge_target`), never the facing; the aim is a yellow hover (`PlanHover`) before the click; the
      pick follows it; "Open edge (erase)". Red test `an_edge_brush_aims_at_the_nearest_side…`.
    - [x] Fix (2026-10-24, the owner's third report): every height step rebuilt the tools column unscrolled;
      a rebuilt column keeps its panel's scroll (`Drawn.scrolls`, restored to the panel added next frame).
      Red test `a_rebuilt_tools_column_keeps_its_scroll`.
    - [x] Proposed (owner 2026-10-24), for the M5.6 plan: a **fixed tool bar** carrying the attributes every
      cell has (height first, later the rest), always in reach instead of a row among the brushes; the
      owner suggested the top of the window. Built as M5.6a, the attribute bar over the plan (owner's choice
      of place 2026-10-24).
    - [x] Deferred (owner 2026-10-24, after M5.6 at the earliest): **boundary tools** beyond the edge brush. The
      owner wants both a **point-by-point** tool (click corner after corner, the run of walls follows) and a
      **freehand** tool (a drawn line), with a **normalisation** of those lines to the map's granularity: the
      line is snapped to the grid's corners and rasterised onto cell sides, the result previewed in the hover
      colour before it is taken, one undo step. The nearest-side edge brush and its hover stand meanwhile.
      **Built 2026-10-25, accepted by the owner the same day** (`tasks/acceptance/boundary-tools.md`; owner's choices in the planning: walls only,
      as two palette entries "Walls by corners" and "Walls freehand"; a freehand line taken on release, a corner
      run pending until a click on its last corner, Enter or Take run; Backspace drops a corner): `editor/run.rs`
      (`sides(a, b)` the grid-line walk that hugs the line, `paints` each side once with the boundary left out,
      `Editor::pointer` the plan's pointer for every tool, `take_run`, `preview`), `View::corner`, the preview in
      the plan's raster (`Source.preview`, `minimap::PREVIEW`), `Document::paint_many` (one step, all or nothing).
      Follow-on: an `editor.run` op when a headless author asks for one (a run is `stroke` paints over the socket
      today).
  - [x] M5.6 Placements, terrains, levels and notes forms; `tasks/acceptance/m5-6.md`. Planned 2026-10-24 (plan
    mode, approved): the owner chose the bar above the plan and **dedicated row editors** for list fields
    (stacks, contents, items, the random table) over RON text. **Accepted by the owner 2026-10-24** after the
    marks and the encounter defaults below ("walked a party around the map and things look pretty good").
    - [x] 6a (2026-10-24) The attribute bar over the plan (`view::bar`, `Column::Bar`, `ScreenId::EditorBar`):
      the cursor's cell's terrain, height and stair rise stepped where they stand, one undo step each
      (`EditAct::{CellTerrain, CellHeight, CellRise}`). A column whose words change keeps its nodes
      (`OfScreen` markers, `sync` shared with the screens); the field with the keyboard keeps it through a
      rebuild; a plan click takes the keyboard from a field. `Kind::Text.lines`; editor fields carry
      `EditorPress`. Tests `tests/editor_forms.rs` (3), four mutations killed.
    - [x] 6b (2026-10-24) The generic form (`editor/form.rs`: `Form { kind, heading, at, fields, lists, remove,
      fresh }`, `Value::{Text, Choice, Number, Flag}`, `Slot::{Top, Row}`, rows added and removed, `id`/`dice`
      parsers) and the placements (`forms.rs`: `Placing`, `Form::placing/existing/random/rest_event/request`,
      `find`); `Editor { form, place_kind }`, `here()`, acts `SetStart`, `PlaceKind`, `Add`, `EditAt`, `RemoveAt`,
      `Random`, `RestAdd/Edit/Remove`, `Field`, `ListAdd`, `ListRemove`, `Apply`, `Cancel`, `Discard`; the
      inspector's start, placement, Add and map rows; `route` reads editor fields and the slider's `Slid` (left
      by the screen router when no screen is up); the editor's columns give the keyboard only to a named
      field. Tests: three in `tests/editor_forms.rs` (the portal typed on Bevy's keyboard path), two unit;
      five mutations killed.
    - [x] 6c (2026-10-24) `FormKind::{Terrain(Option<char>), LevelAdd, Level(i8), Note(key)}`, `Change::{Place,
      Edit, Note}` as a form's request; `Form::terrain/level_add/level/note`; acts `TerrainAdd`, `TerrainEdit`
      (the brush's terrain), `LevelAdd`, `LevelEdit`, `Note(NoteOf::{Cell, Level, Map})`; `Editor::note_key/note`,
      `change()`; the tools' Add/Edit terrain and Add level/Level… buttons, the inspector's note rows; a note
      written at once through `Session::set_note`, said as not undone. Tests: three more in
      `tests/editor_forms.rs` (the note on a scratch pack), mutations killed.
    - [x] Marks in the game (2026-10-24, owner's direction): the minimap (on any known tile, derived from the
      map) and the 3D floor mark a portal with a rectangle and a circle (doors keep the cross); a site's cell
      carries the activities' placeholder smiley on the floor. A stored automap bit was tried and backed out:
      it would fold every older save on load.
    - [x] Marks on the plan and encounters that start valid (2026-10-24, owner's direction): `plan::Mark` and
      `plan::marks(def, level)` (portal rectangle+circle, encounter triangle, object square, pile dot, lore
      corner, site smiley); `forms::encounter` prefilled with a free id and one stack of the first loaded
      monster, the Stacks list at `MAX_STACKS`; `Form::keys` refuses a taken encounter/object/lore id and a second
      encounter on a cell; `apply_form` says the loader's first error after a change. Tests in
      `tests/editor_forms.rs` and `plan.rs`.
    - [ ] Proposed (owner 2026-10-24): an **activities** category of map item, the town services first, later
      fountains, fixed NPCs, mechanisms, anomalies: what takes the party out of its stride into a decision. In
      the editor one Add choice "Activity" opening a list of what to create; each needs a representation in the
      game (a placeholder smiley for now). The owner's problem: the town's services take up too much of the map
      for one-time placements (building terrains three cells wide for one site each). Decide with the owner at
      the next planning: whether `Site` becomes one kind of a wider `Activity` placement in the data (a schema
      move, M5.10 re-authors the maps anyway) or stays as is with the editor grouping only.
  - [x] **Icons** (owner 2026-10-24: "its own step after M5.6"; planned and approved the same day: a new crate
    `omnis-icon`, the whole Lucide set compiled in; `tasks/acceptance/icons.md`). **Accepted by the owner
    2026-10-24** ("Manually tested"). Owner's direction for what follows: with the icons handy, the map and the UI
    are to move away from text where it makes sense: a design pass on which controls, marks and HUD lines
    become icons, planned with M5.7's forms or as its own step.
    - [x] 1 (2026-10-24) `crates/omnis-icon`: `build.rs` compiles `assets/svg-icons/icons/*.svg` and the ISC
      licence in (`catalogue::{get, names, text, count, LICENSE}`); `svg.rs` reads the set's subset and refuses
      the rest; `path.rs` every command, implicit repeats, the compact syntax, arcs by F.6.5; `shape.rs` circles,
      ellipses, rounded rects (through the path code), lines, polylines, polygons; `Icon { name, strokes }` with
      `segments(size)`, `width(size)`, `bounds()`. Tests: all 1,903 icons read within their box (a pinned count),
      the shapes, the licence, the path syntax; `examples/show.rs` prints an icon as text.
    - [x] 2 (2026-10-24) The client draws icons: `omnis-vector` depends on `omnis-icon`; `Raster::icon` (the round
      brush at any size), `geometry::icon_on_floor`, `icons::activity()` = Lucide `face-grinning` read once; the
      plan's site mark and the 3D activity mark draw it in place of the hand-made smiley. An image cache for UI
      nodes waits for the first control that takes an icon (a design pass with the owner).
  - [x] M5.7 The 3D preview from the cursor; `tasks/acceptance/m5-7.md`. **Accepted by the owner 2026-10-24**
    after the gating fix ("Arrows, WASD, and 3D view work"). Planned and approved 2026-10-24: an
    `Occupancy` trait so the scene builds with no world, `AsPlaced` for the editor, a second camera on layer 2
    drawing into an image the right column shows. Owner's decision in the planning: **activities** are an editor
    grouping now (Add → Activity → a list, Service site first, an icon per `ServiceKind`), the schema later when
    a second kind has engine rules (the small step after M5.7).
    - [x] 1 (2026-10-24) `scene::Occupancy` (+ `impl for SimWorld`), `Scene::build_from(data, &MapData, id, level,
      &dyn Occupancy)`; `api::placed_on_level`/`standing_as_placed` (hidden objects shown); `editor/preview.rs`
      (`AsPlaced`, `scene`, `map_id`, `pose`, `look`, `SIGHT`); `render::{colour, fade, fog_end, CREST}` shared.
      Tests `tests/editor_preview.rs` (3), the engine's as-placed test.
    - [x] 2 (2026-10-24) `shell/preview.rs`: `PreviewTarget`, `PreviewCamera` (layer 2, into an 800×450 image,
      on while `Editing` exists), `PreviewGizmos`, `PreviewFills`, `Preview { scene_key: (resolved_at, level),
      scene }`, `rebuild`/`place`/`draw` in `VectorSet::Draw`; `Editor.resolved_at`; `Column::Preview` over
      `Column::Inspector` in a right column node; `main.rs` adds `(EditorPlugin, PreviewPlugin)`. Acceptance
      steps 2–5 for the owner's window.
    - [x] Fix (2026-10-24, the owner's first hand test: "an empty viewport"): the preview's systems were in
      `VectorSet::Draw`, gated on a game session; now after `VectorSet::Screens` on the editor's own
      condition, the plugin headless-safe and tested in the shell. "Level…"/"‰" glyphs the font lacks
      replaced. WASD reported broken: not reproduced (a test with the focus plugins on passes); the owner
      found it working after the fix.
    - [x] Activities in the editor (2026-10-24; `tasks/acceptance/activities.md`): `Placing::Activity` replaces
      `Site` in the Add choice; the site form names its kind ("Service site", `ACTIVITIES`) and service;
      `plan::Mark::Activity(Option<ServiceKind>)` and `geometry::activities(data, map, level, ground)` (from
      `scene::level_of`) draw `icons::service(kind)` (bed, church, dumbbell, anvil, beer, coins, shield), the
      grinning face for a service the packs lack. **Accepted by the owner 2026-10-24.**
  - [x] M5.8 Playtest and Back to editor; `tasks/acceptance/m5-8.md`. Built 2026-10-24 (plan mode, approved);
    **accepted by the owner 2026-10-24**:
    `editor/playtest.rs` (`blocked`, `data_for` (a copy of the packs with the edited map interned and put in),
    `teleport`), `EditAct::Playtest` → `Asked::Playtest`, `shell/editor.rs::start_playtest` (`Session::with_party`
    with `party::fixed()`, the teleport ordered, the pose set) and the editor parked in `Playtesting(Editor)`
    (moved out, not cloned; its layer and preview go down on the resource's removal); `Ctx.playtest`,
    `ControlId::BackToEditor`, `Effect::ToEditor` (the game removed, the parked editor re-inserted); Pause and
    Defeat in a playtest. `refresh` tolerates the frame the editor is taken away. Tests
    `tests/editor_playtest.rs` (3); three mutations killed.
  - [x] M5.9 The RON text editor (Check, Save verbatim); `tasks/acceptance/m5-9.md`. Built 2026-10-24 (plan mode,
    approved): `editor/files.rs` (`Mode`, `FileEdit { rel, text, disk, errors, fresh }`, `blocked_save`, `verdict`),
    `Editor { mode, files, file }`, acts `Plan`, `Files`, `File(n)`, `FileText`, `Check`, `WriteFile` (→ `Asked::Saved`,
    the map's own file reopening the document), `RevertFile`; `view::tools` with the Plan|Files row and `file_rows`,
    `view::text` (`ScreenId::EditorText`, one forty-line field), the inspector's verdict; `Column::Text` shown by
    mode in the centre; a field of several lines takes its row's width. Tests `tests/editor_files.rs` (3) and the
    unit test. Accepted 2026-10-25 after M5.9b (the file picker as a modal over the editor,
    `screen/editor_files.rs`), M5.9c (Open file... asks about unsaved text; `Kind::Document` with line numbers
    and both scrollbars) and M5.9d (the wheel zooms the plan about the pointer; the zooms run to 128 px a cell).
  - [x] M5.10 (2026-10-25) The four maps re-authored (comments to notes), pack resealed, replays rebaselined,
    documents as built; `tasks/acceptance/m5-10.md`, accepted by the owner 2026-10-25. Built in three commits: the pack (`tests/edit_authoring.rs`
    builds every shipped map from blank with the editor's requests alone and the save writes the shipped bytes;
    the ignored rewrite; `notes/<map>.ron`; sealed 0.2.0 and the bank cells restored, the owner's choices;
    tests read the manifest's version instead of a literal; replays rebaselined), the split of `editor/mod.rs`
    and `editor/forms.rs` (`acts.rs`, `forms/fields.rs`), the documents (ARCHITECTURE §8.2,
    presentation-ARCHITECTURE §6.1–§6.4 and §8.6, PRD §10 and §13, README, code-map).
    **Review:** M5 closed, ten steps from 2026-10-21 to 2026-10-25, each accepted by the owner's hand test;
    the proof of the "done when" is the authoring test. Left as follow-ons: the activities schema below, the
    map's own fields with Resize and New map as forms, the icon design pass (the boundary tools: built
    2026-10-25, above).
- [x] **The editor's left column as a drawing GUI** (owner, 2026-10-25; **five steps built and accepted by 2026-10-31**, on accepting the boundary tools: the
  column "is functional, but not intuitive or easy on the eyes"). The owner's picture, as given: a **grid of
  icons for the pointer states**, as drawing programs have them: select (chooses, draws nothing), brush (draws
  the chosen palette entry: terrain, wall, encounter or object), move (a selected item, or the map when nothing
  is selected), lasso (area select), fill (fill a space with the palette entry), eraser (a brush that only
  deletes), undo (a history tool); a **grid for the palette** (terrains, walls, encounters, objects), with the
  chosen entry's details shown somewhere; and **under the two grids** Save, Revert, Playtest and the menu. This
  is the icon design pass (above) applied to the editor, and the frame the run tools and a terrain freehand
  belong in (a pointer state times a palette entry). Planned in plan mode, in steps with a hand test each.
  Planned 2026-10-25 (plan mode, approved); owner's choices: the view controls on the bar over the plan, the
  entry's details under the palette grid, the first step without Fill, the eraser erases what is under the
  pointer.
  - [x] Step 1 (2026-10-25, two commits, **accepted by the owner the same day**; `tasks/acceptance/editor-column-1.md`): the kit draws icons
    (`Kind::Icon { name, chosen }`, `Kind::Swatch { rgb, glyph, chosen }`, the chosen state in `Screen::shape`,
    `shell/icons.rs`'s image cache and `fill`, `screen_text` prints `[icon] "name"`, shades on the editor's
    buttons, editor buttons 120 wide at least); the column as the tool grid (`Tool::ALL`: Select, Brush, Run by
    corners, Freehand run, Eraser; Undo, Redo), the palette grid (`brush::Entry`: terrains as swatches, Wall,
    Door, Ramp, Height as icons), the entry's details, the files, Save/Revert/Playtest/Menu; the bar carries
    the view icons and the level choice, the inspector Add level and Level...; keys V, B, P, N, X; Select moves
    the cursor; the Eraser opens an edge, removes the first placement or clears the ground under the pointer; a
    run takes the entry's edge kind ("3 sides set to door").
  - [x] Step 2 (2026-10-25, three commits, **accepted by the owner the same day**; `tasks/acceptance/editor-column-2.md`): Fill (the region of one ground
    under the click, joined through open sides, every closed side a bound, one step; a terrain or the height) and
    Move (the placement under the press carried to the cell let go on, one step, refused where one of its kind
    already is; a press elsewhere pans the plan by whole cells, as the middle button does under any tool).
    Owner's choices in the planning: Move grabs what is under the pointer (no selection until the lasso), any
    closed side bounds a fill, the pan moves by whole cells. Later: a smooth (sub-cell) pan; a fill's region
    previewed on hover. **Open (owner, 2026-10-25, left for later with the entrances and exits work):** in the
    owner's window Move does not carry the start marker, while Move carries the services and Set start here
    works. `40863b52` made the press take the start first (`grab::Held::Start`); the headless move test and a
    test through Bevy's real UI focus path (`editor_pointer.rs`, the press on the start cell under the cursor)
    both pass, and the binary the owner ran was built after the fix. Not reproduced. Next: with the game left
    open after an attempt, read the undo depth and the message over the dev socket; ask whether the plan pans
    (the press misses the start: look at the pack and level the window has) or the cursor follows and nothing
    lands (the release fails).
  - [x] Step 3 (2026-10-25, one commit, **accepted with 3b on 2026-10-31**; `tasks/acceptance/editor-column-3.md`): placements as palette stamps.
    The palette ends in a stamp a placement kind (portal, encounter, activity, object, ground pile, lore), each
    with one detail chosen under the palette (the map, the monster, the service, the object kind, the item, the
    text key); a click with the brush places one at once through the kind's form, valid as it lands, refused
    where its kind already is, one undo step; a drag places once; the inspector's Add row is gone (Edit and
    Remove stay); a right click on a placement picks its stamp and detail. Owner's choices in the planning:
    places at once; one detail per stamp; the Add row goes; an occupied tile refuses. Found and fixed on the
    way: a placement edited under a new id left its old entry (now `[Remove(old), new]` in one step,
    `Place::key`; lesson in LESSONS). Later: a stamp's mark previewed under the pointer.
  - [x] Step 3b (2026-10-31, one commit, **accepted by the owner the same day**; `tasks/acceptance/editor-column-3b.md`): the stamp's detail as a modal
    picker, the owner's direction from step 3's hand test ("the left/right selection box is inadequate, the
    texts overflow"): under the palette the stamp's icon and Select... open a modal with a search field, every
    id of the kind, several chosen at once up to what one cell takes, Finish and Cancel; for encounters,
    objects and piles Finish opens a properties modal (a count a monster or an item, the disposition and stand,
    an object's place on the cell), reopened by a sliders icon; a click drops it all in one step. Owner's
    choices in the planning: the properties open after every Finish (one monster still wants its count);
    piles get them too. Not in the data model, so not offered: a per-placement loot (a monster's loot is its
    definition's table; a container's contents are). Later: a quantity for an object kind (several of one kind
    in a click); the fields' words read against the column's width everywhere.
  - [x] Step 4 (2026-10-31, one commit and a fix after the owner's hand test: the loop in progress previewed;
    **accepted by the owner the same day**; `tasks/acceptance/editor-column-4.md`): the lasso (a freehand loop, previewed as it is drawn,
    selects the cells inside it, outlined on the plan, counted under the tool's name; a click clears), Move
    carrying a selection pressed inside it by the drag's offset (everything on the cells: terrain, heights,
    sides, placements, the start; the source bare, the destination replaced, a portal, site or pile already
    there refusing; one undo step, `Document::batch`; the outline a ghost while carried), and runs with a
    terrain or the height as cell runs (the line's cells between the points painted with the entry). Owner's
    choices in the planning: a freehand loop; everything moves; the destination is replaced. Later: the
    eraser or Delete clearing a selection's cells; copy and paste of a selection; a selection across levels;
    Esc dropping a loop or a run (Esc is the menu's).
  - [x] Step 5 (2026-10-31, one commit, **accepted by the owner the same day**; `tasks/acceptance/editor-column-5.md`): Undo as a history list. Every
    undo step carries the words of what it did (`edit/words.rs`; a stroke counts its cells; the editor's own
    richer words replace them, `Document::name_step`); a History icon after Redo opens a modal over the editor
    listing the map as opened and every step oldest first, the current one marked, a row jumping there
    (`Document::goto`) with the modal up, Undo, Redo and Close. Owner's choices in the planning: a History icon
    beside Undo and Redo; a modal with the plan visible beneath. Before it, the owner's hand test of step 4 had
    saved onto `packs/test` (the depths): kept at the owner's word, replays re-baselined, own commit. Later:
    `editor.history` over the socket; a saved mark on the row the file holds.
- [x] **The entrances and exits enhancement** (owner, 2026-10-25; **four steps built and accepted by 2026-11-08**: "we are going to have to enhance the
  entrance/exits later anyway", back to the hand test of 2026-09-27: the town's way out and the dungeon's
  entrance invisible, "required validations on sub maps and maybe maps in general"; M7 3b made every portal
  visible, the model never moved). Scope fixed by the owner 2026-11-01, all four: loader rules on landings and
  starts; every map reached and every portal paired (the "no way-out rule" of 2026-09-27 withdrawn); two-way
  portals authored once in the editor; the start as a real entrance. The Move-the-start report of 2026-10-25 is
  closed ("works now"). Planned in plan mode 2026-11-01; owner's choices: **the strict mirror** (the way back
  stands directly behind the landing, the party arrives facing away from it), **reachability for the maps of the
  pack that sets `entry`** (fixture packs exempt), **the landing picked on the far map's plan**. Three steps, one
  commit each, `tasks/acceptance/entrances-N.md`:
  - [x] Step 1 (2026-11-01): the rules. `MapData::enterable`; a landing refused on no ground, on terrain the
    party cannot enter, on a portal; `way_back` (the portal directly behind the landing leading back) and "has no
    way back" for this map's portals and, through `check_inbound`, the other maps' portals landing here; a start
    refused off enterable ground; `unreached` (the entry pack's maps with a portal that the entry does not reach;
    a map with no portals is unfinished, the deadlock a New map would otherwise meet) in the loader and in
    `MapData::resolve`; the engine's landing and the save's `standable` read `enterable`. **The shipped pack
    moves** (0.2.4): the stairs into the dungeon and into the depths land facing east, away from the way back
    (two arrivals faced sideways to it); the replay scripts turn right after each stairs; replays re-baselined.
    Corpus `tests/packs-bad/bad-ways` (seven rows), `bad-objects` gains the way back its portal lacked. A portal
    stamped alone says "has no way back" until step 2 writes the pair (`editor_stamp`, `editor_pick` pin it).
    The owner's hand test (2026-11-01) found Save and Playtest swallowed by the error's words: both now dim with
    the count alone, the words stay the inspector's and the message's (fixed the same day, `tasks/LESSONS.md`).
  - [x] Step 2a (2026-11-01): **linked portals, the model** (re-planned on the owner's direction after the hand
    test: a pair could not be authored while each half needed the other on disk; owner's choices: a link stated
    by one side is enough, the landing derived from the partner, ids unique across the world). `Portal { id, link,
    facing, marker }`: a portal is linked when its link names a partner or one portal anywhere names it; going
    through lands the party ahead of the partner facing its way (the mirror by construction); the loader refuses a
    link to nothing, to itself, to a portal linking a third, a portal two portals claim, a landing ahead of the
    partner that cannot be stood on, an id shared across maps. An **unlinked** portal is legal: saved, drawn on the
    plan as the rectangle alone, a plain tile in play. Reachability over linked pairs, asked of a sound world. The
    editor: the portal form (id, link, facing, marker; a fresh one minted free of every map's ids and facing the
    author), the stamp (nothing chosen places an unlinked portal; a chosen unlinked portal's id links it; the
    choice clears after a linked placement), the inspector's "Portal 'id' to 'link'". `packs/test` 0.2.5 (ids and
    three one-sided links; landings unchanged), replays re-baselined; corpora `bad-ways` rewritten (ten link rows)
    and `bad-reach` new (reachability alone). The `editor.place` Portal payload changed shape: `docs/api.md`.
  - [x] Step 2b (2026-11-01): **linking in the editor.** `Picking { purpose, options, labels, .. }` built at
    open (`pick::options`: a portal's rows "id  -  Map (x, y)" over the portals nothing links to, the open map's
    unsaved ones among them; the search matches the label), `For::{Stamp, Field}`: the portal form's **Link...**
    opens the picker for its link field (the current partner offered and shown chosen) and Finish fills it,
    **Unlink** clears it; the inspector's line under a portal ("to Meadow 'meadow-gate'" whichever side states
    the link, an unresolved link, "unlinked"); a right-click copy of a portal drops its link.
    `tests/editor_link.rs` (the pair across maps through the picker, the one-sided save, the far map's error on
    a removal; the form's Link... and Unlink on one map). `tasks/acceptance/entrances-2.md`.
    **Found in the owner's hand test (2026-11-07), fixed:** the picker's Finish pressed with the mouse closed
    the modal and the plan took the same press beneath it (the stamp dropped its linked portal under the
    button, the owner's click placed an unlinked one); a press on the frame a screen closes is that screen's
    (`shell/editor.rs` `route`, `editor_link.rs` with a real mouse press). The owner's pack moved to 0.2.9
    during the test (the replays and pins follow in their own commit).
  - [x] Step 2c (2026-11-07, owner's direction after the leak fix: "an autocomplete function on the form";
    chosen "always listed while editing"): **the list under the portal form's link field** (`editor/links.rs`:
    every portal nothing links to and the current partner, narrowed by the field's text, the whole id marking
    its row; a row pressed fills the field, the marked one clears it; at most 10 rows); Link... and the form's
    picker gone (`pick.rs` the stamp's alone); `Form::set_text` + `refill` draw a model-set field anew (a link
    switched to another portal was undone by the window's frame before). `tests/editor_link.rs` (the list,
    the switch), `pick.rs` `fits`. **Steps 1, 2 and 2c accepted by the owner 2026-11-07** ("I think this is
    usable"; "That fixed it, accepted" after the two tools-column overflows the rename to `the_lower_kingdoms`
    exposed: the root on its own row, `d8f53d94`; the map chooser by the map's name, `74ad5f74`).
  - [x] Step 3 (2026-11-07): **the start as a real entrance.** `MapDef.start_level` (the ground when left out: a
    ground start's file is written as before) and `Place::Start { level, x, y, facing }`; the loader refuses a start
    on a level the map lacks or off that level's ground ("start (x, y) on level N is on no ground"; corpus
    `bad-ways` `up.ron`, `far.ron`); a new game and the save's fallback begin on the start's level; Set start here
    on the shown level, the Move tool and a moved selection carry it there, a map opens on its start's level, the
    plan's diamond notched toward the start's facing (`plan::start_pixels`); a level removed returns its start to
    the ground. `tasks/acceptance/entrances-3.md`; tests on the tower (`editor_start.rs`, `edit_document.rs`,
    `resolve.rs`, `omnis-sim/tests/levels.rs`, `save_check.rs`). **Accepted by the owner 2026-11-08** ("Step 3
    tested, accepted", on a scratch copy of the `levels` fixture's tower; no finding).
    **Review:** the enhancement closed, four steps from 2026-11-01 to 2026-11-08, each accepted by the owner's
    hand test; two findings from those tests fixed in their own commits (the error words over Save and
    Playtest, step 1; the picker's Finish leaking a press to the plan, step 2b) and one direction taken as a
    step (2c, the link list under the form). What the loader now holds: every landing on enterable ground with
    its way back behind it, every portal paired or openly unlinked, every map of the entry's pack reached, the
    start on a level of its map. Left as follow-ons: a start mark in the 3D preview (the icon pass); `omnis-cli`
    and `editor_new` still start a new map at (0, 0) on the ground; Playtest from the start rather than the
    cursor if the owner wants it.
- [x] Ops for pack definitions and text labels (owner, 2026-10-08): a `data.*` op family so a JSON client, an
  agent authoring content among them, turns ids and text keys into names and definitions; the main known gap in
  `docs/api.md` §14; additive, protocol stays 2. With the editor's ops, since both read the same registry.
  **Planned 2026-11-11** (plan mode, approved; the owner's direction: "forward thinking flexibility above all
  else, TTRPGs are filled with creative solutions and we want to capture some of that magic"). Owner's choices:
  a stable envelope around `def`, the definition as the pack wrote it (`def` follows `omnis_data::SCHEMA`, not
  `PROTOCOL`); the author's tier, everything loaded, a per-party gate a requirement for D30; generic filters in
  the first version (kind, prefix, a substring, `refers_to`); provenance recorded now (`pack`, `replaces`).
  Four ops: `data.kinds`, `data.list`, `data.get`, `data.text` (ops 44 to 48, tools 48, `Reply` 31, `OpError`
  12 with `UnknownId`). Rules files are not a kind (`rules.*` serves them).
  - [x] **Step 1, the catalogue in `omnis-data`** (built 2026-11-11; first as a walk over a
    definition's compact RON, the serializer as planned since the owner's choice the same day): `strings.rs` (a serializer that collects every string of a
    definition, so references and text keys are found with no code per kind), `catalog.rs` (the one table of
    kinds, `Definition`, `links`, `referrers`), `Data.sources` from the loader. Tests on the frozen fixture,
    the `objects` pack and a scratch pack that overrides a base item.
  - [x] **Step 2, the ops**: `omnis-sim/src/data_ops.rs`, the four ops in `dispatch`, every count pin, the MCP
    tools and schemas, the CLI's schema dump, the vocabulary test (the envelope checked, `def` skipped by name).
  - [x] **Step 3, the socket and the documents**: `data.*` answers at the title and in the editor (from the
    game's data when one runs, else the packs loaded at start); `docs/api.md`, ARCHITECTURE §4.9 and §9.3,
    `horizons.md` (the knowledge gate), `tasks/acceptance/data-ops.md`.
  **Review (2026-11-11; accepted by the owner the same day, "It seems to work well")**: four ops over one table of 14 kinds; `cursor` for
  the page; `omnis-cli op <name> [<args>]` added so the hand test needs no client; the gate's hole found and
  closed on the way (`scripts/test-summary.sh`). Departures and what is left: `tasks/acceptance/data-ops.md`.
- [x] Rule priority and conflict tracking (owner, 2026-11-11: "We do need to start tracking rule conflicts and
  consider all rules have a required priority value that is used to determine rule precedence"; plan mode,
  approved the same day). Found by the `data.*` survey: rules files merged in file-id order, the last
  definition of a name winning silently, with nothing recording where a rule came from. Owner's choices: a
  required `priority` on every slot, value and table (the higher number wins; no file-level priority); two
  definitions of one name at one priority are always a load error; an override declares the inputs of the rule
  it replaces, else a load error; shown in `rules.list`/`rules.get`, `omnis-cli validate`, `data.*` (rules
  files as a kind) and an editor rules view. `SCHEMA` stays 1 (nothing released; every rules file changes in
  the same commit). Additive on the wire, protocol stays 2.
  - [x] **Step 1, the format and the merge** (`omnis-data`; built 2026-11-11): entries with `priority` in `rules.rs`; the merge
    there in place of `content.rs` `build_rules` (highest priority stands; a tie, an input mismatch and a
    losing formula that does not compile are errors); `Standings` and the raw files kept on `Data`; every
    rules file rewritten (base 10, `objects` 1, the bad fixtures 3), base's version raised and resealed;
    ARCHITECTURE §5.
  - [x] **Step 2, showing it over the API and the CLI** (built 2026-11-11; the rules ops moved to
    `omnis-sim/src/rules_ops.rs`): `rules` the 15th row of `catalog::KINDS`; `origins`
    on `rules.list` and `origin` on `rules.get`; the override lines of `omnis-cli validate`; pins and
    `docs/api.md`.
  - [x] **Step 3, the editor rules view** (`omnis-vector`; built 2026-11-11): a modal list of every slot, value and table with
    its file, pack and priority, an overridden rule marked with what it beat, Open file into the text editor.
  - [x] `tasks/acceptance/rule-priority.md`, the review line here, `tasks/CONTINUITY.md`.
  **Review (2026-11-11; accepted by the owner the same day, "manually tested. Looks like it is working")**: the merge is one function and the only place a
  rule set is made; nothing about file ids or load order decides between files any more. Departures: the
  golden replays rebaselined (base resealed at 0.1.1; the compiled rules unchanged), the editor's mark a word
  and not an icon, a rule's file opened only under its id's name. Not built: the topological pack order of
  ARCHITECTURE §6.1. `tasks/acceptance/rule-priority.md`.
- [x] A dev command to pass time (owner, 2026-10-08; built 2026-11-11 at the owner's word): `DevCommand::Pass { minutes }` lives the minutes at the
  region's company, bounded, refused in a fight; a debug-panel field and the MCP schema (`dev_schema` 15).
  Replaces the 720 short rests in `tasks/acceptance/m8.md`.
  **Review (2026-11-11; accepted by the owner 2026-11-12, "Manually confirmed")**: one arm in `dev.rs` over `apply::advance`, the function a
  step and a rest spend minutes through, so the age, the shared time, the date and timed effects move as they
  do in play; no food, no healing, no encounter. One to `DevCommand::MAX_PASS` (518,400, a year of 360 days),
  else `OutOfRange`; `WrongMode` in a fight or an encounter. The Debug World page has a Minutes field (60
  until typed over, kept after a press) and Pass, lit by the engine's trial. `DevCommand` 15, the schema proof
  118 instances and 169 offered. Mutations caught: the mode check, each bound, the age not lived, the panel's
  number. To try: `just run --windowed --pack packs/base --pack packs/the_lower_kingdoms`, New game with
  devtools on, Esc, Debug, World, type 1440, Pass: the HUD's day moves on by one.
- [ ] A bridge probe in the repo: `scripts/mcp-probe.py` drives `target/debug/omnis-mcp` over stdio so a session
  whose MCP client did not launch the bridge still runs the MCP acceptance steps. When needed.
- [x] Windows CI at W1's close (D31; built 2026-11-08, **advisory** at the owner's word, as the Linux job): a
  `windows-latest` job with the same steps under Git Bash, `workflow_dispatch` so a branch's run can be read
  before its PR, the `windows_subsystem` line on the client with a failed start's reason kept in
  `.omnis/last-error.txt` (owner's choices: the attribute now; the reason in a file, no unsafe). A sweep of the
  tree before the build found two faults that would have failed the job before any Windows code ran, both
  fixed: git on Windows checks out CRLF and the seals, the fingerprints, the frozen world folders and
  `duplicates.allow` are hashed or compared as text (`.gitattributes`, `* text=auto eol=lf`; shown on a
  scratch clone: both packs "changed since sealed"); and ron's pretty serializer ends lines with CRLF on
  Windows, so a seal, a world id or a fingerprint computed there differed from macOS's
  (`ron_io::to_string` `.new_line("\n")`, own commit). The Bevy-free crates type-checked for
  `x86_64-pc-windows-msvc` from the Mac (the client needs MSVC's assembler for `blake3`, the runner's); the
  first run on GitHub is the Bevy build's proof (`tasks/acceptance/windows-ci.md`). Whether it blocks
  is the owner's call later. Captures stay on the owner's machine until the look settles at W2. **First run
  2026-11-08**: green through the toolchain, fmt, the scripts and clippy; `cargo test` stopped in `omnis-data`'s
  integration tests at one fault, the editor's loader words naming a map's file with a backslash
  (`MapData::resolve` built the name with `Path::join`; now it takes the file as the session names it, with a
  test under a file named otherwise than its id). The crates after `omnis-data`, the replays and the validation
  have not run on Windows yet: the second run reads them. **Second run green, accepted by the owner 2026-11-10**
  ("Windows CI now passes"): the client builds on Windows, every crate's tests, both replays' fingerprints and
  the validation; `tasks/acceptance/windows-ci.md`. **Review:** three Windows faults in all, none in the job's
  own YAML; the sweep found two before a run, a pattern (a name joined with the platform's separator) found the
  third, now a lesson. Advisory stays until the owner says otherwise.

## W2 — The look, and the engine under it (presentation-PRD §11 steps 3 and 5; no PRD §13 phase)
Done when a region's biosphere sets what the player sees from pack data, the first authored set piece stands in a
map, mana is in the PRD and in the packs, and the performance floor is read against the finished geometry (X20).
- [ ] Bevy 0.20 (`tasks/plans/bevy-0.20-migration.md`, re-audited and approved 2026-11-12; `=0.20.0`, 0.20.1
  being under 30 days old until 2026-11-27). First, so shaders are written once. One commit a step:
  - [x] 0. Housekeeping: the plan rewritten on today's tree, baseline captures on 0.19.1 in the scratchpad.
  - [x] 1. The engine moves, behaviour held (2026-11-12): `=0.20.0` with every `bevy_*` crate held at 0.20.0
    in the lock and six young crates held back (161 moved crates, none published after 2026-10-13);
    `render::glow()` (HDR, bloom, `Tonemapping::Linear`) on the three cameras; `TextInput` on the text
    fields; `screen_draw::keep_keyboard` (Escape no longer takes the keyboard from a field); `controls::
    pressable()` on Bevy's new `Button` (the old one is private in 0.20), `Interaction` allowed until step 2.
    Review: gate 1258. Found: the title and new-game captures are identical to 0.19's; the view's differ in
    14% of channel values, nearly all by two levels or fewer (read as dither, which `None` drew without;
    not yet confirmed by eye); the `walk` capture varies run to run and is no baseline; `check-duplicates.sh` dies
    silently when a new duplicate appears (the loop's `head` under `pipefail`), to fix with step 3.
  - [x] 2. Buttons on `Hovered` and `Pressed` (2026-11-12): `controls::Control` (a live control; `pressable()`,
    `Pressing`, `Over`/`over_control`) with its own left-button observers keeping `Pressed`. Not Bevy's
    `ui_widgets::Button`, a departure from the plan found at the build: that one is pressed by any mouse
    button, and the right button here is the action key and the editor's pick. A press still acts on button
    down. The tests' ten hand-written helpers are one `set`/`let_go`/`rest` over `Pointing`. Review: gate
    1261; ten mutation checks caught (three survived first and got tests: the shade on release, a click on
    a button taken by the view, a press read while held); a press through Bevy's real picking is tested
    in the editor harness (`editor_pointer.rs`; picking reads `WindowEvent`, not the plain messages).
    Not covered headless: a held movement button through real picking (the owner's hand test).
  - [x] 3. Runtime checks against the captures, the documents that name 0.19, `tasks/acceptance/bevy-0.20.md`
    (2026-11-12). Review: the title and new-game captures are byte-identical to 0.19.1's; the 3D view's are
    not an exact baseline (it eases by real time; BACKLOG). The window checks are the owner's script: a window
    from the agent's shell draws no frames. `check-duplicates.sh` now names a new duplicate before it exits.
  - [x] The owner's first hand test (2026-11-12): "I lost control of the left click button on the mouse". A
    button that opens a screen over itself stayed `Pressed` (the pointer's release reaches only what is under
    it) and took every click from the view. Fixed: `controls::button_up` lets every control go when the left
    button comes up; `column.rs` has the test; `tasks/LESSONS.md`. Also reported: a ground pile placed in the
    editor "did not show up upon searching". Search finds hidden objects, not piles (a pile is drawn on its
    cell and picked up from the Inventory screen); asked of the owner, nothing changed.
  **Hand-tested by the owner 2026-11-12**: "the upgrade went pretty smoothly"; "the left click loss is fixed".
  Not yet said: the departure (`controls::Control`, not Bevy's `ui_widgets::Button`, which any mouse button
  presses), and the word "accepted".
- [ ] **From the hand test of 2026-11-12: piles you can find and take, and a flat ceiling over a dip**
  (`tasks/plans/loot-window-and-ceiling.md`, approved 2026-11-12). Owner's choices: the click on a pile opens a
  loot window, reusable "for various situations where items are available for the party to take, store, or
  destroy"; a pile is in reach as objects are (own cell and the faced cell across an open edge); the ceiling is
  flat over dips only. One commit a step:
  - [x] 1. The ceiling (2026-11-12): `Ground::roof` (`max(floor, level base) + wall height`) in the lines, the
    fill, a block's top and a wall's top; BACKLOG "ceiling height controls" and "hidden piles". Review: one
    test on the town with a dip and a rise made in the test (no fixture has a dip), four mutations caught.
    Not seen in a window by me: the owner's dungeon cell (5, 22).
  - [x] 2. Reach (2026-11-12): `PickUp` and `ObjectsView.ground` over the party's cell and the faced cell
    (`ground::in_reach`, over `objects::reach`); Search's "On the ground" line and the Inventory's rows follow
    with no client change. `ground` is one entry an item across both piles; `PickUp` takes underfoot first,
    the faced pile making up the rest, one `ItemMoved`. No wire shape moved. Review: three sim tests on the
    yard and one client Search test; three mutations caught (faced cell dropped, edge check dropped, order
    reversed).
  - [x] 3. The loot window (2026-11-12; `screen/loot.rs`, `ScreenId::Loot`, opened on an `ItemPlace`: Take, Take
    all, Take everything, who receives), a left click on a pile in reach (`aim.rs`, `shell/aim.rs`, the hand
    cursor), "Pick up..." in the Actions menu and as the default action. Review: eight tests in
    `tests/loot.rs`, the projection checked against the camera's own rotation; twelve mutations caught.
    Tier 1 gained `piles_in_reach`. **Open**: an open container has no way into the window yet (the window
    shows one when opened on it, and the Inventory still lists it); ask the owner where it opens from (my
    recommendation: a "Take from..." under an open container on the verbs screen, and a click on it once
    objects are clickable). Not seen in a window by me.
  - [x] From the owner's try of steps 2 and 3 (2026-11-12: "now I can get to the loot pile"), two reports,
    both answered by the owner the same day ("Picker then click; do both changes and the quiet period at 8"):
    - [x] The pile held one arrow whatever loot was chosen: the first click dropped the packs' first item and
      the second click, with the loot chosen, was refused. A pile stamped on a cell that has one now takes its
      place (one undo step, "replaced at"), and the inspector says what a pile holds ("Ground pile (dagger x2,
      longsword)"). Test `editor_pick::a_pile_stamped_on_a_pile_takes_its_place_...`.
    - [x] Random encounters felt far above 3%: the roll is right (150,000 probe steps: 2.95%, 2.8% straight
      after one), independent rolls cluster. The quiet period: rules value `encounter_quiet_steps` (base 8),
      `World::quiet`, no roll and no draw for that many steps after any encounter begins. Base resealed at
      0.1.2, the replays rebaselined. Tests `encounter::after_an_encounter_the_table_is_quiet_...` and
      `encounter::a_placed_group_starts_the_quiet_...`.
  - [x] 4. Store and destroy (2026-11-12): `ItemCommand::Stash` and `ItemCommand::Destroy` (`from: ItemPlace`),
    `Event::ItemDestroyed` (74 events), every pin; the two buttons, Destroy asked first. Added beyond the
    plan's words: `Rejection::NotFromThere` (72) for the "any other place is refused" the plan asks, since no
    existing refusal says it. Neither command names a member, so both stay lit when nobody can carry.
    `Item` has no "kept" mark (no quest items yet), so Destroy is never dimmed for one. Sim `tests/stash.rs`
    (4 tests), client `tests/loot.rs` (3 new).
  - [x] `tasks/acceptance/loot-and-ceiling.md` (2026-11-12).
  - Review (2026-11-12): the plan's four steps are built (`ead8e8c0`, `ee803d45`, `b6492d3b`, `e3467963`) with the
    two changes from the owner's first try (`175621bd`). Gate 1262 to 1285. One addition beyond the plan's words
    (`Rejection::NotFromThere`). The owner, after a try the same day: "tested piles now store and release chosen items. I think
    we can move on." (Store and the takes named; the ceiling, Destroy, the pile stamp and the quiet not named.)
    Open: an open
    container's way into the window; Store and Destroy act on a whole row (no count asked).
- [x] The rendering direction, size 1 built 2026-11-12, awaiting the owner's try (BACKLOG, 2026-10-20, "GPU-first"). **The owner chose 2026-11-12: size 1 with size 2
  designed in** ("sounds correct"; the direction is to lean on Bevy more: its meshes and `Material`, not its
  debug gizmos). Plan mode next. Before the choice: the owner chooses a size; the recommendation
  is size 1 (retained line meshes with an own WGSL material) straight after 0.20, with size 2 designed in the
  same presentation-PRD amendment so width, fog and relief are in the shader from the start. A plan under
  `tasks/plans/` and the amendment text come with the choice.
  - Plan approved 2026-11-12: `tasks/plans/rendering-size-1.md`. The owner also chose to keep the line width
    now (each segment a quad widened in the shader), since a plain line list is 1 pixel wide.
  - [x] Baseline: not taken apart; `--lines gizmo|mesh` gives both renderers from one binary, which is the
    better comparison. Timing waits for step 4 (release build).
  - [x] 0. Spike (2026-11-12): the embedded WESL loaded and drew on this machine (Metal) at the first try; it
    was kept as step 2's shader, not thrown away.
  - [x] 1. `strokes.rs` (2026-11-12): the vertex buffers, `colour` and `CREST` moved from `render.rs`.
  - [x] 2. `shell/lines.rs`, `lines.wesl`, the game view's line entity beside the gizmos (`--lines gizmo|mesh`,
    default gizmo); capture pairs compared (2026-11-12, one commit with step 1). Pixels differing by more than
    8 of 255, gizmo against mesh, standing: town 0.33% (1600x900) and 0.24% (1920x1080), hills 0.05%, tower
    0.27%, yard 0.34% and 0.11% (1280x720); the differences are the frame-rate text and lines that now fade
    along their length. **Two departures from the plan's detail**, for the same look: one line mesh, not
    structure and grid apart (of two lines in one place the gizmos showed the first drawn, so the mesh keeps
    the scene's order and the grid setting rebuilds it; `Scene::structure`/`grid` were not needed); and the
    material writes depth and tests `Greater` in `specialize`, as the gizmo pipeline does, so a near line is
    not painted over by a far one. Tests `tests/strokes.rs` (3), `tests/lines.rs` (3); nine mutations caught.
    Captures by `<scratchpad>/pairs.sh` and `pixdiff.py`; the hills, tower and yard through scratch copies of
    the fixture packs with an `entry`.
  - [x] 3. Crests and the editor preview through the same path (2026-11-12). Crests are a second mesh a
    view, found again only when the eye has moved or the scene changed, replaced in place
    (`lines::hang_crests`, `CrestsAt`). The preview has its own line and crest entities on layer 2 and its own
    material (sight 8). The quality's width reaches every `LineMaterial` in `apply_quality`. Line entities
    carry `NoFrustumCulling` (a quad's four vertices all sit on one end, so the mesh's bounds are not the
    lines'). **Added for verification**: `--editor <map> --screenshot <png>` now captures the editor (it was
    refused), so the preview can be seen offscreen. Captures, gizmo against mesh: three hill viewpoints with
    crests 0.05% to 0.10%; the editor with its preview 0 pixels (and a shader tinted red for one run changed
    only the preview's rectangle, so the picture is the new shader's). Tests: `lines::the_crests_are_found_...`,
    `editor_preview::the_preview_s_lines_are_meshes_...`, the flag in `editor::the_command_line_...`; nine
    mutations, eight caught at once and the ninth after the test's expectation stopped using the code under
    test.
  - [x] 4. The swap (2026-11-12): the gizmo loops of the view and the preview, `LineRenderer` and `--lines`
    are gone; `RenderPlugin` adds `LinesPlugin`. Timing before the swap, both renderers in one release binary
    (300-frame walk, three runs, town and hills, 1920x1080 and 5120x1440): gizmo 1.95 to 2.10 ms mean, mesh
    1.97 to 2.15 ms mean. No difference; the move is for capability. Final captures against the old renderer
    with the eye's easing off: 0.02% to 0.10% of pixels (the frame-rate text, lines fading along their
    length); the preview identical. A finding on the way: the eye eases by real time, so two builds differ in
    0.3% to 0.9% of pixels from that alone; comparisons now use `reduced_motion` (`verification.md`). Docs:
    presentation-ARCHITECTURE §8.2, §8.6, VA1, VA4, Q3; presentation-PRD §11 Built; BACKLOG; code-map. The
    fight screen's 2D lines stay gizmos. `fade` (the CPU fade) went with its last caller; the shader's comment
    carries the formula.
  - [x] 5. Size 2's design (2026-11-12): `tasks/plans/rendering-size-2.md`, six parts in a build order, with
    the proposed presentation-PRD amendment at its end (not applied: the owner's to approve).
  - [x] `tasks/acceptance/rendering-size-1.md` (2026-11-12).
  - Review (2026-11-12): built as planned in commits `536efc5a` (steps 0 to 2), `81e127e8` (3), `23459e72`
    (4). Gate 1285 to 1293. Would a staff engineer approve: the shader is checked only by captures on one
    machine (Metal); no window, no hand test, no other GPU. The owner, after a try in the window the same
    day: "Rendering switch seems to hold up, might even be significantly smoother." (The offscreen timing
    showed no difference; the window's frame pacing was not measured, so "smoother" is the owner's
    observation, unexplained.) Open: size 2's decision.
- [x] Rendering size 2, part 1: edge feathering (2026-11-12; the owner: "edge feathering first makes sense.
  Please go ahead."). `lines.wesl` only: the quad is half a pixel wider each side and the alpha is the line's
  cover of the pixel across its width. Checked by capture (the gate cannot see it): zoomed crops of the yard
  at Low and High, before and after: stair-stepped and broken-looking slanted lines are smooth; no notch
  where a near line crosses a far one; about 1.0% of pixels change at 1600x900 (line edges). Not seen in a
  window by me. **Fixed on the way** (a defect of size 1's step 3, found in the capture's log): a view with
  no crests hung an empty mesh, which Bevy warns about each time it is uploaded; now the crest entity has no
  mesh while there are none (`hang_crests`; the two tests say so; two mutations caught). Gate 1293.
  Parts 2 to 6 of `tasks/plans/rendering-size-2.md` are not started and not approved.
- [x] Rendering size 2, part 2: line patterns and weight (plan `tasks/plans/rendering-size-2-patterns.md`,
  approved 2026-11-12; the owner chose dashes measured in the world, and a provisional table from
  presentation-PRD section 5.3). Built 2026-11-13; not yet tried by the owner.
  - [x] Step 0: the plan filed; baseline captures.
  - [x] Step 1: `style::Pattern` and `Weight`, `strokes::channels` by `SegKind`, written into the mesh's style
    (`f8401a19`; six mutations caught).
  - [x] Step 2: dashed, dotted and double in `lines.wesl`, with the far filter; captures and timing.
  - [x] Step 3: documents, the acceptance sheet, continuity.
  - Review. The planned dash (1/4 cell, open gaps) broke small shapes, the risk the plan named; I stopped and
    asked. The owner refused three dash lengths, then asked for a lighter line between the dashes in place
    of a void, chose 1/8 cell and asked that it read at Low. Built so: between dashes a third of the width
    and 0.4 of the brightness (the dimming is what reads at Low and on glowing colours). The period is
    fitted to each segment so corners are drawn. Checked by zoomed captures at Low and High: yard (objects,
    piles dashed; the rat's two rails at High), a portal's dotted mark (a scratch copy of the fixture turned
    to face it). Double shows only on lines 3 pixels wide, so at High only; at Low and Medium a being is told
    by weight and colour. Not checked: dashes while walking (stills only), the editor's preview, the far
    filter by eye beyond the yard's depth. Timing 2.35 to 2.39 ms (dev profile, yard), not comparable with
    size 1's 2.0 ms (release, town and hills); no like-for-like before and after was measured. Would a staff
    engineer approve: the shader's constants have no test (the gate has no GPU), and the first gate run was
    red from my own mutation script leaving a mutant compiled (lesson written).
    Also built, at the owner's request: `--entry <map-id>` and `just qa [map]` (the fixtures' maps in a
    window; a test starts a game on the meadow and refuses an unknown map).
- [x] **A ground pile that is easy to get to** (built 2026-11-13, awaiting the owner's try) (`tasks/plans/pile-reach.md`, approved 2026-11-13). From the
  owner's hand test the same day: the click target was too small, and a pile underfoot was unseen.
  - [x] Step 0: the plan filed.
  - [x] Step 1: the floor of a pile's cell is its click target (`aim::floor`, `aim::inside`, `aim::on_heap`;
    `tests/pile_reach.rs`, four tests; five mutations caught).
  - [x] Step 2: the floor's outline lights under the pointer (`Aimed` as the cell, `render::ViewAim`,
    `lines::aim_mark`, `lines::hang` shared with the crests; one test, five mutations caught). Not seen in a
    window by me: no capture has a pointer.
  - [x] Step 3: a "Pick up..." button on the view while anything on the ground is in reach (`shell/aim.rs`
    `PickUp`, `offer`, `pick_up`; one test, seven mutations caught). Seen in a capture standing on the pile at
    1280 by 720: over the bottom rows, clear of both groups.
  - [x] Step 4: documents, `tasks/acceptance/pile-reach.md`, continuity.
  - Review (2026-11-13). Built as planned (`e404b395`, `d5bbd8b3`, and the two commits after). Gate 1295 to
    1301. The button stands 102 pixels up, above both bottom rows, so it does not depend on the window's
    width. Not seen in a window by me: the hand and the outline under a real pointer (no capture has one).
    The "not exploring" case is tested by moving the view's mode alone, since no fixture has a pile where a
    fight starts. Facing a pile, the button lies over part of the floor target; both do the same thing.
    Would a staff engineer approve: the tests were written after the code in steps 1 to 3 and proved by
    mutation (seventeen caught, none survived), not red first as the plan said.
- [x] **Found items land on the ground and the loot window opens** (built 2026-11-13, awaiting the owner's try) (`tasks/plans/found-items.md`, approved
  2026-11-13). From the owner's try of the pile reach: the stone that hides a gem opened no loot picker; the
  owner chose "Drop it, open the loot window".
  - [x] Step 0: the plan filed.
  - [x] Step 1: the engine rule. `GiveItem` lays the item on the object's cell when the party reaches it,
    else on the party's (`ground::found`, `ground::found_at`), as `ItemMoved` from the object to the ground;
    `ItemFound` is no longer produced. Tests red first (`object_act`, two new), six mutations caught. Two
    older tests moved to the new rule (`objects_view`, vector `object_lines`); the log says "Found gem; on
    the ground". Not tested: a found item on a pile at `MAX_PILE_STACKS` (the packs have 24 items, a full
    pile needs 32 kinds); the code has no limit check to mutate.
  - [x] Step 2: the loot window opens on what was found (`Session.found`, `session::finds`, `shell/aim.rs`
    `found`; `tests/found.rs`, three tests red first, five mutations caught).
  - [x] Step 3: documents, `tasks/acceptance/found-items.md`, the lesson, continuity.
  - Review (2026-11-13). Built as planned, with these to say. The yard's stone has one verb, so the Actions
    menu's "Use" takes it directly and the loot window opens in the menu's place; **the window opening over
    the verbs screen is not tested**, since no fixture object with several verbs gives an item (`Nav` stacks
    overlays, so it should). A full pile is not tested (above). The object's own message ("Under the stone
    lies a gem.") is not carried into the loot window: `Forms::shown` clears a screen's message when the
    top screen changes, so it is read in the log and as the notice once the window closes. Not seen in a
    window by me. `ARCHITECTURE.md` and `docs/api.md` changed under the approved plan.
- [x] **The loot window opens on fight spoils and on an opened container** (built 2026-11-13, awaiting the owner's try) (`tasks/plans/loot-everywhere.md`,
  approved 2026-11-13; client only). The plan lists every way the party gains items or gold.
  - [x] Step 0: the plan filed.
  - [x] Step 1: the move. `shell/loot.rs` (`open(.., from)`, the find's opener, the "Pick up..." button,
    `action_keys` from `shell/screen.rs`, now 956 lines); `Session.found` became `Session.offer:
    Option<ItemPlace>`. No behaviour change; the tests pass with two import paths moved.
  - [x] Step 2: fight spoils. `session::finds` answers for a victory that left items; the opener waits out
    the fight screen's last frame. Tests red first in `tests/found.rs` (the yard's pack rats fought and won
    with the fight's own commands; a won fight that left nothing; `finds` over built events); five
    mutations caught. A flight is covered by the `finds` test only, not by a fought one.
  - [x] Step 3: containers. `session::opens` (an `ObjectState` whose object now shows contents),
    `Does::Loot(ItemPlace)`, `Effect::Loot`, Actions' "Take from <name>...". Tests red first (three in
    `tests/found.rs`). Nine mutations: seven caught at once; two survived (any new state, or an empty
    chest, making the offer) because the window closes itself on a place with nothing to show, so a direct
    test of `opens` was added and both are caught.
  - [x] Step 4: documents, `tasks/acceptance/loot-everywhere.md`, continuity.
  - Review (2026-11-13). Built as planned, client only; gate 1306 to 1312. Not tested: a command that both
    opens a container and gives an item (the find should win; no fixture does both); a flight, beyond the
    `finds` unit test. Not changed and named for the owner: gold goes straight to the party; a dead
    member's kit goes to the stores unseen at a fight's end. Not seen in a window by me.
- [ ] **Rendering size 2, part 3: colour roles as a palette, with biospheres in pack data** (M8.5 step 5's
  colour half; `tasks/plans/rendering-size-2-palette.md`, approved 2026-11-13; presentation-PRD X23 applied).
  The owner's answers: nine roles (structure, ground, passage, thing, party, hostile, magic, danger,
  interface); a region names its biosphere, a map may override; a terrain names its role; the 3D view and
  the editor preview now, the fight screen, minimap and HUD in a following plan before part 4.
  - [x] Step 0: the plan filed; X23 and the additions to X10, X18 and X19 applied; §12.2 and §12.3 answered.
  - [x] Step 1: data. `omnis-data/src/biosphere.rs` (`ColourRole`, `Ink`, `BiosphereDef`), the `biosphere`
    kind held by id as rules files are (no new registry table, so no save shape moves), `Terrain.role`,
    `biosphere` on a region and a map, `Data::biosphere_of`, the catalogue (16 kinds), the CLI schema,
    `docs/api.md`. Fixtures: `objects` 0.1.3 (ember, named by the yard's region; the loft floor's role),
    `levels` 0.1.1 (frost on the region, deep on the tower; the hole's role). Gate 1312 to 1319; nine
    mutations caught. Brought forward from step 5: the terrain form's role field, because a form without
    it would have erased a terrain's role on every edit.
  - [x] Step 2: roles on the CPU. `style::Palette` (sixteen slots, a built-in default, `Palette::of` a
    biosphere), `strokes::{Look, look, role, strength}`, `Segment.role` (the terrain's named role, where
    `rgb` was), the role's number in `style.z`. **The look changed here**: the view is drawn in the default
    palette, so floors no longer take their terrain's colour, cliffs are the walls' green, piles the
    objects' violet. Until step 3 the default's ink is laid into the vertex. Gate 1319 to 1321; nine
    mutations caught.
  - [x] Step 3: the palette on Bevy's line material (`LineMaterial.palette`, sixteen four-float slots at
    binding 1; `lines.wesl` lays the role's ink on the vertex's strength). The view's follows the party's
    map (`render::tune`), the preview's the edited map (`preview::paint`, `Data::biosphere_for`); the mesh
    holds no ink. Seen in captures at Low and High (`target/shots/p3/`): the town in the default, the yard
    in ember, the tower in deep with its hole's floor in danger red. The fixture ember's ground was
    brightened after the first Low capture (objects 0.1.4). Gate 1321 to 1323; seven mutations caught.
    Not seen in a window by me.
  - [x] Step 4: `contrast.rs` (`clashes`, `distance`, `Vision`, `DRAWN`, `APART` 0.1, `FROM_DARK` 0.45):
    each of the seven roles the world's lines carry against every other and every role against the dark,
    in Oklab, in full colour and under simulated protanopia, deuteranopia and tritanopia. **Measured first:
    the old hues failed** (walls' green against doors' amber 0.048 for the green-blind; orange beings
    against red holes 0.052 for the blue-blind). The default was moved the least that passes, by search:
    walls lean to teal, doors to yellow, a hole's red is purer. Ember and deep were tuned the same way
    (objects 0.1.5, levels 0.1.2); frost is left failing on purpose as the test's bad palette. The party's
    and the interface's roles are held off the dark only, since they draw no line in the world yet.
    Gate 1323 to 1326; seven mutations caught. The checks run in tests on the default and the fixtures;
    nothing yet tells an author about a pack's own palette (asked at the end).
  - [x] Step 5: documents (presentation-ARCHITECTURE 7 and 8.2, `rendering-size-2.md`, code-map),
    `tasks/acceptance/rendering-size-2-palette.md`, continuity. The terrain form's role was built in step 1.
  - Review (2026-11-13). Built as planned in five commits, gate 1312 to 1326, 32 mutations caught and one
    that could not compile. Departures, all said in their steps: the terrain form's role moved from step 5
    to step 1; biospheres are held by id, not interned (no save shape moves); the override fixture is in
    `levels`, as the plan allowed; `Data::biosphere_for` was added for the editor's unsaved map; the default
    palette's hues moved because the old ones failed the checks. Not built, and open: the fight screen,
    minimap, picture window and interface on the palette (the next plan); a report of a pack's own
    palette to its author; a preset in Settings; the channel check of 7.3. Consequences the owner should
    judge by eye: floors no longer differ by terrain; cliffs are the walls' colour; piles are the objects'
    colour; the walls' green differs from the HUD's. Not seen in a window by me.
- [ ] **A terrain's block face, the editor form's layout, and tooltips** (owner's try of part 3, 2026-11-13;
  `tasks/plans/editor-forms-and-tips.md`, approved in plan mode the same day). The meadow has no cliffs: the
  owner saw the trees terrain's blocks, which part 3 made the walls' colour. The form in the editor's right
  column is unclear and has no tooltips.
  - [x] Step 0: this plan, the lesson (a look change's report names every kind it merges, blocks included).
  - [x] Step 1: `Terrain.face` (`Face::{Plain, Hatched}`, left out of a file when plain, so no seal moves):
    a hatched block adds three verticals a side at the cliff's quarter spacing (`geometry::hatched`), its box
    whole; the terrain form's `face` choice; the fixture `heights` (0.1.2) gains a hatched boulder at (8, 4).
    Gate 1326 to 1329; five mutations caught. Seen in captures at Low and High from a scratch copy of the
    fixture started beside the boulder (`target/shots/face/`). The owner's trees stay plain until the owner
    sets their face.
  - [x] Step 2: `Row::field` (`Row.aligned`): the field's name dim in a labels' column of one width, its
    input in its own box filling the rest of the row; `Kind::Flag`, a tick box one press turns; a form's
    fields under group headings (`editor/tips.rs::section`), the terrain form reordered to fit them
    (identity, tiles, movement, look, map colour). A list's row is its fields one under another, then
    "Remove row" (the plan said a label before each input on one line; the column is too narrow for two
    inputs side by side). The properties modal's own fields use the same rows. `screen_draw.rs` passed
    1000 lines, so the arrows, slider and tick box moved to `shell/screen_parts.rs`. Tests on a laid-out
    window (`ui_editor_app`; the plain headless app has no layout, every rectangle is 0). Gate 1329 to
    1332; five mutations caught. Seen in one capture through a temporary probe that opened the form at
    start (`target/shots/form/terrain.png`); the probe is not in the tree.
  - [x] Step 3: `Control.tip` (in the screen's shape) and `shell/tips.rs`: a node that carries a `Tip`
    shows it in a box once the pointer has rested 0.4 s on it, and at once while it holds the keyboard;
    the box goes when the pointer leaves or a control is pressed. The box is a child of the node placed
    by Bevy 0.20's `ui_widgets::Popover` (left of a field's row first, so it falls over the plan; under
    any other control first), with `OverrideClip` so the scrolling column does not cut it. A field's row
    carries its field's tip, so the name explains it too. The terrain form's fields have their text
    (`editor/tips.rs::field`); the rest is step 4. Gate 1332 to 1337; seven mutations caught, and one
    survivor that showed a redundant branch, removed. Seen in one capture through the temporary probe
    (`target/shots/form/tip.png`); not seen following a real pointer.
  - [ ] Step 4: tip text for the whole editor (`editor/tips.rs`), listed in the plan for the owner.
  - [ ] Step 5: documents, `tasks/acceptance/editor-forms-and-tips.md`, continuity.
- [ ] **M8.5 step 5, the colour scheme and biosphere palettes** (the colour half for the 3D view and the
  editor preview is built, 2026-11-13, the item above; what remains: the fight screen and the HUD on the
  palette, relief noise, glyph markers and motion) (owner's 2 and 5; presentation-PRD §5.2, X18, X19,
  §11 step 3): the fixed colour roles, a biosphere's palette and style set in pack data, the relief noise per
  terrain kind (presentation only; as displacement on the GPU if size 2 is designed in), the first non-colour
  channels (line pattern, weight, glyph markers, motion), applied to the 3D view, the fight screen and the HUD,
  with a simulated colour-deficiency check (XR8; it covers the health colours). The owner's regions name their
  biospheres already (piedmont, coastline, mountains; mystic isles, volcanic; archipelago, jungle, savannah,
  desert). Open for the owner: the role set and glyph vocabulary (§12.3); whether a biosphere is assigned per
  region, per map or per terrain, and whether it also picks the relief noise and shape generators (§12.2). Living
  things and resources stay in M10.
- [ ] Shaped structures and set pieces (presentation-PRD §11 step 5, X12, §12.4): shape generators per terrain
  kind, then the first authored set piece. The owner's locality names the first two: the Old Dam across the P'ta
  and the Great Goblin Head. The set-piece format (a RON line model, SVG, glTF or an in-editor tool) is decided
  here; the editor places them.
- [ ] **M8.5 step 6, magic and mana in the map packs** (owner's 6). Neither the PRD nor ARCHITECTURE has mana in a
  place (members have spell points), while the lore does: Toel's abundant ambient mana, the taint Resdie's
  wizards cleanse from the fields. Needs the owner's intent before a plan, and a PRD section with it: what a
  place's mana does (spell power, recovery, wild or dead zones, taint), whether it is per cell, per map or per
  region, and whether it changes over time (M10). Shown in the magic colour role from step 5.
- [ ] Interface polish the owner sets a time for: Look and Map in vector (deferred at P9: Look, a button beside
  Camp and L, the first spyglass a member who can act carries; Map, the large automap over the view, M); the art
  update (a figure per monster, the fight screen's scale on large windows, deferred 2026-10-03); the distant
  level and the blueprint automap (presentation-PRD §12.5, §12.7).
- [ ] Dependencies: the rhai Socket re-audit (owner: after the migration); `smartstring` RUSTSEC-2026-0249 and
  `ttf-parser` RUSTSEC-2026-0192 reviewed with 0.20 (BACKLOG).

## W3 — The world's structure (closes PRD §13 Phase 2)
Done when a seed produces a traversable world of at least 100 regions with towns and dungeons the player can
complete, regenerating one layer keeps the edits in the others, and the Three Kingdoms' locality is laid out as
regions on Parva with its authored places in them.
- [ ] **M9 procedural world** (`omnis-gen`, ARCHITECTURE §7, §16): first the world's layout as data, since the
  owner's documents are shaped world, continent, locality, region (climate, biospheres) and today a region names
  only its kind, company, stability, rule, couplings and maps, and maps join by portals; regions laid out with
  edge links; travel between a locality's regions. Then the layered pipeline (terrain, hydrology and roads,
  settlements, dungeons, encounters and loot, quest hooks), each layer a seeded pure function over the LockMask so
  regeneration keeps hand edits; lazy `Pending` and `Materialized` maps and regions in the world folder; the
  editor's procgen panel; `gen region`; goldens seed to fingerprint. Generated maps may carry levels and joins.
  The ruins strewn over the Three Kingdoms are what the dungeon and settlement layers fill between the authored
  set pieces.
- [ ] With M9, the first milestone that opens `World` (D30; `tasks/plans/study-multiplayer.md` §5 requirements 3
  to 6 and 8): the party's state as one `PartyState` bundle the world holds per party, a world-unique `CharacterId`,
  a journal entry naming its party, `reconcile_party` taking the party, the party's streams emitted as
  `party:<id>`; a refactor with one schema bump and a migration, no new behaviour, both replays rebaselined. From
  W1 on, no new per-party state enters `World` as a bare field: it goes where `Checkpoint` keeps it.

## W4 — The living world (closes PRD §13 Phases 3 and 4)
Done when clearing a lair measurably changes encounters and prices in coupled regions on the next contact, visible
through rumours on the teller's clock; the main quest is a graph with a static completability proof; generated
quests appear in every generated town.
- [ ] **M10 ecosystem** (`omnis-eco`, ARCHITECTURE §7.2, PRD §9.2): the simplified 4X engine the owner named
  (2026-10-17) driving biospheres, settlements, regions and the world; `RegionState` (populations, factions,
  resources, prosperity, danger, weather, actors, stability); typed `RegionEvent`s; the ordered tick run through
  reconciliation catch-up; derived outputs (encounter weights, prices, rumours, quest availability); the fold
  thresholds of D28; `eco.*` ops and `bench eco`; the biosphere's living things and resources (split from M8.5
  step 5). The lore's factions come first: the Wardens, House Tourien, the merchants guilds, the clans of
  Karinguard, the Van Trahruns.
- [ ] The bus's designed expansions (ARCHITECTURE §4.8), each with its first consumer: a topic hierarchy with the
  first topic below a region (a map section or a city district); saved deferred signals with the first news that
  waits for the party (event rumours or M10's catch-up); pack-declared subscribers with the Rhai `Script` profile.
- [ ] **M11 story engine** (`omnis-story`, ARCHITECTURE §7.3, PRD §9.3): quest graphs with Rhai guards and the
  static check at pack load; templates bound to towns, dungeons and populations; the journal over `QuestState`;
  dialogue as data; story points that fold (D28); `story.*` ops; the editor's graph view.
- [ ] Multiplayer's first step (D30), where in W4 the owner sets: asynchronous play over a shared world folder, a
  character save per party, the merge fold, parties met as beings by the time model; planned in plan mode. A
  live transport and command signing wait on D29's keypair; two parties never share a fight in this step.

## W5 — Release (closes PRD §13 Phase 5)
Done when external playtesters complete the opening act and a modder outside the team ships a working pack from
the documentation alone.
- [ ] **M12 base campaign and release**: the campaign's content (whether the Three Kingdoms is its own pack over
  `packs/base`'s SRD material or `base` itself is decided here), the balancing pass, packaging with the
  platform's settings directory (an own path helper, no `dirs` crate, D31), mod documentation, the licensing audit,
  the Windows CI job if it has not landed at W1's close (Linux is advisory since W0.6), the credits.
- [ ] Signed packs and saves (D29), with the mod documentation: `pack seal --sign` on a local Ed25519 keypair
  (`identity.ron` beside the settings file; `ed25519-compact`, Socket read first, N-1 and 30 days), `signer` and
  `signature` in `pack.ron`, content hashes to blake3-256 in the same change, every pack resealed once, trust on
  first use with the project's key pinned for `base`, unsigned packs and unverified saves load with a warning; the
  save signature as a sidecar alongside, no crate. Verification in the loader and the hosts, never the simulation.
- [ ] GFDL backgrounds (Farmer, Squire, Metal Worker, Adventurer, Cook in
  `docs/background/opensource_backgrounds_and_skills.md`) as a separate `packs/backgrounds-gfdl` pack with its own
  license text and the GFDL copy; the base pack stays CC-BY-4.0 (owner, 2026-09-19).

## Far horizons (after the W0.5 studies; the owner schedules them)
- **Accounts, multiplayer, multi-platform**: chosen 2026-10-21 (PRD D29, D30, D31) and placed in the phases above:
  the Linux advisory job (W0.6), Windows CI (W1), the `PartyState` refactor (M9, W3), multiplayer's first step (W4),
  signed packs and saves (W5). Still beyond the phases: a live transport and a headless host for several parties, a
  browser build (a demo at W3 at the soonest, a target after W5), mobile (after touch controls become a goal), Steam
  Deck verification (after a gamepad layer). Every phase keeps what the documents keep for them: commands carry no
  client state (ARCHITECTURE §4.6); every party is a holder of its own clock (D25); the character save and the
  world file are apart (D28); the simulation crates stay portable by the lint.
- From the PRD's §13 Later, unchanged: free agency for important non-player characters (after M10), the hireling
  sidecar (D10), aging (D13), general scripting for mods on the sandboxed engine.

## Open, outside the phases (the owner sets the order)
- [ ] **The client's use of Bevy's ECS** (owner, 2026-11-07: "could we be using Bevy ECS better?"; the survey that
  day: 27 resources, about 45 components, 73 systems; the game and editor state in four large resources handed
  whole to Bevy-free view functions; dirtiness tracked by hand; every press found by polling `Changed<Interaction>`
  in five routers; observers twice, messages once). **Kept as is:** the simulation crates stay Bevy-free (the lint,
  ARCHITECTURE §4.1, §11); the thin-host pattern (big resources, Bevy-free `Screen` and `Scene` builders) stays,
  since it is what keeps the client testable without a window; one mesh and gizmo lines for the scene until the
  look lands in data (M9). Four tasks, in the order of their value; each its own commit, plan mode when reached,
  the full gate and the window tests as today:
  - [ ] **1. A press is a message, not a poll.** One system turns a control's `Interaction::Pressed` into a
    `Pressed(ControlId)` message (`add_message`, written once a frame per control); the screen router
    (`shell/screen.rs::route`), the editor's (`shell/editor.rs::route`), the fight's (`combat.rs`), the roster's and
    the notice's consume it, none of them reading `Interaction` or the raw mouse for a press again, so the order
    between routers is structural and the modal-closing frame cannot leak a press to the plan. The frame-memory
    guard `stood` in `shell/editor.rs` (2026-11-07) goes, its test `editor_link.rs::a_press_that_closes_the_picker_
    places_nothing_on_the_plan_beneath_it` rewritten against the message; the harness's `press_ui`/`press_edit`
    send the message (or set `Interaction` and run the one producer). Arrows and sliders follow the same path if
    the change is small; otherwise they wait for 2.
  - [ ] **2. A text field speaks through its change event.** Today `shell/editor.rs::route` compares every drawn
    `EditableText` with the model every frame (the field reads as changed under layout), with `fresh` and `refill`
    guards; `screen_draw::sync` never rewrites a field. Find what Bevy 0.19's text input offers (an observer on
    the field's value change, as `On<ValueChange<f32>>` serves sliders through `Slid`); the model hears typing
    from it alone; a model-set text (`Form::set_text`) still draws the field anew. The guards `fresh`/`refill`
    stay only if the event path cannot carry their cases; the tests `window_touches` (editor_link.rs) and the
    "replaced under the window's churn" tests (editor_forms.rs) are the acceptance.
  - [ ] **3. The two big routers split.** `shell/editor.rs::route` (174 lines, 15 parameters behind `Inputs`) and
    `refresh` (202 lines), `shell/screen.rs::paint` (17 parameters): smaller systems, one concern each (fields,
    presses, the plan's pointer, the wheel; per-column refresh), ordered with explicit `.after()` edges or a
    chained tuple that says the order out loud, their shared state as `SystemParam` bundles. No behaviour change;
    the editor suites and `kit_icons`/`screens` are the acceptance; file sizes under 1000 lines.
  - [ ] **4. Change detection where the hand keys are coarse anyway.** Measure first: a `resource_changed::<Session>`
    run condition on `VectorSet::Draw` (and `Editing` on the editor's refresh) against the hand-made keys
    (`Session.reshape`, `Preview.scene_key`, `Screens.drawn`, `editor::Drawn`, the minimap's `Local`); keep a hand
    key only where change detection would redraw too often (a resource mutated every frame). Record the frame
    counts in `tasks/knowledge/verification.md`; drop what the measurement does not justify.
- [ ] Party creation: remove or edit a member after adding them (owner, 2026-10-11; found in P3 as "not in P3").
  Today Add member is final: the CreateParty screen can only add, and Back drops the whole unfinished game. Neither
  client could ever do this, and the engine has no command for it (`PartyCommand` is `Create`, `Reorder`,
  `Tactics`). Plan in plan mode when reached; open questions for the owner then:
  - **Where the change lives.** Option 1: engine commands (for example `PartyCommand::Remove { member }`, and Edit
    as remove plus create, or its own command) under protocol 2, additive, with an op, an MCP tool and
    `docs/api.md`. Option 2: the client keeps drafts until Begin and sends the `Create`s only then (no engine
    change, but agents creating through MCP get nothing).
  - **When it is allowed.** Only during creation, before Begin (a new `creating` state in the world, or a rule
    such as "no turns taken yet"), or also later, as dismissing a member in town (gear to the party inventory, as
    permadeath does).
  - **Start again.** `Session::drafts()` re-creates the members from the `Party(Create)` commands in the binder
    log; a removal or edit must change what Start again re-creates (fold the log, or keep the final drafts).
  - **Identity.** A removed member's `CharacterId` is never reused (the vocabulary test checks member keys against
    ids ever held); the slots and marching order close the gap.
  - **The screen.** A member row in the Party list becomes pressable: Edit loads the draft back into the form
    (name, race, class, background, alignment, bought scores, picks; the racial bonus is not part of the draft),
    Remove asks to confirm. The member's draft must be recoverable (from the log, or a `MemberView` field the view
    does not yet carry).
  - Tests: a removed member is gone from the views and from Start again; an edited member keeps their slot; the
    party-full reason clears after a removal; the replays and `SAVE_SCHEMA` 9 do not move (or the save schema
    bumps with a migration if a world state is added).
- Tactics beyond declared reactions (auto play, further runbooks, encounter criteria, the library editor, monster
  runbooks): unscheduled since M7c (PRD §14, `tasks/knowledge/horizons.md`).
- Small findings and unused assets: `tasks/BACKLOG.md`, to review for fixing or removing.
