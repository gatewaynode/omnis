# Acceptance: found items land on the ground and the loot window opens

The script for your acceptance of the plan you approved on 2026-11-13 (`tasks/plans/found-items.md`), from
your try of the pile reach: the stone that hides a gem opened no loot picker. You chose "Drop it, open the
loot window". Built and tested headless; not seen in a window by me.

Start: `just qa` (the yard; the loose stone is on the start cell).

1. Press Space (or click Actions) and choose "Use".
   *You will see* the stone go and the loot window open at once: "On the ground", the gem, Take, Store,
   Destroy, Take everything. The log says "Found Gem; on the ground".
   Covered by `tests/found.rs::a_find_made_from_the_actions_menu_opens_the_window_in_its_place` and sim
   `tests/object_act.rs::what_an_object_gives_is_laid_where_it_stood_for_the_party_to_take`.
2. Take the gem.
   *You will see* it go to the member named under "Taken by", and the window close.
3. Start again, Use the stone, and this time close the window without taking.
   *You will see* a small heap where the stone was, the "Pick up..." button at the bottom of the view, and
   the notice "Under the stone lies a gem." The window does not open by itself a second time; the button,
   a click on the cell's floor from the next cell, or Actions opens it.
   Covered by `a_found_item_opens_the_loot_window_and_is_taken_or_left_as_a_heap`.
4. Drop an item, or pick one up from the arrows.
   *You will see* no window open by itself: only a find opens it.
   Covered by `only_a_find_opens_the_window_not_a_drop_or_a_pick_up`.

## What to judge
- The object's own words ("Under the stone lies a gem.") are not on the loot window; they are in the log and
  in the notice after it closes. Say if they should stand on the window.
- Gold an object gives still goes straight to the party (it has no pile).
- Nothing found goes into the stores unless you press Store.
