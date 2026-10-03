# Acceptance a: M7a (town, services, rest)

The script for the owner's acceptance of M7a (`tasks/plans/m7-town.md` step 9). Each step says
what you will see and names the automated test that already covers it, so a failure can be told
apart from a gap. Play it in the window with a dev build (`cargo run -p omnis-app`).

**Order matters.** The inn's room and a long rest in the field share one once-a-day clock (1440
minutes; `rest.rs::a_rest_in_the_field_counts_against_the_inn_and_the_inn_against_the_field`).
Nothing in the game skips time. So the script takes the inn's room first, sees the camp's long
rest refused for that reason, and uses a second new game for the camp's long rest.

**Money.** Prices are in copper (100 cp = 1 gp). Backgrounds start a member with 10 or 15 gp. A
room is 50 cp a member, food 50 cp a unit, and raising a level-1 member 2,500 cp. Three members
are enough for the whole script without the debug menu. If the purse runs short, Raise is dim
with its price as the reason, and the debug menu's Gold row (MENU → Debug menu) tops it up.

## Part 1: the town

1. **New game.** Set saving to *Inns only*, create three members, and start.
   *You will see* the town street, facing west, at (10, 2). The tool bar is ITEMS SPELLS SHEET
   CAMP / LOOK MAP MENU.
   Covered by the new-game form's unit tests in `menu.rs` and by every app test that starts in
   town (`omnis-app/tests/service.rs::town`).
2. **The street refuses a save.** MENU → Save.
   *You will see* "The save rule forbids saving here".
   Covered by `town.rs::inn_only_saves_are_allowed_in_an_inn_and_nowhere_else`.
3. **The smith** is the first door on the south side. Step in; a question asks first (Go or
   Stay). Buy one item, then sell it back.
   *You will see* the purse drop by the list price and rise by half of it. The item moves in and
   out of the stores.
   Covered by `services.rs::the_smith_buys_at_list_and_sells_at_half` and
   `omnis-app/tests/service.rs::every_offer_sends_what_the_view_promised`.
4. **The tavern** is the second door on the south side. Buy food and hear a rumor.
   *You will see* the stores' food rise and a rumor line in the log.
   Covered by `services.rs::the_tavern_sells_food_and_tells_rumors`.
5. **The inn** is the first door on the north side. Take a room.
   *You will see* every member at full hit points and spell points, and no food eaten.
   Covered by `services.rs::a_room_is_a_long_rest_once_a_day`.
6. **Save at the inn.** While you are inside, MENU → Save.
   *You will see* "Saved to …". Stay or leave as you like.
   Covered by `town.rs::inn_only_saves_are_allowed_in_an_inn_and_nowhere_else` and
   `pointer.rs::the_pause_menu_saves_and_loads_by_mouse`.

## Part 2: the dungeon and the camp

7. **The road out.** Take the signpost at the east end of the street (11, 2) to the meadow, then
   the stairs down at (16, 5) to the dungeon. Fight until someone is hurt.
8. **The camp.** Press CAMP or R. It opens only on the map, never inside a service or a fight.
   *You will see* one row per member with HP and hit dice, a slider for each hurt member with
   dice left, the food line, and Short rest, Long rest and Close.
   - Slide one member to 1, then press Short rest. *You will see* the die spent, HP up, a log
     line, and the camp still open.
   - Long rest is dim with "rested too recently (…)". The inn's room in step 5 counts.
   - Close or Escape goes back to the map.

   Covered by `omnis-app/tests/camp.rs` (all five tests).
9. **A dead member.** The natural way is to fall in a fight. Otherwise use MENU → Debug menu: choose
   the member, set HP to 0 and turn on the `dead` condition.
   Walk back to town, then go to the temple, the second door on the north side, and Raise.
   *You will see* the member alive at 1 HP and the purse 2,500 cp lighter.
   Covered by `services.rs::the_temple_heals_cures_and_raises_for_a_price`.
10. **Load.** MENU → Load.
    *You will see* yourself back inside the inn, as at step 6: the purse before the raise and
    the dungeon's wounds undone.

## Part 3: the camp's long rest (a second new game)

11. New game with three members. Walk to the dungeon (step 7), take no room, and open the camp.
    The food line reads "The night eats 3 food; the stores hold …". Press Long rest.
    *You will see* the stores down by 3 and everyone at full HP, then Long rest dim with "rested
    too recently". With too little food, the reason is the food.
    The test dungeon has a 30 per mille chance of an ambush each long rest. If it happens, the
    camp gives way to the fight at once, and nothing is restored.
    Covered by `camp.rs::the_long_rest_eats_and_then_waits_a_day` and
    `camp.rs::an_ambush_leaves_the_camp_for_the_fight`.

## Part 4: one number (your machine only)

12. Run `cargo run -p omnis-app -- --frame-stats`. Note the frame time on the map alone, then
    with a service panel open (the inn is fine). Paste both numbers back. They close the
    Feathers experiment's open measurement (`tasks/plans/feathers-experiment.md`). Agent-launched
    windows draw no frames, so this one is yours.

## Result

Report pass or fail by step number. Any failure becomes a bug in `tasks/BUGS.md` with a test
that reproduces it before it is fixed.
