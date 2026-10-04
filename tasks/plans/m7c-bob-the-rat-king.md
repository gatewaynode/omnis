# M7c interlude: Bob the Rat King, the first monster that casts

## Context
Step 5's table (`974cdc9`) measured the turn budget against monsters that only swing weapons. The owner
(2026-10-04): baseline data, but not indicative once runbooks and fuller magic and melee exist. **Add a map boss,
5 rats and an enemy wizard "Bob the Rat King", so magic against magic can be seen; then prepare for a compact
before step 6.**

Today monsters cannot cast:
- `resolve.rs::monster_turn` makes weapon attacks only.
- `Trigger::EnemyCasts` exists but nothing raises it.
- Shield answers only an attack roll, never Magic Missile.
- `SpellCast` is member-only (`caster: CharacterId`).
- Every caster helper (`cast.rs`, `omnis-rules/src/spell.rs`) takes a `Character`.

Owner decisions (2026-10-04):
1. **Bob is a homebrew CR 1 caster on SRD rules**, marked homebrew in his file.
2. **His actions are a dice roll** among what he can do now. Monster runbooks and a priority list come when they exist.
3. **He lives in a new room in the depths.**
4. **SRD magic-versus-magic rules come with him:**
   - Shield stops Magic Missile, on both sides;
   - Bob's casts raise `EnemyCasts`;
   - members save against his spells.

## 1. Data, the depths and the text (one commit)
- **`omnis-data/src/monster.rs`**: `Monster` gains `#[serde(default)] casting: Option<MonsterCasting>`.
  - Fields: `spell_attack: i8`, `save_dc: u8`, `caster_level: u8` (for cantrip dice), `points: u8` (each individual's pool), `spells: Vec<String>`.
  - Validated:
    - the spell ids, in `content.rs::check_references`, by the existing `require(file, "spell", …)` pattern;
    - `save_dc` 1..=30 and `caster_level` 1..=20;
    - each spell's effect is one this build lets a monster use: `Attack`, `AutoHit`, `Save`, or a `Reaction` armor bonus. Anything else is refused by name.
  - New `bad_packs` rows cover each refusal.
- **`packs/test/data/monsters/bob_the_rat_king.ron`** (homebrew, built the way an SRD block is):
  - Medium; AC 12; 5d8 HP; abilities (9, 14, 12, 16, 12, 11); CR 1, 200 XP.
  - A quarterstaff melee attack, +1, 1d6−1 bludgeoning.
  - Gold 3d6.
  - Casting: spell attack +5, DC 13, caster level 5 (Fire Bolt 2d10), 5 points.
  - Spells: `base:spell:fire_bolt`, `magic_missile`, `thunderwave`, `shield`.
  - Text keys go in `packs/test/text/en/monsters.ron`.
- **`depths.ron`** grows from 12 to 18 wide.
  - Two new rooms in the east.
  - The throne room (x 12–17, y 6–11) has a door from the south-east room and one from the new north-east antechamber.
  - Bob's group is appended last, `once`: `[("test:monster:giant_rat", 3), ("test:monster:giant_rat", 2), ("test:monster:bob_the_rat_king", 1)]`. With two front stacks, Bob stands in the back row until a rat stack falls; the 3 + 2 split is how "5 rats" keeps him behind them.
  - The header comment says so.
- **Pins:**
  - both replays are rebaselined (the test pack hash);
  - `load_test_pack` gains Bob's group and the depths' width;
  - the base tuple is unmoved (Bob is test content).

## 2. Monster casting in the simulation (one commit)
- **Per-individual state**: `Stack` gains `#[serde(default)] spent: Vec<u8>` (points spent, aligned with `hp`; missing means none).
  - It is removed in lockstep where an individual dies (`resolve.rs:126`).
  - There is no `SAVE_SCHEMA` bump; this follows the `spell_picks` and `budget` precedent.
- **Shields**: `CombatState` gains `#[serde(default)] monster_shields: Vec<(u8, u8)>` (stack, individual).
  - Each entry is cleared at its stack's turn and shifted when an individual dies.
- **New `combat/monster_cast.rs`**, keeping `resolve.rs` and `cast.rs` from growing:
  - **`options`**: for one individual, the weapon attack if one reaches (as `pick_attack` decides today) plus each spell that has a target and enough points, never Shield.
  - **The dice roll**: one roll on the combat stream, uniform over the options. It is recorded in the trace, so replays reproduce it.
  - **Casting** pays the points and emits `Event::MonsterCast { caster: ActorRef, spell }`. It then raises `EnemyCasts` through a new `reaction::on_enemy_cast` (members in marching order; nothing in the packs answers it yet, see below), then resolves the effect:
    - **Attack (Fire Bolt)**: a random living member, `attack_roll` with `spell_attack`, `reaction::on_attack` (the member's declared Shield can answer), then damage. Cantrip dice come from a new `omnis-rules` `cantrip_dice_at(level, …)`, which the member `cantrip_dice` now calls. After damage come the member's wound reactions and concentration, by the existing `monster_attacks_member` chain. `hurt_member` and `keep_concentration` become `pub(crate)`.
    - **AutoHit (Magic Missile)**: a random living member.
      - A member already under Shield takes nothing (SRD).
      - Otherwise the member's declared `Attacked` Shield may fire first. A new `reaction::on_missile` treats it as an outcome the shield changes, and the shield fires only when the member's own criteria hold.
    - **Save (Thunderwave, reach `Stack`)**: every living front-row member. Each makes `stats::save(Con)` against `save_dc` and takes `saved_damage`, with wound reactions per member. Thunderwave's push is not modelled (rows have no distance), as for members' casts.
- **Bob's Shield (a built-in rule, like opportunity attacks)**:
  - It fires on a member's weapon or spell attack that hits Bob and would miss at +5, when Bob's stack has a reaction left and the individual has the points.
  - It also fires on a member's Magic Missile at him under the same conditions.
  - It spends the stack's reaction and the point and emits `MonsterCast { reaction }`.
  - The hooks: the member weapon attack and `cast_attack`/`cast_auto` on a stack.
- **`answers()`** accepts nothing for `EnemyCasts` yet. The trigger is raised, and the trigger-field horizon makes it answerable. The log and the docs say so.
- **The app log**:
  - `spell_text` gains "Bob the Rat King casts Magic Missile" and "… raises a shield", using `names.actor`.
  - A fit test checks both lines against `LONG_CELLS`/`SHORT_CELLS`.
- **Tests (`tests/monster_cast.rs`, real packs, hand-checked SRD numbers)**:
  - The roll picks among the options deterministically, and the options shrink as points run out (Fire Bolt and the staff remain).
  - Fire Bolt rolls +5 against the member's AC; a declared member Shield turns a hit and is spent.
  - Magic Missile does nothing to a shielded member, and a declared Shield fires on it.
  - Thunderwave makes each front member save (DC 13), half on a success.
  - A member's attack or Magic Missile at Bob draws his Shield once a round, and the point is spent.
  - Points stay aligned when an individual of a two-Bob stack dies.
  - `EnemyCasts`: no test can observe the raise while nothing answers it. The review records this honestly, and the trigger field makes it testable.
  - Old saves load with `spent` and `monster_shields` empty.
- **Mutation pass** over the new rules, each break named by the test that fails it.
- **Pins**: the fight replay is rebaselined if the dungeon fight moves (it should not: its rats do not cast).

## 3. Measured (one commit)
- **`measure/boss.rs`, `boss_over_seeds`**:
  - The setup: Bob's group alone, fought by new parties of 2, 4 and 6 brought to levels 1, 2 and 3 (the `to_town` level-up), under the budget policy with Shield declared. 3,000 seeds.
  - The columns:
    - wipe %;
    - rounds;
    - Bob's casts per fight by spell;
    - his Shields;
    - the party's Shields;
    - missiles stopped;
    - members' saves made.
- **Re-run** `budget_over_seeds` and `clear_over_seeds`, since Bob is now the depths' last group.
- **Into the TODO**: both tables, plus any tuning with its reason (Bob's numbers are homebrew and tunable; the SRD rules are not).

## 4. Compact preparation (one commit)
- `tasks/CONTINUITY.md` is rewritten: state, pins, the Bob decisions, and step 6 next.
- `horizons.md` gains:
  - the dice roll as a stand-in until monster runbooks;
  - `EnemyCasts` answerable with the trigger field;
  - monster heals and buffs;
  - Thunderwave's push.
- The TODO gains a Bob block between steps 5 and 6.
- The tree is clean and the gate green before the compact.

## Size
- About 650 src and 550 tests.
- **Pressure files**:
  - `resolve.rs` and `cast.rs` only gain hooks;
  - `ops.rs` and `command.rs` are untouched;
  - `depths.ron` grows by about 25 lines.

## Verification
- **Every commit**:
  - `DEVELOPER_DIR=/Library/Developer/CommandLineTools scripts/verify.sh` prints `VERIFY-GREEN`;
  - Sentrux: `git add`, scan `/Users/john/code/omnis/crates`, then `check_rules`;
  - sim lints (integers, `BTreeMap`/`Vec`);
  - files are staged by name; the owner pushes.
- **Pins**:
  - replays are rebaselined in step 1, and in step 2 only if they move;
  - the base tuple stays `(4, 4, 3, 24, 16, 18, 3, 34, 7)`;
  - `SAVE_SCHEMA` stays 6;
  - MCP 20 tools; proof 93/141 unmoved (no command changes).
- **Behaviour**:
  - real-data tests with hand-checked numbers (DC 13, +5, 2d10 Fire Bolt at caster level 5, half on a save);
  - the mutation pass;
  - the boss table before the compact.
- **By hand** (the owner, later): walk to the depths' east room and fight Bob. The log shows his casts, both sides' shields and the saves.

## Out of scope (horizons)
- Monster runbooks, priority lists and auto play.
- Monster heals and buffs.
- Counterspell and any answer to `EnemyCasts`.
- Thunderwave's push.
- Bob's points in the MCP and CLI views (step 6 can add them).
- The fight screen on `bevy_ui`.
