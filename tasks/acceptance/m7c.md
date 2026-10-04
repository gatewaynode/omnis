# Acceptance c: M7c (the turn budget) — closes M7

The script for your acceptance of M7c (`tasks/plans/m7c-turn-budget.md`, step 8). It covers:
- the turn budget;
- the first class features with effect;
- reactions declared on the tactics panel;
- Bob the Rat King, the first monster that casts.

Each step says what you will see and names the automated test that already covers it, so a failure can be
told apart from a gap. Play it in the window with a dev build (`cargo run -p omnis-app`).

**What changed in a fight.**
- A turn is no longer one command. Each member has an action, a bonus action and a reaction (the SRD's one of
  each), shown on the budget line above the action row: `Action 1   Bonus 1   Reaction 1 (on)`.
- A turn ends by itself when no action is left and nothing the bonus action could pay for remains. END (`n`)
  ends it sooner.
- A fighter holding Action Surge, or a level-2 rogue (Cunning Action is always there to spend), keeps the turn
  until you press END. This is by design.
- A bonus-action spell does not limit the action to a cantrip (PRD D24). A cleric may cast Healing Word with
  the bonus action and Bless or Cure Wounds with the action in the same turn.

**What does not work yet.**
- Nothing answers a member's or an enemy's cast: `SpellCast` and `EnemyCasts` are raised, but no declared
  reaction can take them.
- Nothing raises "an enemy flees" or "own turn".
- Items and features cannot be declared as reactions.
- The Use picker shows six rows; a long kit is cut after the features.
- Sneak Attack and Extra Attack are labels only.

## Setup
1. **New game** with four members in this order:
   1. a fighter;
   2. a rogue;
   3. a cleric;
   4. a wizard.

   The first three stand in the front row and the wizard in the back. Any save rule.
2. **Level 2 for everyone:**
   - set each member's XP to 300 (MENU → Debug menu → XP);
   - train at the trainer (7, 1);
   - for the cleric's spell pick, choose **Healing Word**.

   *You will see* "… reaches level 2" for each member. The fighter gains Action Surge and the rogue gains
   Cunning Action.
   Covered by `training.rs::a_level_is_granted_at_the_trainer_for_its_price` and
   `level.rs::features_with_effect_come_by_level_and_rests_give_their_uses_back`.

## Part 1: the turn budget (the plan's eight steps)
Go out through the gate (11, 2) and fight anything: the meadow's road, or the dungeon's first rats at (3, 8).

1. **The fighter's turn.** Open USE: the first rows are the fighter's features, "Second Wind" and
   "Action Surge", each with "1 use", before any items. Pick Action Surge.
   *You will see* "… uses Action Surge" and the budget line reading `Action 2`. Attack twice, then pick
   Second Wind from USE.
   *You will see* the fighter heal, `Bonus 0`, and the turn pass once the actions are spent.
   Covered by `turn_budget.rs::action_surge_gives_a_second_action_and_second_wind_heals_once_a_rest` and
   `omnis-app/tests/combat.rs::second_wind_is_used_from_the_use_picker_by_mouse_and_spends_the_bonus_action`.
2. **The cleric's turn.** Open CAST: Healing Word's row reads "bonus action". Cast it on a member.
   *You will see* `Bonus 0` with `Action 1` left. The turn goes on: attack, or cast a second spell (Bless,
   Cure Wounds) with the action.
   Covered by `turn_budget.rs::two_spells_a_turn_within_the_budget_and_a_readied_spell_cannot_take_the_bonus`
   and `spell_menu.rs::a_bonus_action_spell_is_cast_with_the_bonus_action_and_leaves_the_action`.
3. **The rogue hides.** USE shows "Cunning Action: hide" and "Cunning Action: exchange". Pick hide.
   *You will see* "… hides: success (…)" or "failure". On a success the rogue's next attack rolls with
   advantage (two d20s in the log's math).
   Covered by `turn_budget.rs::hiding_gives_the_next_attack_advantage` and
   `use_menu.rs::cunning_action_s_exchange_goes_to_the_selected_member_and_hide_needs_none`.
4. **Exchange with and without an opportunity attack.**
   - Select the wizard on the band. On the rogue's turn pick "Cunning Action: exchange": the rogue and the
     wizard swap, and nothing swings.
   - Later, select a back-row member and use the fighter's plain EXCHANGE.
   *You will see* "Opportunity attack on …" from each front stack that still has its reaction, then their
   attack lines.
   Covered by `turn_budget.rs::cunning_action_exchanges_without_a_swing_and_a_plain_exchange_draws_them`.
5. **Shield, declared on the tactics panel.** Outside a fight, open SHEET on the wizard, then TACTICS (or T).
   - The action menu offers Shield.
   - The trigger menu offers only "attacked".
   - Leave the conditions empty (always), or Add one: kind "hit points %", who "me", "<", 50.
   - Save.

   *You will see* a declared row like "Shield on attacked". Close goes back to the sheet. In a fight, when a
   hit would land on the wizard, *you will see* "… reacts: Shield", the hit judged again, and at most one
   shield a round.
   Covered by `omnis-app/tests/tactics.rs::a_wizard_declares_shield_under_a_condition_edits_it_and_removes_it`
   and `reactions.rs::shield_declared_for_every_hit_fires_on_hits_once_a_round`.
6. **The reactions switch.** In a fight, on the wizard's turn, press REACT (`o`).
   *You will see* "…'s reactions are off" and the budget line ending `(off)`. No shield fires until you press
   it again. TACTICS is grey in a fight; the tool bar keeps the sheet shut there anyway.
   Covered by `omnis-app/tests/combat.rs::react_switches_the_acting_member_s_reactions_by_mouse`,
   `reactions.rs::the_switch_turns_every_reaction_off_and_is_the_only_tactics_command_in_a_fight` and
   `omnis-app/tests/tactics.rs::in_a_fight_tactics_stay_shut`.
7. **A save from before M7c.** Put an M7b quick save at `.omnis/quick.ron` and choose Load.
   - *You will see* "Load failed: save was made with different packs". M7c's content renumbered the pack ids,
     as M7b's did.
   - Over the MCP, `save_read` with `force: true` loads it. `party_get` then shows a wizard who had shield
     switched on with a declared set named after the spell (`Attacked`, `WouldChangeOutcome`).

   Covered by `save_and_replay.rs::loads_are_checked` and `save_and_replay.rs::a_schema_5_save_migrates`.
8. **END.** On any member's turn with something left, press END (`n`).
   *You will see* the next combatant act. A fighter who has attacked but still holds Action Surge keeps the
   turn until END.
   Covered by `turn_budget.rs::end_turn_passes_the_turn_and_a_reaction_is_never_a_turn_s_command` and
   `turn_budget.rs::a_held_surge_keeps_the_turn_and_a_spent_one_lets_it_end`.

## Part 2: Bob the Rat King
9. **The way down.**
   - Go through the dungeon to its stairs at (23, 23).
   - In the depths (you arrive at (1, 0)), go south through the door at (2, 5), then east along row 8 through
     the doors at (5, 8) and (11, 8).
   - Bob's group stands just inside the south-east room, at (13, 8): three and two giant rats in front, Bob
     behind them.

   The depths' four rooms of goblins and skeletons are optional. Bob is a wall for a level-1 party of four
   (49% of fights measured end in a wipe) and fair from level 2 (3.5%).
10. **His casts.**
    *You will see* lines like "Bob the Rat King casts Fire Bolt" (or Magic Missile, or Thunderwave), each
    followed by its attack, damage or save lines. Thunderwave makes each front-row member save on
    Constitution against DC 13.
    Covered by `monster_cast.rs::bob_rolls_among_his_staff_and_the_spells_his_points_pay_for`,
    `fire_bolt_is_a_plus_five_spell_attack_of_2d10_that_a_declared_shield_answers` and
    `thunderwave_makes_each_front_member_save_on_constitution_at_dc_13`.
11. **Shields on both sides.**
    - With the wizard's Shield declared (step 5), a Magic Missile at the wizard reads "…'s shield stops the
      missile".
    - Bob raises his own Shield against a hit it would turn or against your wizard's Magic Missile, and it
      holds until his next turn.

    Covered by `monster_cast.rs::magic_missile_stops_at_a_shield_raised_or_already_up` and
    `bob_shields_himself_from_a_missile_and_a_hit_once_a_round`.

## Result
Report pass or fail by step number. Any failure becomes a bug in `tasks/BUGS.md` with a test that reproduces
it before it is fixed. When every step passes, M7 is closed.
