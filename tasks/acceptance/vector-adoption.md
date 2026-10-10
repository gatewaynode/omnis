# Acceptance: the vector client becomes the game (V1–V5)

The script for your acceptance of the vector adoption (`tasks/plans/vector-adoption.md`). It covers what
changed when `omnis-vector` joined this tree: it runs on protocol 2, `just run` starts it, the town start
does not trap the party, and the fight follows the turn budget. Everything the vector client does not
have yet (character creation, saves, town services, inventory, the dev socket) is still in `omnis-app`
(`just run-app`) and comes over screen by screen (`tasks/TODO.md`, parity P0–P9).

**What you will see.** The 3D view as you played it on the experiment, starting on the town's street
instead of the meadow. The fight menu has **End turn**, and its first line shows what the turn has left.
Stepping into a service shows a notice with **Leave**: services are not in this client yet.

## Part 1: start and walk
1. `just run` (or `just run --windowed`).
   *You will see* the town's street at (10, 2) facing West: the status line says `test:map:town`, yellow
   doors on both sides, the minimap top right.
2. Walk down the street and back with W/S or the buttons.
   *You will see* `refusals 0  disagreements 0` stay at zero.

## Part 2: a service
1. From the start, turn right (E) to face the guild's door, open it (Space, then Interact, or Shift+Space),
   and step in (W).
   *You will see* the panel titled with the guild's name, two lines saying services are not in this
   client yet and to use `just run-app`, and one button, **1 Leave**.
   Covered by `omnis-vector/tests/shell.rs::a_service_shows_the_town_notice_and_leave_steps_back_out`.
2. Press **Leave** (or 1).
   *You will see* the panel close; you can walk again.

## Part 3: out of town and a fight
1. Walk east to the end of the street and through the signpost (11, 2).
   *You will see* the meadow at (16, 30) facing North.
2. Walk north up the road into the stairs (16, 5), then south and east in the dungeon to the rats behind
   the door at (3, 8), or fight whatever finds you first.
   *You will see* the fight screen.
3. Choose **Fight**. On a member's turn:
   *You will see* the first line read "Round 1: <name>'s turn (1 action, 1 bonus action)", and the entries
   Attack, Cast, Use, Dodge, Swap, **End turn**, Flee.
4. Attack. *You will see* the turn stay with the member when a bonus action is left and something could
   use it; **End turn** passes it on.
   Covered by `omnis-vector/tests/combat_menu.rs::end_turn_passes_the_turn_with_budget_left`.
5. On Durin's turn, Cast, then a spell. *You will see* the spells with their cost; a spell the bonus
   action can pay for says "bonus" and leaves the action (none of the fixed party knows one at level 1;
   the test teaches Durin Healing Word).
   Covered by `combat_menu.rs::a_bonus_action_spell_is_paid_with_the_bonus_action_and_leaves_the_action`.
6. Swap two members, then target a member by clicking. *You will see* the member you click is the
   member named, wherever they now stand.
   Covered by `combat_menu.rs::a_member_is_picked_by_identity_wherever_they_stand`.
7. Read the roll log. *You will see* spells, items and members named, never `?` or a number.

## Part 4: the other client
1. `just run-app`.
   *You will see* `omnis-app` exactly as before (title screen, new game, services, saves).

## Covered throughout
`omnis-vector/tests/agreement.rs::the_mirror_agrees_with_the_simulation_everywhere` holds the client's
collision to the simulation on every cell and heading of all four test maps. The gate:
`tests passed 670 failed 0 ignored 13`; both golden replays unchanged; no simulation crate touched.
