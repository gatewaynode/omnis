# Acceptance: a ground pile that is easy to get to

The script for your acceptance of the plan you approved on 2026-11-13 (`tasks/plans/pile-reach.md`), from
your hand test the same day: the click target was too small, and a pile underfoot was unseen. Everything
here is built and tested; the hand and the outline have not been seen under a real pointer by me.

Start: `just qa` (the yard; the arrows lie two cells ahead of the start).

## Part 1: the floor is the target
1. Walk forward once, so the arrows are in the cell ahead. Move the pointer onto that cell's floor, away
   from the heap.
   *You will see* the pointer become a hand and a rose square light up on the floor, a little inside the
   cell. Move the pointer off the cell (the next cell on, the sky, a cell beside).
   *You will see* the hand and the square go.
   Covered by `tests/pile_reach.rs::the_floor_of_the_faced_pile_s_cell_is_under_the_pointer_and_the_cells_around_are_not`
   and `the_floor_s_outline_is_drawn_under_the_pointer_and_gone_when_it_leaves_or_a_screen_opens`.
2. Left click inside the square.
   *You will see* the loot window. Close it and click beyond the cell.
   *You will see* the mouse look turn on, as before.
   Covered by `a_click_on_the_floor_opens_the_window_and_one_beyond_the_cell_is_the_view_s`.

## Part 2: the "Pick up..." button
1. Still facing the arrows.
   *You will see* a "Pick up..." button at the bottom middle of the view, above the two rows of buttons.
   Click it.
   *You will see* the loot window.
2. Close the window and walk forward onto the arrows' cell.
   *You will see* the button still there, though the heap is below the view. Click it.
   *You will see* the loot window on the same pile.
3. Turn away from a pile you are not standing on, or open any screen.
   *You will see* the button go. Take everything from a pile.
   *You will see* the window close and the button go with the heap.
   Covered by `pick_up_stands_on_the_view_while_a_pile_is_in_reach_and_opens_the_window`.

## What to judge
- Whether the whole floor of the cell is the right size of target, or too much of the view.
- The outline's look: its colour (the pile's rose) and its inset.
- Where the button stands, and its words.
