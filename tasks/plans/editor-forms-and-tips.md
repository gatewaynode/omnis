# A terrain's block face, the editor form's layout, and tooltips

Approved by the owner 2026-11-13 (plan mode). The tip texts are listed at the foot of this document when
step 4 writes them (one table, `crates/omnis-vector/src/editor/tips.rs`).

## Context

The owner tried rendering size 2 part 3 (2026-11-13) and reported two things.

1. **"No hatching on the cliff faces in the meadow."** The meadow has no cliffs (`heights: []`). What the
   screenshot shows are the `trees` terrain's solid **blocks**. Before part 3 a block took its terrain's
   colour (dark green); since part 3 a block with no named role is *structure*, so trees are drawn exactly
   like walls and read as rock faces. Cliff hatching itself still draws (capture of `heights:map:hills`).
   My report named the merges of floors, cliffs and piles but not blocks; that goes to `tasks/LESSONS.md`.
2. **The editor's form in the right column is unclear**: label and input share one bordered box in one
   colour, inputs start at different places, and nothing explains a field.

The owner's decisions (asked in plan mode, 2026-11-13):
- **Blocks**: a terrain picks its block's face (plain box, or hatched like a cliff).
- **Layout**: two aligned columns, dim labels left, each input in its own box right, fields grouped under
  small headings (the chosen preview also shows a flag as a tick box).
- **Tooltip**: a floating box beside the control after the pointer rests on it; the field holding the
  keyboard shows the same box.
- **Scope of tip text**: the whole editor (every form field, tool icons, palette entries, bar and file
  buttons). The facility is the kit's, so any screen can use it later.

## Design

### A. A terrain's face (`omnis-data`, `omnis-vector/src/geometry.rs`)
- `omnis-data/src/map.rs`: `enum Face { Plain, Hatched }` (default `Plain`), `Terrain.face: Face` with
  `#[serde(default, skip_serializing_if = ...)]`, so **no existing seal moves** and `PROTOCOL` stays 2.
  An enum, not a flag, so further faces can be added.
- `geometry.rs::cells`: a `Hatched` block adds verticals at the cliff's quarter-cell spacing (`HATCH`) on
  each of the box's four sides, from the block's base to its top, as `SegKind::Block` (so its role, weight
  and pattern stay a block's). The box's twelve edges stay whole.
- Terrain form (`editor/forms.rs::terrain`, `terrain_fields`): a `face` choice. The editor's 3D preview
  shows it at once.
- Fixture: one block terrain of a fixture pack other than `test`/`base` (the `heights` pack) is hatched;
  resealed, version bumped. `docs/api.md` changelog, `omnis-cli/src/schema.rs` terrain example.
- **The owner's pack is not touched**: the owner sets `face` on `trees` in the terrain form.

### B. The form's layout (kit and editor)
- `screen/mod.rs`: `Row::field(label, control)`: a row whose label is a dim span in a **label column of
  fixed width** and whose control is drawn **without its label inside its box** (`Row.aligned`, in
  `Screen::shape`). The control keeps `label` for the text readers and routing.
- `screen/mod.rs`: `Kind::Flag(bool)`: a tick box, one press turns it. `form::Value::Flag` maps to it
  instead of a no/yes stepper. Routed as a press (`Payload::Press`), drawn in `shell/screen_draw.rs`.
- `shell/screen_draw.rs::rows` / `control_node`: the aligned row; text inputs take the rest of the row's
  width. Other screens (Settings and the rest) keep today's rows.
- `editor/form.rs`: `Form.sections: Vec<(usize, &'static str)>` (a heading before field `i`; field indices
  and `EditAct::Field(i)` do not move). `Form::rows` emits headings and `Row::field`. In a list's row each
  input keeps a dim label before it and its own box. The properties modal (`screen/editor_props.rs`) uses
  the same rows.
- Sections for every form live in a new `editor/tips.rs` (below), keeping `forms.rs` (978 lines) under
  1000; if the `face` choice pushes it over, the terrain form moves to its own file.

### C. Tooltips (kit, shell, editor)
- `screen/mod.rs`: `Control.tip: Option<String>` (its presence in `shape`).
- `shell/tips.rs` (new), adopting **Bevy 0.20's `ui_widgets::Popover`** (already in the tree; positions a
  child beside its parent and keeps it inside the window): a control node with a tip carries `Tip`. After
  the pointer has rested on it about 0.4 s, or while it holds the keyboard (`InputFocus`), a small bordered
  box of body text is spawned as its child with `Popover` (left of the control first, then right, below,
  above, so the right column's tips fall over the plan, not over other fields), above every screen
  (`GlobalZIndex`). It goes when the pointer leaves, the control is pressed, or focus moves. A dim control
  shows its tip too. The box is ordinary text, so `screen.text` reads it when shown; no format change.
- `editor/tips.rs` (new, Bevy-free): `field(kind, list, label) -> &'static str` and
  `sections(kind)`; tips for tools, palette entries (edge, height, stamp, terrain swatch), the bar, file
  buttons and the form's Apply/Cancel/Remove/Add row. Each field's tip says what it means, its range, and
  what empty means. ASCII only. The full text is listed in the plan document for the owner to correct.

## Steps (one commit each; tests red first; mutation checks; fmt, clippy, the gate against 1326, Sentrux)

0. **Documents.** This plan to `tasks/plans/editor-forms-and-tips.md` (with the tip texts) and
   `tasks/TODO.md`; the lesson in `tasks/LESSONS.md`; if `presentation-PRD.md` or
   `presentation-ARCHITECTURE.md` describes the kit's controls, a line for tooltips and the tick box
   (flagged in the commit as approved by this plan).
1. **Face.** Data, geometry, the form's choice, fixture and reseal, api.md, schema. Tests: a hatched block
   has the verticals on four sides and a plain one none; the old RON loads as `Plain` and re-serialises
   unchanged; the form keeps `face` across an edit. Mutations: face ignored; hatching on plain. Captures at
   Low then High of the hatched fixture block.
2. **Aligned rows and the tick box.** Kit, drawing, `Form::rows`, sections. Headless tests by `screen_text`
   rectangles: every top field's label starts at one x and every input at one x; a label is not inside its
   input's box; a tick box turns on one press; typing keeps its caret across a redraw (existing tests stay
   green). Mutations: label column width dropped; flag press ignored.
3. **The tooltip facility.** `Control.tip`, `shell/tips.rs`. Tests: no tip before the rest time; shown
   after; gone on leave and on press; the focused field shows its; the box lies inside the window at the
   column's edge. Mutations: delay skipped; not removed on leave.
4. **The editor's tip text.** `editor/tips.rs` wired into every form, the tool grid, palette, bar and
   files. Test: every control the editor draws (each form kind opened on the fixture, each tool) has a
   non-empty ASCII tip. Mutation: one tip emptied.
5. **Documents.** code-map, `tasks/acceptance/editor-forms-and-tips.md`, TODO review, CONTINUITY.

Reported as "built, please try".

## Verification
- Per commit: `cargo fmt --all`; workspace clippy; `scripts/verify.sh` to a scratchpad log, committed only
  on `VERIFY-GREEN`, the count read as a number; Sentrux `rescan` and `check_rules`.
- Layout and tips: headless tests through `screen_text` rectangles; a capture of the editor with the
  terrain form open if the capture flags reach it, otherwise I say it was not seen.
- By hand (owner): `just editor`, open the trees terrain, see the grouped two-column form, rest the pointer
  on `climb` and on a tool icon, set `face` to hatched, Apply, and see the trees hatched in the preview and
  in `just qa`.

## Risks, said plainly
- **Hover in headless tests**: the repo's tests press by inserting `Pressed`; hover is Bevy picking's
  `Hovered`. If a test cannot drive it faithfully, the timing logic is tested Bevy-free and the wiring is
  checked by hand, and I say so.
- **`Popover` is new to this crate**; I read its source first. If it cannot hang from a control inside a
  scrolling column (clipping), the alternative (a root-level box placed from the control's rectangle) is
  asked before it is built.
- **The focused field's tip** may feel noisy while typing down a form; it is one constant to turn off, and
  the acceptance sheet asks.
- Forty-odd tip texts are my wording of your rules; expect corrections.
- Every block-hatched map gains four to twelve lines a block cell; part 5 (instancing) is where line count
  is dealt with.

## Not in this plan
Tips on the game's own screens; the new layout on Settings and other screens; a `[tip]` listing in
`screen.text` for agents; the fight screen, minimap and HUD on the palette (still next, then part 4); the
five decisions from part 3's sheet, which stay open; `packs/the_lower_kingdoms` (the owner's).

## The tip texts (step 4)

Generated from `crates/omnis-vector/src/editor/tips.rs`, which is where a wording is corrected. Each is
shown beside its control when the pointer rests there, or while its field holds the keyboard.


### Form fields

- **Terrain: glyph** - The one character that stands for this terrain in the map's layout. Not a space, and not one another terrain of the map uses.
- **Terrain: name** - The terrain's name in the editor and in rest events. One word.
- **Terrain: floor** - The tileset surface drawn as its ground. Needed.
- **Terrain: ceiling** - The tileset surface overhead. Empty is open sky.
- **Terrain: block** - The tileset surface of a solid block that fills the cell: rock, trees, a pillar. Empty is no block.
- **Terrain: passable** - Whether the party may step onto it.
- **Terrain: opaque** - Whether it blocks the line of sight: rock and dense trees do, water does not.
- **Terrain: visibility depth** - How many cells the party sees while standing on it.
- **Terrain: step minutes** - The minutes one step onto it costs. 0 is free.
- **Terrain: climb** - The largest height difference a step onto it may cross, in eighths of a cell (a wall is 8). A larger difference is a cliff.
- **Terrain: join** - How a cell of it joins its level to the next: stairs up or down, a ladder, a hole. None for plain ground.
- **Terrain: role** - What its lines mean in the 3D view, and so their colour from the biosphere's palette. Default is ground underfoot and structure where it is a block.
- **Terrain: face** - How its block is drawn in the 3D view: plain is its box alone, hatched adds a cliff's upright lines, which tells trees or rock from a wall.
- **Terrain: red | green | blue** - Its colour on the automap and on this plan, 0 to 255. The 3D view takes its colour from the role.
- **Portal: id** - The portal's name, which another portal's link names. Unlike any other portal's on any loaded map.
- **Portal: link** - The id of the portal this one leads to; choose it in the list under the field. Empty is a portal that leads nowhere yet.
- **Portal: facing** - The way the party faces coming out of it: it arrives on the cell ahead of the portal, facing this way.
- **Portal: marker** - The tileset object drawn on the portal's cell: stairs, a signpost, a gate.
- **Encounter: id** - The encounter's name on this map. One word; encounters and objects share their names, so no object may have it.
- **Encounter | Object | Lore: x** - Its cell's column, counted from 0 at the west side. Changing it moves it.
- **Encounter | Object | Lore: y** - Its cell's row, counted from 0 at the north side. Changing it moves it.
- **Encounter: disposition** - How the creatures meet the party: hostile attack on sight, wary fight when pressed, neutral can be bought or slipped past, friendly let it pass.
- **Encounter: once** - Ticked, once the party clears it, it is gone for the rest of the game.
- **Encounter: stands apart** - Ticked, the creatures stand in view at the place the three fields under this give; unticked, they are unseen until the party steps onto the cell.
- **Encounter: stand x** - With stands apart: how far east of the cell's middle they stand, in eighths of a cell, -4 to 4.
- **Encounter: stand y** - With stands apart: how far south of the cell's middle they stand, in eighths of a cell, -4 to 4.
- **Encounter: facing** - With stands apart: the way they face.
- **Encounter (list row): creature** - The id of a monster of the loaded packs, like base:monster:goblin.
- **Encounter (list row): count** - How many of that creature, 1 to 255.
- **Random: chance percent** - The chance in a hundred that a step onto a cell of this map meets one of the entries. 0 is never.
- **Random (list row): weight** - How often this entry is the one met, against the other entries' weights.
- **Random (list row): disposition** - How these creatures meet the party: hostile attack on sight, wary fight when pressed, neutral can be bought or slipped past, friendly let it pass.
- **Random (list row): creatures** - The creatures and how many, each an id and its dice, separated by commas: base:monster:goblin 1d4, base:monster:wolf 1.
- **Site: kind** - The kind of activity. A service's site is the one kind so far.
- **Site: service** - The service the party finds on this cell: an inn, a temple, a smith.
- **RestEvent: terrain** - The terrain the party must be resting on for this to happen.
- **RestEvent: rest** - The kind of rest it may happen in: a short one, a long one, or any.
- **RestEvent: chance per mille** - The chance in a thousand that it happens in one such rest. 0 is never.
- **RestEvent: text** - The text key of what the party is told when it happens.
- **Object: id** - The object's name on this map. One word; objects and encounters share their names, so no encounter may have it.
- **Object: kind** - What it is: an object kind of the loaded packs, which gives its shape, its states and what can be done with it.
- **Object | Ground: pos x** - How far east of the cell's middle it lies, in eighths of a cell, -4 to 4.
- **Object | Ground: pos y** - How far south of the cell's middle it lies, in eighths of a cell, -4 to 4.
- **Object: facing** - The way its front faces.
- **Object: state** - The state it starts in, one of its kind's. Empty is its kind's own first state.
- **Object: hidden** - The difficulty of finding it, 1 to 30. 0 is in plain sight.
- **Object: name** - The text key of a name of its own. Empty is its kind's name.
- **Object: lore** - The text key of what a search of it may tell. Empty is nothing to learn.
- **Object: lore DC** - The Investigation roll its lore needs, 1 to 30. At 0 every search tells it.
- **Object: lore once** - Ticked, its lore is told on the first search that meets it and never again.
- **Object | Ground (list row): item** - The id of an item of the loaded packs, like base:item:torch.
- **Object (list row): count** - How many of that item it holds, 1 to 65535.
- **Ground (list row): count** - How many of that item lie there, 1 to 65535.
- **Lore: id** - The lore's name on this map. One word, unlike any other lore's here.
- **Lore: text** - The text key of what a search of this cell may tell, one of the loaded packs' keys.
- **Lore: DC** - The Investigation roll it needs, 1 to 30. At 0 every search tells it.
- **Lore: once** - Ticked, it is told on the first search that meets it and never again.
- **LevelAdd: level** - The new level's number: above the ground is 1 and up, under it -1 and down. The ground is 0.
- **LevelAdd | Level: name** - The text key of the level's name. Needed.
- **LevelAdd | Level: base** - The height added to every cell of the level, in eighths of a cell (a wall is 8). Negative for a level under the ground.
- **Note: note** - Your own note on this place. Kept in the pack's notes, never shown in the game; written at once, outside undo.

### Tools

- **Select** - Select (V): a click puts the cursor on a cell and paints nothing. The inspector shows what is there.
- **Brush** - Brush (B): a click paints the palette's entry on the cell or the side under the pointer; a drag is one stroke and one undo step.
- **Move** - Move (M): drag the selection, or the thing under the pointer, to another cell. A drag on nothing pans the plan.
- **Lasso** - Lasso (L): drag a loop; the cells inside it are selected. A click clears the selection.
- **Fill** - Fill (F): a click fills the region of one ground under the pointer with the palette's entry, stopping at every closed side.
- **Corners** - Run by corners (P): click corner after corner, then the last one again to take the run: walls, doors or ramps along it, or ground and height inside it.
- **Freehand** - Freehand run (N): drag a run; it is taken when the button is let go.
- **Eraser** - Eraser (X): a click opens the side, removes the first thing placed on the cell, or clears its ground.

### Palette entries

- **Terrain(_)** - A terrain of this map: the brush paints it as a cell's ground. Edit terrain under the palette changes what it is.
- **Edge(Wall)** - Wall: closes the side of a cell under the pointer.
- **Edge(Door)** - Door: a door in the side of a cell under the pointer.
- **Edge(Ramp)** - Ramp: a walkable slope across the side of a cell, between two heights that would be a cliff.
- **Edge(Open)** - Open edge: takes a wall, door or ramp off a side.
- **Height** - Height: sets a cell's height to the eighths chosen under the palette (a wall is 8 high).
- **Stamp(Portal)** - Portal: a way to another place. Select... chooses the portal it links to.
- **Stamp(Encounter)** - Encounter: creatures on a cell. Select... chooses the monster.
- **Stamp(Activity)** - Activity: a town service's site. Select... chooses the service.
- **Stamp(Object)** - Object: a thing standing on a cell. Select... chooses its kind.
- **Stamp(Ground)** - Ground pile: items lying on a cell. Select... chooses the item.
- **Stamp(Lore)** - Lore: something a search of the cell may tell. Select... chooses its text.

### Buttons, choices and icons

- **Map** - The pack's maps. Step to one, then press Open.
- **Open** - Open the chosen map. Unsaved changes to the map now open are dropped.
- **Level** - The level shown on the plan and edited. 0 is the ground.
- **ZoomIn** - Zoom the plan in (]). The wheel over the plan zooms too.
- **ZoomOut** - Zoom the plan out ([).
- **Centre** - Put the cursor at the plan's middle (C).
- **FaceLeft** - Turn the cursor's facing a quarter left (Q). A placement takes the cursor's facing, and an edge is painted on the side it faces.
- **FaceRight** - Turn the cursor's facing a quarter right (E).
- **TakeRun** - Take the run: what it draws is made, in one undo step.
- **DropCorner** - Take back the run's last corner.
- **DropRun** - Give the run up; nothing changes.
- **Eighths** - The height the brush sets, in eighths of a cell. A wall is 8 high; a step of more than a terrain's climb is a cliff.
- **Stamp | Pick** - Choose what the stamp places: opens the list of the loaded packs' own.
- **Props** - Set what each thing the stamp places starts with: its place on the cell, its state, its counts.
- **Unlink** - Clear the link: the portal leads nowhere until it is linked.
- **CellTerrain** - The terrain of the cursor's cell. Steps through the map's terrains and no ground.
- **CellHeight** - The height of the cursor's cell, in eighths of a cell. Steps by one.
- **CellRise** - The way the stair on the cursor's cell climbs.
- **SetStart** - Put the map's start on the cursor's cell, facing as the cursor does.
- **EditAt(_)** - Open this thing's form.
- **RemoveAt(_)** - Remove this thing from the cell. One undo step.
- **Random** - The map's random encounters: the chance at each step and what may be met.
- **RestAdd** - Add something that may happen while the party rests on a terrain.
- **RestEdit(_)** - Open this rest event's form.
- **RestRemove(_)** - Remove this rest event. One undo step.
- **ListAdd(_)** - Add a row to the list.
- **ListRemove(..)** - Take this row out of the list.
- **Apply** - Make the form's change. One undo step.
- **Cancel** - Close the form; nothing changes.
- **Discard** - Remove what the form edits, and close it.
- **TerrainAdd** - Add a terrain to this map's list.
- **TerrainEdit** - Change the chosen terrain: its tiles, how it is walked, its look and colour.
- **LevelAdd** - Add a level over or under the ground.
- **LevelEdit** - Change the shown level's name and base height, or remove it.
- **Note(Cell)** - Your own note on the cursor's cell. Never shown in the game.
- **Note(Level)** - Your own note on the shown level. Never shown in the game.
- **Note(Map)** - Your own note on the map. Never shown in the game.
- **Undo** - Undo the last step (Cmd Z).
- **Redo** - Do the undone step again (Shift Cmd Z).
- **History** - The list of every step made; a click goes back to it.
- **Rules** - Every rule the loaded packs set: its value, the pack it came from and what it replaced.
- **Save** - Write the map to its file and reseal the pack (Cmd S).
- **Revert** - Open the map's file again, dropping every unsaved change.
- **Menu** - The editor's menu: back to the title, or quit (Esc).
- **Playtest** - Start a game on this map at the cursor, as the editor holds it, saved or not. The map is not changed and the game is never saved.
- **Plan** - Show the map's plan.
- **Files** - Show the open file's text.
- **OpenFile** - Open one of the pack's files as text.
- **Check** - Ask the loader whether the text is a file it takes; nothing is written.
- **WriteFile** - Write the text to the file exactly as typed and reseal the pack.
- **RevertFile** - Put the text back as the file holds it.
