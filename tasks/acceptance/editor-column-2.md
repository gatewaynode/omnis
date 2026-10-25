# Acceptance: the editor's left column as a drawing GUI, step 2 (Fill and Move)

Step 2 of the owner's drawing-GUI column (`tasks/TODO.md`, W1; step 1 accepted 2026-10-25,
`tasks/acceptance/editor-column-1.md`): the fill and the move join the tool grid. Owner's choices in the
planning (2026-10-25): **Move grabs what is under the pointer** at the press (the first listed placement, as
the eraser erases; any other cell pans the map; a chosen selection waits for the lasso, step 4); **any edge
that is not open bounds a fill** (walls, doors and ramps alike); **the pan moves by whole cells** (a smooth
pan is a later item).

**A Save reseals the pack on disk.** Hand-test on a scratch copy: `cp -R packs/test /tmp/omnis-scratch`, named
as the last pack.

## What was built
- **Fill** (the bucket, key F): a click paints the palette's entry over the region of one ground the cell
  belongs to: the cells of the same terrain (or of no ground) joined through open sides; a wall, a door or a
  ramp bounds it. One undo step. A terrain the region already is fills nothing ("already r street"). The fill
  takes a terrain or the height: choosing it with an edge chosen chooses the first terrain, choosing an edge
  while it is current hands over to the brush. A drag fills once.
- **Move** (the four arrows, key M): a press on a cell takes what the inspector lists there first, the map's
  start or the first placement; the cursor follows the drag and letting go puts it on that cell: the start in
  one step, facing as it did ("Start moved to (5, 2)"; fixed after the owner's hand test, 2026-10-25: "I
  couldn't move the start marker"), a placement as a removal and a placement in one undo step ("Portal to
  test:map:meadow moved to (8, 2)"). A portal, a site or a ground pile is refused where one of its
  kind already is ("a site is already at (4, 1)"). Let go on its own cell or off the plan, nothing moves. A
  press on any other cell drags the plan itself by whole cells.
- **The middle button** drags the plan under any tool, a shortcut for Move's pan. Any act (a key, a button)
  ends a drag where it is.
- The tool grid reads: pointer, brush, move, bucket, pen, pencil, eraser, then undo and redo.

## Steps (in the window)
1. `just run --windowed --pack packs/base --pack /tmp/omnis-scratch`, Editor, open the town. *You will see*
   seven tool icons: pointer, brush, move, bucket, pen, pencil, eraser.
2. Press the bucket (or F) with the wall chosen. *You will see* the first terrain framed instead, "Fill  F"
   under the grid. Choose the green swatch, click inside the inn (top left). *You will see* the whole inn
   green and "6 cells filled with , green"; Undo brings the inn back in one step. Click the street with the
   green chosen. *You will see* the street green and the green plot at the right unchanged (a different
   ground, though the side between is open).
3. Choose the mountain icon, set the eighths with `=`, click a shop. *You will see* "6 cells set to height N"
   and the shop's cells lifted in the preview.
4. Press the move icon (or M). Drag the signpost cell (11, 2) to (8, 2). *You will see* the cursor follow,
   then the portal listed in the inspector at (8, 2) and "Portal to test:map:meadow moved to (8, 2)"; Undo
   returns it. Drag the inn's site (1, 1) onto the temple's (4, 1). *You will see* "a site is already at
   (4, 1)" and nothing moved. Drag it to (2, 1). *You will see* the inn's service listed there. Drag the
   start marker (10, 2) to (5, 2). *You will see* the mark on (5, 2), still facing west, "Start moved to
   (5, 2)" and the inspector's start line reading (5, 2); Undo returns it.
5. With Move, drag a bare street cell. *You will see* the plan pan by whole cells. Press C. *You will see* it
   centre on the cursor again. With the brush chosen, drag with the middle button (when the mouse has one).
   *You will see* the same pan, and nothing painted.
6. Save. *You will see* "sealed at" the next patch.

Covered by `crates/omnis-vector/tests/editor_fill.rs` (the inn filled and undone as one step, the open side to a
different ground not crossed, "already", a drag fills once; the height filled, the entry rules, the key),
`tests/editor_move.rs` (the portal carried and undone, the note staying the cell's, the site refused, the same
cell and off the plan, the start carried with its facing and undone, a bare cell's pan, the middle button's pan under the brush, an act ending a drag), the
`fill.rs` unit test (a parted grid: a wall, a door and a ramp bound; an opened door joins; no ground is its
own region), `omnis-data/tests/edit_document.rs::many_places_are_one_step_and_fail_together`, and the step-1
suites unchanged but for the tool count.

## Known and expected
- The pan moves by whole cells: imperceptible to 24 px a cell, a visible jump at 64 and 128 (smooth pan is a
  later item). A fill's region is not previewed on hover (later).
- A note is the cell's, not the placement's: moving the signpost's portal leaves the note on (11, 2).
- Move takes what the inspector lists first on a cell: the start, then the first placement. On a cell that
  has both, the start moves first; the inspector's rows still edit and remove a chosen placement, and Set
  start here puts the start back.
  Encounters, objects and lore keyed by id may share a cell with their kind; portals, sites and piles may not.

## Covered throughout
- Gate VERIFY-GREEN; Sentrux rules pass.
- `SAVE_SCHEMA`, `PROTOCOL`, the op count and the replays unchanged (no change to `packs/`).
