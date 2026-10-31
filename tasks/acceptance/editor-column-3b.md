# Acceptance: the editor's left column as a drawing GUI, step 3b (a stamp's detail in a modal picker)

Step 3b of the owner's drawing-GUI column (`tasks/TODO.md`, W1), the owner's direction from the hand test of
step 3 (2026-10-31): "the left/right selection box is inadequate, the texts overflow ... that part of the UI
needs an icon indicator on the left of the row and a select button that opens a modal showing a searchable
select list of what is available in that category, multi select should be supported and a button for finish
added at the bottom ... for encounters and objects multiple selection should trigger another modal ... allowing
for setting properties like quantity, starting position, encounter hostility, special loot or other details".
Owner's choices in the planning: **the properties open after every Finish** for the kinds that have them (one
monster still wants its count and disposition), **piles get them too** (a count an item).

What the data model has, and so what the properties offer: an encounter's count a monster, its **disposition**
(the model's word for hostility: Hostile, Wary, Neutral, Friendly), once, and where it stands (stands apart,
stand x, stand y in eighths, facing); an object's place on the cell (pos x, pos y), facing, state, hidden DC,
name and lore, one section a chosen kind; a pile's count an item and its place. **Not in the model, so not
offered: a per-placement loot.** A monster's loot is its definition's loot table and gold; a container object's
items are its contents, in the inspector's Edit.

**A Save reseals the pack on disk.** Hand-test on a fresh scratch copy: `rm -rf /tmp/omnis-scratch && cp -R
packs/test /tmp/omnis-scratch`, named as the last pack. The object steps need a pack with object kinds (the
shipped packs have none): add `crates/omnis-sim/tests/packs/objects` to the launch and open `objects:map:yard`.

## What was built
- **Under the palette, for a stamp:** its icon (framed), **Select...**, and for an encounter, an object or a
  pile a **sliders icon**; under them a line of what the stamp drops ("giant_rat x3, goblin"; "nothing chosen,
  the first: goblin" until something is chosen: the stamp drops the packs' first of its kind meanwhile).
- **Select... (or the icon) opens the picker**, a modal over the editor: "CHOOSE MONSTERS", a search field
  holding the keyboard, "n of N chosen" (N what one cell takes: one map or service, 4 monsters, 16 object
  kinds, 32 items, 16 texts), a row an id marked `[ ]` or `[x]`, Finish and Cancel. Typing filters the rows to
  the ids holding the text, in any case. A click on a row chooses or lets go; a kind that takes one moves its
  choice; at the limit the unchosen rows are dim with "at most N monsters". Finish is dim while nothing is
  chosen. Cancel (or Esc) keeps what the stamp held.
- **Finish holds the choice in the stamp**; what was held carries over for an id kept (a count, the
  disposition, an object's place). For an encounter, an object or a pile it goes on to the **properties**: a
  modal of the kind's own form rows without the fields a click decides (the id and the cell), a count beside
  each monster or item, the group's disposition, once and stand, each object's place and the rest under its
  kind's heading. Finish holds them; Cancel keeps what was held. The sliders icon reopens them.
- **A click drops it all**, one undo step, through the kinds' own forms: ids minted apart (`object_5_3`,
  `object_5_3_2`), the message naming each ("Object object_5_3 (chest), Object object_5_3_2 (lever) placed at
  (5, 3)"), refused as before where a portal, a site, a pile or an encounter already is.
- **A right click on a placement copies it whole into its stamp** (an encounter with its stacks and
  disposition; every object on the cell); the picker then shows its ids chosen.

## Steps (in the window)
1. Editor, the town, press the swords. *You will see* under the palette the swords icon framed, "Select...", a
   sliders icon, and "nothing chosen, the first: <the packs' first monster>".
2. Press Select.... *You will see* "CHOOSE MONSTERS" over the editor: a search field with the caret, "0 of 4
   chosen", a row a monster, Finish (dim) and Cancel. Type `rat`. *You will see* only the ids holding "rat".
   Click two. *You will see* `[x]` on them and "2 of 4 chosen". Clear the search, click two more. *You will see*
   "4 of 4 chosen" and the rest dim. Click one back off, then press Finish.
3. *You will see* "ENCOUNTER PROPERTIES": a count beside each monster, disposition, once, stands apart, stand x,
   stand y, facing. Step the first count to 3 and disposition to Friendly. Press Finish. *You will see* the
   editor again and the line under the palette naming the three, the first "x3".
4. Click a street cell (row 2). *You will see* the triangle and "Encounter encounter_x_2 placed at (x, 2)";
   Edit in the inspector shows the stacks with their counts and Friendly. Press the sliders icon. *You will
   see* the properties again, as set; Cancel.
5. Press the layers, Select..., choose arrows and a dagger, Finish. *You will see* "GROUND PILE PROPERTIES";
   set the counts to 10 and 2, Finish, click a street cell. *You will see* one pile with both; Edit shows them.
6. On the yard with the objects pack: press the package, Select..., choose two kinds, Finish. *You will see*
   "OBJECT PROPERTIES" with a section a kind; set pos x to 2 on the second; Finish; click a bare cell. *You
   will see* two objects, "Object object_x_y (a), Object object_x_y_2 (b) placed at (x, y)".
7. Press the door, Select.... *You will see* "CHOOSE MAPS", "1 of 1 chosen" once one is clicked; clicking
   another moves the mark. Press Esc. *You will see* the editor; the line under the palette says what was
   held before. Press the scroll (lore), Select..., choose two texts, Finish, click a street cell. *You will
   see* two lore entries listed in the inspector.
8. Right click the smith's cell (7, 1). *You will see* the grinning face framed and "smith" under the palette;
   Select... shows the smith's service chosen. Right click a bare street cell. *You will see* the street swatch
   framed as before.
9. Save. *You will see* "sealed at 0.2.1" (or the next patch).

Covered by `crates/omnis-vector/tests/editor_pick.rs` (seven tests: the row and the picker's search, marks,
limit and Cancel; the properties and the drop as set, then the sliders; two object kinds minted apart with their
places; a pile's counts and two lore entries; a portal's moving choice and the dim Finish; a right click's copy;
the search field keeping the keyboard through the rows' rebuild), the rewritten `tests/editor_stamp.rs`, and
`editor/pick.rs`'s unit tests. Mutation checks red: the limit check dropped, the minted ids not added to
`taken`, the filter compared case-sensitively.

## Known and expected
- The properties modal shows every field of the kind's form but the id and the cell: an object's state, name
  and lore too, as text fields. A per-placement loot is not in the data model.
- One object a chosen kind in a click; several of one kind are several clicks (a quantity for a kind is a later
  item).
- The picker's rows are the full ids (`test:monster:giant_rat`); the line under the palette shows the last
  segment.
- The hover stays the cell's outline, not the stamp's mark; a preview under the pointer is a later item.
