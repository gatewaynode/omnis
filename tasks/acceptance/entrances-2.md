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
- **The portal's form** (Edit in the inspector): id, link, facing, marker; **Link...** opens the same picker for
  the link field (the current partner shown chosen when it has one), Finish fills the field, **Unlink** clears
  it; Apply is one undo step. **The inspector** lists "Portal 'id' to 'link'" (or "Portal 'id'") with a line
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
   second; press Link... *You will see* the picker with "portal_16_24  -  Meadow (16, 24)" alone (this portal
   is left out; every shipped portal is paired). Press it, Finish: the link field reads "portal_16_24". Apply.
   *You will see* both marks full and "to Meadow 'portal_16_24'". Edit it again, Unlink, Apply: both hollow,
   "unlinked".
8. `omnis-cli validate packs/base crates/omnis-data/tests/packs-bad/bad-ways` (with `packs/test` between). *You
   will see* the thirteen link and landing rows (`bad_packs::every_entrance_error_is_reported`); with `bad-reach`
   in place of `bad-ways`, the one row "map 'reach:map:foot' cannot be reached from the entry map
   'reach:map:stand' through portals".

Covered by `omnis-data/tests/resolve.rs` (seven link tests), `bad_packs.rs` (`bad-ways`, `bad-reach`),
`omnis-sim/tests/movement.rs::an_unlinked_portal_is_a_plain_tile`, `omnis-vector/tests/editor_link.rs` (the two
flows above: the picker across maps with the search and the one-sided save, the form's Link... and Unlink on one
map), `editor_forms`, `editor_stamp`, `editor_pick`, `editor_move`, `pick.rs` unit tests. Mutation checks red: the
inbound claim ignored, the landing behind the partner, reachability over every portal-bearing map, the mark drawn
linked for an unlinked portal (2a); the form's picker offering the edited portal, the stamp keeping its choice,
the field's Finish writing nothing (2b).

## Known and expected
- The `editor.place` Portal payload changed shape (a host-tier op; `PROTOCOL` stays 2; `docs/api.md` changelog).
- A link typed by hand into the form's field is taken as typed; the loader's word follows if it names nothing.
- Two unlinked portals on one map link through the form or the stamp alike; a portal cannot link to itself.
- The inspector's partner line names the map by its display name ("Meadow"), the picker's rows too.
- Step 3 (the start on any level, its facing drawn) follows.
