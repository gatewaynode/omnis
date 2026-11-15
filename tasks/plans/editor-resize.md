# Resize a map from the editor, on all four sides

## Context

The owner asked whether a hallway can be added to an existing map. It cannot from the window: the
engine has `Edit::Resize { width, height }`, reachable only through the API, and it only grows or
crops on the east and south. The owner's order (2026-11-14): "plan the Resize form, all four
sides. Shrink is destructive, a warning and y/n accept buttons should be thrown when shrinking a
map destroys things." Their answers in plan mode: a grow **keeps the old outer wall** as an inside
wall; the warning comes for **anything but empty cells**; and the form is then used to **put the QA
hall inside the yard**, as the last step.

Found while reading (`crates/omnis-data/src/edit/document.rs:869`, `grid.rs:345`), all fixed here:
- A grow east or south drops the old east and south walls (only north and west edges are copied).
- What falls outside a shrink is dropped without a word; the step says only "resized to W x H".
- New ground cells are terrain 0 even when terrain 0 is a stair, which the loader then refuses.
- Notes on cells are not moved or told; they are keyed by `x,y`.

## The rule

- **One edit, four signed numbers**: `Edit::Extend { north, east, south, west }` (`i16` each, cells
  added on that side; negative removes). The new size must be 1..=`MAX_MAP_SIDE` a side
  (`OutOfRange` otherwise). `Edit::Resize { width, height }` stays on the wire and becomes
  `Extend { east: width - w, south: height - h }`: one implementation.
- **Everything moves together** by `(west, north)`: every level's cells, heights and edges, and
  `stairs`, `portals`, `encounters`, `sites`, `objects`, `ground`, `lore` and the start.
- **A grow keeps the old outer wall** on that side as an inside wall; only the new outer edge is
  walled. A shrink walls the new edge, as today.
- **New cells**: on the ground, the first terrain that is not a join (stair, ladder, hole); no
  ground if the map has none such. On other levels, no ground (as today).
- **What a shrink destroys** is listed before it happens (`Document::losses(&Edit) -> Losses`):
  cells with ground, a wall, a door or a height, counted; each stair, portal, encounter, site,
  object, pile and lore by kind and id or cell; the start, when it falls outside (it is put on the
  nearest cell inside, as today); cell notes. Empty cells are no loss.
- **Notes follow their cells**: the document carries the shift since it was last saved (in its
  `State`, so Undo and Redo carry it too); `Session::save` rekeys the map's cell notes by that
  shift and drops those of cells that are gone, then resets it. Notes are written at once and are
  outside Undo, so they move at Save and not before.
- **One undo step**, named for what it did: "grew 3 west, 2 south", "cut 2 east (1 portal, 4
  cells lost)".
- **The API applies without asking**, as `Resize` does today (`editor.edit`): the question is the
  window's. `docs/api.md` and the MCP tool text say what is destroyed and name `losses`.

## The form

- **Entry**: the inspector's map line (`"{id}  {w} x {h}"`, `editor/view.rs` `inspector_plain`)
  gains a **Resize...** button beside it. A modal over the editor, as the owner's pickers are
  (`ScreenId::EditorResize`, `Asked::Resize`, modelled on `EditorRules` / `EditorProps`).
- **Body**: "Test Yard is 7 x 5." Four number fields, **North, East, South, West**, "cells to add;
  a negative number removes", each typed or stepped; a line that follows them: "It becomes 9 x 5."
  A second line when west or north is not 0: "Every cell moves 2 east: a saved game's party on this
  map will stand where the old cell was." **Apply**, **Cancel**. Apply is dim with the reason when
  a side would leave 1..=256 or all four are 0.
- **The warning**: Apply on a shrink that destroys something does not resize. The modal shows
  "Cutting 2 off the east destroys:" and the list (the first eight lines, then "and 5 more"), with
  **Yes, resize** and **No**. No returns to the fields as they were. A shrink of empty cells, and
  any pure grow, applies at once.
- **After**: the modal closes, the plan refits to the new size, the cursor and the selection are
  cleared or moved with the cells, the message line says the step's words, and the loader's errors
  show as for any edit (a cut portal that another map links to dims Save with its count).
- Every control has a tip (`editor/tips.rs`); strings ASCII.

## Reused

`Document::change` (one step, undo), `words::edit`, `Grid::blank`/`wall_boundary`
(`omnis-data/src/edit/grid.rs`), `Editor::report` and `changed` (`editor/mod.rs`),
`shell/editor.rs::answer`, `Effect::Editor(act, payload)`, `form::number`, the file picker's
question (`screen/editor_files.rs`) and the loot window's `asking` as the confirmation's shape,
`notes_store` (`in_map`, `orphans`), `tests/common/editor.rs` (`in_editor_scratch`, `press_edit`,
`type_ui`, `top_screen`), `Session::save`.

## Steps (one commit each: tests red first, mutation checks, fmt, clippy, gate, Sentrux)

0. **Documents**: this plan to `tasks/plans/editor-resize.md`, the item in `tasks/TODO.md`, the
   "Later" line in `tasks/CONTINUITY.md` struck.
1. **Engine**: `Edit::Extend`, `Grid::extend` (offset copy of tiles, heights and all four edges),
   the shift of every placed thing and the start, new-cell terrain, `Resize` rerouted, the step's
   words. The kept-wall fix lands here and is a behaviour change to `Resize` (said in the commit
   and `docs/api.md`). New file `omnis-data/src/edit/extend.rs` (document.rs is at 891 lines).
2. **Losses**: `Losses` and `Document::losses`, tested against the fixture town and yard.
3. **Notes at Save**: `State.shift`, the rekey in `Session::save`, undo and redo across it.
4. **API**: MCP `editor_edit` schema and tool text (`editor_schema.rs`, `tools.rs`), CLI schema
   example, `docs/api.md`, the pinned branch counts.
5. **The form and the warning**: `editor/resize.rs` (model: the four numbers, `asking`),
   `screen/editor_resize.rs`, `EditAct::{Resize, ResizeSide(u8), ResizeApply, ResizeYes,
   ResizeNo}`, the inspector's button, tips, `nav` and `answer`. `editor/mod.rs` (1038 lines)
   gains nothing but the field and the module line; something small moves out of it to bring it
   under 1000 while I am there.
6. **The QA hall inside the yard** (owner's choice): the yard grown south with the new edit, the
   hall behind a shut gate, the shamblers at its far end, `objects:map:hall` removed, the fixture
   resealed. Existing yard cells keep their coordinates. **If more than a handful of yard tests
   move, I stop and report before rewriting them.**
7. **Documents**: the owner's sheet `tasks/acceptance/editor-resize.md`, the code map, continuity.
   Report: built, please try.

## Verification

- Data tests (`omnis-data/tests/edit_document.rs`): grow on each side keeps every cell's content
  at its shifted place and the old wall where it stood; a west grow of 2 puts the town's portal 2
  east and its landing with it; shrink on each side; a round trip (grow 3 west, cut 3 west) is the
  map it started as; the size limits; undo and redo; `Resize` as before except the kept wall;
  `losses` names each kind and is empty for empty cells; notes after Save, and after Undo then Save.
- Client tests (`omnis-vector/tests/editor_resize.rs`): the button opens the modal; the "becomes"
  line follows the fields; a grow applies at once and closes; a destructive shrink shows the list
  and Yes / No; No changes nothing and keeps the numbers; Yes applies as one undo step; Apply dim
  with its reason; the loader's error count after a cut portal.
- `scripts/verify.sh` to a log, commit only on `VERIFY-GREEN`.
- By hand (owner): on a scratch copy of their map, grow west by 3, draw a hallway, Save, reopen;
  cut it off again and read the warning.

## Said plainly

- **Saved games**: growing west or north moves every cell of the map, and a save keeps the party's
  position, the automap and object states by cell. The form says so; nothing rewrites saves. On
  load the engine already moves a party that stands off the map to the nearest cell.
- **Other maps** are not rewritten: a cut portal leaves its partner unlinked or erring, and the
  loader says so (Save dims with the count). Undo brings it back.
- **No preview on the plan** of the new outline while the numbers change; the "becomes" line is
  the preview. A drawn outline is a follow-on.
- The map's other own fields (`Edit::Map`: name, tileset, `tile_feet`, `door_make`) and New map as
  forms stay on the later list; this plan is Resize only.
- Step 6 changes a fixture many tests stand on; it is last so the form ships whatever it finds.
