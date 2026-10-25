# Acceptance: the boundary tools (after M5)

The owner's deferred item of 2026-10-24 (`tasks/TODO.md`, under M5.5): boundary tools beyond the edge brush, a
**point-by-point** tool and a **freehand** tool, both normalised to the map's granularity (snapped to the
grid's corners, rasterised onto cell sides), previewed in the hover colour before taken, one undo step.

Owner's decisions in the planning (2026-10-25): the runs paint **walls only**, as two palette entries; a
**freehand line is taken on release**; a corner run is pending until a click on its last corner, Enter or
Take run; Backspace drops the last corner.

**A Save reseals the pack on disk.** Hand-test on a scratch copy: `cp -R packs/test /tmp/omnis-scratch`, named as
the last pack.

## What was built
- **Two brushes** after Open edge in the palette: **Walls by corners** and **Walls freehand**. With either, the
  pointer aims at the grid corner nearest it (within half a cell of the map, so the boundary's corners are
  reachable from just outside), shown as a small yellow square; the side hover is gone.
- **By corners:** a click starts the run, each click adds a corner, and the run follows: straight along a grid
  line, a staircase hugging a slanted line. The whole run and the stretch to the pointer are previewed in
  yellow. A click on the run's last corner takes it; so do Enter and the tools column's **Take run**.
  **Drop corner** (Backspace) takes the last corner back; **Drop run** drops it all. The column says "Run: N
  corners" while one is pending.
- **Freehand:** press, draw, let go. Each corner the pointer passes joins the run as you draw (the yellow line
  snaps to the grid under the pointer); letting go takes it, on or off the plan.
- **One undo step, nothing lost:** the run's sides become walls through one change of the document
  (`Document::paint_many`: all land or none), each side once however the line doubled back. The map's boundary
  is walls already and is left alone, counted in the message: "6 sides walled", "2 sides walled, 2 on the
  boundary stay walls", "nothing to wall".
- **A pending run belongs to its place and tool:** choosing another brush, a right click (which picks the
  nearest side's kind, as the edge brushes do), a level or map change, Undo or Redo drop it. Zoom, centring
  and the facing keep it.

## Steps (in the window)
1. `just run --windowed --pack packs/base --pack /tmp/omnis-scratch`, Editor, open the town; press **Walls by
   corners**. Move over the plan.
   *You will see* a small yellow square snapping from grid corner to grid corner; no side bar.
2. Click a corner on the street's upper edge, move right.
   *You will see* a yellow run along the grid line following the pointer. Move diagonally.
   *You will see* the run turn into a staircase hugging the line. Click two more corners.
   *You will see* the run grow; the tools column shows "Run: 3 corners", Take run, Drop run, Drop corner.
3. Press Backspace.
   *You will see* the last corner and its stretch gone. Click the run's last corner again (or Enter, or Take
   run).
   *You will see* the yellow become green walls; "N sides walled" in the tools column. Undo (Cmd Z).
   *You will see* the whole run gone in one step.
4. **Walls freehand**: press on a corner, drag a curve across the shops, let go.
   *You will see* the yellow line snap to the grid as you draw and become walls on release; one Undo takes it
   back.
5. Drag along the map's top edge, just outside the map.
   *You will see* nothing change and "nothing to wall, N on the boundary stay walls".
6. Start a run by corners, then right click a door.
   *You will see* the Door brush chosen and the run gone; the side hover is back.
7. Save.
   *You will see* "sealed at 0.2.1" (the shipped pack is 0.2.0).

Covered by `crates/omnis-vector/src/editor/run.rs`'s unit tests (the walk along the grid, the paints and the
boundary), `plan.rs` (`View::corner`), `crates/omnis-data/tests/edit_document.rs`
(`many_paints_are_one_step_and_fail_together`), `crates/omnis-vector/tests/editor_run.rs` (a run by corners
previewed and taken, the last-corner click and the buttons, the run dropped with its tool, level, history or a
pick, a freehand drag taken on release, the boundary, the preview's pixels) and `tests/editor_pointer.rs` (a
corner click through Bevy's own layout).

## Known and expected
- A slanted line becomes a staircase: the grid has no diagonal sides. The walk steps along the axis that keeps
  it nearer the line, horizontal at a tie, so the staircase is even.
- The runs paint walls only (owner, 2026-10-25). A line of doors or an erased line is drawn side by side with
  the edge brushes.
- A run is not an op: over the socket a headless author paints a run as `editor.paint` with `stroke`
  (`docs/api.md` §5). An `editor.run` op is a follow-on when one asks.

## Covered throughout
- Gate VERIFY-GREEN; Sentrux rules pass.
- `SAVE_SCHEMA`, `PROTOCOL`, the op count and the replays unchanged (no change to `packs/`).
