# Acceptance: the editor's left column as a drawing GUI, step 3 (placements as palette stamps)

Step 3 of the owner's drawing-GUI column (`tasks/TODO.md`, W1; steps 1 and 2 accepted 2026-10-25): the palette
holds a stamp per placement kind and a click places one. Owner's choices in the planning (2026-10-25): **a
stamp's click places at once** (one undo step; the inspector's Edit opens the form for the rest); **one detail
per stamp** chosen under the palette (portal: the map it leads to; encounter: the monster; activity: the
service; object: the object kind; ground pile: the item; lore: the text key); **the inspector's Add row goes**
(Edit and Remove on the listed placements stay); **a stamp on a cell that already holds its kind is refused**
and says why.

**Hand-tested by the owner 2026-10-31 and sent back with a direction** (the detail's choice box): built as step 3b
(`tasks/acceptance/editor-column-3b.md`) and **accepted with it the same day**.

**A Save reseals the pack on disk.** Hand-test on a fresh scratch copy: `rm -rf /tmp/omnis-scratch && cp -R
packs/test /tmp/omnis-scratch`, named as the last pack. The object stamp needs a pack with object kinds (the
shipped packs have none): add `crates/omnis-sim/tests/packs/objects` to the launch and open `objects:map:yard`
for step 5, or expect "nothing to stamp: the packs have no object kind" on the town.

## What was built
- **Six stamps** end the palette after the height: a door (portal), swords (encounter), the grinning face
  (activity), a package (object), layers (ground pile), a scroll (lore). Choosing one chooses the brush; the
  details under the palette read "<kind>: ..., placed with a click" and hold the stamp's **detail choice**
  (to map, monster, service, kind, item, text), stepped with its arrows or a press, over everything the loaded
  packs have of it; dim with "the packs have no <what>" when they have none.
- **A click places at once**, one undo step, the cursor on the cell, the message "<what the inspector lists>
  placed at (x, y)" (with the loader's first word appended when it has one, as the forms' Apply says). The
  placement is built by the kind's own form, so **every new placement starts valid**: a free id (`encounter_x_y`,
  `object_x_y`, `lore_x_y`; objects and encounters share one id space), a portal landing at the chosen map's
  start with a marker of the map's tileset (the marker is now a choice in the form), one of the chosen item
  in a pile, a defined text key on a lore.
- **Refused where its kind already is**: "a portal is already at (x, y)", "a site is already at (x, y)", "a
  ground pile is already at (x, y)", "cell (x, y) already has an encounter"; nothing changes.
- **A drag places once** (the press); the stamp is a click, like the fill.
- **A right click on a placement picks its stamp and its detail** (the inn's cell: the activity stamp with the
  inn's service; the signpost: the portal stamp with the meadow); a bare cell picks its terrain as before.
- **The inspector's Add row is gone.** The listed placements keep Edit and Remove.
- **Fixed on the way**: editing a placement under a new id (Edit, type another id, Apply) left the old entry
  beside the new one; the loader then refused two encounters on a tile. The edit now removes the old entry in
  the same undo step (`Place::key`; lesson in `tasks/LESSONS.md`).
- The forms' Apply now always says what it did ("Placed", "Changed", "Written"), the loader's word appended
  when there is one; before, it spoke only when the loader had one.

## Steps (in the window)
1. Editor, open the town. *You will see* the palette end in six more icons: a door, swords, a grinning face, a
   package, layers, a scroll. *You will see* no "Add" row in the inspector.
2. Press the grinning face. *You will see* the brush chosen, "Activity: a service's site, placed with a click"
   and a "service" choice under the palette; step it to `base:service:tavern`. Click a street cell (row 2).
   *You will see* the beer icon on the cell, "Service base:service:tavern placed at (x, 2)", the inspector
   listing it with Edit and Remove. Undo takes it back in one step. Click the inn's cell (1, 1) with it. *You
   will see* "a site is already at (1, 1)" and the inn unchanged.
3. Press the swords, click a street cell. *You will see* the encounter's triangle, "Encounter encounter_x_2
   placed at (x, 2)"; press Edit in the inspector. *You will see* its form: the id, one stack of the chosen
   monster. Cancel. Click the same cell again. *You will see* "cell (x, 2) already has an encounter".
4. Press the door, step "to map" to `test:map:meadow`, click a street cell. *You will see* the portal mark,
   "Portal to test:map:meadow placed at (x, 2)"; its form (Edit) shows to x, to y and facing as the meadow's
   start and "marker" as a choice (signpost among the options). Press the layers, then the scroll, each clicked
   on a street cell. *You will see* each placed, and Save stays lit (the loader takes the map).
5. (With the objects pack, on the yard.) Press the package, click a bare cell. *You will see* "Object
   object_x_y (<kind>) placed at (x, y)". On the town instead: *You will see* "nothing to stamp: the packs have
   no object kind" and the choice under the palette dim.
6. Right click the smith's cell (7, 1). *You will see* the grinning face framed and "service" on
   `base:service:smith`; right click a bare street cell. *You will see* the street swatch framed as before.
7. With a stamp chosen, drag across three street cells. *You will see* one placement, not three.
8. Press Edit on an encounter you placed, type another id, Apply. *You will see* one encounter on the cell
   under the new id, not two, and Save still lit.
9. Save. *You will see* "sealed at 0.2.1" (or the next patch).

Covered by `crates/omnis-vector/tests/editor_stamp.rs` (seven tests: the activity stamp and its refusal, the
encounter's free id and second-on-cell refusal, the pile, lore and portal stamps valid as they land, the object
stamp on the objects pack and the shared id space, a drag placing once, the right click's pick, the tools'
entries and the missing Add row), the rewritten Add-driven tests in `tests/editor_forms.rs`, `brush.rs`'s
palette test and `omnis-data/tests/edit_document.rs::a_place_names_the_key_it_is_put_by`. Mutation checks red:
the site line dropped from `Form::keys`, a stamp keeping the stroke open, an object's id minted over objects
alone.

## Known and expected
- The digits reach the first nine palette entries only; a stamp has no key (an icon's press or a right click).
- A long detail list (the text keys) steps one by one; a modal picker for it is a later item (TODO).
- The plan's hover is the cell's outline, not the stamp's mark; a preview under the pointer is a later item.
- Sites keep the loader's own rules (not on a portal's or an encounter's tile): the stamp lands and the
  message says what the loader says, as the forms do.
