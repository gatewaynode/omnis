# Acceptance: the entrances and exits enhancement, step 1 (the rules)

Step 1 of the owner's entrances and exits enhancement (`tasks/TODO.md`, W1; scope fixed 2026-11-01). The loader,
and so the editor, now refuses: a portal landing the party cannot stand on (no ground, terrain it cannot enter, a
portal standing there; an object across the centre as before); a portal without its way back **directly behind
its landing** (the owner's strict mirror: the party arrives facing away from the way back); a start off ground
the party can enter; a map of the pack that sets `entry` that has portals yet cannot be reached from the entry
map (a map with no portals is unfinished, not unreached; the test fixtures' maps belong to other packs and are
exempt).

**The shipped pack moved** (0.2.4): the stairs down into the dungeon and into the depths both landed at (1, 0)
facing south with the stairs back up at (0, 0) to the side; they now land facing east, away from the way back.
Nothing else in the maps changed.

**A Save reseals the pack on disk.** Hand-test on a fresh scratch copy: `rm -rf /tmp/omnis-scratch && cp -R
packs/test /tmp/omnis-scratch`, named as the last pack.

## Steps
1. `omnis-cli validate packs/base packs/test`. *You will see* `ok: 2 packs, 4 maps, ...`.
2. `omnis-cli validate packs/base packs/test crates/omnis-data/tests/packs-bad/bad-ways`. *You will see* seven
   errors: a start on no ground, a start the party cannot enter, a landing on rock, on no ground, where a portal
   stands, a portal with no way back, and `map 'ways:map:foot' cannot be reached from the entry map
   'ways:map:stand' through portals`.
3. Editor, the town (the scratch copy). Choose the portal stamp (meadow), click a street cell such as (5, 2).
   *You will see* the mark and `Portal to test:map:meadow placed at (5, 2); the loader says: data/maps/town.ron:
   portal at (5, 2) has no way back: no portal behind its landing (16, 16) on 'test:map:meadow' leads to
   'test:map:town'`; Save refused with the same words. Undo. (Step 2 makes the stamp write the pair.)
4. Open the dungeon. Choose Move, drag the stairs up at (0, 0) one cell east. *You will see* `... the loader
   says: data/maps/dungeon.ron: test:map:meadow: portal at (16, 5) has no way back: ...` (the meadow's stairs
   down have lost their return). Undo.
5. New game, walk out of the town's gate, up the road and down the stairs. *You will see* the dungeon with the
   stairs up on your left: you arrive facing east. The automap shows the way back beside you.

Covered by `omnis-data/tests/resolve.rs` (a landing on rock and on a portal; no way back from both maps; a
start on rock; a cut-off map with its orphaned way back), `bad_packs::every_entrance_error_is_reported` (the
`bad-ways` corpus, seven rows), `load_test_pack` and `movement::portals_link_the_maps_both_ways` (the east
arrival), `movement::a_portal_whose_landing_cannot_be_stood_on_is_a_plain_tile`, `binding::the_portal_snaps_
the_pose_into_the_dungeon`, `editor_stamp` and `editor_pick` (a lone portal stamp says it has no way back).

## Known and expected
- A portal placed by the stamp has no way back until step 2 writes the pair; Save is refused meanwhile.
- A landing on a service site stays legal: arriving there enters the service, as a step does.
- The two re-faced arrivals move the golden replays (re-baselined) and the fight route turns right after the
  stairs; the frozen schema 8 and 9 saves still load and are compared with a fresh replay only when the packs are
  the ones they were made with (`report.fit`).
