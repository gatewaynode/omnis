# M7: town, services, rest and progression (the turn budget as its last acceptance point)

## Context
M6 is closed (owner, 2026-09-20): M6c's play-test went well and the Feathers experiment is "a success … committing to
moving the rest of the play UI over bit by bit" (`tasks/plans/feathers-experiment.md`). M7 closes the Phase 1 loop of
PRD §13: create a party, clear the dungeon, return to town, level up. Today none of it exists: no rest (pools only
empty), no town map (the base pack has no maps; `packs/test` has `dungeon.ron` and `meadow.ron`), `Interact` opens doors
only, `cost_cp` is read nowhere, `Party.food` is never eaten, `level_for_xp` has no caller, class features are text
labels, `Settings::may_save(at_inn)` is always called with `false`.

Owner decisions for this plan (2026-09-20, asked in plan mode):
1. **Town first, the turn budget last in M7**: M7a town, services and rest; M7b trainer and level-up; M7c the turn
   budget, declared reactions and the first features with effect, **outlined here and planned in plan mode when reached**.
2. **The shipped build carries `bevy_ui` at M7's start**; the canvas party-creation screen goes in that step; M7's
   screens are born in `bevy_ui` with the creation panel's pattern.
3. **Rules to level 20, content to level 3**: the dungeon is filled out and measured; a few 2nd-level spells; ability
   score improvement, subclasses, Extra Attack are horizons.
4. **ARCH §4.7 stands**: an auto member's turn is resolved inside the simulation as a monster's is; the log holds the
   player's commands only (not built in M7).

CI for the push to `5bce8b7` was still running when planning started (it is the first Linux build of `bevy_ui`): **a red
result is fixed before step 1.** `c426d8f` and `53d5320` are unpushed docs commits.

## What binds the design (read from the documents and the code; re-check before each step)
- PRD §7.4: "Every service is a data-defined building placed on a map tile; there is nothing special about 'a town'
  beyond its tiles." Inn: save, rest (swap, hire: D10 sidecar, horizon). Temple: heal, cure, resurrect (donate: horizon).
  Training: level up for a fee. Blacksmith: buy, sell (identify, repair: no such item state yet, horizon). Tavern: food,
  rumors (hirelings, quest hooks: horizon). Bank: deposits (interest runs on the region clock: M8). Guild: spells for sale.
- PRD §8.2/§8.3: long rest "costs food, restores fully, may be ambushed"; short rest as the SRD (spend hit dice; hit dice
  do not exist yet); "Experience accrues anywhere; the level is granted in town"; "Gold cost is data"; prepared-list
  classes "gain a fixed number per level and buy or find the rest"; inn-only saves are D17's hardest setting.
- ARCH §4.1/§4.2 already name `Mode::Town(ServiceState)`, `Command::Rest`, `Command::Service(ServiceCommand)`,
  `Event::LevelUp { member, level, gains }`; §4.4: a rest or a service advances the party clock by data-defined minutes;
  events carry ids, never text; validate, then mutate; dice on a copy of the stream.
- Code: `MapDef` has `kind: MapKind::Town`, `portals` (cross-map moves work, `apply.rs` `r#move`), terrains as data
  (`glyph`, `color`, `floor`, `block`); the painter knows walls, doors and open edges only. `advance()` is the one clock
  writer. `party::heal`, `items::{add_to, take_from, count_of, consume}`, `stats::{level_for_xp, proficiency_bonus,
  spell_point_pool}`, slots `hit_points.per_level` and `spell_points.pool(level, …)` exist. Adding `#[serde(default)]`
  fields needs a three-line `v4_to_v5`; any serialized-`World` or `packs/` change rebaselines both replays in its commit.
  A new `Command` variant needs a schema arm and a successor arm in `omnis-mcp/tests/schema_proof.rs` (the build fails
  until it has one). The base-pack tuple `(4,4,3,24,16,11,3,19)` moves with every new slot, item or spell.
- App: a `bevy_ui` screen is a Bevy-free model with `apply`, scenes with ids, one observer per payload, `reconcile` by a
  shape hash, `sync`, and a headless test file (census, events, pointer, layout faults, text tree). Generic already:
  `Payload`, `fitted_scale`, `scale`, `place`, the fonts (but `wear` only looks under the creation root), most test
  helpers. Creation-typed: `Control(PanelId)`, `Shown(LabelId)`, `Reports`, `escape_abandons`. The tool pad (six) and the
  pause overlay are full. Canvas creation is used by `tests/pointer.rs` (8 uses) and `tests/combat.rs` (3).
- Gap found: CI does not run the gate's release clippy line (`--no-default-features --lib`).

## M7a: town, services, rest
0. **Docs.** The M7 block in `tasks/TODO.md` with these items. Vision edits, each flagged in the commit so the owner can
   strike any: ARCH A11 and §8.4 (the experiment's outcome: `bevy_ui` with Feathers is the play interface's toolkit, the
   canvas toolkit stays until each screen moves, the screen pattern; **the editor on Feathers instead of `bevy_egui` is
   proposed, not assumed: struck unless the owner says yes, and only then does the 2026-10-08 re-audit lapse**), §8.1
   (features as step 1 leaves them), §4.7 (owner confirmed auto resolution; the budget is M7c; its save schema is 6
   because M7a takes 5), PRD §11.1 and D26 (outcome, Inter, the scale rule), §14 (what closed; open: frame time on a
   visible window, vendoring at Bevy 0.20). ~80 lines.
1. **The shipped build carries the interface.** The `feathers` cargo feature is removed: `bevy` gets `ui` and
   `bevy_feathers` unconditionally, the `cfg(feature = "feathers")` lines go, `capture.rs` and the socket's window target
   stay under `devtools`. The gate's release clippy line now covers a build with `bevy_ui`, and **CI gains that line**.
   Removed: the canvas creation painter (`screens.rs::creation`), `CreationForm`'s key path, `Screens.skin`, the
   `menu_keys` arm; `Menu::Covered` stays as what the canvas shows under any `bevy_ui` screen. `tests/pointer.rs` and
   `tests/combat.rs` get their party from a `party_by_command` helper; tests of the canvas creation screen itself go
   (the panel's tests cover the same rules, and the form's unit tests stay). **Reported**: shipped crate count and binary
   size before and after. Two debts with tests: the name limit counts characters in the form as the input does; the
   scale slider is capped at what the window holds (`layout_faults` names the fault today). ~−600/+250.
2. **The interface kit.** Out of `feathers_creation.rs` into `ui_kit.rs` (+ `ui_kit_scenes.rs` if size asks): the row,
   column, button, title and dropdown-from-a-menu scenes; one `PanelRoot`; `place`, `scale`, fonts on every panel; one
   app-wide id `UiId::{Creation(PanelId), Service(..), Camp(..)}` on `Control`/`Shown` so the five observers stay single
   and hand each report to its screen's `apply` (if `bsn!` refuses the nested enum, per-screen marker components with a
   shared observer helper instead; found out here, before anything is built on it). Test helpers become id-generic. No
   behaviour change: the panel's 17 tests pass unedited except for imports. ~400 moved.
3. **Data.** `omnis-data`: `ServiceDef { schema, id, name, kind: Inn|Temple|Trainer|Smith|Tavern|Bank|Guild, items,
   spells, rumors }`; `MapDef.sites: Vec<Site { x, y, service }>` (serde default); validations (unknown service, a site
   on an impassable tile, stock naming unknown items or spells) with bad-pack rows. Rules file `services.ron` (slots
   `shop.buy_price(cost_cp)`, `shop.sell_price(cost_cp)`, `inn.room_cost(members)`, `tavern.food_cost(count)`,
   `temple.heal_cost(missing)`, `temple.cure_cost`, `temple.raise_cost(level)`, `trainer.cost(level)`,
   `guild.spell_cost(spell_level)`; value `service_minutes`) and `rest.ron` (`rest.ambush_chance(map_chance)`; values
   `short_rest_minutes 60`, `long_rest_minutes 480`, `long_rest_every_minutes 1440`, `rest_food_per_member 1`,
   `rest_ambush_surprise 0`). Base pack: seven services. Test pack: `town.ron` (kind `Town`, one room behind a door per
   service, a terrain per service so the automap tells them apart), portals to and from the meadow, **the game starts
   in town**. Tuple updated (a ninth number: services), both replays rebaselined. ~450.
4. **Sim: the world's shape and the town.** Save schema 5 in one step: `Character.hit_dice_spent`, `Party.bank`,
   `Party.last_long_rest` (defaults, `v4_to_v5`, fixture `tests/saves/v4.ron`). `Mode::Town(ServiceState { service })`:
   stepping onto a site enters it (as a portal fires), `Interact` on a site enters again, `ServiceCommand::Leave` returns
   to the tile. `ServiceCommand::{Leave, Room, BuyFood, Rumor, Heal, Cure, Raise, Buy, Sell, Deposit, Withdraw}` (Train
   and Learn in M7b), each refused with a named `Rejection` at the wrong service, without the gold, with nothing to cure;
   every price through its slot; bought items land in the stores; each advances the clock; events `ServiceEntered`,
   `ServiceLeft`, `Bought`, `Sold`, `FoodBought`, `Rumor { key }`, `Treated`, `Raised`, `Banked`. The inn's room is a
   long rest with no food cost and no ambush; **`may_save` is true in an inn**, so `SaveRule::InnOnly` starts to mean
   something. Rebaseline. One test file per system (`tests/town.rs`, `tests/services.rs`). ~700.
5. **Sim: rest.** `Command::Rest(RestCommand::{Short { dice: Vec<u8> }, Long})`. Short: the minutes, each member's
   chosen hit dice rolled with the Constitution modifier (SRD). Long: refused without food or inside
   `long_rest_every_minutes`; the ambush roll first (maps with a random table, never a town); ambushed means part of the
   time passes, nothing is restored, no food is eaten and the encounter opens with surprise off; otherwise hit points and
   spell points full, half the hit dice back, effects pruned by the clock. `Event::Rested`, `RestInterrupted`. The debug
   menu's refill stays. **Measured before the owner plays** (`tests/measure.rs`, integers): ambush rate and wipe rate
   for a spent level-1 party of 2 and of 6 over 300 seeds; the default chance is set from that table. ~400.
6. **Ops, MCP, CLI.** `ops::service_view(world, data)` (the service, its rows with prices already evaluated, what each
   member may do and why not): the one model both the panel and agents read; MCP tool 19 `service_get`; `party.get`
   gains bank, hit dice and the last rest; `game.status` names the service; schema arms and proof instances for `Rest`
   and `Service`; script words (`rest`, `short-rest-…`, `leave`, `room`, `buy-…`, `sell-…`). With it the debt from the
   experiment: a `screen_text` op over the `bevy_ui` text tree (tool 20), so an agent reads a panel without a PNG. ~450.
7. **App: the service panel.** `PlayState::Service` following `Mode::Town`; `service_panel.rs` (Bevy-free: ids, `apply`
   from a report to a `ServiceCommand`) and `service_scenes.rs` on the kit: the service's name, gold, food and bank, one
   row per thing on offer with its price and a button, member rows at the temple, Deposit and Withdraw by number input,
   the last refusal from `CommandRefused` as a message, Leave (Escape too); log lines for every new event
   (`service_text.rs`, keys in the text pack). Tests as the creation panel's: census per kind, every control by event,
   a purchase and a refusal by pointer, layout at 1280×720 and 5120×1440, the text tree dumped. ~800.
8. **App: camp.** A `bevy_ui` camp panel: a hit-dice slider per member for the short rest, the long rest with its food
   cost and the reason when refused. **Its door is measured first** (the tool pad and the pause overlay are full): a
   third tool row, a 4×2 tool grid, or a first `bevy_ui` button beside the viewport, each captured at both sizes, the
   owner picks from the captures; the key is R. The sheet shows hit dice. ~350.
9. **M7a docs and review**: ARCH §4.5 "as built", knowledge files, measurements in the TODO review. **Owner
   acceptance a**: start in town, buy and sell, eat, rest at the inn and in the dungeon, raise a dead member, save under
   inn-only; and once, `--frame-stats` with a panel open against the canvas alone (the experiment's open number).

## M7b: progression
**Re-planned 2026-10-03** in `tasks/plans/m7b-progression.md`, which supersedes steps 10–13 below where they differ.
Owner decisions: spell picks are owed by a level (`Train { member }`) and spent one per press (`Choose { member, spell
}`, no checkboxes); the temple sells spells as the guild does (PRD §8.2); content is four 2nd-level and three 1st-level
spells. The outline below is kept as written.

10. **Rules and sim.** `omnis-rules/level.rs`: `ready(character, data)`, `level_up(character, data, picks) -> Gains`
    (average hit points through `hit_points.per_level`, proficiency by the table, the pool through
    `spell_points.pool`, the features the class lists for the level, still labels). `ClassDef.casting` gains
    `spells_per_level` (serde default), `casting.ron` a `max_spell_level` table. `ServiceCommand::Train { member,
    spells }` (refused: not enough XP, gold, wrong pick count, a spell off the list or above the level) and
    `Learn { member, spell }` at the guild; `Event::LevelUp`, `SpellLearned`. Rebaseline if the class files change. ~450.
11. **Content, measured first.** The dungeon gains goblin and skeleton placements and a deeper wing; four 2nd-level SRD
    spells whose effects fit the existing `SpellEffect` shapes (chosen in the step from `spell.rs`; one that needs a new
    shape is left out and named). **Table before the owner plays**: XP and gold per clear for parties of 2, 4 and 6,
    wipe rate, clears to level 2 and 3, the trainer's and the temple's prices against that income. Tuple, rebaseline. ~300.
12. **Ops, MCP, app.** Schema arms; `service_view` rows for the trainer (XP, next threshold, cost, picks owed) and the
    guild; the panel's trainer and guild bodies (spell picks as checkboxes, as the creation panel's skills); the sheet
    shows XP and the next level; log lines. Tests as step 7. ~450.
13. **Docs and review**; **owner acceptance b = the done-when**: create a party, clear the dungeon, return, level up.

## M7c (outline; planned in plan mode when reached)
The turn budget (slots `turn.actions`, `turn.bonus_actions`, `turn.reactions`; several commands per turn and
`CombatCommand::EndTurn`; `act_inner` no longer implies `step_current`; budget state in `CombatState`), the cost field
on spells, items and features, D24's three spell fields, declared reactions with the closed trigger list and the
row/stack mapping (opportunity attacks return), `Character.tactics` replacing `auto_cast` (save schema 6), and the first
features with effect: Second Wind, Action Surge, Cunning Action. The fight screen is the natural first canvas screen to
move to `bevy_ui`. Measured over seeds before content (R12).

## Horizons this plan adds (to `horizons.md` in step 0)
Swapping and hiring at the inn, donations, identify and repair, limited shop stock, bank interest (M8), rumors from the
story engine (M11), cantrips gained by level, ability score improvement, subclasses, Extra Attack, 3rd-level spells,
rolled hit points as an option, copper and silver as coins.

## Verification
Every commit: `scripts/verify.sh > log 2>&1 && git commit …` by path, status read, never piped; Sentrux `scan`
(`/Users/john/code/omnis/crates`, new files `git add`ed by name first) and `check_rules` (functions ≤ 100 lines with
tests and scenes, complexity ≤ 25, no cycles); sim lints (integers, `BTreeMap`/`Vec`); both replays rebaselined in the
commit of every `packs/` or `World` change and unmoved in every other; the schema proof and the tool count (18 → 20)
moved with their steps; never pushed, `.claude/` never staged. Behaviour: sim tests per system with real data; the
measurement tables in the TODO before each acceptance; headless panel tests by event and by pointer at both window
sizes; the text tree dumps; on the running game over the dev socket, `service_get`, `screen_text` and a `screenshot`
with `target: "window"` of a service panel; the owner's two acceptance plays on the ultrawide, saving by mouse first.

## Risks
- Removing the feature touches the release configuration, CI and eleven canvas-creation test call sites at once: step 1
  is its own commit and reports its numbers before step 2 starts.
- The kit's one-id-for-all-screens may not pass through `bsn!`; the fallback is named in step 2.
- Schema 5 then 6 inside one milestone: two migrations, each with a fixture; ARCH §4.7's "schema 5" is corrected in step 0.
- Rest ambush and the new dungeon can end a party before the owner's first level: both are measured, surprise stays off.
- Size: about 5,000 lines before M7c. Acceptance a after step 9 is the early exit; M7b does not start on a red CI.
