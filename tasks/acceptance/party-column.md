# Acceptance: the party column, and the roll log at the left

The script for your acceptance of the change you asked for on 2026-10-11: in a fight the roll log
sits on the left under the action choices, and a party column stands on the right, in explore and
in a fight; a member's entry opens the row of their screens.

## Part 1: explore
1. `just run --windowed`, New game, make a party (or `--seed 7` for the fixed four), Begin.
   *You will see* under the minimap, at the right, a "Party" column: "front", then each front-row
   member as a button "Brenna  Fighter 1" with "HP 12/12  SP 0/0  AC 18" under it, then "back" and
   the rest. Actions and Menu stay visible below it.
   Covered by `tests/column.rs::the_column_lists_the_party_and_a_member_opens_their_row`.
2. Click a member's entry.
   *You will see* under that member a row: "Sheet: Not built yet (P6)", "Inventory: Not built yet
   (P5)", "Spells: Not built yet (P5)", "Journal: No quests yet", all dim. Click the entry again and
   the row goes away; click another member and it moves to them.
3. Press Esc.
   *You will see* the pause, and no party column behind it. Save, then Load: the column returns
   with nothing selected.
   Covered by `the_column_hides_at_the_title_on_creation_and_under_the_pause`.

## Part 2: a fight
1. Walk into monsters (the dungeon's placed group, or any encounter).
   *You will see* the picture at the top left, the choices under it, the roll log under the choices
   ("Met 2 Giant Rat (hostile)"), and the status lines at the bottom left: one column. The figures
   stand in the middle, and the party column at the right, above Menu.
   Covered by `the_column_stands_beside_a_fight` and the layout tests in `arena.rs` and
   `tests/arena.rs`.
2. Fight.
   *You will see* the acting member's entry marked "> " in the column, and each hit's line added at
   the bottom of the log on the left. Hit points under a quarter turn red; a downed member's line
   is dim with "down".
   Covered by `tests/roster.rs`.

## Covered throughout
Vector 105 integration + 32 unit tests; both gate clippy lines clean; mutations 4 of 4 caught.
Captures at 1600x900 and 1280x720 of explore and an encounter.
