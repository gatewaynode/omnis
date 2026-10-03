# M7c: the turn budget, declared reactions, the first class features with effect

## Context
M7b is closed (owner manual test 2026-10-03, CI clear). M7c is M7's last acceptance point (PRD §14, owner
2026-09-20): the turn budget (D21), declared reactions (D22, §7.9), D24's spell fields and the first class features
with effect: Second Wind, Action Surge, Cunning Action. Closing M7c closes the M7 series.

What exists today:
- A turn is one command.
  - `act_inner` always ends with `step_current` and `run_until_member` (`omnis-sim/src/combat/turn.rs:161`).
  - The only early exit is a successful Run.
- Shield is the only reaction.
  - `try_shield` (`combat/reaction.rs`) casts it for a member whose `Character.auto_cast` names it.
  - It only fires when the shield would turn a hit into a miss.
  - It has no per-round limit beyond "no armor bonus already up".
  - It has one call site, `resolve.rs:239`.
- Class features are labels: `ClassFeature { level, name }`, "display only".
  - Nothing tracks uses per rest.
  - The rogue class exists, with `cunning_action` at level 2.
- Monsters never flee and have no reactions.
- The fight screen is drawn on the raster canvas.
  - The files are `combat_menu.rs` (810 lines), `combat_screen.rs`, `spell_menu.rs` and `combat_text.rs` (815).
- `SAVE_SCHEMA` is 5, and there is no v5 fixture.
- There are 31 rule slots, pinned in the tuple `(4,4,3,24,16,18,3,31,7)`.

Owner decisions (2026-10-03, asked in plan mode):
1. **The fight screen stays on the canvas.** Moving it to `bevy_ui` is deferred to horizons (the UI branch). The
   canvas gets an End turn action, a budget line and the reactions switch.
2. **Tactics: the full data shape, reactions only.**
   - `Character.tactics` holds ARCH §4.7's whole shape: the switch, the auto flag, the criteria library, the runbooks
     and the default runbook.
   - M7c builds the reactions: the closed trigger list, the criteria walk, the in-fight switch, and a Feathers tactics
     panel beside the sheet. The panel edits the default runbook's reaction entries from the predicates the game
     offers.
   - There is no runbook editor. `auto` is stored but has no effect. Both stay unscheduled (PRD §14).
3. **Opportunity attacks: monsters take them, by a built-in rule** until monster runbooks exist.
   - Each front stack with a reaction left makes one attack with its lead individual. It attacks a front-row member who
     leaves the engagement:
     - when that member exchanges to the back row;
     - when the party runs successfully: a random front-row member.
   - Cunning Action's exchange provokes none.
   - Members may declare an opportunity attack on "an enemy flees". It cannot fire until monsters can flee.
4. **String ids stay a horizon.** Schema 6 carries only the tactics and the turn state.
5. **Cunning Action is an exchange plus Hide.**
   - Dash and Disengage fold into one option: an exchange paid with the bonus action that provokes no opportunity
     attack.
   - Hide is a Stealth check against the best passive Perception (10 + Wis modifier) among the living stacks. On a
     success, the rogue's next attack this fight has advantage.
   - Sneak Attack stays a label.
6. **Preparation: the fields only.**
   - Every spell file carries D24's three fields, and they are validated.
   - `bonus_action_available` is honoured.
   - A spell with `preparation_required_for_bonus_action` cannot be cast with the bonus action yet. `Prepare` stays
     open in PRD §14.

**One proposed change to ARCH §4.7's wording, for your approval with this plan.**
- ARCH ends a turn "when no action and no bonus action is left".
- A wizard who attacks would then have to press End turn every turn, holding a bonus action they cannot spend.
- Proposed rule: the turn also ends by itself when no action is left and the member has nothing a bonus action can pay
  for. That means:
  - no spell they may cast with the bonus action and can afford;
  - no bonus-action feature with uses left.
- `EndTurn` ends a turn early.
- The ARCH text is shown to you in step 8 before it changes. (Approved with the plan, 2026-10-03.)

## Step 0: docs and the v5 fixture (two commits)
- **Plan:** this plan is saved as `tasks/plans/m7c-turn-budget.md`.
- **TODO:** the M7c block in `tasks/TODO.md` records the decisions above.
- **Horizons:** `horizons.md` gains:
  - the fight screen on `bevy_ui`;
  - the runbook editor, encounter criteria and auto play;
  - monster and hireling runbooks;
  - opportunity attacks as monster tactics;
  - Sneak Attack;
  - Prepare;
  - members' "enemy flees" and "enemy casts" triggers, which arrive when monsters can flee and cast.
- **v5 fixture:**
  - A `capture_schema_5_fixture` test (ignored, the existing pattern at `save_and_replay.rs:489`) writes
    `tests/saves/v5.ron`.
  - The party for the fixture has a wizard with shield in `auto_cast` and is saved mid-fight.
  - It is committed on the current build, before anything changes.

## Step 1: split the two big app files (one commit, no behaviour change)
- **`combat_menu.rs` (810 lines):** `FightView` and `fight_view` move to `fight_view.rs`.
- **`combat_text.rs` (815 lines):** the round and wound lines move to `round_text.rs`.
- **Pins:** the gate stays green, and every pin, replay and the tuple are unmoved. Sentrux is checked.

## Step 2: data (one commit)
- **New data types in `omnis-data`:**
  - `Cost { Action, BonusAction, Reaction }`.
  - Spells gain `cost: Cost` (default `Action`) and D24's three bools, with serde defaults.
  - Validation:
    - a `Reaction` spell must have a `Reaction` effect, and the other way round;
    - `bonus_action_available` is refused on a reaction spell;
    - `preparation_required_for_bonus_action` requires `bonus_action_available`.
- **Spell files:**
  - All 18 spells state the fields explicitly.
  - Healing Word and Spiritual Weapon are `bonus_action_available` (SRD casting time: 1 bonus action).
  - Shield is `cost: Reaction`.
- **Class features:**
  - `ClassFeature` gains `effect: Option<FeatureEffect>`, `cost: Cost` and `uses: Option<Uses { count, per: Rest }>`.
  - `FeatureEffect` has three variants:
    - `Heal { dice, per_level }`: Second Wind, 1d10 + fighter level, bonus action, once per short rest;
    - `ExtraAction`: Action Surge, no cost, once per short rest;
    - `Cunning`: bonus action, no uses.
  - `fighter.ron` and `rogue.ron` are filled. Other features stay labels.
- **Rule slots:** `turn.actions`, `turn.bonus_actions` and `turn.reactions` go in `combat.ron`.
  - Inputs: `level` and `is_member`.
  - Each slot's expression is `1`, the SRD baseline.
- **Test pack:** a reaction spell `test:spell:ward` (a heal costing a reaction) proves triggers other than Attacked.
- **Pins moved:**
  - the tuple: slots 31 → 34;
  - `bad_packs` rows for each new refusal;
  - both replays are rebaselined (the packs change).

## Step 3: the turn budget in the simulation (one commit)
- **Turn state:**
  - `CombatState` gains `budget: Budget { actions, bonus_actions }` for the current actor.
  - It also gains `reactions: Vec<(ActorRef, u8)>`.
  - Both are evaluated from the slots at the start of each actor's turn. Reactions refresh at the start of a
    combatant's own turn; stacks are refreshed on their turn.
- **`act_inner`:**
  - It no longer always steps.
  - After a command it pays the cost. It then ends the turn when one of these holds:
    - no action is left and no bonus option is payable (`turn::can_continue`);
    - the command was `EndTurn`;
    - the fight ended.
- **Commands:**
  - `CombatCommand::EndTurn` is new.
  - `Cast` gains `pay: Pay { Action, BonusAction }`, with serde default `Action`.
  - Attack, Use, Dodge, Exchange and Run cost an action.
- **New rejections:** `NoActionLeft`, `NoBonusActionLeft`, `NotABonusAction` (and one for a spell whose bonus action
  needs preparation), `NoUsesLeft`, `NoSuchFeature`. Each is checked before the first die, on the roller's copy, as
  today.
- **Class features:**
  - `CombatCommand::Feature { feature: u8, choice: FeatureChoice { None, Exchange { with }, Hide } }`, where `feature`
    is the row in the member's class features with effect.
  - `Character.feature_spent: Vec<(String, u8)>` (sorted, serde default) is reset by both rests in `rest.rs`.
  - Second Wind heals 1d10 + level.
  - Action Surge adds one action to the current budget.
  - Cunning Action:
    - the exchange is paid with the bonus action and provokes no opportunity attack;
    - Hide makes a Stealth check against `max(10 + wis_mod)` over the living stacks;
    - a success gives an `ActiveEffect` granting advantage on the next attack. It ends when used or when the fight
      ends, and reuses the existing advantage roll.
- **Opportunity attacks:**
  - A front-row member exchanging to the back row (an action, not Cunning) provokes them.
  - A successful Run provokes them too, before `finish`.
  - Each front stack with a reaction left: its lead individual makes one melee attack, and the stack's reaction is
    spent.
  - Event: `Event::OpportunityAttack { stack, member }`, followed by the usual attack events.
- **Events:** `Event::BudgetSpent { actor, budget }` after each paid command, for the screen and the MCP, and
  `Event::FeatureUsed`.
- **The test policies:** `measure.rs`'s policy and `common` loop send `EndTurn` when a member's budget remains but the
  policy has nothing to add.
- **Tests (sim, real data):**
  - one per rejection;
  - a wizard's turn ending by itself after an attack;
  - a cleric's attack plus Healing Word in one turn;
  - a fighter's Action Surge giving two attacks;
  - Second Wind's dice and its use spent and reset by a short rest;
  - Cunning exchange with no opportunity attack and a plain exchange provoking one;
  - Hide's advantage, spent on the next attack;
  - reactions refreshing per round.
- **Mutation pass:** every new rule.
- **Pins:** the fight replay is rebaselined.

## Step 4: declared reactions and schema 6 (one commit)
- **Types (new `omnis-rules/src/tactics.rs`):**
  - ARCH §4.7's types: `Tactics`, `CriteriaSet`, `Criteria`, `Predicate`, `Runbook`, `Trigger`.
  - Also `ActionRef { Attack, Spell(SpellId), Item(ItemId), Feature(String) }`, `Cmp` and `Who`.
  - `limits.rs` caps them:
    - names of 32 bytes, printable;
    - criteria depth 4 and 16 nodes;
    - library 32;
    - runbooks 8;
    - entries 16.
- **Walk:**
  - `tactics::holds(criteria, context)` evaluates integer predicates over a `Context` built from the world, the combat
    state and the trigger.
  - `WouldChangeOutcome` is the shield rule, moved out of `try_shield`.
- **Resolution (`combat/reaction.rs`, rewritten):**
  - `fire(trigger, subject)` walks members in marching order.
  - Each member is considered only if `reactions_on` is set and they have a reaction left. Proximity follows PRD §8.3
    (same row, the engaged lead stack, anyone for ranged).
  - For each member, the default runbook's entries are tried in order. The first whose trigger matches, whose criteria
    hold and whose cost can be paid resolves like a command:
    - it spends the reaction;
    - it emits `Event::Reaction { actor, trigger, action }` ahead of its own events.
  - At most one reaction fires per combatant per trigger.
- **Triggers raised in M7c:**
  - `Attacked` and `MemberAttacked`, before a monster's attack is judged;
  - `MemberWounded`, after damage;
  - `MemberDying`, at 0 hit points;
  - `SpellCast`, after a member's cast.

  `EnemyFlees`, `EnemyCasts` and `OwnTurn` are in the list but have no source yet.
- **Commands:**
  - `PartyCommand::Tactics(TacticsCommand)` replaces `AutoCast`.
  - `SetReactions { member, on }` costs nothing and is accepted in a fight.
  - Accepted while exploring:
    - `PutReaction { member, at: Option<u8>, set: CriteriaSet }` stores the set in the library (deduplicated) and puts
      the entry in the default runbook;
    - `RemoveReaction { member, at }`.
  - Only reaction-cost actions the member has can be declared.
  - Every input is validated (names, caps, ids, the member owning the spell).
- **Save schema 6:**
  - `Character.tactics` replaces `auto_cast`. `CombatState` and `feature_spent` get their new fields.
  - `v5_to_v6` turns each auto-cast spell into a criteria set `(Attacked, WouldChangeOutcome)` in the default runbook,
    so an M6 or M7 save fights as it did.
  - Test: `a_schema_5_save_migrates` against the step 0 fixture, plus the existing migration tests moved to 6.
- **Tests:**
  - shield through tactics;
  - one reaction per round;
  - the switch off in a fight;
  - `test:spell:ward` on `MemberWounded` for a same-row ally only;
  - criteria `All`/`Any`/each predicate;
  - each cap refused;
  - a replay reproducing a declared reaction.
- **Mutation pass.**
- **Pins:** both replays are rebaselined, and `SAVE_SCHEMA` becomes 6.

## Step 5: measured (one commit, harness then table)
- **Harness:** `measure.rs` gains `budget_over_seeds`. It uses 300 seeds and parties of 2, 4 and 6, with the
  `clear_over_seeds` route.
- **What it compares:**
  - M7b's one-command turns, simulated by a policy that ends every turn after one command;
  - M7c's policy, which also uses:
    - Second Wind under half hit points;
    - Action Surge on round 1;
    - Healing Word with the bonus action;
    - shield declared;
    - opportunity attacks on.
- **Columns:**
  - wipe %;
  - rounds per fight;
  - XP per member;
  - shield fired per fight;
  - opportunity attacks per run;
  - end level.
- **Tuning:** the table goes in the TODO before you play. Any number tuned is recorded with its reason. The slots stay
  at 1 (SRD), and the open-world guidance applies (sanity checks, tuned for four).

## Step 6: MCP and CLI (one commit)
- **Schema:**
  - Combat branches go 5 → 7: `EndTurn` and `Feature`, and `Cast` gains `pay`.
  - Party branches: `AutoCast` is replaced by `Tactics` (three commands).
  - `schema_proof`'s counts and successor arms move.
- **`combat.get`:** the view carries the budget, the reactions left, each member's features with uses left, and
  `reactions_on`.
- **`party.get`:** the view carries each member's tactics (the default runbook's reactions).
- **CLI:** script words `end`, `cast` with a bonus-action form, `feature-F[-W|-hide]`, `react-M-on|off`. The CLI's
  schema print is updated.
- **Pins:** 20 tools stay; the proof counts are recorded.

## Step 7: the app (one or two commits)
- **Fight screen (canvas):**
  - The action row gains End and React (on/off for the acting member). The column constants and their compile-time
    asserts move.
  - A budget line shows "Action 1 · Bonus 1 · Reaction 1".
  - The Use picker lists the member's features with effect: Second Wind, Action Surge, and Cunning Action as two rows,
    Exchange and Hide. Each row shows its uses left, or is dim with its refusal.
  - The cast picker sends `pay: BonusAction` when the spell allows it and a bonus action is left, otherwise `Action`.
  - The shield "auto" row is replaced: reactions are set on the tactics panel.
- **Log lines:** `Reaction`, `OpportunityAttack`, `FeatureUsed`, Hide and the turn's end, in `combat_text`/`spell_text`.
  Each fits `LONG_CELLS`/`SHORT_CELLS`.
- **Tactics panel (Feathers, new `UiScreen::Tactics`):**
  - It is reached by a TACTICS button on the sheet, while exploring.
  - Bevy-free model: `tactics_panel.rs`. Layout: `feathers_tactics.rs`.
  - Per member:
    - the reactions switch (a checkbox);
    - the default runbook's reaction entries, each with an action dropdown and a trigger dropdown;
    - a list of predicates combined by an All/Any dropdown, each predicate set by dropdowns and number inputs;
    - add, remove and save buttons.
  - Reuses `dropdown`, `offer_list`, the checkbox and `FeathersNumberInput`.
- **Tests:**
  - by event and by pointer at both sizes;
  - `layout_faults`;
  - the text tree;
  - the fight's End and React by click (`tests/combat.rs`);
  - every line fits.
- **Pins:** no replay moves.

## Step 8: docs, review, acceptance c (the M7 done-when)
- **ARCH v0.7:** §4.7 becomes "as built in M7c", including the turn-ending rule above, plus the §9 tool rows.
  - The text is shown to you first.
- **PRD §14:** a line saying the preparation and runbook questions remain open after M7c. I'll ask before editing.
- **Knowledge files:** `code-map.md` and `verification.md`.
- **The M7c review in the TODO:** commits, tests, rebaselines, pins, size against the estimate, and files over 800
  lines.
- **`tasks/acceptance/m7c.md`:** a script for you, each step naming its test:
  1. a fighter surges and second-winds;
  2. a cleric attacks and casts Healing Word in one turn;
  3. a rogue hides, then attacks with advantage;
  4. Cunning exchange versus a plain exchange that draws an opportunity attack;
  5. shield declared on the tactics panel and firing once per round;
  6. the reactions switch off in a fight;
  7. an M7b save refused or migrated as expected;
  8. End turn.
- **Close M7:** after your test, M7 is marked closed in the TODO and the plan.

## Size
- **Estimate:** about **2,300 src and 1,900 tests**. The tactics panel (about 700) and step 3 (about 600) are the
  largest pieces.
- **M7b's record:** src came in at 0.8× its estimate and tests at 1.2×.
- **Pressure files:**
  - `turn.rs` (398) grows. Feature resolution goes in `combat/feature.rs` and opportunity attacks in
    `combat/opportunity.rs`.
  - `ops.rs` (829) is not grown; new view fields live in `view.rs`.
  - `screen.rs` and `plan.rs` are untouched.

## Verification
- **Every commit:**
  - `DEVELOPER_DIR=/Library/Developer/CommandLineTools scripts/verify.sh > log 2>&1`, then `VERIFY-GREEN`;
  - Sentrux: `git add` new files, `scan` `/Users/john/code/omnis/crates`, then `check_rules`;
  - the sim lints (integers, `BTreeMap`/`Vec`);
  - stage by name, one commit per item; you push.
- **Pins:**
  - replays are rebaselined only in steps 2, 3 and 4;
  - the tuple moves only in step 2 (slots 34; spells unchanged at 18 in base);
  - `SAVE_SCHEMA` goes 5 → 6 in step 4;
  - the MCP stays at 20 tools, with the proof counts recorded at step 6.
- **Behaviour:**
  - real-data sim tests with hand-computed SRD numbers (Second Wind 1d10 + level, Hide's DC);
  - mutation passes in steps 3 and 4, each break named by the test that fails it;
  - the measured table before play;
  - headless app tests by event and by pointer;
  - your acceptance c.

## Out of scope (horizons)
- The fight screen on `bevy_ui`.
- The runbook editor, encounter criteria, auto play and `tactics::choose`.
- Monster and hireling runbooks.
- Prepare.
- Sneak Attack damage.
- Turn-budget curves above 1.
- String ids in saves.
- Extra Attack.
- Members' "enemy flees" and "enemy casts" triggers firing.
