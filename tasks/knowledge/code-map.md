# Code map

Where each system lives, by crate and file, as of M7c (2026-10-04). Behaviour is described in
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
(cross-file checks such as the component threshold), `limits.rs`, `loader.rs`, `map_data.rs` (`MapData` and its lookups by tile, M8.5 step 2), `object.rs` (`ObjectKind`, `Look`, `Verb`, `Requirement`, `Effect`, `Interaction`; cross-file checks and flag interning in `object/check.rs`; M8.5 step 2), `footprint.rs` (`Footprint::{of, path, overlaps, cells, ray_span}` in sixteenths of a cell, shared by the engine and the client's mirror), `placement.rs` (`Placement`, `GroundDef`, `Pile`, `ResolvedPlacement`; shapes checked with the map, kinds, ground, spills and arrival centres at resolution; `in_state`, `looks`, `covers_centre`; `MapData::{objects, object_cells, ground, objects_at}`), `loot.rs` (`LootTable { picks, entries }`, `LootEntry { item, count, weight, chance_percent }`; `Monster.loot` names one), `registry.rs`,
`service.rs` (`ServiceDef { kind: ServiceKind, items, spells, rumors }`, a list only for its kinds,
spells on a guild or a temple;
`MapDef.sites: Vec<Site>`, shapes in `validate_sites`, the service and the tile in
`resolve_sites`; `MapData::site_at`), `map.rs` (`Portal.marker`, required, an `Object` surface
checked in `loader::check_surfaces`; `MapData::marker_at`). M7c: `action.rs` (`Cost { Action,
BonusAction, Reaction, Free }`, `Uses`, `Recharge`, `FeatureEffect { Heal, ExtraAction, Cunning }`);
`spell.rs` carries `cost` and D24's three fields (required in every file); `character.rs`'s class
features carry `effect`, `cost`, `uses`; `monster.rs` `Monster.casting` (`MonsterCasting`: attack,
DC, caster level, points, spells). The `turn.*` slots are in `rules/combat.ron` (inputs `level`,
`is_member`).
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
- Entry: `command.rs` (`Command::{Step, Turn, Interact, Party, Encounter, Combat, Cast, Item, Service, Rest, Dev}`,
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
  `search`); `ground.rs` (`pile_at`, `Drop`, `PickUp`, `lay` for loot); `combat/loot.rs` (`of`, `draw`);
  `items.rs` (`from_kit`, `from_list`, `out_of_kit`, `actor`); `objects_view.rs` (`objects_view` for
  `objects.get`, `in_sight` for `ViewportModel.objects`/`.piles`). Fixture pack
  `crates/omnis-sim/tests/packs/objects` (the yard); tests `objects.rs`, `object_act.rs`, `object_use.rs`,
  `ground.rs`, `loot.rs`, `notice.rs`, `objects_view.rs`.
- Time (M8): `time.rs` (`PartyTime { shared_milli, date, era }`, `advance` at the region's company,
  `moved` raises `Signal::Entered`, the `Sim` host and `Reconcile`, `subscriptions`, `region_clock`,
  couplings at depth one on `time:<a>:<b>`), `time_view.rs` (`TimeView`, `DateView`, `ContactView`,
  `time_view`, `party_date`; `time.clocks`); `World.{contacts, party_time, bus}`; `migrate::v6_to_v7`.
  Data: `omnis-data/src/region.rs` (`RegionDef`, `Region`, `RegionKind`, the load checks),
  `MapData.region`, `Data::calendar()` → `omnis_core::{Calendar, Date}`, `text::fill` (`{ago}`);
  `rules/time.ron` (`time.settled`, `time.wild`, the calendar values). App: `panels::clock_text`
  (the HUD clock line), `text::ago_text` (a rumor's age).
- The engine's API (ARCHITECTURE §4.9, M8 step 8): `api.rs` re-exports exactly the contract; `ops.rs` (`PROTOCOL`,
  `Op`, `Reply` tagged `reply`, `OpError`, `Status`, `dispatch`, `Op::is_host`, `MAX_SCRIPT`, the host ops' rules
  `save_text`, `load_text`, `check_reload`, `rules_set`); `names.rs` (protocol 2: `Place`, `id_of` (`#n` for a
  number no pack names), `action_named`: events and views name definitions by string id here); the views `query::here`, `party_view.rs`, `view.rs`,
  `service_view.rs`, `rest_view.rs`, `time_view.rs`, `cast_view.rs`, `objects_view.rs`; `omnis-cli/src/headless.rs` (`Headless`, the in-process host);
  `omnis-vector/src/shell/{socket,host}.rs` (the dev socket host, `MAX_LINE`); `omnis-mcp/src/tools.rs` (tool → op).
- Bus vocabulary: `bus.rs` (`Topic { Region, Battle }`, `Subscriber { Reconcile, Reactions }`,
  `Signal { Entered, Battle(Cue) }`, `type Bus`); the hosts are `time.rs::Sim` and
  `combat/reaction.rs::Fight`.
- World: `world.rs` (`World`, `Settings { save_rule, permadeath, devtools }`, `Automap`,
  `Known`, `layer::{TERRAIN 1, STRUCTURE 2, VISITED 4, REMOTE 8}`, layer-aware `record`),
  `party.rs` (`heal`, `bury`), `visibility.rs` (`depth`, `cone`, `ray`), `query.rs`, `view.rs`,
  `migrate.rs` (one function per schema step, `v4_to_v5` the copper purse), `replay.rs`,
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
- Dev: `dev.rs` (`DevCommand`, twelve variants, gated by `Settings.devtools`).
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
  `Quality`, `Face`, the lenient settings file `read`/`write`, `clamp_scale`, `fits`), `rolllog.rs` (the roll log; `Names`, which since M8.5 2b also names objects on the party's map, `locate`, `object`,
  definitions by string id), `cinema.rs` and `raster.rs` (the picture window).
- Objects (M8.5 step 2b): `shapes.rs` (`Body`, `shapes`: a look as lines and fills on its footprint, turned by facing,
  on the home floor; a shut arch crossed; `PILE_LOOK`); `scene.rs` `Scene::build(data, ..)` adds each level's
  objects and piles from `SimWorld::objects_on` (`api::objects_on_level`); `SegKind::{Object, Pile}`; the
  shell redraws on `session::reshapes` (doors, object state, removal, notice, ground moves, `CombatEnded`).
  Tests `tests/shapes.rs`. `collide::blocks` refuses a step through a footprint via `SimWorld::object_blocks`
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
  `ALEGREYA` at fixed handles, `FontsPlugin`, `wear`), `hud.rs`, `notice.rs` (2026-10-16: the outcome notice centred where the screens stand, faded by `crate::notice::alpha` and dropped out of range; it replaced P5's bottom message line), `minimap.rs`, `render.rs`, `capture.rs` (`--screenshot`,
  offscreen; the socket's live capture).
- Screens (parity P2–P6; presentation-ARCHITECTURE §6): `src/screen/` is Bevy-free: `mod.rs` (`Screen`,
  `Panel`, `Row` (with `after`), `Span`, `Control` with `why_not`, `Kind::{Button, Choice, Number, Text}`,
  `Payload` (with `Set`), `stepped`, `Effect`, `HostOp`, `Refused` (with `NotDigits`), `Ctx`/`Game`/`Slot`,
  `Forms` (with `bank`, `selected` (the column's selection, the target), `owner` (whose kit, spells, sheet or tactics), P6: `dice`, `page`, `tactics`, P7: `fight` (`FightForm`), P8: `settings`, `debug`; `Ctx.prefs`, `Ctx.fits`; `accepted`, `shown`: the outcome cleared on a new top screen), `content`, `apply`),
  `nav.rs` (`Nav`: overlays over the world's base, `creating` until Begin; `settle`, `escape` (leaves a
  service), `paused`, `show` (replaces whatever stands beside the column), `allowed` (public, P6); `ScreenId::beside_column`), one file per screen (`title`, `new_game`, `create_party`, `pause`, `defeat`,
  `actions` (`Offer`, `Does::{Command, Choose}`: Use opens the verbs when a trial of `Interact` asks or refuses; Search here; `default_action` never searches), `verbs` (M8.5 2b: the faced door and `Views.objects`' verbs, `ControlId::Verb(n)`, lit by the view then a trial), `service`: P4's panel, `lists` by kind, `offer_row`, `bank_row`, `AMOUNT_MAX_GP`; `inventory` (P5): `ItemAct` (M8.5 2b: `Drop`, `PickUp`, `TakeOut(k)`, `PutIn(k)`, `k` the open container's place in `Views.objects`), `owner`, `owner_choice`, `command`, `here` (the pile and open containers under the stores), every button lit by a trial; `cast` (P5): road spells of `Forms.owner` on the selection; `camp` (P6): `counts` capped at `spendable`, both rests lit by the rest view then a trial; `sheet` (P6): `Page`, Stats/Magic/Gear; `tactics` (P6): `TacticAct`, `TacticsForm` (load, set, the lock), `LOCKED`; `tactics_draft` (P6): `Kind`, `Field`, `Draft`, `Choices`, `criteria_text`, a port of the other client's; `fight` (P7): `FightAct`, `FightForm`, `Place`, `place`, `settle`, the fight's controls, never on the Nav stack; `settings` (P8): `Field`, the draft, Apply; `debug` (P8): `Page`, `Num`, `DebugAct`, `DebugForm` (`accepted`), `unavailable`, every button lit by a trial), and `roster.rs` (the party column, a panel beside every base screen: entries,
  `Tool` with `why_not` and `screen`, `RosterEffect::Open`, `settle`). `shell/screen.rs` routes and paints (`Screens::reset`; `ScreenLayer`, `Beside` and `make_room`: the screen centres left of the column standing beside it); `shell/screen_draw.rs` builds the nodes (the slider and its thumb included; `rows` with
  `Mark::{Screen, Side, Fight}`, `FightPress`; the in-place word markers only under `Mark::Screen`; `Scrolls` panels and `wheel`); `shell/roster.rs` draws the column at the right and routes its `Side` buttons; it stays shown beside the screens of `ScreenId::beside_column`, and `Open` sets `Forms.owner` and calls `Nav::show`. Party creation reads `Views.creation` (`omnis_sim::creation_view`).
  `Session::{new_game (no party), with_party, start (the fixed four), drafts, restart}`. `labels.rs` turns ids and text keys into words (the only `Data::label` caller; the service words:
  `offer_label`, `offer_caption`, `offer_note`, `service_reason`, `rumor`, `ago`; P5: `slot`, `count_label`, `reason` for item and cast refusals; P6: `race`, `background`, `ability`, `signed`, `pay`, and rest and tactics refusals in `reason`; M8.5 2b: `verb`, `check`, `object_refusal`, `refused`); `shell/text.rs` the HUD's
  event lines, the town's and the kit's and road spells' included, P6 the rest's and the tactics' (`camp`), P8 the dev edits' (`dev`), M8.5 2b the objects' (`objects`: checks, a failed use, states, removals, finds, items spent, searches, notices);
  `style.rs` the roles and the type scale; `game_io.rs` save and load (the pause and the socket share it).
  `VectorSet::Screens` runs every frame; `Grab`/`Input`/`Move` only while `playing`, `Draw` while a game
  runs.
- Tests: one `integration` binary (`tests/main.rs`). `common::session` places the party on the meadow cell
  the walking tests were written on (16, 16, North); `town_session` is the game's own start and replays. `tests/service.rs` (P4): `inside(data, site)`, the
  seven site tiles, the screen model and the shell's service tests. `common::ctx(data, world, views)` builds a screen `Ctx`. `tests/inventory.rs`, `tests/cast.rs`, `tests/notice.rs` (the outcome notice, 2026-10-16); `tests/piles.rs` (the inventory's ground and containers, 2026-10-16); `tests/object_lines.rs` (the objects' HUD lines, 2026-10-16); `tests/camp.rs`, `tests/sheet.rs`, `tests/tactics.rs` (P6); `tests/settings.rs`, `tests/debug.rs` (P8). `common::app::{press_ui, tap, top}` (P6).
- The engine boundary (parity P0): `sim.rs` holds `SimWorld` with its world private (methods mirror
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
`omnis-mcp/src/{tools,schema,bridge,backend,rpc}.rs`: 25 tools, the hand-written `Command`
schema (proven against the Rust types by `tests/schema_proof.rs`), `compact_tiles` for `Visible` and `Sensed`. `omnis-cli/src/{headless,args,schema,bake}.rs`:
`Headless` (a devtools world) is what the MCP `--headless` mode and the tests drive.
`omnis-mcp/tests/`: `api_doc.rs` (`docs/api.md` names every wire variant), `replies.rs`, `schema_proof.rs`,
`vocabulary.rs` (protocol 2: one meaning per JSON key across commands, rejections, events and views),
`common/commands.rs` (the schema proof's `Command` instances, shared with the vocabulary test).

Tests: each crate's `tests/main.rs` declares its test files as modules of one `integration` binary, with
the helpers in `tests/common/`; `omnis-cli` `headless` and `omnis-sim` `measure` are
their own binaries (verification.md, "Test binaries").
`bake.rs`: `BakeSpec.key` (colour key for `Object` crops), `SurfaceSpec.size` in tiles,
`surface_texture`, `standee` (an `Object` upright at the tile's centre).
