# Acceptance: line weight and pattern in the 3D view (rendering size 2, part 2)

The script for your acceptance of the plan you approved on 2026-11-12
(`tasks/plans/rendering-size-2-patterns.md`), as changed by your choices on the captures: dashes of 1/8 cell,
and a lighter line between the dashes in place of a void, readable at Low quality.

What I could and could not check: the gate has no GPU and cannot see a shader. I looked at zoomed offscreen
captures on this machine at Low and High. I have not seen it in a window, in motion, or in the editor's preview.

Start: `just qa` (the test yard, in a window). Other maps: `just qa heights:map:hills`, `just qa
levels:map:tower`, `just qa test:map:town`, `just qa test:map:dungeon`. Quality is in Menu, Settings.

## Part 1: weight
1. Look at the floor and ceiling grids.
   *You will see* them thinner than the walls (half the width; at Low, one pixel and fainter).
2. Look at the crates, the statue, the chest and the pile on the floor.
   *You will see* their lines heavier than the walls' (half as wide again).

## Part 2: pattern
1. Look at an object or the pile, near you.
   *You will see* its outline whole, with heavy bright dashes on a thinner, dimmer line: about eight dashes
   to a cell's length, and a dash at every corner.
2. Walk toward it and around it.
   *You will see* the dashes stay where they are on the object. Not checked by me: I compared stills only.
3. Look at an object several cells away.
   *You will see* the dashes merge into an even line rather than shimmer. Checked only as far as the yard is
   deep.
4. Set the quality to Low, then Medium, and look again.
   *You will see* the dashes still read, by brightness more than by width.
5. Find the pack rats in the yard (east of the start) at High quality.
   *You will see* the figure drawn in two thin rails. At Low and Medium it is a solid heavy line: two rails
   need three pixels of width.
6. `just qa test:map:dungeon`, turn right (west) at the start.
   *You will see* the portal's mark on the floor as a beaded line: short bright dots on a dim line.
7. Walls, doors, cliffs, ramps, stairs and ladders.
   *You will see* them as before: solid, the normal width.

## Part 3: the editor
1. Open the editor on a map with objects and look at the 3D preview.
   *You will see* the same weights and patterns. Not checked by me by eye; it shares the code path, and
   `tests/editor_preview.rs` holds the preview's meshes.

## What is provisional
Which kind of line gets which weight and pattern is one table (`strokes::channels`), after the examples of
presentation-PRD 5.3, to be replaced when the roles are designed (M8.5 step 5). Sawtooth, glyph markers and
motion are not built.
