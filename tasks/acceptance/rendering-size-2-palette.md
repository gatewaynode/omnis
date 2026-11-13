# Acceptance: colour roles and biosphere palettes in the 3D view (rendering size 2, part 3)

The script for your acceptance of the plan you approved on 2026-11-13
(`tasks/plans/rendering-size-2-palette.md`), with your four answers of that day: nine roles; a region names
its biosphere and a map may name its own; a terrain names its role; the 3D view and the editor preview now.

What I could and could not check: the gate has no GPU and cannot see a shader. I looked at offscreen captures
on this machine at Low and High (`target/shots/p3/`). I have not seen it in a window, in motion, or in the
editor's preview on screen. The preview's palette is checked headless only.

Start: `just qa` (the test yard). Other maps: `just qa test:map:town`, `just qa levels:map:tower`,
`just qa heights:map:hills`. Quality is in Menu, Settings; look at Low first.

## Part 1: the default palette (`just qa test:map:town`)
1. Look down the street.
   *You will see* walls in a green that leans to teal, doors in a yellow amber, the floor grid a dim green.
   This is a change: the walls are no longer the HUD's green and the doors are yellower. The old hues were
   too alike for green-blind players (measured), so the default moved.
2. Compare with the dungeon (`just qa test:map:dungeon`).
   *You will see* its floor in the same dim green as the town's. Floors no longer take their terrain's
   colour; the minimap still does.

## Part 2: a region's biosphere (`just qa`, the yard)
1. Look around the yard.
   *You will see* the fixture biosphere "ember": hedges a pale orange, the ground brown, crates and the pile
   cyan, the pack rats pink, the ladder yellow.
2. Notice the pile and the crates.
   *You will see* them in one colour (both are "things"); the pile was rose and the crates violet before.

## Part 3: a map's own biosphere (`just qa levels:map:tower`)
1. Look at the tower.
   *You will see* walls violet and stairs mint (the map's own "deep"), though its region names "frost".
   The ground is the default's green, because "deep" writes only two roles.
2. Climb to the upper floor and find the hole over the road.
   *You will see* the hole's cell's floor in red: its terrain names the danger role.

## Part 4: cliffs (`just qa heights:map:hills`)
1. Look at a cliff face.
   *You will see* its hatching in the walls' colour. Before, cliffs were a warm rock colour of their own;
   under the nine roles a cliff is structure.

## Part 5: the editor
1. Open the editor on a map, choose Edit terrain.
   *You will see* a "role" row: default, structure, ground, passage, thing, party, hostile, magic, danger,
   interface. Set it and Apply; the preview's floor for that terrain changes colour. Not seen by me.
2. A biosphere is a text file: `data/biospheres/<name>.ron` (`omnis-cli schema dump` shows the shape); a
   region or a map names it with `biosphere: Some("<pack>:biosphere:<name>")`.

## Decisions I would like from you
1. **The walls' green.** Keep the tuned teal-green, or keep the HUD's green on walls and accept that walls
   and doors are close for green-blind players (pattern and shape still tell them apart)?
2. **Cliffs, and piles against objects.** Each shares a role's colour now. If either should stand apart, it
   is a tenth role or a strength, not a palette change.
3. **Floors alike.** Grass and road differ underfoot only if their terrains name different roles. Is that
   enough, or should "ground" have variants?
4. **Telling an author.** The contrast check runs in tests only. Should the editor or `omnis-cli pack` report
   a pack's palettes that fail it?
5. **Next**: the fight screen, minimap and HUD on the palette (planned before part 4), then part 4.
