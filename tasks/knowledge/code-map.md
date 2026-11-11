# Code map

Where each system lives, by crate and file, as of M5.10 (2026-10-25). Behaviour is described in
ARCHITECTURE.md §4.5; this file is the index into the code. Verify a name before leaning on it.

## Crates
`omnis-core` (ids, `Fixed`, dice, `Pcg32` streams, geometry, time, `money`) → `omnis-expr` (Rhai, `no_float`,
`only_i64`, checked) → `omnis-data` (the pack loader; the only file I/O) → `omnis-rules`
(characters, attacks, conditions, spells, effects, equipment) → `omnis-sim` (the world) →
`omnis-vector` (the game client, Bevy, binary `omnis`; `omnis-app`, the previous client, was retired at P9), `omnis-cli` (headless), `omnis-mcp` (the bridge). The client reaches data through
`omnis_sim::omnis_data`; the sim reaches `Value` through `omnis_data::omnis_expr::Value`.
`omnis-bus` (M8 step 7b, A16) is a leaf beside them: the signal bus mechanism (`Bus<T, S>`, the
`Signal` and `Host` traits, `drain`, `MAX_DEPTH` 4, `MAX_SIGNALS` 64), generic and knowing no game type;
only `omnis-sim` imports it, and its vocabulary is `omnis-sim/src/bus.rs`.

## omnis-data
`spell.rs` (`SpellEffect`, `Reach`), `item.rs` (`ItemKind`, `Slot`, `UseEffect::{Heal, Sense}`,
`consumable`, `description`), `sense.rs` (`SenseSource { geometry: Geometry::Ray, fidelity, check,
persistence, minutes }`, `Fidelity::rank()`), `rules.rs` (slots and values), `content.rs`
(cross-file checks such as the component threshold), `limits.rs` (`MAX_FILE_BYTES` 4 MiB, `MAX_SAVE_BYTES` 8 MiB for a character save, M8.5 7f), `flag.rs` (`FlagDef`, `FlagScope`, `data/flags`, M8.5 7c), `seal.rs` (`status`, `seal`: `pack.ron`'s `hash`, checked in `loader::load_one`; `loader::content_hash`, M8.5 7h1), `migration.rs` (7h6: `Migration`, `Step`, `Kind`, `PackMigration`, `Target`; `version`, `compare`, `chain` (the files after a save's version), `names` (a chain's name steps composed); `check_pack` in `load_one`, `check_loaded` in `resolve`; `Data.migrations`), `ron_io.rs` (`read_under`, `read_save_under` (the save's limit), `write_atomic`, `write_files`, `list_under`, `remove_under`), `loader.rs`, `map_data.rs` (`MapData` and its lookups by tile, M8.5 step 2), `object.rs` (`ObjectKind`, `Look`, `Verb`, `Requirement`, `Effect`, `Interaction`; cross-file checks and flag interning in `object/check.rs`; M8.5 step 2), `footprint.rs` (`Footprint::{of, path, overlaps, cells, ray_span}` in sixteenths of a cell, shared by the engine and the client's mirror), `lore.rs` (`Lore` on kinds and placements, `CellLore` in `MapDef.lore`, `MapData.lore` by cell; owner 2026-10-16), `placement.rs` (`Placement`, `GroundDef`, `Pile`, `ResolvedPlacement`; shapes checked with the map, kinds, ground, spills and arrival centres at resolution; `in_state`, `looks`, `covers_centre`; `MapData::{objects, object_cells, ground, objects_at}`), `loot.rs` (`LootTable { picks, entries }`, `LootEntry { item, count, weight, chance_percent }`; `Monster.loot` names one), `registry.rs`,
`service.rs` (`ServiceDef { kind: ServiceKind, items, spells, rumors }`, a list only for its kinds,
spells on a guild or a temple;
`MapDef.sites: Vec<Site>`, shapes in `validate_sites`, the service and the tile in
`resolve_sites`; `MapData::site_at`), `map.rs` (`Portal { id, link, facing, marker }`, W1 step 2: linked by id, `Portal::{ID_MAX, id_fault, landing}`; `marker` required, an `Object` surface
checked in `loader::check_surfaces`; `MapData::marker_at`). M7c: `action.rs` (`Cost { Action,
BonusAction, Reaction, Free }`, `Uses`, `Recharge`, `FeatureEffect { Heal, ExtraAction, Cunning }`);
`spell.rs` carries `cost` and D24's three fields (required in every file); `character.rs`'s class
features carry `effect`, `cost`, `uses`; `monster.rs` `Monster.casting` (`MonsterCasting`: attack,
DC, caster level, points, spells). The `turn.*` slots are in `rules/combat.ron` (inputs `level`,
`is_member`).
The editor's side (M5, 2026-10-21): `notes.rs` (`NotesFile`, `is_note_key`; `notes/*.ron`, read and checked
by the loader, never hashed, never kept), `map_data.rs` (`MapData::resolve`, `resolve_one`, `MapTargets`,
`check_inbound`/`check_outbound`: the loader's per-map pass shared with the editor; the entrances and exits enhancement, W1 steps 1 and 2, 2026-11-01: `MapTargets::maps()` answers every map as `Target { id, def, levels }` (the loader's `RawTargets`, the editor's `DataTargets` with the edited def standing in); `enterable`, `standing_fault`; links: `pairing(portal, own, maps)` (the partner by the link either side states; the errors: to nothing loaded, to itself, to a portal linking a third, two claimants), `landing(portal, far, q)` (the cell ahead of the partner: outside, no ground, cannot enter, a portal stands), `resolve_portal` (a `ResolvedPortal` for a linked portal alone), `unreached(maps, entry, pack)` over linked pairs (maps with none exempt; asked only of a sound world), `check_reached`; `pub partner_of`, `unlinked_portals`, `unlinked_portals_in` for the editor; `omnis-core` `Facing::{ahead_of, behind}`; the engine's `apply::landing` and `save/places.rs::standable` read `MapData::enterable`; the corpora `tests/packs-bad/bad-ways` (links and landings) and `bad-reach` (reachability alone)), `loader.rs`
(`pub(crate) parse_checked`, `check_manifest`, `check_text_file`, `check_id`, `HasSchema`), `edit/`
(`grid.rs` `Grid`/`EdgeKind`; `kinds.rs` `Brush`, `Place`, `Target`, `Edit`, `NewMap`, `EditorStatus`,
`MapEntry`, `EditError`; `document.rs` `Document` with `UNDO_CAP` 100 and `Arc<Grid>` per level; `session.rs`
`Session` (open, new_map, status, save + `seal::seal(root, None)`, files, read, check, write); `files.rs`
`check_text` by directory; `notes_store.rs` `Notes`; `answer.rs` `Request`/`Answer`/`answer`). `omnis-sim/src/
editor_ops.rs` is the wire glue (`Op::editor()`, `From<Answer> for Reply`, `From<EditError> for OpError`).
Base pack rules: `packs/base/data/rules/{casting,combat,creation,items,leveling,rest,sensing,services}.ron`;
services under `packs/base/data/services/` (seven, one per kind). The test pack depends on the base
pack and starts in `test:map:town` (the gate at (11, 2) leads to the meadow's start; the meadow's
road south at (16, 31) leads back); the dungeon's stairs at (23, 23) lead down to `test:map:depths`
and its stairs at (0, 0) back up). Content ids are interned in file order: a new file renumbers
the ones after it, and saves are refused on the pack fingerprint.

## omnis-rules
`character.rs` (`create`, `starting_kit`), `stats.rs` (checks, saves, pools), `attack.rs`
(`attack_roll`, `rejudge`), `spell.rs` (casting ability, save DC, monster saves, heal rolls,
cantrip dice), `effect.rs` (`ActiveEffect`, `Expiry`, `BuffOn`, `Roll.bonus`), `equip.rs`
(`can_equip`, `equip`, `unequip`, `auto_equip`, `armor_class`, `weapons`), `condition.rs`,
`level.rs` (M7b: `ready`, `next_threshold`, `level_up` → `Gains { hp, spell_points, picks,
proficiency, features }`, `max_spell_level`, `may_learn` → `SpellRefusal`, `eligible`).
M7c: `feature.rs` (`combat_features`, `uses_left`, `spend_use`, `recover_uses`;
`Character.feature_spent`), `tactics.rs` (`Tactics`, `Runbook`, `CriteriaSet`, `Criteria`,
`Predicate`, `Cmp`, `Who`, `Row`, `Trigger` (the closed eight, `ALL`), `ActionRef`, the caps
`TACTICS_NAME_BYTES`, `CRITERIA_DEPTH`, `CRITERIA_NODES`, `LIBRARY_SETS`, `RUNBOOKS`,
`RUNBOOK_ENTRIES`; `check`, `holds` over `Facts`); `Character.tactics` (`legacy_auto_cast` read from
old saves only).

## omnis-sim
- Entry: `command.rs` (16 variants, `Command::{Step, Turn, Interact, Door, Search, Climb, Party, Encounter, Combat, Cast, Item, Service, Rest, Dev, Act, Approach}`,
  `Rejection` with `fmt_play`/`fmt_items`/`fmt_magic`), `word.rs` (script words: `Word`, `parse_script`,
  `Word::command(world, data)` turns a slot or a list row into an identity when applied), `apply.rs`
  (`match (mode, command)`; `advance()` is the only clock writer; effects pruned there),
  `event.rs` (ids and roll traces only; `ItemPlace`, `LayerCheck`, `SensedTile`).
- Objects (M8.5 step 2): `objects/mod.rs` (`ObjectState` in `MapState.objects` by placement id,
  written only once changed; `state_of`, `removed`, `blocks_step`, checked by `apply::landing` after
  `Cliff` as `BlockReason::Object`; re-exported as `api::object_blocks` for the client's mirror;
  `state_in`/`removed_in` from a `MapState` for `visibility::Sheet::object_hides`, which tests each
  walked cell's sight-blocking footprints with `Footprint::ray_span`; `line_of_sight` and `ray` take
  the kinds, `visibility::Kinds`). `objects/act.rs` (`act`, `admit`: the refusals before any die, shared
  with the view; `offered`, `choices` for Use's `ChooseVerb`, `reachable`, `reach`), `objects/outcome.rs`
  (`resolve`: checks on the `object` stream, items used up, effects; `gate`, `check_of`),
  `objects/container.rs` (`contents`, `TakeOut`, `PutIn`), `objects/notice.rs` (`look` after each look,
  `search`, `tell` for lore); `ground.rs` (`pile_at`, `Drop`, `PickUp`, `lay` for loot); `combat/loot.rs` (`of`, `draw`);
  `items.rs` (`from_kit`, `from_list`, `out_of_kit`, `actor`); `objects_view.rs` (`objects_view` for
  `objects.get`, `in_sight` for `ViewportModel.objects`/`.piles`); `beings.rs` (M8.5 2c: `BeingView`,
  `in_sight` for `ViewportModel.beings`, `on_level`, `reached` for `ObjectsView.beings`, `approach` for
  `Command::Approach`; `encounter::meet`). Fixture pack
  `crates/omnis-sim/tests/packs/objects` (the yard); tests `objects.rs`, `object_act.rs`, `object_use.rs`,
  `ground.rs`, `loot.rs`, `notice.rs`, `objects_view.rs`, `beings.rs` (the yard's two pack rats at (5, 1)).
- Time (M8): `time.rs` (`PartyTime { shared_milli, date, era }`, `advance` at the region's company,
  `moved` raises `Signal::Entered`, the `Sim` host and `Reconcile`, `subscriptions`, `region_clock`,
  couplings at depth one on `time:<a>:<b>`), `time_view.rs` (`TimeView`, `DateView`, `ContactView`,
  `time_view`, `party_date`; `time.clocks`); `World.{contacts, party_time, bus}`; `migrate::v6_to_v7` (the last step: older saves stop at 7 and are converted to a character save on load, `save/disk.rs`).
  Data: `omnis-data/src/region.rs` (`RegionDef`, `Region`, `RegionKind`, the load checks),
  `MapData.region`, `Data::calendar()` → `omnis_core::{Calendar, Date}`, `text::fill` (`{ago}`);
  `rules/time.ron` (`time.settled`, `time.wild`, the calendar values). App: `panels::clock_text`
  (the HUD clock line), `text::ago_text` (a rumor's age).
- The engine's API (ARCHITECTURE §4.9, M8 step 8): `api.rs` re-exports exactly the contract; `ops.rs` (`PROTOCOL`,
  `Op`, `Reply` tagged `reply`, `OpError`, `Status`, `dispatch`, `Op::is_host`, `MAX_SCRIPT`, the host ops' rules
  `reload` (by name through `save::examine_running` and `restore`, 7h5; `ReloadRefused`), `rules_set`; the save's are in `save/`); `names.rs` (protocol 2: `Place`, `id_of` (`#n` for a
  number no pack names), `action_named`: events and views name definitions by string id here); the views `query::here`, `party_view.rs`, `view.rs`,
  `service_view.rs`, `rest_view.rs`, `time_view.rs`, `cast_view.rs`, `objects_view.rs`; `omnis-cli/src/headless.rs` (`Headless`, the in-process host);
  `omnis-vector/src/shell/{socket,host}.rs` (the dev socket host, `MAX_LINE`); `omnis-mcp/src/tools.rs` (tool → op).
- The `data.*` ops (W1, 2026-11-11): `omnis-data/src/catalog.rs` (`KINDS`, the one table of kinds, 15 with `rules`: a new kind is a `Definition` variant and a row; `Definition` tagged `kind` beside `def`; `links` and `referrers` from one walk over a definition's strings; `supplier`, `replaced` over `Data.sources`), `omnis-data/src/strings.rs` (the strings of any value, by a serializer that keeps only them), `omnis-sim/src/data_ops.rs` (`Op::data`, `kinds`, `list`, `get`, `text`; `DefinitionRow`, `DefinitionView`, `KindCount`, `Words`; `LIST_MAX`, `MAX_KEYS`). They need no world.
- Rule priority (2026-11-11): `omnis-data/src/rules.rs` (`SlotDef`, `ValueDef`, `TableDef`, each with `priority`; `merge`, the one place files become a rule set: highest priority stands, a tie, an input mismatch and a broken loser are errors; `Standings` by `(RuleKind, name)` with `Beaten`), `Data.standings`, `Data.rule_files`; `rules` the 15th row of `catalog::KINDS`; `omnis-sim/src/rules_ops.rs` (`RulesView`, `SlotView`, `RuleOrigin`, `origin`, `rules_view`, `slot_view`, `rule_reply`, `rules_set`, re-exported from `ops`); `omnis-cli` `validate` prints `override:` lines (`overrides` in `main.rs`). Tests `omnis-data/tests/rule_priority.rs`, `omnis-sim/tests/rules_ops.rs`, `omnis-cli/tests/cli.rs`.
- Bus vocabulary: `bus.rs` (`Topic { Region, Battle }`, `Subscriber { Reconcile, Reactions }`,
  `Signal { Entered, Battle(Cue) }`, `type Bus`); the hosts are `time.rs::Sim` and
  `combat/reaction.rs::Fight`.
- World: `world.rs` (`World`, `Settings { save_rule, permadeath, devtools }`, `Automap`,
  `Known`, `layer::{TERRAIN 1, STRUCTURE 2, VISITED 4, REMOTE 8}`, layer-aware `record`),
  `party.rs` (`heal`, `bury`), `visibility.rs` (`depth`, `cone`, `ray`), `query.rs`, `view.rs`,
  `migrate.rs` (one function per schema step up to 7, `v4_to_v5` the copper purse), `replay.rs`,
  `party_view.rs` (`PartyView`, `MemberView`, `ItemView`, `KnownSpellView` (P6: each known spell's cost and `pay`), `party_view`); `cast_view::cost` is the one place a spell's shown cost is evaluated (road rows, known spells, the fight's `SpellView`).
- Money (schema 5): `Party.gold` and `Party.bank` count copper; `omnis_core::money::{from_gp,
  gp_floor, Coins}`; pack data stays in gold (backgrounds, monster drops) and converts where it is
  read; every rule price is copper (`bribe.cost`, `services.ron`). The app shows `gp_floor` in
  fights and `text::coins` ("15 gp 3 sp 7 cp", `Coins`' `Display`) in the inventory's stores, the
  debug menu, every town line and every refusal that names a price.
- Town (M7 step 4b): `service.rs` (`ServiceState { service, kind }` in `Mode::Town`,
  `ServiceCommand` with `offered_in(kind)`, `enter_here`, `apply`: `quote` builds a `Deal` (every check
  but money), `afford` is the last check, `settle` carries it out; prices by `price(slot)`,
  rumors on the `town` stream). `service_level.rs` (M7b step 10): `Train`, `Choose` and `Learn`
  as deals; a level is worked out on a copy of the member (`Deal::Train { after }`), a pick is
  free and takes no time (`Deal::Spell { pick }`). `service_view.rs` (M7 step 6, `service.get`): `ServiceView` and
  `OfferView`, each offer quoted on its own unstored copy of the stream; the one model the
  service panel and agents read. M7b: `Train` per member, `Choose` per class-list row for a
  member owed picks, `Learn` per stocked spell and member (`worth_offering`: learnable now, or
  too high and shown dim; known, cantrips and off-list spells left out). `ops.rs`: `PartyView`
  has `bank`, `last_long_rest`, `long_rest_wait`; `MemberView` `hit_dice`, `hit_dice_left`,
  `spell_picks`, `ready`; `Status.service`, `Status.groups_cleared` (`encounter::groups_cleared`:
  the map's `once` groups cleared, of how many). Script words
  with numbers: `word.rs::parse_town` (`buy-R-N`, `heal-M`, `short-rest-A-B`, …).
- Rest (M7 step 5): `rest.rs` (`RestCommand`, `apply`: `check_dice` or `too_soon` and
  `food_needed`, then `ambush_after`, `rest_events` and `roll_hit_dice` on copies of the
  `encounter` and `rest` streams, then the changes; `long_rest_restore` and `too_soon` are the
  inn room's too). `encounter::ambush` opens an `EncounterSource::Ambush` from the map's table.
  `rest_view.rs` (M7 step 8b): `rest_view` → `RestView { refusal, members: Vec<CampMember>,
  long_food, food, long }`, read-only, from the checks `rest::apply` makes (`too_soon`,
  `food_needed`, `food_need`, `hit_die`); `CampMember.spendable` is 0 when dead or at full hit
  points. What the camp panel reads; `tests/rest_view.rs` holds it to the command's answers.
  A map's rest events: `omnis-data/src/rest_event.rs` (`RestEventDef`, `RestKind`, resolved to
  `MapData.rest_events`); log lines for rests in `omnis-vector/src/shell/text.rs`. `apply.rs`: `landing()` is the
  pure half of a step, shared by `r#move` and `query::{step_lands, site_ahead}`; `arrive` enters
  a site (no encounter roll) or triggers the tile's encounter; `step_out` leaves only when the
  step goes somewhere. `World::may_save` counts an inn; a saved `Town` must match the tile
  (`LoadError::BadTown`).
- Fights: `encounter.rs`, `combat/{mod,state,turn,resolve,cast,reaction}.rs` (`CombatCommand::
  {Attack, Cast { pay }, Use, Dodge, Exchange, Run, EndTurn, Feature}`, `Plan`, `Roller::take`/
  `take_stream`, `run_until_member`, `end_of_round`, `settle`; `payable`, the one answer to "can
  this spell be paid this way now" for the command and the view). M7c: `combat/budget.rs` (the
  `turn.*` slots at each turn's start, `refresh_reactions`, `affordable`, `spend`, `goes_on`: the
  turn-ending rule), `combat/feature.rs` (Second Wind, Action Surge, Cunning Action's exchange and
  Hide), `combat/opportunity.rs` (monsters' built-in opportunity attacks), `combat/reaction.rs`
  (`on_attack`, `on_wound`, `on_cast`, `on_missile`: declared reactions walked in marching order),
  `combat/monster_cast.rs` (a monster's dice roll among its weapon and affordable spells; attack,
  missile and save spells; a caster's own Shield). `CombatState` gains `budget`, `reactions`,
  `hidden`, `monster_shields`; `Stack.spent` (points per individual). `tactics.rs`
  (`TacticsCommand { SetReactions, PutReaction { at }, RemoveReaction }`, `answers`: what an action
  may answer). `view.rs` (`combat_view`: budget, reactions, hidden, members' switch and features,
  points, shields, spell rows' `blocked` and `bonus`); `party_view.rs` (moved out of `ops.rs` in
  M7c step 6a; `MemberView.tactics`: the switch, the declared rows, `answers`).
- Magic and effects: `casting.rs` (explore casting), `effects.rs`, `checks.rs::roll` (the one
  check wrapper; guidance is spent here), `utility.rs` (`open_door_ahead`).
- Items: `items.rs` (`ItemCommand`, `UsePlan`, `UseKind`, `validate_use`, `use_item`, `transfer`,
  `drop_worn`, stock helpers `add_to`/`take_from`/`count_of`/`consume`/`item_id`).
- Sensing: `sense.rs` (`LAYERS`, `best_eyes`, `dc`, `reach_for_layer`, `resolve`).
- Dev: `dev.rs` (`DevCommand`, 14 variants, gated by `Settings.devtools`).
- Saves (PRD D28, M8.5 7a–7f; ARCHITECTURE §4.1.1): `journal.rs` (`Journal`, `Entry { minute, at, command, changes }`,
  `Change` values after, `Held` (the before-and-after compare in `apply`), `apply_changes`, `journal_len`, `archive_len`);
  `log.rs` (`Logged { minute, map, event }`, `told`, `history`, `bound` to `LOG_CAPACITY`; stamped in `World::record`);
  `save/mod.rs` (`Split`, `Checkpoint`, `Holdings`, `Bases`, `WorldBase`, `RegionBase`, `Manifest`), `save/split.rs`
  (`split`), `save/join.rs` (`join`, `JoinError`; `assemble`, the parts unchecked, 7h3), `save/report.rs` (`split_report`), `save/character.rs`
  (`CharacterSave`, `BaseNames`, `world_id`, `is_world_id`), `save/disk.rs` (`save_files` with `fits`, each file
  against its reader's limit; `load_save`, `Restored`, `PackFit`, `fit` (7h3: same packs, same names, renumbered and folded, or forced; 7h6: migrated), `region_number`, the v1–7 conversion, `CHARACTER_SCHEMA`, `region_path`, `archive_path`),
  `save/fold.rs` (`fold`, `fold_from` (7h3), `Archived::{Held { journal, from }, Named(Segment)}`, `Segment`, `fold_at`, `due`, `FOLD_INTERACTIONS`; safe points call
  `due` in `apply`), `save/tidy.rs` (`tidy_list`, `unnamed`; schema 8 and 9 saves both count), `save/names.rs` (7h2: `NameTable::of`,
  `Signature`, in `Manifest.names`), `save/rows.rs` (7h2: a schema 8 save's encounter rows read as `#n` markers and turned into ids), `save/remap.rs` (7h3: `world` renumbers the live world by name through `Remapper`, `Registered` per id kind, terrain rows by key, `used` rows by `Signature`, `time:` streams, the bus's region topics; `log`; `Missing`, `NameKind`), `save/check.rs` (7h4: `examine`, `restore`, `SaveCheck`, `Verdict`, `PackChange`; 7h5: `examine_running`, the shared `refit`; 7h6: `NeedsMigration`, `migrations`, `restore`'s `migrate`), `save/chain.rs` (7h6: `Chain`, a save's pack migrations: the name steps the remap reads (`Remapper::found`, `keep`), the place steps on the remapped world, whether a step found anything; `Migrated`, `Offered`), `save/places.rs` (7h4: the place checks and fixes, `PlaceWarning`, `PlaceFix`). `World.{known, journal, origin, archive, log}`.
  Tests: `journal.rs`, `split.rs`, `disk.rs` (both goldens saved and loaded after every command; fixtures
  `tests/saves/v1..v7.ron`, the schema 8 and 9 ones), `by_id.rs` (7h2: the v8-cleared fixture, the name table), `by_name.rs` (7h3: the `ahead` fixture pack moves every number; every view equal after a load by name, exploring, mid-fight, in town and in the yard), `fold.rs`, `history.rs`.
- Tests: one file per system under `tests/`, `common/mod.rs` builders (`data`, `world`,
  `new_world`: a new game placed on the meadow's start, where tests from before the town begin;
  `party_of`, `act`: one command played as a whole turn). M7c: `turn_budget.rs`, `reactions.rs`,
  `monster_cast.rs`, `views.rs`. Measurement (ignored) is `tests/measure/` (`main.rs` single fights
  and ambushes, `clear.rs` the dungeon clear over a `Play`, `budget.rs` `budget_over_seeds`,
  `boss.rs` `boss_over_seeds`). The replays leave town by a real `Step(Back)`.

## omnis-vector
The game client since 2026-10-10 (ARCH A18; specified in `presentation-ARCHITECTURE.md`). A Bevy-free core in `src/`
and a thin shell in `src/shell/`; it depends only on `omnis-sim` and Bevy (`2d`, `png`, `bevy_pbr`, `ui`).
- Core: `grid.rs`/`geometry.rs`/`geom.rs` (the map as line segments, cell and yaw maths), `pose.rs` (the
  free pose), `bind.rs` (the binder: cell crossings become `Step`, cardinal yaws `Turn`, refusals and
  disagreements counted; it keeps the command log that replays), `collide.rs` (the collision mirror, held
  to `apply(Step)` by `tests/agreement.rs` on every map), `minimap.rs`, `party.rs` (the fixed party of four),
  `trial.rs` (`why`/`accepted` on a world clone; `why` words the refusal by `labels::reason`), `combat_menu.rs`
  (the fight's choices; members by `CharacterId`, spells, features and items as rows of the views turned into
  string ids by `resolve`; P7: `Step::Features`, `Action::Feature` (Cunning Action's swap), `Entry.note`, item
  rows from `CombatView.items`, the spell notes, `budget_line`, `points`, `picking`), `arena.rs` (the fight
  screen's layout and picks: the left column of picture, actions, log and status; the `party` rectangle at the
  right; P7: `picker`, the pickers' panel; P8: `fit`, `zoom`, the fight's units at the interface scale), `settings.rs` (P8: `Prefs`,
  `Quality`, `Face`, the lenient settings file `read`/`write`, `clamp_scale`, `fits`), `history.rs` (the full log's lines, stamped, `matching`; 2026-10-17), `screen/log.rs` (the full log's screen), `screen/load_report.rs` (7h4: the save that does not load, its packs and missing names, Close; 7h5: the packs that do not reload, `Refusal`; 7h6: "These packs moved on", the migrations' steps, Apply and load or reload), `rolllog.rs` (the roll log; `Names`, which since M8.5 2b also names objects on the party's map, `locate`, `object`,
  definitions by string id), `cinema.rs` and `raster.rs` (the picture window).
- Objects (M8.5 step 2b): `shapes.rs` (`Body`, `shapes`: a look as lines and fills on its footprint, turned by facing,
  on the home floor; a shut arch crossed; `PILE_LOOK`); `scene.rs` `Scene::build(data, ..)` adds each level's
  objects and piles from `SimWorld::objects_on` (`api::objects_on_level`); `SegKind::{Object, Pile}`; the
  shell redraws on `session::reshapes` (doors, object state, removal, notice, ground moves, `CombatEnded`).
  Tests `tests/shapes.rs`. Beings (M8.5 2c): `shapes::figures` on `SegKind::Being`, `SimWorld::beings_on`,
  Approach in `screen/verbs.rs` and `screen/actions.rs` (`approached`); tests `tests/beings.rs`.
  `collide::blocks` refuses a step through a footprint via `SimWorld::object_blocks`
  (the engine's `api::object_blocks`); `tests/agreement.rs` covers the yard, gate shut and open.
- Shell: `session.rs` (`Config` flags incl. `--save`, `--settings`, `--screen`; `Packs`, always a resource; `Session`, only
  while a game runs: `start`, `new_game`, `loaded`, `restart`, `order`, `ordered` (the newest line or the refusal), `notice` (the outcome notice: the lines an `ordered` command said but a move's, or its refusal in alert, with where it was done; `take_in` under `note`), `save_log`), `movement.rs`,
  `controls.rs` (the pad; Camp, Actions and Menu bottom right; P6: `light` lights Camp by `Nav::allowed`; R toggles it in `screen.rs::action_keys`), `screen.rs` (the screen renderer and router:
  `Screens { nav, forms }`, `Pressable`, `Arrow`, `ctx`, `playing`, the chrome hidden with no game; the root's
  height capped, `PLAYED_ROOM` 64% while played),
  `combat.rs` (the fight screen; P7: draws `ScreenId::Fight` with `screen_draw::build(.., Mark::Fight)` in the
  action column or the picker panel and routes `FightPress`, its arrows and number keys through `screen::apply`;
  `FightScreen.controls`; P8: `FightScreen.zoom`), `settings.rs` (P8: `Current`, `Fitted`, `SettingsPlugin`:
  `apply_quality` on the HDR cameras and gizmo groups, `apply_scale` to `UiScale`), `fonts.rs` (P8: `INTER`,
  `ALEGREYA` at fixed handles, `FontsPlugin`, `wear`), `hud.rs`, `notice.rs` (2026-10-16: the outcome notice centred where the screens stand, faded by `crate::notice::alpha` and dropped out of range; it replaced P5's bottom message line; 2026-10-17: gone by the cell's edge, an "x" closes it), `minimap.rs`, `render.rs`, `capture.rs` (`--screenshot`,
  offscreen; the socket's live capture).
- The editor (M5.4, `tasks/plans/m5-editor-v1.md`): `src/editor/` is Bevy-free: `mod.rs` (`Editor` over an `omnis_data::edit::Session`: `maps`, `chosen`, `level`, `cursor`, `facing`, `view`, `pane`, `resolved` (the last valid `MapData::resolve`), `revision`; `open`, `changed`, `point`, `step`, `show_level`, `laid_out`, `apply(EditAct, step) -> Asked`; M5.10: `EditAct` and `apply` live in `acts.rs`, re-exported), `plan.rs` (`View { origin, zoom }` over `ZOOMS`, `fitted`, `centred_on`, `zoomed`/`zoomed_at` (M5.9d: one step about a pane pixel, the cell under it kept), `hit`, `cell_rect`; `paint(Source, view, pane) -> Raster` of the cells in the pane only; `fill`), `view.rs` (`tools` and `inspector` as kit `Screen`s, ids `EditorTools`/`EditorInspector`; the tools carry the brush rows, Undo/Redo, Save/Revert). M5.5: `brush.rs` (`Tool { Terrain(u8), Edge(EdgeKind), Height, Clear }`, `Target { cell, edge }`, `palette(def)`, `Tool::first/label/brush`, `EIGHTHS` −64..=64, `held`); `Editor` adds `tool`, `eighths`, `stroke` (the last target of the held button's stroke), `status` (cached `EditorStatus`, refreshed in `changed`), `paint(data, target, stroke) -> bool`, `pick`, `end_stroke`, and `apply` the acts `Brush(u8)`, `Eighths`, `Undo`, `Redo`, `Save` (→ `Asked::Saved`), `Revert`; `plan::View::target` (the edge within a quarter of the side) and `View::edge_target` (the nearest side, wherever in the cell); `Editor::aim(px, py)` picks between them by the tool and `Editor.hover` keeps the aim under the pointer. `shell/editor.rs` (`Editing(Editor)` resource, `EditorPlugin`: `enter`/`leave` on the resource, `route` (`EditorPress` and the arrows of `ControlId::Edit`, dim ones refused by `live`; on the plan pane through `RelativeCursorPosition` → `View::target`: a left click paints, a held button paints each new target as one stroke, a right click picks (`normalized` is Bevy's −0.5..0.5 from the pane's centre); the arrow keys and WASD, Enter, the digits, `-`/`=`, `[`/`]`, C, Q/E, and with Cmd/Ctrl Z, Shift+Z, Y, S, while no screen covers it (since 2026-11-07 `shell/editor_keys.rs::keys`, `route` being at Bevy's sixteen parameters, its keys, mouse and wheel one `Inputs` SystemParam; a press on the frame a screen over the editor closes is that screen's: `route`'s `Local<bool>` `stood`); M5.9d: the wheel over the plan is a zoom step a notch about the pointer through `Editor::zoom_at`, the `Zooming` resource adding a trackpad's pixels up to the kit's `WHEEL_LINE` and dropped off the plan), `act(editing, packs, act, step)` (a Save reloads `Packs` from disk), `refresh` (columns rebuilt when their `Screen` changed, the plan repainted on `(revision, pane)` into one `ImageNode`, the cursor a bordered node, the hover a `PlanHover` node placed by `place_hover`: a cell's outline or a bar along the aimed side, in the caution colour; `run.rs` (2026-10-25, the boundary tools: `Corner`, `Side`, `Run`, `Preview`, `Pointer`/`Press`, `sides(a, b)`, `paints(size, sides)`, `Editor::pointer`, `take_run`, `drop_corner`, `drop_run`, `preview`; `Tool::Corners` and `Tool::Freehand` in the palette, `View::corner`, `Source.preview` painted last by `plan::paint`); the column as a drawing GUI, step 1 (2026-10-25): `brush.rs` holds `Tool { Select, Brush, Corners, Freehand, Eraser }` (`ALL`, `label`, `icon`, `key`, `is_run`) and `Entry { Terrain(u8), Edge(EdgeKind), Height, Stamp(Placing) }` (`palette(def)`: terrains, Wall, Door, Ramp, Height, the six stamps; `first`, `label`, `describes`, `icon`, `is_edge`, `brush`); `Editor { tool, entry }`, `aim` by tool and entry (select the cell, an edge entry the nearest side, else the band), `paint` dispatches (select → `point`; brush → the entry's brush; eraser → an open edge, `Place::Remove` of the first of `here_at(cell)`, or `Brush::Clear`), `pick` → brush + entry or the eraser; `EditAct::Tool(Tool)`, `Brush(u8)` the entry (choosing an entry chooses the brush unless a run tool can take an edge; a run tool with a terrain chooses the wall); `run::paints(size, sides, kind)` and `take_run` with the entry's kind; `view::tool_rows`, `palette_rows` (swatches and icons, the details, `run_rows`), `tools` (grids, files, buttons), `bar` (view icons + the level choice), `inspector` (Add level, Level... beside the level line); keys V B P N X in `route`; tests `tests/editor_column.rs`, `tests/kit_icons.rs`; step 2, Fill and Move: `fill.rs`, `grab.rs`, the section below); a rebuilt column's new panel takes the scroll of the one it replaced through `Drawn.scrolls`)). M5.6a: `view::bar` (`ScreenId::EditorBar`, over the plan in `Column::Bar`: since 2026-10-25 a first row of view icons `ZoomOut`/`ZoomIn`/`Centre`/`FaceLeft`/`FaceRight` (`zoom-out`, `zoom-in`, `focus`, `rotate-ccw`, `rotate-cw`) with "N px a cell" and the keys after them, moved from the tools column; then the cursor's cell's `Terrain`, `Height` and, on a stair, `Rise` as choices; `Editor::{cell_terrain, cell_height, cell_rise}`, acts `CellTerrain`/`CellHeight`/`CellRise` painting through `Editor::brushed` or `Editor::edited`, one undo step each); `refresh` decides per column with `need` → `Need::{Nothing, Words, Nodes}`: the same shape has its words rewritten by `screen_draw::sync` (nodes kept), a new shape is rebuilt with `keep` = the focused editor field, else the column's first field (`first_field`); `route` clears `InputFocus` on a plan click while a field has it. M5.6b: `editor/form.rs` (the generic `Form`: `Field { label, value: Value, slot: Slot }`, `List { label, template, max }`, `rows() -> Vec<Row>`, `typed`/`step`/`set`/`add_row`/`remove_row`/`fill`, lookups by label `text_of`/`number_of`/`chosen_of`/`choice_of`/`flag_of`, parsers `id` and `dice`) and `editor/forms.rs` (`Placing::ALL`, `Form::placing/existing/random/rest_event`, `Form::request -> Place`, `find(def, &Target) -> Found`; M5.10: its limits, shared choice fields and readers in `forms/fields.rs`); `Editor { form, stamps }`, `Here { label, key }` from `here()`, `placed`/`took`, `typed`, `set_field`, `open_form`, `apply_form`; `view::inspector` draws the form's rows first, then the start row (`SetStart`), the `here()` rows (`EditAt`/`RemoveAt`), `Add` (`PlaceKind` + `Add`), the random table (`Random`) and the rest events (`RestAdd`/`RestEdit`/`RestRemove`); `shell/editor.rs::route` reads `(Ref<EditableText>, &EditorPress)` into `typed` and takes `screen::Slid` (now `pub(super)`, left by the screen router when no screen is up) into `set_field`; a rebuilt editor column gives the keyboard only to a named field (`keep`), the first field of a `fresh` form. Test helpers `type_into`, `field_entity`, `field_of`, `keyboard_on`/`keyboard` (moved to `common/app.rs`). M5.6c: `form::Change::{Place, Edit, Note { key, text }}` is what `Form::request` returns and `Editor::change` makes (`placed`, `edited`, or `Session::set_note` at once, outside the history); `FormKind::{Terrain(Option<char>), LevelAdd, Level(i8), Note(String)}`, `Form::terrain/level_add/level/note`, `Form.hint`, `remove_label`; `Editor::note_key(NoteOf)`/`note`; acts `TerrainAdd`, `TerrainEdit`, `LevelAdd`, `LevelEdit`, `Note(NoteOf)`; `view::tools` adds the terrain and level buttons, `view::inspector` the `note_row`s. Test helpers `scratch_pack` and `in_editor_scratch` in `common/editor.rs`. **Marks (owner, 2026-10-24):** `minimap::paint` draws a portal on any known tile (derived from `map.portal_at`, nothing stored: a stored automap bit would fold every older save on load) as `raster::portal_pixels` (a rectangle inset a quarter with a circle, `rect_pixels` + `circle_pixels`) in `DOOR`; `geometry::extract` adds `SegKind::Portal` (a rectangle and a 16-sided circle on the floor at the patch's heights, `ring`/`floor_at`/`circle`) and `SegKind::Activity` (a smiley on a site's cell, the activities' placeholder), coloured in `shell/render.rs::colour`; `raster.rs` also holds `triangle_pixels` for the plan and `Raster::icon(&Icon, (left, top), size, colour)` (the round brush, radius the icon's stroke width); `icons.rs` names the icons the client draws (`ACTIVITY` = `face-grinning`, `activity()` read once in a `OnceLock`); `geometry::icon_on_floor` lays an icon's lines on a cell inside `ICON_MARGIN` 0.15, `activity()` uses it; `plan::paint` draws `Mark::Site` through `Raster::icon` inside `ICON_MARGIN` 0.1. `plan::Mark { Portal, Encounter, Object, Ground, Lore, Site }`, `plan::marks(def, level)` and `plan::mark_pixels(scale, cell, mark) -> (pixels, colour)`; `Source.marks` painted after the edges, before the start. `forms::free_id`, `forms::encounter(def, data, at, was)` prefilled for a new one; `Form::keys` (the loader's keys before the upsert) and `Form::original_id`; `Editor::apply_form` says "Placed; the loader says: …" when the map stops loading after a change. `screen/editor_menu.rs` (`ScreenId::EditorMenu`, built from `Ctx.editing: Option<EditingCtx { open, dirty, errors }>`: Save → `Effect::Editor(EditAct)`, carried out by the router on `Host.editing`); `Nav.editing`/`edit()`; `Effect::OpenEditor` from the title's Editor; `Config.editor` (`--editor [map-id]`); the socket answers `Op::editor()` on `Editing`. Tests: `tests/editor.rs`, `tests/editor_paint.rs`, `tests/editor_pointer.rs` (the pointer through Bevy's own `UiPlugin`, layout and focus with a real `Window` cursor and no renderer), helpers in `tests/common/editor.rs` (`point_at`, `point_frac`, `leave_plan`, `mouse`, `press_edit`, `arrow`). `raster.rs` holds the shared pixel helpers (`EdgeKey`, `edge_key`, `tick_pixels`, `line_pixels`, `join_pixels`, `edge_pixels`) the minimap and the plan draw with.
- Screens (parity P2–P6; presentation-ARCHITECTURE §6): `src/screen/` is Bevy-free: `mod.rs` (`Screen`,
  `Panel`, `Row` (with `after`), `Span`, `Control` with `why_not`, `Kind::{Button, Choice, Number, Text}`,
  `Payload` (with `Set`), `stepped`, `Effect`, `HostOp`, `Refused` (with `NotDigits`), `Ctx`/`Game`/`Slot`,
  `Forms` (with `bank`, `selected` (the column's selection, the target), `owner` (whose kit, spells, sheet or tactics), P6: `dice`, `page`, `tactics`, P7: `fight` (`FightForm`), P8: `settings`, `debug`; `Ctx.prefs`, `Ctx.fits`; `accepted`, `shown`: the outcome cleared on a new top screen), `content`, `apply`),
  `nav.rs` (`Nav`: overlays over the world's base, `creating` until Begin; `settle`, `escape` (leaves a
  service), `paused`, `show` (replaces whatever stands beside the column), `allowed` (public, P6); `ScreenId::beside_column`), one file per screen (`title`, `new_game`, `create_party`, `pause`, `defeat`,
  `actions` (`Offer`, `Does::{Command, Choose}`: Use opens the verbs when a trial of `Interact` asks or refuses; Search here; `default_action` never searches), `verbs` (M8.5 2b: the faced door and `Views.objects`' verbs, `ControlId::Verb(n)`, lit by the view then a trial), `service`: P4's panel, `lists` by kind, `offer_row`, `bank_row`, `AMOUNT_MAX_GP`; `inventory` (P5): `ItemAct` (M8.5 2b: `Drop`, `PickUp`, `TakeOut(k)`, `PutIn(k)`, `k` the open container's place in `Views.objects`), `owner`, `owner_choice`, `command`, `here` (the pile and open containers under the stores), every button lit by a trial; `cast` (P5): road spells of `Forms.owner` on the selection; `camp` (P6): `counts` capped at `spendable`, both rests lit by the rest view then a trial; `sheet` (P6): `Page`, Stats/Magic/Gear; `tactics` (P6): `TacticAct`, `TacticsForm` (load, set, the lock), `LOCKED`; `tactics_draft` (P6): `Kind`, `Field`, `Draft`, `Choices`, `criteria_text`, a port of the other client's; `fight` (P7): `FightAct`, `FightForm`, `Place`, `place`, `settle`, the fight's controls, never on the Nav stack; `settings` (P8): `Field`, the draft, Apply; `debug` (P8): `Page`, `Num`, `DebugAct`, `DebugForm` (`accepted`), `unavailable`, every button lit by a trial), and `roster.rs` (the party column, a panel beside every base screen: entries,
  `Tool` with `why_not` and `screen`, `RosterEffect::Open`, `settle`). `shell/screen.rs` routes and paints (`Screens::reset`; `ScreenLayer`, `Beside` and `make_room`: the screen centres left of the column standing beside it); `shell/screen_draw.rs` builds the nodes (the slider and its thumb included; `rows` with a `Spawn { id, height, mark, keep }`,
  `Mark::{Screen, Side, Fight, Editor}`, `FightPress`, `EditorPress`; 2026-10-25: `Kind::Icon { name, chosen }` drawn by `icon_node` (a square box, `pressed_by` the mark's marker, the inner node `IconOf` + `NeedsIcon`), `least_width(mark)` (120 under `Mark::Editor`, else 150), `shell/icons.rs` (`IconImages` cache by `(name, px, colour)`, `fill` system, `tint(role)`, `CHOSEN` white, `IconsPlugin` added by `ShellPlugin`), `screen_text` prints `[icon] "name"`, `controls::shade` on `Button`s also under `Editing`; the in-place word markers (`TitleText`, `SpanAt`, `LabelOf`, `MessageText`, each with `OfScreen(ScreenId)`) under the marks that `rewrites()` (Screen, Editor), and `sync(screen, placed)` rewriting one screen's words; `text_field(value, max, lines)` (M5.6: `Kind::Text.lines` above one allows newlines); an editor text field carries `EditorPress`, the others `Pressable`; `Scrolls` panels and `wheel`); `shell/roster.rs` draws the column at the right and routes its `Side` buttons; it stays shown beside the screens of `ScreenId::beside_column`, and `Open` sets `Forms.owner` and calls `Nav::show`. Party creation reads `Views.creation` (`omnis_sim::creation_view`).
  `Session::{new_game (no party), with_party, start (the fixed four), drafts, restart}`. `labels.rs` turns ids and text keys into words (the only `Data::label` caller; the service words:
  `offer_label`, `offer_caption`, `offer_note`, `service_reason`, `rumor`, `ago`; P5: `slot`, `count_label`, `reason` for item and cast refusals; P6: `race`, `background`, `ability`, `signed`, `pay`, and rest and tactics refusals in `reason`; M8.5 2b: `verb`, `check`, `object_refusal`, `refused`); `shell/text.rs` the HUD's
  event lines, the town's and the kit's and road spells' included, P6 the rest's and the tactics' (`camp`), P8 the dev edits' (`dev`), M8.5 2b the objects' (`objects`: checks, a failed use, states, removals, finds, items spent, searches, notices);
  `style.rs` the roles and the type scale; `game_io.rs` save, load and tidy (the pause, the debug page and the socket share it; `HostOp::Tidy`, `DebugAct::Tidy`); `shell/failure.rs` (`record`, `.omnis/last-error.txt`: the line a failed start said, since Windows shows the GUI binary no console; 2026-11-08); `shell/retold.rs` (`retold`: the full log and the names retold from the party's log after a load, M8.5 7f); `history.rs` (`History`, `HISTORY` = `LOG_CAPACITY`, `Entry.folded` the filter's lowercase line).
  `VectorSet::Screens` runs every frame; `Grab`/`Input`/`Move` only while `playing`, `Draw` while a game
  runs.
- Tests: one `integration` binary (`tests/main.rs`). `common::session` places the party on the meadow cell
  the walking tests were written on (16, 16, North); `town_session` is the game's own start and replays. `tests/service.rs` (P4): `inside(data, site)`, the
  seven site tiles, the screen model and the shell's service tests. `common::ctx(data, world, views)` builds a screen `Ctx`. `tests/inventory.rs`, `tests/cast.rs`, `tests/notice.rs` (the outcome notice, 2026-10-16); `tests/full_log.rs` (the full log, 2026-10-17; retold after a load, 7f); `tests/game_io.rs` (saves, limits, tidy); `tests/piles.rs` (the inventory's ground and containers, 2026-10-16); `tests/object_lines.rs` (the objects' HUD lines, 2026-10-16); `tests/camp.rs`, `tests/sheet.rs`, `tests/tactics.rs` (P6); `tests/settings.rs`, `tests/debug.rs` (P8). `common::app::{press_ui, tap, top}` (P6).
- The engine boundary (parity P0): `sim.rs` holds `SimWorld` (`faced_door` since M8.5 2b lore) with its world private (methods mirror
  `omnis_sim::api`; it derefs to `World` only under feature `test-fixtures`, which the tests turn on) and
  `Views` (`here`, `position`, `party`, `combat`, `creation`, `service`, `casts` (`SimWorld::casts`), `rest` (`SimWorld::rest`, P6), `flags` (`SimWorld::flags`, only in a devtools world, P8)), which `Session::note` refreshes after every accepted
  command; a test that writes a fixture calls `Session::refresh`. The library is clippy-checked without the
  feature (`verify.sh`), so a field read does not compile.
- Dev socket (parity P1, feature `devtools`): `shell/socket.rs` (`DevSocketPlugin`, the framing), `shell/host.rs` (host ops; commands through `dispatch`, then into the binder's log, the pose
  snapped to the party when its place or facing changed; a load, a pack reload or `rules.set` set
  `Session::unreplayable`; with no game, every op but the screen's answers `NO_GAME`),
  `shell/screen_text.rs` (every shown `Text`, `[button]` when pressable, `[field]` for an `EditableText`),
  `capture.rs` `ShotRequest`/`ShotSaved` (both cameras into an image of the window's size for 4 frames).

## omnis-mcp and omnis-cli
`omnis-mcp/src/{tools,schema,editor_schema,bridge,backend,rpc}.rs`: 48 tools (17 the editor's, M5.3; 4 `data_*`, W1), the hand-written `Command` and editor argument
schemas (proven against the Rust types by `tests/schema_proof.rs`), `compact_tiles` for `Visible` and `Sensed`. `omnis-cli/src/{main,headless,args,schema,bake,wander,draft}.rs` (`main.rs` has `world tidy [--yes]`, and since M8.5 7h6 `save check` and `migration draft`; `draft.rs` drafts a pack's next migration from what its old version had; `wander.rs` the seeded wandering party, M8.5 7a):
`Headless` (a devtools world) is what the MCP `--headless` mode and the tests drive.
`omnis-mcp/tests/`: `api_doc.rs` (`docs/api.md` names every wire variant; ARCHITECTURE, README and this map state the code's counts, M8.5 7g), `replies.rs`, `schema_proof.rs`,
`vocabulary.rs` (protocol 2: one meaning per JSON key across commands, rejections, events and views),
`common/commands.rs` (the schema proof's `Command` instances, shared with the vocabulary test).

Tests: each crate's `tests/main.rs` declares its test files as modules of one `integration` binary, with
the helpers in `tests/common/`; `omnis-cli` `headless` and `omnis-sim` `measure` are
their own binaries (verification.md, "Test binaries").
`bake.rs`: `BakeSpec.key` (colour key for `Object` crops), `SurfaceSpec.size` in tiles,
`surface_texture`, `standee` (an `Object` upright at the tile's centre).

## `omnis-icon` (2026-10-24)
`crates/omnis-icon`: the Lucide set compiled in (`build.rs` → `$OUT_DIR/catalogue.rs`: `ICONS: &[(&str, &str)]`
sorted, `LICENSE`). `svg.rs` (`read(text) -> Vec<Element { name, attrs }>`, `Element::attr/number`, the allowed
root attributes `ROOT_ATTRS` and children `CHILDREN`, `Error::{UnknownElement, UnknownAttribute, Malformed}`),
`path.rs` (`parse(d) -> Vec<Stroke>`, `arc(...)` F.6.5, `PIECE` 0.25, `PIECES_MAX` 16, `ARC_STEP_DEG` 15),
`shape.rs` (`strokes(&Element)`, `SIDES` 32, a rounded rect through `path::parse`), `icon.rs` (`Icon { name,
strokes }`, `Stroke { points, closed }`, `SIDE` 24, `WIDTH` 2, `Icon::parse`, `segments(size)`, `width(size)`,
`bounds()`), `catalogue.rs` (`get`, `names`, `text`, `count`). Tests `tests/{catalogue,path}.rs`; the set's
size 1,903 is pinned there. `examples/show.rs` prints an icon as text.

## M5.7 preview (2026-10-24)
`scene::Occupancy` (`door_open`, `objects_on`, `beings_on`; `impl for SimWorld` in `sim.rs`) is what `extract`,
`edges`, `fills` and `level_of` take (`&dyn Occupancy`); `Scene::build_from(data, &MapData, map_id, level, occupancy)`
is the body, `Scene::build` the game's wrapper. Engine: `objects_view::placed_on_level(data, &MapData, level)`
(placed state, hidden shown, the map's piles) and `beings::standing_as_placed`, both in `api`. `editor/preview.rs`:
`AsPlaced { map }`, `SIGHT` 8, `map_id(data, map)`, `scene(data, map, level)`, `pose(cursor, facing)`,
`look(scene, pose) -> (eye, yaw)`. `shell/render.rs`: `colour`, `CREST`, `fog_end`, `fade(distance, sight)` are
`pub(super)` for the preview's draw. Tests `tests/editor_preview.rs`. `shell/preview.rs`:
`PreviewTarget(Handle<Image>)`, `PreviewCamera` (layer `LAYER` 2, `SIZE` 800×450, `Camera { is_active, order: -1 }`
switched by `switch::<ON>` on `Editing` added/removed), `PreviewGizmos`, `PreviewFills`, `Preview { scene_key:
(resolved_at, level), scene }`; `rebuild` → `preview::scene`, `place` → `preview::look` + `render::rotation`,
`draw` → `render::{colour, fade, fog_end, CREST}` with `preview::SIGHT`; `picture_height(width)`. `Editor.resolved_at`
stamps the revision at which `resolved` last changed. The editor's right column is a node holding `Column::Preview`
(an `ImageNode` of the target when `PreviewTarget` exists) over `Column::Inspector`. **Activities (2026-10-24):** `forms::Placing::Activity` (label "Activity") opens the site form
(`FormKind::Site`, fields `kind` over `ACTIVITIES` = ["Service site"] and `service`); `here()` labels a site
`Service <id>`; `icons::{KINDS, service_icon(kind), service(kind), activity_of(Option<ServiceKind>)}`;
`plan::Mark::Activity(Option<ServiceKind>)` with `plan::marks(def, data, level)`; `geometry::activities(data, map,
level, ground)` draws each resolved site's icon (called from `scene::level_of`; `extract` no longer draws sites).

## M5.8 Playtest (2026-10-24)
`editor/playtest.rs`: `blocked(editor) -> Option<String>` (no map; the loader's errors; no ground or impassable
under the cursor; no dev tools), `data_for(editor, data) -> Option<Arc<Data>>` (a clone, `registry.maps.intern`,
`maps.insert` of `resolved`), `teleport(editor) -> Option<DevCommand>`. `shell/editor.rs`: `Playtesting(pub Editor)`
(the editor parked; `route` on `Asked::Playtest` runs `start_playtest(&Editor, &Packs) -> Result<Session, String>`
(`Session::with_party(&temp Packs, seed, game_settings, &party::fixed())`, `ordered(Command::Dev(teleport))`,
`pose = Pose::at(views.position)`), then a queued command moves `Editing` into `Playtesting`, `screens.reset`,
`nav.edit(false)`, `ViewState::Explore`, the session inserted; `refresh` takes `Option<ResMut<Editing>>`).
`screen/mod.rs`: `Ctx.playtest` (from `Host.playtesting`/`paint`), `ControlId::BackToEditor`, `Effect::ToEditor`
(`carry_out`: the session removed, the parked editor re-inserted, `nav.edit(true)`; `ToTitle` also removes
`Playtesting`). `pause.rs`/`defeat.rs` branch on `ctx.playtest`. Tests `tests/editor_playtest.rs`.

## M5.9 RON text editor (2026-10-24)
`editor/files.rs`: `Mode { Plan, Files }`, `FileEdit { rel, text, disk, errors: Option<Vec<String>>, fresh }` with
`read`, `dirty`, `typed`, `checked`, `blocked_save`, `verdict`, `revert`, `written`. `Editor { mode, files, file }`,
`typed_file`, `open_file`, `write_file` (`session.write`, `reopen` when the file is the document's own), `reopen`
(shared with `revert`); acts `Plan`, `Files` (`session.files()`), `File(u16)`, `FileText`, `Check`
(`session.check`), `WriteFile`, `RevertFile`. `view::tools` = the Plan|Files row, the Map row, then `plan_rows` or
`file_rows`, then Undo/Redo, Save/Revert, Playtest, Menu; `view::text` (`ScreenId::EditorText`, `TEXT_LINES` 40,
`MAX_FILE_BYTES`); the inspector's Files branch. `shell/editor.rs`: `Column::Text` in the centre, shown by `mode`
(`Display` toggled in `refresh`, `column_entity`), `Drawn.text`, `PlanNodes` system param (a system takes sixteen
parameters at most), the `fresh` keep for a file as for a form, `route` sends `EditAct::FileText` to `typed_file`.
`screen_draw.rs`: a `Kind::Text` of several lines takes its row's width (`boxed_node`). Tests `tests/editor_files.rs`.
M5.9b (owner, 2026-10-24): the file list is a modal over the editor, `screen/editor_files.rs` (`ScreenId::EditorFiles`,
allowed over the editor only, pausing it; built from `EditingCtx { files, file: Option<(rel, dirty)> }`; the files
grouped under their folders, `ControlId::Edit(EditAct::File(n))` each, the others dim "save or revert <rel> first"
while the open file is unsaved; Cancel = `Resume` → `Effect::Back`); the tools' first row is Plan | Text (dim until
a file is open) | Open file... (`EditAct::OpenFile` lists through `Session::files` and returns `Asked::Pick`, the
layer or the router opens the picker; `File(n)` returns `Asked::Picked` and the router closes it; the open file
picked again is shown without a reread, another is refused in the model while the open one is dirty). A fresh form
or file always rebuilds its column (`f826536`: the words' sync leaves a field's text alone), and `route` ignores a
field while its form or file is `fresh` (`9e38b8c`: in the window Bevy marks every field changed every frame; the
test helper `common::app::churn_fields` reproduces that). M5.9c (owner, 2026-10-24): `EditAct::OpenFile` closes a
clean file and lists, or lists with the picker on a question for an unsaved one (`FileSaveClose` = check, write,
close; `FileDiscard`; Cancel keeps it; the re-pick branch gone, `Editor::check_file` shared with `Check`).
`Kind::Document { value, max }` is the whole-file control: `shell/document.rs` (`spawn`: a `Scrolls` area with
`Overflow::scroll()` holding the `Gutter(field)` numbers and the `DocumentField` (`document_field`: every line
visible, `visible_width` the longest line + 1, `TextLayout` NoWrap), a `Scrollbar` per axis; `size_documents`
keeps the sizes and numbers to the text; `follow_caret` moves the area's `ScrollPosition` by the pure `follow(view,
caret, scroll)` when the field's `EditableTextGeneration` changes; `lines`, `longest_line`, `numbers`);
`screen_draw::rows` lets a document's row take the panel's height, `bars` shows a bar on either axis, `wheel`
scrolls x by the wheel's x or Shift + y. `view::text` is one Document row. Test helper `common::app::ui_editor_app`
(the pointer test's layout app, shared).

## M5.10 re-authoring and close (2026-10-25)
`omnis-data/tests/edit_authoring.rs`: `requests_for(def) -> (NewMap, Vec<Request>)` derives from a map's own
definition the requests that build it from blank (`TerrainAdd`, `LevelAdd`, a `Paint` per tile, height and
inner edge that differs from blank, the stairs in the list's order, a `Place` per placement);
`every_shipped_map_is_rebuilt_from_blank_with_the_requests_alone` runs them through `edit::answer` on a scratch
copy with the map's file removed, then saves and compares the document, the reloaded def and the file's bytes
to the shipped pack; the ignored `rewrite_the_shipped_maps_through_the_editor` is the rewrite itself
(`Session::save` per map, `set_note` for the comments' texts, `seal::seal(root, Some("0.2.0"))`), kept as the
replays' rebaseline tests are. The frozen fixture `crates/omnis-sim/tests/packs/test` (id `test`, 0.2.9, `packs/test` as committed at `8d58c3e2`; since 2026-11-07 every test loads it and never the owner's pack under `packs/`): the four maps in the writer's layout, `notes/{town,dungeon,meadow,
depths}.ron`, sealed 0.2.0. Tests read the manifest's version through `seal::status` instead of pinning a
literal (`edit_session.rs`, `omnis-cli/tests/headless.rs`), and count notes from the shipped ones. Client:
`editor/acts.rs` (`EditAct`, `Editor::apply`) and `editor/forms/fields.rs` (the forms' limits, shared choice
fields and readers) split from `mod.rs` and `forms.rs`.

## Boundary tools (2026-10-25)
`omnis-vector/src/editor/run.rs`: wall runs by corners (`Tool::Corners`) and freehand (`Tool::Freehand`), the
two palette entries after Open edge. `sides(a, b) -> Vec<Side>` walks the grid lines from corner `a` to `b`:
straight along a shared row or column, else one unit a step along the axis that leaves the walk nearer the
line (the cross product with the line's direction; horizontal at a tie), `|dx| + |dy|` sides keyed as
`raster::EdgeKey` in map cells (`(horizontal, col, row)`: a cell's north or west side; row `height` and column
`width` are the boundary). `paints(size, sides) -> (Vec<Paint>, boundary)` dedupes in a `BTreeSet`, maps a
side to `((x, y), Brush::Edge { North | West, Wall })` and drops the boundary (walls already), counting it.
`Editor::pointer(data, Pointer { pixel, left: Press::{Up, Down, Held}, right_down }) -> clicked` is the plan's
pointer for every tool, moved from `shell::editor::route`: with a brush the hover/paint/pick/stroke rules as
before; with a run tool `Editor.corner = View::corner(size, px, py)` (the nearest corner within half a cell of
the map), `hover` None (the side bar hides), a right click picks the nearest side; Corners: Down starts or
extends the run, Down on its last corner takes it; Freehand: Down starts, Held adds each corner passed, Up
takes (off the plan too). `take_run` → `Document::paint_many(level, paints)` (one `change`, all or nothing,
closes a stroke; `apply_brush` extracted from `paint` for both) and says "N sides walled[, M on the boundary
stay walls]" or "nothing to wall". `Editor::apply` wraps `act` and drops a pending run when the tool, level,
map or history depths changed (`run_holds`); `pick` drops it. `EditAct::{TakeRun, DropCorner, DropRun}`; the
tools column's `run_rows` ("Run: N corners", Take run  Enter, Drop run, Drop corner  Backspace) while a run
is pending; Enter and Backspace in `route`. `Editor::preview() -> Option<Preview { sides, corner }>` (the run's
sides, by corners with the stretch to the hovered corner) goes into `plan::Source.preview` and `plan::paint`
draws it last: a bar `(s / 6).max(2)` thick along each side and a square on the corner, `minimap::PREVIEW`
(the caution colour's bytes). Tests: `run.rs` unit tests (the walk, the paints, the boundary),
`plan.rs` (`View::corner`), `omnis-data/tests/edit_document.rs::many_paints_are_one_step_and_fail_together`,
`omnis-vector/tests/editor_run.rs` (six headless tests on the town), `editor_pointer.rs` (a corner click
through Bevy's layout).

## Column step 2: Fill and Move (2026-10-25)
`omnis-vector/src/editor/fill.rs`: `region(grid, start) -> Vec<(u16, u16)>` walks the four neighbours from
`start` (a `VecDeque`, a `BTreeSet` of the taken), taking a cell when its `Grid::tile` equals the start's (the
same terrain index, or no ground with no ground) and the side between is `EdgeKind::Open` (a wall, a door or a
ramp bounds; the boundary is walls); `Editor::fill(data, cell) -> bool` maps the region to the entry's brush
(`Entry::brush`, a terrain rising the way faced, or the height held) and `Document::paint_many`s them as one
step, the cursor on the cell, "N cells filled with r street" / "N cells set to height 3"; a terrain the region
already is says "already r street" and paints nothing; an edge entry is refused ("the fill takes a terrain or
the height"; `apply` keeps it from happening). `grab.rs`: `Held { Start, Place(Here) }` (`label`; `Editor::under(cell)`: the start when it is on the
cell of the ground level, else the first of `here_at`, the inspector's order), `Grab { from: Option<cell>,
held: Option<Held>, press, origin, middle }` in `Editor.grab`; `Editor::grabbing(data, Pointer) -> Option<bool>` is read first by
`run::Editor::pointer`: a grab in progress follows its button (`drag_grab`: a held placement moves the cursor
to the cell under the pointer, a pan sets `View::origin` to the press's origin less the whole cells crossed,
`trunc`), lets go on Up (`end_grab`: a held placement on a map cell other than `from` → `moved`); the middle
button Down starts a pan under any tool; the Move tool's left Down starts a grab of `here_at(cell).first()`
(none → a pan) and its right click picks. `Editor::moved(data, &Here, to) -> bool` clones the def by the
`PlaceKey` (portals, sites and ground piles by `(level, x, y)`, encounters, objects and lore by id), sets its
`x, y`, refuses a tile-keyed kind where one already is ("a site is already at (4, 1)") and
`Document::place_many(vec![Remove(key), new])`s, one step, the cursor on `to`, "Portal 'town-gate' to
'meadow-gate' moved to (8, 2)". `Pointer.middle: Press` (the shell reads `MouseButton::Middle`). `Tool::{Move, Fill}` in
`brush.rs` (`ALL` 7: Select, Brush, Move, Fill, Corners, Freehand, Eraser; icons `move`, `paint-bucket`; keys
M, F in `route`); `aim` the cell for both; `apply` clears `grab` first (any act ends a drag); `Tool(Fill)` with
an edge chosen chooses `Entry::first` (the height on a map without a terrain); `Brush(i)` keeps Fill for a
terrain or the height (a run an edge) and chooses the brush otherwise. `Document::place_many(Vec<Place>)`
(`omnis-data`, beside `paint_many`). Tests: `fill.rs` unit test (a parted grid: a wall, a door and a ramp bound,
an open door joins, no ground is its own region), `omnis-data/tests/edit_document.rs::
many_places_are_one_step_and_fail_together`, `omnis-vector/tests/editor_fill.rs` (two headless tests on the
town), `editor_move.rs` (two: the portal carried, the site refused, the same cell and off the plan, a bare
cell's pan, the middle button's pan, an act ending a drag).

## Column step 3: placements as stamps (2026-10-25)
`omnis-vector/src/editor/stamp.rs`: `Stamps` on `Editor.stamps` (since 3b `[Vec<Place>; 6]`, what each stamp
drops), the detail held per `forms::Placing` (`Placing::index`, `detail` the choice's label, `what` the refusal's word); `stamp::options(kind,
data)` (maps, monsters, services, object kinds, items, `data.registry.text.names()`; sorted), `Stamps::{held, set, ids,
summary}` (3b), `stamp::of_here(def, &Here)` the stamp and detail a listed placement is (through
`forms::find`); `Editor::stamp(data, cell)`: `Form::placing(kind, def, data, at, Some(detail))` → `request` →
`change` (a re-keyed edit in `apply_form` goes as `[Remove(old), new]` through `place_many`, `Place::key` in
`omnis-data/src/edit/kinds.rs`), the message "<Here label> placed at (x, y)" through `Editor::report` (the
loader's first word appended, shared with `apply_form`); "nothing to stamp: the packs have no <what>" when the
kind has no options. `brush.rs`: `Entry::Stamp(Placing)` last in `palette`, `fills()` (a terrain or the height),
`stamp()`, `brush()` None, icons door-open, swords, `icons::ACTIVITY`, package, layers, scroll-text. `mod.rs`:
`aim` a cell for a stamp, `paint` with a stamp returns before `stroke` is set (a drag places once), `pick` takes
the first listed placement's stamp and detail before the terrain. `acts.rs`: `EditAct::Stamp` (since 3b the stamp's icon,
opening the picker; `PlaceKind` and `Add` gone), `Tool(Fill)` with `!entry.fills()` → the first terrain, `Brush(i)` keeps Fill
for what fills. `forms.rs`: `Form::placing(.., detail)`; every new form valid (portal, W1 step 2: a fresh id minted across the loaded maps, the link the stamp holds or none, facing the author,
`marker` a choice over the tileset's `SlotKind::Object` surfaces; object: `free_id` over objects and encounters;
pile: one item; lore: `free_id`, `text` a choice over the defined keys); `Form::keys` refuses a second portal,
site or pile on a new form's tile. `view.rs`: the detail choice under the palette, the inspector's Add row gone.
Tests: `tests/editor_stamp.rs` (7), `tests/editor_forms.rs` (the five Add-driven tests now stamp), `brush.rs`,
`omnis-data/tests/edit_document.rs::a_place_names_the_key_it_is_put_by`.

## Column step 3b: the detail picker and the properties (2026-10-31)
`editor/pick.rs`: `Picking { kind, filter, chosen, props: Vec<Form> }` on `Editor.picking` (`open(kind, &stamps)`,
`shown(&options)` the positions whose id holds the filter in any case, `toggle(id)` (a limit of one moves the
choice; at the limit "at most N <what>s"), `has_props(kind)` for Encounter, Object, Ground); `Editor::{pick_open
→ Asked::PickDetail, pick_filter, pick_toggle(data, i), pick_finish (held = stamp::rebuild; → props_open or
Asked::Picked), props_open → Asked::Props (a `Form::for_place` a held place at the cursor, copies), prop(n, i,
step), props_finish (each form's `value()` into the stamp), typed_in(act, text), set_in(act, n)}`. `stamp.rs`:
`Stamps([Vec<Place>; 6])`, `limit(kind)` (1, `MAX_STACKS`, `MAX_OBJECTS_PER_CELL`, `MAX_PILE_STACKS`,
`MAX_LORE_PER_CELL`), `details(places)`, `short(id)`, `rebuild(kind, chosen, prev, def, data)` (a portal and a
site through `Form::placing(..).value()`, the list kinds as structs, counts and the group's or an object's
properties carried for an id kept), `of_here` the placement whole (every object or lore on the cell),
`Editor::stamp_places` (held, else the packs' first), `Editor::stamp` (a `Form::for_place` a place, `request`,
all through `place_many`, "<labels> placed at"). `forms/props.rs`: `Form::for_place(place, def, data, at,
minted)` (the builder with `was`, a fresh id free of the map's and `minted`, `remove` none), `Form::props_rows(n)`
(the own fields but id, x, y, kind under `EditAct::Prop(n, i)`, list rows as a span and the count). `forms.rs`:
`request` = `keys` + `value()`, `id_taken(def, kind, id)`. `form.rs`: `Field::control_as(id)`. `acts.rs`:
`EditAct::{Stamp (the icon), Pick, PickFilter, PickToggle(u16), PickFinish, Props, Prop(u8, u8), PropsFinish}`.
`mod.rs`: `Asked::{PickDetail, Props}`. `view.rs`: the detail row (icon, Select..., sliders) and the summary line.
`screen/editor_pick.rs`, `screen/editor_props.rs` (`ScreenId::{EditorPick, EditorProps}`, `EditingCtx.pick`,
`Effect::Editor(act, payload)`; `nav::allowed`/`paused`). `shell/editor.rs::answer(nav, asked)` (the one place
the `Asked`s open and close overlays; `Props` replaces the picker on top), `shell/screen.rs::carry_out`
(Text → `typed_in`, Set → `set_in`, Press/Left/Right → `act` stepped). Tests: `tests/editor_pick.rs` (7:
the row and the picker's search, marks, limit and Cancel; the properties and the drop; two object kinds; a
pile's counts and two lore texts; the portal picker's empty list and a lone click's unlinked portal (W1 step 2c, 2026-11-07: `editor/links.rs` lists under a portal form's link field what it may link to (`candidates`, `matches`, `with_list` splicing the rows under the field, `Editor::link(n)`, `unlink`), narrowed by the field's text through `pick::fits`; `Form::set_text` marks `refill` and `shell/editor.rs::refresh` draws the column anew; the picker is the stamp's alone again, `For` gone; W1 step 2b: `pick.rs` `For::{Stamp, Field}`, `Picking { purpose, options, labels }` built by `pick::options(kind, def, data, except)`, a portal's rows "id  -  Map (x, y)" over `unlinked_portals`, `shown()` on labels; `pick_open_for_link`/`unlink` for the form's Link... and Unlink, `EditAct::{LinkPick, Unlink}`; `view::partner_line`; `tests/editor_link.rs`); a right click's copy (a portal's without its link); the
keyboard through the rebuilt rows), `tests/common/editor.rs::{type_ui, top_screen, hold, held}`, `pick.rs` unit
tests. Mutation checks red: the limit dropped, the minted ids not taken, the filter case-sensitive.

## Column step 4: the lasso, moving a selection, cell runs (2026-10-31)
`omnis-data/src/edit/document.rs`: `Document::batch(level, &[Paint], Vec<Place>)` (brushes then places in one
`change`, all or nothing, closes the stroke; `tests/edit_document.rs::a_batch_of_paints_and_places_is_one_step_
and_fails_together`). `omnis-vector/src/editor/select.rs`: `Selection { level, cells: BTreeSet }` on
`Editor.selection`, the loop in `Editor.lasso: Option<Vec<cell>>` while drawn; `inside(loop)` (the loop joined
cell to cell through `run::cells`, closed, plus the bounding box's cells the even-odd rule encloses, `encloses`),
`outline(cells) -> Vec<Side>` (the sides facing out), `moving(grid, def, sel, d) -> (paints, places)` (refuses a
cell off the map and a portal, site or pile on a destination tile the move does not vacate; source paints first:
Clear, Height 0, every side Open; then the destination's tile with its stair's rise, height and sides; boundary
sides skipped; then the start and every placement moved by d, tile-keyed kinds as Remove + new);
`Editor::{lasso_start, lasso_extend, lasso_end ("N cells selected" / "selection cleared" for one cell),
selected (the shown level's), selection_outline (displaced by cursor - grab.from while Held::Selection),
move_selection(data, d) → batch, "N cells moved to (x, y)"}`. `grab.rs`: `Held::Selection` first in `under`;
`grabbing` reads a loop in progress first (Held extends, Up/Down ends) and starts one on the lasso's left Down;
`end_grab` sends a selection with the offset. `run.rs`: `cells(a, b)` (Bresenham, both ends), `run_cells(points)`
(each once), `Editor::cell_run()` (= `entry.fills()`), a cell run's points are cells (`View::hit`, the hover
kept), `take_cells` (the entry's brush a cell through `paint_many`, "N cells painted r street" / "N cells set to
height 2"), `Preview.cells`; `preview()` returns the lasso's loop in progress as `cells` first (`run_cells` of the
loop; the owner's hand-test finding of 2026-10-31). `brush.rs`: `Tool::Lasso` (lasso-select, L), `Tool::ALL` 8. `acts.rs`: a run tool
forces the wall for a stamp only; `Brush(i)` keeps a run tool for any non-stamp entry; `apply` drops the loop.
`mod.rs`: `open` and `show_level` drop the selection; `aim`/`paint` arms. `view.rs`: "Selection: N cells" under
the tool's name. `plan.rs`: `Source.selection: Option<&[Side]>`, `SELECT` (white), `bars(out, view, sides,
colour)` shared with the preview, a cell run's cells as `rect_pixels` outlines. `shell/editor.rs`: key L,
`selection: outline.as_deref()`. Tests: `tests/editor_select.rs` (3: the loop, the count, the outline, a click
clears, an act mid-loop, Open drops; the move with the ghost, the walls, Undo, the two refusals, an overlapping
move, a press outside the selection; the cell runs by corners and freehand, a stamp's fallback), `select.rs` and
`run.rs` unit tests; `editor_run.rs` and `editor_pointer.rs` choose the wall before a run (a terrain is a cell
run now); `editor_column.rs` flipped. Mutation checks red: the even-odd rule inverted, the destination painted
before the source, the line's last cell dropped.

## Column step 5: Undo as a history list (2026-10-31)
`omnis-data/src/edit/words.rs`: `brush(def, level, x, y, &Brush)` (the terrain by name, a rise only for a joining
terrain, a side's kind, a height, a clearing; the level named off the ground), `stroke(cells, first)`,
`place(&Place)`, `edit(&Edit)`, `many(cells, placements)`, `clip` to `WORDS_MAX` 72. `document.rs`: `undo:
Vec<(State, String)>` (the state before a step and its words), `redo: Vec<(State, String)>`, `stroke:
Option<Stroke { first, len }>` in place of `in_stroke`; `change(coalesce, words, f)` (a coalesced paint rewrites
the top's words); `steps()` (done then undone, oldest first), `at()`, `goto(at)` (`NothingToUndo` past the end),
`name_step(words)`. `omnis-vector`: `EditAct::{History, HistoryTo(u16)}`, `Asked::History`; `Editor::{history()
-> Option<(&[String], usize)>, read_history (in changed), name_step (in report, fill, run, grab), history_to}`;
`view.rs` the `rotate-ccw-clock` icon after Redo; `screen/editor_history.rs` (`content`, `apply`), `ScreenId::
EditorHistory` (nav's editor-only lists), `EditingCtx.history`, `shell::editor::answer`. Tests:
`tests/editor_history.rs` (1), `edit_document.rs::steps_are_named_and_goto_walks_the_history`, `words.rs` unit
test. Mutation checks red: `goto` one short, a stroke's words not rewritten, `name_step` a no-op, rows reversed.
