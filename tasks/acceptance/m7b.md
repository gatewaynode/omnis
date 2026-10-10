# Acceptance b: M7b (progression) — the done-when

The script for the owner's acceptance of M7b (`tasks/plans/m7b-progression.md` step 13), which
is M7's done-when from PRD §13: **create a party, clear the dungeon, return to town, level up.**
Each step says what you will see and names the automated test that already covers it, so a
failure can be told apart from a gap. Play it in the window with a dev build
(`cargo run -p omnis-app`).

**What a clear brings.** The numbers come from `clear_over_seeds` (the table is in `tasks/TODO.md`,
step 11), which assumes a party that fights every group and never flees.
- The first map's groups are worth about 1,125 XP. Four members who fight them all get about 281
  each, and level 2 needs 300. The depths' groups bring a party of four to level 2.
- Three members reach level 2 on the first map.
- A pair is hard mode: fixed groups don't shrink with the party.
- If you only want to see the trainer, MENU → Debug menu → XP sets a member's experience.

**The groups.** Your progress also shows as `groups_cleared` in `game.status` over the MCP. The
wary rat at (9, 8) comes back and is not counted.

| Map | Tile | Group |
|---|---|---|
| Dungeon | (3, 8) | 2 rats |
| Dungeon | (15, 8) | 1 rat, friendly; you attack it to clear it |
| Dungeon | (3, 14) | 2 goblins |
| Dungeon | (9, 14) | 2 skeletons |
| Dungeon | (15, 14) | 3 goblins |
| Dungeon | (21, 14) | 3 goblins |
| Dungeon | (3, 20) | 2 goblins and 1 skeleton |
| Dungeon | (9, 20) | 2 skeletons |
| Dungeon | (15, 20) | 2 skeletons and 1 goblin |
| Dungeon | (21, 20) | 2 goblins and 1 skeleton |
| Depths | (3, 3) | 3 skeletons |
| Depths | (9, 3) | 4 goblins |
| Depths | (3, 9) | 2 goblins and 2 skeletons |
| Depths | (9, 9) | 2 skeletons and 2 goblins |

The stairs down are at the dungeon's far corner (23, 23). The tileset has no stairs-down art yet,
so they look like the stairs up. The stairs back up are at the depths' (0, 0).

## Part 1: create a party

1. **New game** with four members, including a cleric and a wizard. Any save rule.
   *You will see* the town street, as in acceptance a.
   Covered by `omnis-app/tests/service.rs::town` and the creation tests.
2. **SHEET** on each caster.
   *You will see:*
   - the wizard knows Fire Bolt, Light, Mage Hand, Magic Missile, Shield, Burning Hands and
     Thunderwave, and no second-level spell;
   - the cleric knows Bless and Cure Wounds among their spells.

   Covered by `creation.rs::casters_get_their_spell_points_and_spells`.

## Part 2: clear the dungeon

3. **The road out:** signpost (11, 2), meadow, stairs (16, 5).
4. **Fight the groups** in the table, resting at the camp (CAMP or R) or in town as you like.
   - XP is split among the living after each victory. Goblins and skeletons drop gold; rats
     don't.
   - The new spells cast like the old ones: Thunderwave hits a whole stack, as Burning Hands does.

   Covered by `measure.rs::clear_over_seeds` (ignored, run by hand) and the combat tests.
5. **The depths**, by the stairs at (23, 23). Take as many groups as you like.

## Part 3: return to town

6. **Back up:** the depths' (0, 0), the dungeon's (0, 0), then the meadow's road south (16, 31).
7. **SHEET** on a member with 300 XP or more.
   *You will see* "Ready to train" after the hit dice.
   Covered by `sheet_menu.rs::the_fighter_and_the_wizard_have_their_numbers`.

## Part 4: level up

8. **The trainer** is the third door on the north side (7, 1).
   *You will see:*
   - two lists, Levels and Spell picks;
   - a Train row per member, such as "Brenna, level 1 to 2" for 20 gp;
   - a member short of experience dim, with the experience they need.

   Covered by `omnis-app/tests/service.rs::the_trainer_grants_a_level_and_its_pick_and_the_temple_sells_a_spell`
   and `training.rs::a_level_is_granted_at_the_trainer_for_its_price`.
9. **Train.**
   *You will see:*
   - the purse down by the price;
   - a log line such as "Brenna reaches level 2: +8 HP, Action Surge";
   - the sheet's level, HP and hit dice up by one level's worth.

   Covered by the same tests and `level.rs::a_fighter_gains_the_average_hit_points_and_the_level_s_features`.
10. **The cleric's pick.** After the cleric trains, the Spell picks list shows Healing Word,
    Guiding Bolt and Inflict Wounds. Spiritual Weapon and Prayer of Healing are dim until
    level 3.
    Choose one. *You will see:*
    - "… chooses Healing Word" (or the one you chose) in the log;
    - the picks list emptying;
    - the spell at the end of the SPELLS list.

    Covered by `training.rs::casters_owe_picks_and_choose_them_one_at_a_time`.
11. **The wizard's picks wait.** After the wizard trains, the sheet shows "Spell picks 2".
    Shatter and Acid Arrow are dim, since a second-level spell needs character level 3. The
    wizard already knows every first-level spell on the list.
    Covered by `service_view.rs::the_trainer_offers_levels_and_picks_and_the_sellers_their_spells`.

## Part 5: spells for sale

12. **The temple** (4, 1).
    *You will see* a Spells list after the treatments, offering the cleric's spells they don't
    know, 50 gp each. Buy one; the log reads "… learns Guiding Bolt for 50 gp 0 sp 0 cp".
13. **The guild** (10, 1).
    *You will see* the wizard's Shatter and Acid Arrow, dim at level 2. Nothing is offered to a
    member whose class can't learn the guild's spells.

    Covered by `training.rs::spells_are_bought_at_the_temple_and_the_guild`.

## Result

Report pass or fail by step number. Any failure becomes a bug in `tasks/BUGS.md` with a test
that reproduces it before it is fixed.
