# Acceptance: the 3D view's lines on Bevy's meshes and Material (rendering size 1)

The script for your acceptance of the plan you approved on 2026-11-12 (`tasks/plans/rendering-size-1.md`):
the 3D view's and the editor preview's lines are retained Bevy meshes drawn by a material and shader of the
client's own, in place of Bevy's debug gizmos, with the line width kept. **Nothing should look different.**
That is the test: what you are looking for is a difference.

What I could and could not check: the gate has no GPU and cannot see a shader. I compared offscreen captures
on this machine (old renderer against new, same binary before the swap, then the final build). I have not seen
it in a window, in motion by hand, or in a fight-and-back.

Start: `just run --windowed --pack packs/base --pack packs/the_lower_kingdoms`.

## Part 1: the view
1. Stand in the dungeon and look around; walk a corridor.
   *You will see* the same glowing lines as before: walls green, doors amber, the floor and ceiling grids dim,
   the same width, the same fade into the fog, lines not showing through walls.
   Compared by capture: town, hills, tower and yard fixtures at 1600x900, old against new with the eye's easing
   off: 0.02% to 0.10% of pixels differ (the frame-rate text, and long lines that now fade along their length
   where they used to fade whole).
2. Look at a long wall or floor line that runs from near you into the fog.
   *You will see* it fade gradually along its length. Before, a line was one brightness from end to end,
   chosen by its middle. This is the one intended difference.
3. Walk on ground with relief (a rise or a dip).
   *You will see* the bright crest line where the ground turns away from you, moving as you move.
   Covered by `tests/lines.rs::the_crests_are_found_from_the_eye_and_again_only_when_it_moves` and three
   hill captures (0.05% to 0.10%).

## Part 2: the settings
1. Menu, Settings: switch the floor grid off and on.
   *You will see* the floor and ceiling grids go and return; everything else stays.
   Covered by `the_grid_setting_and_a_reshape_rebuild_the_mesh`.
2. Change the quality: Low, Medium, High.
   *You will see* the lines 1, 1.5 and 2 pixels wide, and the bloom change as before.
   Covered by `the_fog_follows_the_sight_and_the_width_the_quality`.

## Part 3: things that rebuild the view
1. Open a door; take a pile; store or destroy from a pile; start and finish a fight.
   *You will see* the view change at once, as before (the door open, the heap gone, the group gone).
   The fight screen itself is unchanged: its 2D lines are still Bevy gizmos.

## Part 4: the editor's preview
1. Title, Editor, open a map; move the cursor and turn; paint a wall.
   *You will see* the preview picture at the top right as before, following the cursor and the paint.
   Compared by capture: identical to the old renderer, pixel for pixel
   (`--editor <map> --screenshot <png>` is new, for this). Covered by
   `tests/editor_preview.rs::the_preview_s_lines_are_meshes_on_its_layer_under_its_own_material`.

## Numbers
- Frame time, release, offscreen, a 300-frame walk, three runs each, both renderers in one binary before the
  swap: gizmo 1.95 to 2.10 ms mean (2.42 to 3.05 worst); mesh 1.97 to 2.15 ms mean (2.34 to 3.14 worst), at
  1920x1080 and 5120x1440 on the town and the hills. **No difference.** On these maps the frame is fixed cost.
  The move was for what the shader can now do, not for speed.
- Gate 1293 passed, 16 ignored. Mutation checks caught: 9 (buffers and entity), 9 (crests, preview, quality).

## Departures from the plan, for the same look
- One line mesh a view, not structure and grid apart: of two lines in one place the old renderer showed the
  first drawn, so the mesh keeps the scene's order and the grid setting rebuilds it.
- The material writes depth and tests "nearer", as the old pipeline did, so a near line is not painted over by
  a far one.
- The spike's shader was kept as the real one; steps 1 and 2 went in as one commit.
- Added: an offscreen capture of the editor.

## For your decision
- Size 2's design and a proposed amendment to the presentation PRD: `tasks/plans/rendering-size-2.md`. My
  recommendation there: edge feathering next (small, shader only), the rest with the colour scheme.
