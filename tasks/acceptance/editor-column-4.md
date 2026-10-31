# Acceptance: the editor's left column as a drawing GUI, step 4 (the lasso, moving a selection, cell runs)

Step 4 of the owner's drawing-GUI column (`tasks/TODO.md`, W1; steps 1 to 3b accepted, 3b on 2026-10-31):
"Lasso (area select) and moving a selection; runs that take a terrain entry (a path of cells)". Owner's choices
in the planning (2026-10-31): **the lasso is a freehand loop** (the cells inside the drawn loop are selected);
**Move carries everything on the selected cells** (terrain, heights, the walls and doors on their sides, the
placements, the start when inside) and the source becomes bare; **the destination is replaced** (a portal, site
or pile already on a destination tile refuses the whole move and says why).

**A Save reseals the pack on disk.** Hand-test on a fresh scratch copy: `rm -rf /tmp/omnis-scratch && cp -R
packs/test /tmp/omnis-scratch`, named as the last pack.

## What was built
- **The lasso** (key L, after the arrows in the tool grid): a drag draws a loop through the cells it passes,
  outlined in yellow as it goes; letting go selects the loop's cells and every cell inside it (the even-odd rule
  on the loop's cell centres; a loop that is a line selects its cells). The selection is outlined in white on the
  plan and counted under the tool's name ("Selection: 6 cells"); "6 cells selected" in the message. A click
  without a drag clears it ("selection cleared"). It stands through tool changes; the map or the level changing
  drops it; any act (a key, a button) mid-loop drops the loop.
- **Move carries the selection** when the press is inside it: the white outline follows the cursor as a ghost;
  letting go moves every selected cell by the drag's offset in one undo step: the terrain (a stair with its
  rise), the heights, the sides (walls, doors, ramps), the placements standing on the cells and the start when it
  is inside. The source is left bare (no ground, height 0, open sides); the destination is replaced. "6 cells
  moved to (8, 2)" with the loader's first word when it has one; the selection moves with it. Refused, nothing
  changed: a cell off the map ("the selection would leave the map"); a portal, site or pile already on a
  destination tile the move does not vacate ("a site is already at (7, 3)"). The map's boundary sides stay walls.
  A press outside the selection carries the placement or the start under it, as before.
- **Cell runs**: a run tool (by corners or freehand) with a terrain or the height chosen takes the cells, not the
  grid corners: the points are cells, the preview outlines the cells of the line between them in yellow (with
  the stretch to the pointer's cell by corners), and taking the run (Enter, a click on the last cell, letting go
  freehand) paints the entry on each ("5 cells painted r street", "3 cells set to height 2"), one undo step.
  Choosing a terrain or the height with a run tool keeps the tool; a stamp with a run tool still walls.
- **Data:** `Document::batch` puts brushes and places in one step, all or nothing.

## Steps (in the window)
1. Editor, the town. *You will see* a lasso icon in the tool grid after the arrows; pressed, "Lasso  L" under it.
2. Drag a loop round the inn's cells at the top left (through (0, 0), (2, 0), (2, 1), (0, 1)). *You will see* the
   loop's cells outlined in yellow as you draw, then a white outline round the block, "6 cells selected" and
   "Selection: 6 cells" under the tool's name.
3. Press the arrows (Move), press inside the block and drag right along the street. *You will see* the white
   outline follow the cursor; let go on the street's right end. *You will see* the block there with the inn's bed
   icon, its floor and walls, "6 cells moved to (x, y)"; the old cells bare. Undo puts it back in one step.
4. Lasso the inn again, press Move, drag it onto a shop's door cell (one with an icon). *You will see* "a site is
   already at (x, y)" and nothing moved. Drag it left off the map. *You will see* "the selection would leave the
   map".
5. Press the lasso, click a cell without dragging. *You will see* "selection cleared" and the outline gone.
6. Choose the street swatch, then the pen (run by corners), click a cell, move the pointer. *You will see* the
   cells of the line to the pointer outlined in yellow; click a second cell, press Enter. *You will see* "N cells
   painted r street" and the cells street. Choose the height, the pencil (freehand), set the eighths with = and
   drag across cells. *You will see* "N cells set to height 2".
7. Choose the wall, the pen, click two corners. *You will see* the wall run as before.
8. Save. *You will see* "sealed at 0.2.1" (or the next patch).

Covered by `crates/omnis-vector/tests/editor_select.rs` (three tests: the loop, its count and outline, a click
clears, an act mid-loop drops, Open drops; the move with the ghost, the walls and the site, Undo, the two
refusals, an overlapping move, a press outside the selection; the cell runs by corners and freehand, a stamp's
fallback), `select.rs` and `run.rs` unit tests, `omnis-data/tests/edit_document.rs::a_batch_of_paints_and_
places_is_one_step_and_fails_together`. Mutation checks red: the even-odd rule inverted, the destination
painted before the source, the line's last cell dropped.

## Known and expected
- A note is the cell's and stays where it was when a selection moves (step 2's rule).
- A move onto an encounter's cell or under a blocking object is not refused by the editor; the loader's word
  follows the message, as for stamps.
- The pan under Move needs a press outside the selection on a bare cell, as before; the middle button pans
  anywhere.
- Esc is the menu's: a loop or a run is dropped by any act (a tool key, a button), not by Esc.
- Later: the eraser or Delete clearing a selection's cells; copy and paste; a quantity for a cell run's width.
