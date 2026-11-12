# Acceptance: piles you can find and take, and a ceiling that stays flat over a dip

The script for your acceptance of the plan you approved on 2026-11-12
(`tasks/plans/loot-window-and-ceiling.md`), and of the two changes you asked for after your first
try (a pile stamp replaces a pile; the quiet after an encounter). Everything here is built and
tested; none of Parts 3 to 5 has been seen in a window by me.

Start: `just run --windowed --pack packs/base --pack packs/the_lower_kingdoms`.

## Part 1: the ceiling over a dip
1. In the dungeon, walk to a cell with a negative height (5, 22 is at -2).
   *You will see* the ceiling over it level with its neighbours', and the walls around it reaching
   the ceiling with no gap. A raised cell still lifts its own ceiling.
   Covered by `tests/fill.rs::a_dip_keeps_the_level_s_ceiling_and_a_rise_lifts_its_own` and
   `tests/geometry.rs::a_wall_stands_on_the_ground_and_tops_the_higher_side`.

## Part 2: reaching a pile
1. Stand beside a pile, facing it across an open edge (or stand on it). Search.
   *You will see* "On the ground: ..." naming what the pile holds. Turned away, or with a wall or a
   shut door between, it is not listed.
   Covered by sim `tests/ground.rs::a_pile_in_the_faced_cell_is_listed_and_taken_and_one_beside_is_not`
   and `a_wall_or_a_shut_door_keeps_a_pile_out_of_reach`.

## Part 3: the loot window
1. Move the pointer over the heap.
   *You will see* the pointer become a hand. Left click.
   *You will see* the window "On the ground": "Taken by" with a party member and Close, then each
   item with its count and Take, Take all (when there are several), Store and Destroy, then Take
   everything. A click beside the heap turns mouse look on as before.
   Covered by `tests/loot.rs::a_click_on_the_heap_opens_the_window_and_a_click_beside_it_is_the_view_s`
   and `the_window_lists_the_pile_ahead_with_take_take_all_and_take_everything`.
2. Press Shift+Space with only the pile in reach, or open Actions and choose "Pick up...".
   *You will see* the same window.
   Covered by `shift_space_opens_the_window_take_everything_empties_the_pile_and_it_closes`.
3. Click "Taken by" to step to another member, then Take.
   *You will see* one of the item leave the pile, the log say "<name> picks up <item>", and the
   window stay open. Take everything empties the pile; the window closes and the heap is gone.
   Covered by `each_button_sends_its_take_to_the_one_who_receives`.
4. With the chosen member down or dead.
   *You will see* Take, Take all and Take everything dim with the reason; Store and Destroy stay lit.
   Covered by `take_everything_sends_one_command_an_item_and_a_fallen_receiver_dims_the_buttons`.

## Part 4: store and destroy
1. On a pile, press Store on a row.
   *You will see* all of that item leave the pile, the log say "<item> goes into the stores", and
   the item in the party's stores on the Inventory screen. If it was the last row the window closes.
   Covered by `in_the_app_store_fills_the_stores_and_the_window_closes_on_the_empty_pile` and sim
   `tests/stash.rs::a_pile_in_reach_goes_into_the_stores_and_the_save_keeps_both`.
2. Press Destroy on a row.
   *You will see* the rows replaced by "Destroy <item> xN? This cannot be undone." with Destroy and
   Cancel. Cancel (or Esc): the pile is as it was. Destroy: the items are gone, the log says
   "N <item> are destroyed", nobody holds them and the stores are unchanged.
   Covered by `in_the_app_destroy_asks_first_cancel_changes_nothing_and_yes_ends_the_pile` and sim
   `tests/stash.rs::destroyed_items_are_gone_from_the_place_and_reach_nobody`.
3. Save and load after either.
   *You will see* the pile as you left it.
   Covered by the `round_trip` checks in sim `tests/stash.rs`.

## Part 5: the two changes from your first try
1. In the editor, choose loot in the picker and click a cell that already has a pile.
   *You will see* "Ground pile (...) replaced at (x, y)"; the inspector line lists what the pile
   holds; Undo brings the old pile back. Save to keep it.
   Covered by `tests/editor_pick.rs::a_pile_stamped_on_a_pile_takes_its_place_and_the_inspector_lists_what_it_holds`.
2. In play, after any encounter begins.
   *You will see* no random encounter for the next 8 steps; a placed group on a tile is still met.
   The number is `encounter_quiet_steps` in `packs/base/data/rules/combat.ron`.
   Covered by sim `tests/encounter.rs::after_an_encounter_the_table_is_quiet_for_the_rule_s_steps`
   and `a_placed_group_starts_the_quiet_is_met_during_it_and_a_save_keeps_it`.

## Not in this version
- An open container has no way into the loot window yet (the window itself serves one; its tests
  open it on a chest). Recommended: "Take from..." on the verbs screen. Your decision.
- No item is marked as kept (there is no such mark in the item data), so Destroy is offered for
  every item.
- Store and Destroy act on all of a row's count; there is no "one" or "how many".
- Loot from a won fight still goes to the kits and stores as before, not through the window.

## Covered throughout
Gate 1285 passed, 16 ignored; workspace clippy clean; mutation checks caught: 4 (ceiling), 3
(reach), 12 (window and click), 6 (pile stamp and quiet), 9 (store and destroy). Sentrux rules pass.
