# Acceptance: the editor's left column as a drawing GUI, step 1

The owner's direction of 2026-10-25, on accepting the boundary tools: the tools column "is functional, but not
intuitive or easy on the eyes". Drawing programs have a grid of icons for the pointer states and a grid for
the palette, with the chosen entry's details shown and the file buttons beneath. Step 1 builds the frame: the
kit draws icons, the column becomes the two grids, Select and the Eraser arrive. Fill, Move, placements as
stamps, Lasso and a history list are the later steps (`tasks/TODO.md`).

Owner's decisions in the planning (2026-10-25): the view controls move to the bar over the plan; the chosen
entry's details sit under the palette grid; the first step has no Fill; the eraser erases what is under the
pointer.

**A Save reseals the pack on disk.** Hand-test on a scratch copy: `cp -R packs/test /tmp/omnis-scratch`, named as
the last pack.

## What was built
- **The kit draws icons** (`96237cbf`): an icon control is a square box with a Lucide icon, drawn once per name,
  size and colour and kept; the chosen one of a grid is framed twice as thick and drawn white; the editor's
  buttons shade under the pointer as a game's do and are narrow enough for two a row. A swatch is a square of
  one colour with a glyph on it.
- **The tool grid:** Select (the pointer), Brush, Run by corners (the pen), Freehand run (the pencil), Eraser,
  then Undo and Redo. The chosen tool is framed and named under the grid with its key: "Brush  B". Keys V, B,
  P, N, X.
- **The palette grid:** a swatch a terrain, in its colour with its glyph, then Wall, Door, Ramp and Height as
  icons; the chosen entry framed. Under it the details: the entry's words with its digit (the digits still
  choose the first nine), the height's eighths choice, Edit terrain for a terrain, Add terrain; a pending run's
  rows. Choosing an entry chooses the brush; choosing a run tool with a terrain chosen chooses the wall, since a
  run takes an edge kind: a run of doors reads "3 sides set to door".
- **Select** puts the cursor on the cell and paints nothing; the inspector follows.
- **The Eraser** erases what is under the pointer, the aim shown first: near a cell's side, that edge opens; on a
  cell with a placement, the first listed placement is removed (one step of its own); otherwise the cell becomes
  no ground, a drag one stroke. A boundary side is refused and said. Open edge and No ground left the palette:
  they are the eraser's.
- **The bar** over the plan carries zoom out and in, centre, turn left and right as icons, the level choice and
  "N px a cell" with the keys; the inspector carries Add level and Level... beside its level line.
- **Under the grids:** the map choice and Open; Plan, Text, Open file...; Save and Revert; Playtest and Menu, two
  a row.

## Steps (in the window)
1. `just run --windowed --pack packs/base --pack /tmp/omnis-scratch`, Editor, open the town.
   *You will see* the left column: a row of icons (pointer, brush, pen, pencil, eraser, undo, redo) with the
   brush framed bright and "Brush  B" under it; a row of coloured squares carrying the terrains' glyphs and
   four icons (wall, door, ramp, mountain) with the first terrain framed; "1  r road" (or the first terrain's
   words) with Edit terrain and Add terrain; the Map choice and Open; Plan, Text, Open file...; Save, Revert,
   Playtest, Menu two a row. Hover a button. *You will see* it lighten.
2. The bar over the plan: the cell, then zoom out, zoom in, centre, turn left, turn right as icons and the
   Level choice. Press the zoom icons. *You will see* the plan zoom as `[` and `]` do.
3. Press the pointer icon (or V), click cells. *You will see* the cursor move and the inspector follow;
   nothing paints; Undo stays dim.
4. Press the eraser (or X). Hover a wall's side. *You will see* the yellow bar on the side; click.
   *You will see* the wall gone. Click the road out at (11, 2). *You will see* the portal gone from the
   inspector; Undo brings it back. Click a street cell. *You will see* no ground. Drag over three cells,
   Undo once. *You will see* all three back.
5. Press the pen icon (or P) with a terrain chosen. *You will see* the wall icon framed. Press the door icon,
   click two corners and the last again. *You will see* a run of doors; "N sides set to door".
6. Right click an open edge. *You will see* the eraser framed; right click a terrain cell: the brush and
   that terrain's swatch framed, the height taken.
7. Press the mountain icon. *You will see* the eighths choice in the details; `-` and `=` step it.
8. Save. *You will see* "sealed at 0.2.1".

Covered by `crates/omnis-vector/tests/kit_icons.rs` (the icon kind and its chosen state in the shape, the
bar's icons named by the text reader and pressed, the editor buttons' width and shades, the images drawn once
in the role's colour and shared), `tests/editor_column.rs` (the column's grids, details, files and buttons in
ASCII; choosing tools and entries, the keys; Select; the Eraser's three erasures, the boundary, the picks; a
run of doors), the `brush.rs` unit tests (the palette, the tools' names, icons and keys), and the editor suites
moved to the new vocabulary (`editor_paint.rs`, `editor_run.rs`, `editor_pointer.rs`, `editor_forms.rs`,
`editor_files.rs`, `editor_playtest.rs`, `editor_preview.rs`).

## Known and expected
- Fill, Move and Lasso icons are not shown until they exist (a dim icon with no reason would read as broken).
- A dim icon shows no reason; Undo and Redo dim when there is nothing to undo or redo, Save and Revert keep
  their words and reasons.
- The eraser removes the first listed placement on a cell; the inspector's Remove buttons remove a chosen one.
- A right click with Select picks the cell (the hover is the cell): to pick an edge's kind, right click with the
  brush or the eraser.

## Covered throughout
- Gate VERIFY-GREEN; Sentrux rules pass.
- `SAVE_SCHEMA`, `PROTOCOL`, the op count and the replays unchanged (no change to `packs/`).
