# Acceptance: the loot window opens on fight spoils and on an opened container

The script for your acceptance of the plan you approved on 2026-11-13 (`tasks/plans/loot-everywhere.md`).
Client only; built and tested headless; not seen in a window by me.

Start: `just qa` (the yard).

## Part 1: a container
1. Walk to the chest on the east side of the yard (it stands on (5, 3); face it from the cell south of it).
   Actions, "Use...", and force it or pick it until it gives.
   *You will see* the loot window open at once, titled "Chest", with the dagger: Take, Store, Destroy.
   A failed try opens nothing.
   Covered by `tests/found.rs::an_opened_container_opens_the_window_on_it_and_actions_lead_back`.
2. Close the window without taking. Open Actions.
   *You will see* "Take from Chest...". Choose it.
   *You will see* the same window. Take the dagger.
   *You will see* the window close, and "Take from Chest..." gone from Actions.
3. Close the chest and open it again, empty.
   *You will see* no window: there is nothing to take.
   Covered by `only_a_container_with_something_in_it_opens_the_window` and
   `a_new_state_offers_its_object_only_when_it_shows_contents`.

## Part 2: fight spoils
1. Walk north from the chest's side into the pack rats (5, 1) and win the fight.
   *You will see*, when the fight's screen goes, the loot window on what they dropped (a gem or a dagger,
   most times). The Victory line is in the log, and stands as the notice once the window closes.
   Covered by `the_spoils_of_a_won_fight_open_the_window_once_the_view_is_the_map_s`.
2. Win a fight that drops nothing (any fight in `just qa test:map:dungeon`), or run from one.
   *You will see* no window.
   Covered by `a_won_fight_that_left_nothing_opens_nothing` and `only_a_victory_that_left_something_is_a_find`.

## What to judge
- Gold from a fight or a chest still goes straight to the party.
- A dead member's kit still goes to the stores at the fight's end with no window.
- Whether the window right after a fight is welcome, or should wait for a key.
