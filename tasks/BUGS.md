# Bugs

Each bug found in play or review: the report, the cause, the test that reproduces it (written red before
the fix) and the fixing commit. Newest last.

## B1 — A fighter's turn waits for End every round (acceptance c, 2026-10-04)
- **Report** (owner, playing `tasks/acceptance/m7c.md`): "my front line fighter has to manually choose the
  'end' option after her first round of combat to let the combat proceed (doesn't seem quite right)".
- **Cause**: `combat/budget.rs::goes_on` kept a turn open while a free feature had a use or the bonus action
  could pay for a feature. Second Wind (level 1, once a short rest) and Action Surge held every fighter's turn
  until spent; Cunning Action (no use limit) held a level-2 rogue's every turn.
- **Decision** (owner, 2026-10-04): a turn ends when its action is spent, unless a spell that may take the
  bonus action can still be cast; features are used before the action.
- **Test**: `turn_budget.rs::a_turn_ends_when_its_action_is_spent_though_features_are_left` (red on the old
  rule: the fighter still held the turn).
- **Fixed in**: `8491fd9`.

## B2 — The tactics panel offers a reaction that can never fire (acceptance c, 2026-10-04)
- **Report** (owner): "I can access the tactics panel, but there are almost no options to choose from. And
  the trigger 'when enemies flee' is a poor condition, because enemies never flee."
- **Cause**: `omnis-sim/src/tactics.rs::answers` granted the weapon attack the trigger `EnemyFlees`, which
  nothing raises, and `combat/reaction.rs` resolves only spells. The fighter, rogue and cleric were offered
  only that dead row; Shield is the base pack's only reaction (close to the SRD at levels 1–2).
- **Decision** (owner, 2026-10-04): offer only what the game can fire; real martial reactions go to
  `tasks/knowledge/horizons.md`.
- **Test**: `reactions.rs::the_tactics_commands_check_everything_and_change_nothing_when_refused` (the attack
  on `EnemyFlees` is refused; red before the fix), with `views.rs`, `bridge.rs` and
  `omnis-app/tests/tactics.rs::a_fighter_has_nothing_to_declare_yet`.
- **Fixed in**: `34f7eec`.

## B3 — Debug changes to gold, food or items seemed not to stay (acceptance c, 2026-10-04) — open
- **Report** (owner): "changes don't seem to stay when returning to the game world"; asked which, the owner
  named gold, food or items, and will retest with the new entry methods.
- **Checked**: headless with the canvas menu, gold went 3000 → 3200 and food 20 → 21 and both held after
  Escape; given items landed in the kit and the stores. Not reproduced.
- **Test**: `omnis-app/tests/debug.rs::typed_numbers_reach_the_world_once_and_every_field_holds_after_close`
  sets every number field, an item to the member and to the stores and a condition through the new Feathers
  panel, closes it, and checks the world still holds each value.
- **State**: open until the owner's retest with the debug panel.

