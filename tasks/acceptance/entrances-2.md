# Acceptance: the entrances and exits enhancement, step 2 (linked portals: the model, and linking in the editor)

Step 2 of the owner's entrances and exits enhancement (`tasks/TODO.md`, W1), **re-planned 2026-11-01 on the owner's
direction** after the step 1 hand test showed that no portal pair could be authored in the editor (each half was
refused until the other was on disk): "an intermediary state for portals, 'unlinked', that does not display the
portal or allow its use until it is linked to another portal on the same or a different map"; "a freehand
portal_id text field, so editors can match up available portals to link to with ease (best if you can search
through all available portals)". Owner's choices: **a link stated by one side is enough** (the editor never
writes a second map); **the landing is derived from the partner** (the party comes out on the cell ahead of the
partner, facing its way: the strict mirror by construction); **ids are unique across the world**.

Built in two commits: 2a the model (`232cc839`), 2b linking in the editor (this one).

**A Save reseals the pack on disk.** Hand-test on a fresh scratch copy: `rm -rf /tmp/omnis-scratch && cp -R
packs/test /tmp/omnis-scratch`, named as the last pack.

## What was built
- **A portal** is `{level, x, y, id, link, facing, marker}`. `id`: freehand, 1 to 32 printable ASCII characters,
  one across every loaded map. `link`: the id of the portal it leads to, or none. `facing`: the way the party
  comes out of it. The authored landing (`to_map`, `to_x`, `to_y`, `to_facing`) is gone from the files; the
  shipped pack is 0.2.5 with six ids and three one-sided links (the town's gate, the meadow's stairs, the
  dungeon's stairs down), every landing as it was.
- **Linked or unlinked.** A portal is linked when its link names a partner or one portal anywhere names it. The
  loader refuses a link to nothing loaded, to itself, to a portal that links a third, and a portal two portals
  claim; a linked portal's landing (ahead of its partner) must be inside the map, on ground the party can enter,
  with no portal and no blocking object there. An unlinked portal is legal: saved, drawn on the plan as the
  rectangle alone (a linked one has the circle too), and a plain tile in play (no floor mark, no automap mark,
  a step onto it is a step). Reachability runs over linked pairs, asked only of a world whose links are sound.
- **The portal stamp** with nothing chosen places an unlinked portal: a fresh id (`portal_x_y`, free across the
  loaded maps), facing the author (the cursor's opposite, so the party comes back out toward where you stood),
  the tileset's first marker. **Select...** opens the picker over the portals nothing links to, the open map's
  unsaved ones among them, each as "id  -  Map (x, y)"; the search matches the id, the map's name or the cell.
  With one chosen, the click places a portal linked to it and lets the choice go (the partner is taken).
- **The portal's form** (Edit in the inspector): id, link, facing, marker. **Under the link field, the list**
  (step 2c, owner 2026-11-07, "always listed while editing"): every portal nothing links to across the loaded
  maps, this one left out, and the current partner, each a row "[ ] id  -  Map (x, y)", narrowed by what is
  typed in the field (id, map name or cell, any case); the field's text as a whole id marks its row "[x]" and
  shows the whole list, so a chosen link still lets you switch; at most 10 rows, then "and N more: type to
  narrow"; "No portal matches." / "No portal to link: stamp one on any map.". A row pressed fills the field,
  the marked one pressed clears it, **Unlink** clears it; Apply is one undo step. (Link... and its modal are
  gone.) **The inspector** lists "Portal 'id' to 'link'" (or "Portal 'id'") with a line
  under it: "to Meadow 'meadow-gate'" whichever side states the link, "link 'x' unresolved: see the errors", or
  "unlinked". A right click on a portal copies it into the stamp without its link.
- **Move** keeps a portal's id and link. **Removing** a portal another names is the loader's error on this map
  with the other map's id ("test:map:meadow: portal 'x' at (16, 20) links to 'y', which no loaded map has").

## Steps (in the window, the scratch copy)
1. Editor, the town. Turn the cursor to face west (the arrows on the bar), choose the portal stamp, click the
   street at (5, 2). *You will see* a hollow rectangle on (5, 2), "Portal 'portal_5_2' placed at (5, 2)", the
   inspector "Portal 'portal_5_2'" and "unlinked" under it, Save lit. Save: "Saved data/maps/town.ron; test
   sealed at 0.2.6".
2. Open the meadow. Choose the portal stamp, press Select... *You will see* CHOOSE PORTALS with one row,
   "portal_5_2  -  Town (5, 2)". Type "town" in the search: the row stays; type "x": "No portal matches."
   Clear it, press the row, Finish. *You will see* the palette's detail "portal_5_2".
3. Turn the cursor to face west, click the road at (16, 20). *You will see* the full mark (rectangle and circle),
   "Portal 'portal_16_20' to 'portal_5_2' placed at (16, 20)", the inspector "to Town 'portal_5_2'", and the
   palette's detail back to "unlinked". Save: 0.2.7.
4. Open the town. *You will see* the mark on (5, 2) full now; click it with Select: the inspector "Portal
   'portal_5_2'" with "to Meadow 'portal_16_20'" under it (the town's file says nothing: the meadow's states the
   link).
5. Press Remove on it. *You will see* "1 error" on Save and Playtest and the inspector's error "test:map:meadow:
   portal 'portal_16_20' at (16, 20) links to 'portal_5_2', which no loaded map has". Undo.
6. New game, out the gate, up the road to (16, 20) and step onto it. *You will see* yourself on the town's street
   at (6, 2) facing east, the new portal's mark behind you to the west. Step back onto (5, 2): the meadow's road
   at (17, 20) facing east.
7. Editor, the meadow. Choose the portal stamp, click (16, 24) and (16, 26) (both unlinked). Press Edit on the
   second. *You will see* under the link field the row "[ ] portal_16_24  -  Meadow (16, 24)" (this portal is
   left out; the pack's other unlinked portals are listed too). Type "town" in the link field: the meadow's
   rows go ("No portal matches." if nothing else fits); clear it. Press the row: the field reads
   "portal_16_24", the row "[x]". Apply. *You will see* both marks full and "to Meadow 'portal_16_24'". Edit it
   again: the row is marked; Unlink, Apply: both hollow, "unlinked".
8. `omnis-cli validate packs/base crates/omnis-data/tests/packs-bad/bad-ways` (with `packs/test` between). *You
   will see* the thirteen link and landing rows (`bad_packs::every_entrance_error_is_reported`); with `bad-reach`
   in place of `bad-ways`, the one row "map 'reach:map:foot' cannot be reached from the entry map
   'reach:map:stand' through portals".

Covered by `omnis-data/tests/resolve.rs` (seven link tests), `bad_packs.rs` (`bad-ways`, `bad-reach`),
`omnis-sim/tests/movement.rs::an_unlinked_portal_is_a_plain_tile`, `omnis-vector/tests/editor_link.rs` (the two
flows above: the picker across maps with the search and the one-sided save, the list under the form's link
field and Unlink on one map, a link switched to another portal kept through the window's frame), `editor_forms`, `editor_stamp`, `editor_pick`, `editor_move`, `pick.rs` unit tests. Mutation checks red: the
inbound claim ignored, the landing behind the partner, reachability over every portal-bearing map, the mark drawn
linked for an unlinked portal (2a); the form's picker offering the edited portal, the stamp keeping its choice,
the field's Finish writing nothing (2b).

## Known and expected
- The `editor.place` Portal payload changed shape (a host-tier op; `PROTOCOL` stays 2; `docs/api.md` changelog).
- A link typed by hand into the form's field is taken as typed; the loader's word follows if it names nothing.
- Two unlinked portals on one map link through the form or the stamp alike; a portal cannot link to itself.
- The inspector's partner line names the map by its display name ("Meadow"), the picker's rows too.
- Step 3 (the start on any level, its facing drawn) follows.

## Found in the owner's hand test (2026-11-07)
- **The palette's portal stamp never linked**: the picker's Finish, pressed with the mouse, closed the modal
  and the plan took the same button press beneath it (the screens' router runs first in the frame), so the
  stamp dropped its linked portal under the Finish button and the owner's click placed an unlinked one (the
  stray portals at x = 23 on the meadow and the dungeon are these). Fixed: a press on the frame a screen closes
  is that screen's (`shell/editor.rs` `route`; `editor_link.rs` presses Finish and Cancel with a real mouse
  button over the plan: nothing placed, the choice kept, the next click linked; `tasks/LESSONS.md`).
- The form's Link... and Apply were never at fault: the earlier report of 2026-11-01 had the same cause.

## Step 2c (2026-11-07): the list under the link field
The owner's direction after the leak fix: "an autocomplete function on the form, so that the global list provides
a drop down of selectable portal locations to link"; chosen: **always listed while editing**. Built as above
(`editor/links.rs`; the picker is the stamp's alone again). Found and fixed with it: a link switched from one
portal to another was undone by the window's frame (the drawn field was never redrawn when the column's shape
stood); every text the model sets in a field now draws the field anew (`Form::set_text`, `tasks/LESSONS.md`).

## Accepted
**Steps 1, 2 and 2c accepted by the owner 2026-11-07** ("I think this is usable, how close are we to closing out
the portals work?"; "That fixed it, accepted"), after the hand tests of 2026-11-01 and 2026-11-07 on their own
world. Between the word and the acceptance the same day: the tests moved to the frozen fixture
`crates/omnis-sim/tests/packs/test` and the owner's canvas became `packs/the_lower_kingdoms` (`741171c9`,
`2c2bd3a7`), and the longer pack name showed two overflows in the editor's tools column, fixed before the
acceptance: the pack root on its own row under the title (`d8f53d94`) and the map chooser naming a map by its
last segment (`74ad5f74`). Step 3 (the start as a real entrance) follows.
