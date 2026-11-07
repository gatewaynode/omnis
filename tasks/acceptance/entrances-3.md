# Acceptance: the entrances and exits enhancement, step 3 (the start as a real entrance)

Step 3 of the owner's entrances and exits enhancement (`tasks/TODO.md`, W1; scope fixed by the owner 2026-11-01:
"the start as a real entrance"). Built 2026-11-07 in one commit after steps 1, 2 and 2c were accepted the same day.

**A Save reseals the pack on disk.** Hand-test on a fresh scratch copy of the `levels` fixture, never the fixture
itself: `rm -rf /tmp/omnis-levels && cp -R crates/omnis-sim/tests/packs/levels /tmp/omnis-levels`.

## What was built
- **The start is on a level.** `MapDef.start_level` beside the start tuple, the ground when left out, so every
  map file whose start is on the ground is written byte for byte as before (`edit_authoring.rs` still proves the
  round trip). `Place::Start` is `{level, x, y, facing}`, `level` optional (the ground) on the wire:
  `editor.place` takes it, the MCP schema offers it, `PROTOCOL` stays 2.
- **The loader asks the start's level.** A start on a level the map does not have: "start at (x, y) is on level
  N, which the map does not have"; off that level's ground or on terrain the party cannot enter: "start (x, y) on
  level N is on no ground" (the ground's words unchanged); an object may not block the start's centre on its
  level. Corpus `bad-ways` gains `up.ron` and `far.ron`, one row each.
- **A new game begins on the start's level**, facing as the start does; a save whose party can stand nowhere on
  its map falls back to the start on its level (`party_moved`).
- **The editor.** Set start here works on the shown level (the dim "the start is on the ground level" is gone;
  "the start is here" stays). The inspector's row reads "Start (4, 2) on level Upper floor (1) facing north" off
  the ground. The Move tool and a moved selection carry the start on its level. A map opens on its start's level
  with the cursor on the start. The plan draws the start on its level alone, as a diamond **notched toward its
  facing**: the vertex the party walks out of runs on to the cell's side. A level removed returns a start on it
  to the ground at its cell (undo brings it back; the loader names the cell if the ground has none there).

## Steps (in the window, the scratch copy)
1. `just run --windowed --pack packs/base --pack crates/omnis-sim/tests/packs/test --pack /tmp/omnis-levels
   --editor levels:map:tower`. The plan opens on the ground level: the diamond at (0, 2) on the road, its notch
   pointing east (the party walks in eastward).
2. Level `>` to Upper floor (1): no diamond anywhere (the start is the ground's).
3. Click the bridge at (4, 2); turn to face north (Q and E turn the cursor; the preview shows the way). In the
   inspector: "Start (0, 2) facing east" and Set start here, offered. Press it: the diamond appears at (4, 2)
   with its notch up; the row reads "Start (4, 2) on level Upper floor (1) facing north".
4. Level `<` to the ground: no diamond. Level `>` back: it is there.
5. Move tool on (4, 2), drag to (4, 1): the start moves along the bridge, the notch kept; Undo brings it back.
6. Save. Then Menu, Editor again (or reopen the tower from the tools): the plan opens on Upper floor (1) with the
   cursor on (4, 2).
7. Known: Playtest starts at the cursor as before; New game on these packs is the town's (the levels fixture
   sets no entry), so a new game on an upper start is the tests' to show (`omnis-sim/tests/levels.rs`).

## Known and expected
- The 3D preview draws no start mark (it never did); the icon pass later may give it one.
- A start on a level the map lacks can only come from a file or the socket: the editor sets it on a shown level.
- `omnis-cli` and the MCP's `editor_new` still start a new map at (0, 0) facing east on the ground.
