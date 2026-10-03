# M7b: progression (the trainer, spell picks, spells for sale, the deeper dungeon)

## Context
M7a is closed (acceptance a passed 2026-10-03, CI green on `ba79a29`). M7b closes the Phase 1 loop of PRD §13:
create a party, clear the dungeon, return to town, level up. This track is mechanics first: new screens are
minimal Feathers panels on the existing kit, enough for manual testing.

What exists today:
- XP accrues: `award_xp`, `combat/turn.rs:366` splits it among the living. Nothing levels a member.
- `level_for_xp` (`omnis-rules/src/stats.rs:35`) has no caller.
- `hit_points.per_level` (`packs/base/data/rules/leveling.ron`) is never evaluated.
- The trainer and guild have data and price slots, but `service_view.rs:110` gives them no offers and every
  command inside them is `NotOffered`. The app shows a "(M7b)" note.
- Known spells are fixed at creation by list order (`character.rs:337`). A wizard knows every levelled spell on
  its list at level 1; a cleric never learns `healing_word`.
- The dungeon (`packs/test/data/maps/dungeon.ron`) places rats only. Goblins and skeletons are defined but not
  placed, there is no way down, and nothing defines a "clear".
- The sheet already shows XP and the next threshold (`sheet_screen.rs:85`).

Owner decisions (2026-10-03, asked in plan mode):
1. **Picks owed, one per press.** `Train { member }` grants one level and adds the class's picks to a new
   `Character.spell_picks`. The trainer then offers a free `Choose { member, spell }` for each eligible spell while
   picks remain. Every offer stays one exact command, as in M7a, and the panel is buttons only. A pick with nothing
   eligible waits for a later level. Wizard 2 a level (the SRD spellbook); cleric 1 a level (the "level" term of
   "Wis mod + level", PRD §8.3).
2. **The temple sells spells too** (PRD §8.2 "Guilds and temples sell spells"), as data. A `spells` list is allowed
   on a temple as on a guild. `Learn` works in either, gated by the caster's class list and maximum spell level.
3. **Spells: four 2nd-level and three 1st-level.**
   - 2nd: Shatter (exact); Acid Arrow (the delayed damage dropped); Spiritual Weapon (one attack, no persistence);
     Prayer of Healing (one target).
   - 1st: Thunderwave (the push dropped); Guiding Bolt (the advantage rider dropped); Inflict Wounds (exact).
   - Every deviation is named in its spell file. All seven are in `docs/dnd.srd.5.1/07_Spells`.

## Step 10: rules and sim (one commit; a docs commit first)
- **Docs first:** the M7b block in `tasks/TODO.md` and `tasks/plans/m7-town.md` get the three decisions. The
  horizons gain level-ups for half casters; cantrips gained by level are already there. No vision edit yet.
- **Data (`omnis-data`):**
  - `Casting.spells_per_level: u8` (serde default 0): wizard 2, cleric 1.
  - `casting.ron` gets a `max_spell_level` table, the SRD's full-caster column for levels 1–20
    (1,1,2,2,3,3,4,4,5,5,6,6,7,7,8,8,9,9,9,9). It is read by the existing `table()` helper.
  - `ServiceDef.spells` is now valid on `Temple` and `Guild` (validation at `service.rs:58-60`, with a bad-pack row
    for the trainer).
  - The price slot `guild.spell_cost` is renamed `spell.learn_cost`, since both places use it. The slot count is
    unchanged.
- **New `omnis-rules/src/level.rs`:**
  - `ready(character, data) -> Result<bool>`: `level_for_xp(xp) > level` and `level < 20`.
  - `level_up(character, data, rng) -> Result<Gains>`, where `Gains { hp, spell_points, picks, proficiency }`.
    - Hit points come from `hit_points.per_level` with the dwarf's per-level effect (reuse `character.rs:209`), so
      the average is taken.
    - The pool is recomputed by `spell_point_pool` at the new level; current points rise by the difference.
    - `hp` and `hp_max` rise by the gain. The hit dice total is the level, so one die is added unspent.
    - `picks += spells_per_level`.
    - Proficiency comes from the table.
    - The level's features stay labels: `features_at(class, level)`.
  - `max_spell_level(level, data)` and `eligible(character, data) -> Vec<SpellId>`: on the class list, level 1 or
    higher, at or below the maximum, and not known.
- **`Character.spell_picks: u8`** (serde default). `SAVE_SCHEMA` stays 5: a v5 save without the field means 0 picks,
  which is correct. Schema 6 stays M7c's (ARCH §4.7). Both replays are rebaselined anyway, because the packs change.
- **Sim (`omnis-sim/src/service.rs`, following Raise at :255):**
  - `ServiceCommand::Train { member }` (trainer).
    - Refusals: unknown or dead member; `NotReady { xp, needed }`; `MaxLevel`.
    - Price: `trainer.cost(level = new level)`, then `afford`.
    - Takes `service_minutes`.
    - Emits `Event::LevelUp { member, level, gains, features }`. The feature text keys are ids.
  - `Choose { member, spell }` (trainer, free, no minutes).
    - Refusals: `NoPicks`, `NotOnList`, `SpellTooHigh`, `AlreadyKnown`.
    - Emits `SpellLearned { member, spell, cost: 0 }`.
  - `Learn { member, spell }` (guild or temple).
    - `kind()` can no longer map Learn to one kind, so the "offered here" check becomes
      `offered(kind, command)`.
    - Refusals: `NotStocked`, then the same spell checks as Choose.
    - Price: `spell.learn_cost(spell_level)`.
    - Takes `service_minutes`.
    - Emits `SpellLearned { member, spell, cost }`.
  - Each refusal is a new `Rejection` variant, worded in `fmt_town`. Everything is validated in full before anything
    changes; the gains roll on the `town` roller copy.
- **Schema and proof:** `omnis-mcp` gets three service branches (9 → 12) and three successor arms in
  `next_service` (proof 77 → 80 instances, 111 → 114 branches). The tool count stays 20. The `service_get` and
  `Command` descriptions name the new commands.
- **Tests:** sim tests per command and per refusal, with real data, in `omnis-sim/tests/services.rs`. The test
  asserting trainer and guild are `NotOffered` (:338-370) is rewritten. Also covered:
  - level 1 → 2 → 3 for each class, with exact HP, pool, picks and proficiency against hand-computed SRD values;
  - a dwarf's extra hit point each level;
  - a v5 fixture still loads.

  Mutation check: every rule break fails a test.
- Estimate about 500 src, 450 tests.

## Step 11: content, measured first (two commits: the harness, then the content)
- **Harness (`omnis-sim/tests/measure.rs`, ignored, printed):** `clear_over_seeds`.
  - 300 seeds, parties of 2, 4 and 6 (`common::party_of`), policy `cast_or_attack`.
  - Every `once` placement on both dungeon maps is fought in order (`run_fight`'s pattern). The party's state
    carries over between fights. A short rest spends dice for any member under half, and one long rest is taken
    before the deeper level.
  - Columns: wipe %, XP per member, gold per clear, and fights to levels 2 and 3. The trainer's level 2 and 3 costs
    and the temple's raise are shown against gold per clear.
  - The harness is committed first and run on today's dungeon as the baseline.
- **Content:**
  - `dungeon.ron` gains goblin and skeleton placements (once) in unused rooms, and stairs down at a tile away from
    the tested paths (`walk_to_the_rats`, the replays).
  - A new `packs/test/data/maps/depths.ron` (the deeper wing): goblin and skeleton placements, a random table of
    both, and the stairs back up.
  - The seven spell files go in `packs/base/data/spells/`.
    - Class lists: the wizard gets Thunderwave, Shatter and Acid Arrow; the cleric gets Guiding Bolt, Inflict Wounds,
      Spiritual Weapon and Prayer of Healing.
    - The guild stocks the wizard's new spells; the temple stocks the cleric's.
- **The table goes in the TODO before you play.** Placement counts and the trainer's price (today a placeholder) are
  tuned from it, and the numbers you choose are recorded.
- **Moved in this commit:**
  - the tuple: spells 11 → 18, rule slots unchanged;
  - `load_test_pack.rs`'s encounter assertions;
  - `load_base_pack.rs`'s wizard list length;
  - both replays are rebaselined.
- Estimate about 350 data, 200 tests.

## Step 12: ops, MCP, app (one commit)
- **`service_view.rs`, one offer per exact command:**
  - Trainer: `Train` for each member (priced, or dim with `NotReady` and its XP), then `Choose` for each eligible
    spell of each member with picks.
  - Guild and temple: `Learn` for each caster and each stocked spell on that caster's list (dim when too high or
    known).
  - The view-equals-command fingerprint test is extended.
- **`ops`:** `game.status` gains the current map's placements cleared out of the total, so a "clear" can be seen.
  This goes in its own small function outside `ops.rs` (817 lines). `party.get` gains `spell_picks` and `ready`.
- **Script words:** `train-M`, `choose-M-<spell>`, `learn-M-<spell>`, with the spell named by its short name.
- **App:**
  - `service_panel.rs` captions for the new offers (for example "Train Brenna to level 2", "Corin learns Healing
    Word"). The trainer, guild and temple use the existing `offer_list` (`feathers_service.rs:89`), and the "(M7b)"
    note is removed.
  - Log lines for `LevelUp` (with the new features' names) and `SpellLearned`, in `service_text.rs`.
  - The sheet adds "ready to train" beside XP and a "Spell picks N" line, following the hit dice line
    (`sheet_menu.rs:98`, `sheet_screen.rs:104`).
- **App tests**, following step 7:
  - extend `tests/service.rs::every_offer_sends_what_the_view_promised` to the trainer, guild and temple;
  - rewrite `every_service_opens_its_panel`'s "(M7b)" assertion;
  - check by pointer and the layout at both sizes;
  - dump the text tree.
- Estimate about 400 src, 400 tests.

## Step 13: docs, review, acceptance b
- **ARCH §4.5:** a "Progression as built in M7b." paragraph. I'll show it to you as a diff and apply it only after
  you approve (ask first). `code-map.md` and `verification.md` get the new files and counts.
- **The M7b review in the TODO:**
  - commits, tests, rebaselines, pins;
  - the size against the estimate below;
  - the files over 800 lines;
  - the clear table.
- **`tasks/acceptance/m7b.md`:** a script for you, with the covering test named for each step:
  1. create a party;
  2. clear both dungeon levels (`game.status` shows the count);
  3. return to town;
  4. train;
  5. choose a spell;
  6. buy one at the guild or the temple;
  7. see the sheet and the log.

## Size
M7a overshot: net +4,940 lines of src against an estimate meant to cover M7a and M7b together. For M7b I
estimate about **1,300 src and 1,050 tests**. The review measures it against this number.

Pressure files:
- `service.rs` (565) grows by about 200. If it passes 800, the spell checks move to `service_spells.rs`.
- `ops.rs`, `screen.rs`, `combat_text.rs` and `plan.rs` are not grown.

## Verification
- **Every commit:**
  - `DEVELOPER_DIR=/Library/Developer/CommandLineTools scripts/verify.sh > log 2>&1`, then `VERIFY-GREEN`;
  - Sentrux `scan` of `/Users/john/code/omnis/crates` and `check_rules`;
  - the sim lints (integers, `BTreeMap`/`Vec`);
  - stage by name, one commit per item; you push.
- **Pins:**
  - the replays are rebaselined only in steps 10 and 11 (the packs change), and stay unmoved in 12;
  - the tuple moves only in step 11;
  - `SAVE_SCHEMA` stays 5;
  - the MCP stays at 20 tools, with the proof at 80/114 from step 10.
- **Behaviour:**
  - real-data sim tests, including SRD values for each level-up hand-computed in the test;
  - a mutation pass over every new rule;
  - the clear table before you play;
  - headless panel tests by event and by pointer;
  - on the running game over the dev socket: `service_get` at the trainer, guild and temple, `screen_text`, and a
    screenshot;
  - your acceptance b.

## Out of scope (horizons)
- Spell picks at creation (the cleric's `healing_word` comes at level 2 or from the temple).
- Cantrips gained by level.
- Half casters.
- Ability score improvement, subclasses, Extra Attack.
- Features with effect (M7c).
- 3rd-level spells.
- Splitting the files over 800 lines.
