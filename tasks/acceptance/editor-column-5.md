# Acceptance: the editor's left column as a drawing GUI, step 5 (Undo as a history list)

Step 5 of the owner's drawing-GUI column (`tasks/TODO.md`, W1; steps 1 to 4 accepted, 4 on 2026-10-31): "Undo as
a history list"; the owner's picture of 2026-10-25, "undo (a history tool)". Owner's choices in the planning
(2026-10-31): **a History icon beside Undo and Redo** opens the list (the Undo and Redo icons and keys stay); the
list is **a modal over the editor**, the plan visible beneath so the owner can scrub.

**Hand-tested and accepted by the owner 2026-10-31** ("Looks good in manual testing, let's wrap it up"). The
column's five steps are closed.

**A Save reseals the pack on disk.** Hand-test on a fresh scratch copy: `rm -rf /tmp/omnis-scratch && cp -R
packs/test /tmp/omnis-scratch`, named as the last pack.

## What was built
- **Every undo step carries words** (`omnis-data`, `edit/words.rs`, `Document`): a paint names its terrain and
  cell ("painted street at (4, 3)"), a stroke counts its cells under its first paint's words ("a stroke of 3
  cells: painted street at (4, 3)"), a side, a height, a clearing, a placement, a removal and a map edit name
  themselves; several in one step count ("6 cells and 2 placements"). The editor replaces the words with its own
  where it has richer ones ("6 cells painted r street", "6 cells moved to (8, 2)", "Inn placed at (3, 1)", a
  form's Apply). The socket's edits (`editor.paint` and the rest) take the document's words. ASCII, 72 bytes.
- **The History icon** (a clock run back, after Redo in the tool grid) opens the **HISTORY modal**: "the map as
  opened", then a row a step, oldest first, "N  words"; the row the map stands after is marked "> " and dim
  ("where the map stands"); "no steps yet" on a fresh map. **A row jumps there** (`Document::goto`): the plan
  beneath shows the map as it stood, the modal stays up, the mark moves, "at step N of M" (or "the map as
  opened"). Undo and Redo at the bottom walk one step (dim with "nothing to undo" / "nothing to redo"); Close.
  A change after a jump drops the steps undone, as Undo always did; a pending run is dropped by a jump.
- The Undo and Redo icons and their keys (Cmd/Ctrl+Z, Shift+Z, Y) are as they were.

## Steps (in the window)
1. Editor, the town. *You will see* a clock-with-arrow icon after Redo in the tool grid. Press it on the fresh
   map. *You will see* HISTORY with "> the map as opened" and "no steps yet", Undo and Redo dim, Close. Close it.
2. Choose the street and the brush, drag across three cells; choose the eraser, click a cell; choose the pen
   (run by corners), click two cells apart, press Enter. Press History. *You will see* "1  a stroke of 3 cells:
   painted street at (x, y)", "2  cleared (x, y)", "> 3  N cells painted r street" marked.
3. Press row 1. *You will see* the plan behind with the stroke only (the cleared cell and the run undone), row
   1 marked, rows 2 and 3 still listed, "at step 1 of 3". Press Redo in the modal: row 2 marked, the cleared
   cell back. Press row 3: all redone.
4. Press "the map as opened". *You will see* the plan as the file holds it, "the map as opened", Save dim.
   Close; paint one cell; History again. *You will see* "1  a stroke of 3 cells: ..." and "> 2  painted street
   at (x, y)" only: the undone steps are gone.
5. Close; Cmd+Z and Cmd+Shift+Z still walk one step; the Undo and Redo icons dim with their reasons.
6. Save. *You will see* "sealed at 0.2.3" (or the next patch).

Covered by `crates/omnis-vector/tests/editor_history.rs` (the icon, the empty list, three named steps, a row
jumping with the modal up and the mark moving, Undo and Redo in the modal, the map as opened, a change after a
jump dropping the undone steps, a pending run dropped), `omnis-data/tests/edit_document.rs::steps_are_named_and_
goto_walks_the_history` (the words of a paint, a stroke, a place, an edit, a batch, a renaming; `goto` back,
forth, past the end; the cap), `words.rs` unit test. Mutation checks red: `goto` stopping one short, a stroke's
words not rewritten, `name_step` a no-op, the rows newest first.

## Known and expected
- The row "the map as opened" is also where the map stood before the oldest step kept, once the cap (100) has
  dropped steps.
- The modal lists at most 100 steps and scrolls past the window's height.
- The keys do not reach the editor while the modal is up (an overlay covers it); the modal's buttons stand in.
- Later: `editor.history` over the socket (the words to an agent); a saved mark on the row the file holds.
