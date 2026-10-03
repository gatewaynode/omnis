# Code map

Where each system lives, by crate and file, as of M7a (2026-10-03). Behaviour is described in
ARCHITECTURE.md §4.5; this file is the index into the code. Verify a name before leaning on it.

## Crates
`omnis-core` (ids, `Fixed`, dice, `Pcg32` streams, geometry, time, `money`) → `omnis-expr` (Rhai, `no_float`,
`only_i64`, checked) → `omnis-data` (the pack loader; the only file I/O) → `omnis-rules`
(characters, attacks, conditions, spells, effects, equipment) → `omnis-sim` (the world) →
`omnis-app` (Bevy), `omnis-cli` (headless), `omnis-mcp` (the bridge). The app reaches data through
`omnis_sim::omnis_data`; the sim reaches `Value` through `omnis_data::omnis_expr::Value`.

## omnis-data
`spell.rs` (`SpellEffect`, `Reach`), `item.rs` (`ItemKind`, `Slot`, `UseEffect::{Heal, Sense}`,
`consumable`, `description`), `sense.rs` (`SenseSource { geometry: Geometry::Ray, fidelity, check,
persistence, minutes }`, `Fidelity::rank()`), `rules.rs` (slots and values), `content.rs`
(cross-file checks such as the component threshold), `limits.rs`, `loader.rs`, `registry.rs`,
`service.rs` (`ServiceDef { kind: ServiceKind, items, spells, rumors }`, a list only for its kinds,
spells on a guild or a temple;
`MapDef.sites: Vec<Site>`, shapes in `validate_sites`, the service and the tile in
`resolve_sites`; `MapData::site_at`), `map.rs` (`Portal.marker`, required, an `Object` surface
checked in `loader::check_surfaces`; `MapData::marker_at`).
Base pack rules: `packs/base/data/rules/{casting,combat,creation,items,leveling,rest,sensing,services}.ron`;
services under `packs/base/data/services/` (seven, one per kind). The test pack depends on the base
pack and starts in `test:map:town` (the gate at (11, 2) leads to the meadow's start; the meadow's
road south at (16, 31) leads back).

## omnis-rules
`character.rs` (`create`, `starting_kit`), `stats.rs` (checks, saves, pools), `attack.rs`
(`attack_roll`, `rejudge`), `spell.rs` (casting ability, save DC, monster saves, heal rolls,
cantrip dice), `effect.rs` (`ActiveEffect`, `Expiry`, `BuffOn`, `Roll.bonus`), `equip.rs`
(`can_equip`, `equip`, `unequip`, `auto_equip`, `armor_class`, `weapons`), `condition.rs`,
`level.rs` (M7b: `ready`, `next_threshold`, `level_up` → `Gains { hp, spell_points, picks,
proficiency, features }`, `max_spell_level`, `may_learn` → `SpellRefusal`, `eligible`).

## omnis-sim
- Entry: `command.rs` (`Command::{Step, Turn, Interact, Party, Encounter, Combat, Cast, Item, Dev}`,
  words and `parse_script`, `Rejection` with `fmt_play`/`fmt_items`/`fmt_magic`), `apply.rs`
  (`match (mode, command)`; `advance()` is the only clock writer; effects pruned there),
  `event.rs` (ids and roll traces only; `ItemPlace`, `LayerCheck`, `SensedTile`).
- World: `world.rs` (`World`, `Settings { save_rule, permadeath, devtools }`, `Automap`,
  `Known`, `layer::{TERRAIN 1, STRUCTURE 2, VISITED 4, REMOTE 8}`, layer-aware `record`),
  `party.rs` (`heal`, `bury`), `visibility.rs` (`depth`, `cone`, `ray`), `query.rs`, `view.rs`,
  `migrate.rs` (one function per schema step, `v4_to_v5` the copper purse), `replay.rs`, `ops.rs`
  (`PartyView`, `MemberView`, `ItemView`, `party_view`).
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
  service panel and agents read. `ops.rs`: `PartyView` has `bank`, `last_long_rest`,
  `long_rest_wait`; `MemberView` `hit_dice`, `hit_dice_left`; `Status.service`. Script words
  with numbers: `command.rs::parse_town` (`buy-R-N`, `heal-M`, `short-rest-A-B`, …).
- Rest (M7 step 5): `rest.rs` (`RestCommand`, `apply`: `check_dice` or `too_soon` and
  `food_needed`, then `ambush_after`, `rest_events` and `roll_hit_dice` on copies of the
  `encounter` and `rest` streams, then the changes; `long_rest_restore` and `too_soon` are the
  inn room's too). `encounter::ambush` opens an `EncounterSource::Ambush` from the map's table.
  `rest_view.rs` (M7 step 8b): `rest_view` → `RestView { refusal, members: Vec<CampMember>,
  long_food, food, long }`, read-only, from the checks `rest::apply` makes (`too_soon`,
  `food_needed`, `food_need`, `hit_die`); `CampMember.spendable` is 0 when dead or at full hit
  points. What the camp panel reads; `tests/rest_view.rs` holds it to the command's answers.
  A map's rest events: `omnis-data/src/rest_event.rs` (`RestEventDef`, `RestKind`, resolved to
  `MapData.rest_events`); log lines for rests in `omnis-app/src/service_text.rs`. `apply.rs`: `landing()` is the
  pure half of a step, shared by `r#move` and `query::{step_lands, site_ahead}`; `arrive` enters
  a site (no encounter roll) or triggers the tile's encounter; `step_out` leaves only when the
  step goes somewhere. `World::may_save` counts an inn; a saved `Town` must match the tile
  (`LoadError::BadTown`).
- Fights: `encounter.rs`, `combat/{mod,state,turn,resolve,cast,reaction}.rs` (`CombatCommand::
  {Attack, Cast, Use, Dodge, Exchange, Run}`, `Plan`, `Roller::take`/`take_stream`, `run_until_member`,
  `end_of_round`, `settle`, `try_shield`).
- Magic and effects: `casting.rs` (explore casting), `effects.rs`, `checks.rs::roll` (the one
  check wrapper; guidance is spent here), `utility.rs` (`open_door_ahead`).
- Items: `items.rs` (`ItemCommand`, `UsePlan`, `UseKind`, `validate_use`, `use_item`, `transfer`,
  `drop_worn`, stock helpers `add_to`/`take_from`/`count_of`/`consume`/`item_id`).
- Sensing: `sense.rs` (`LAYERS`, `best_eyes`, `dc`, `reach_for_layer`, `resolve`).
- Dev: `dev.rs` (`DevCommand`, twelve variants, gated by `Settings.devtools`).
- Tests: one file per system under `tests/`, `common/mod.rs` builders (`data`, `world`,
  `new_world`: a new game placed on the meadow's start, where tests from before the town begin;
  `party_of`), `measure.rs` ignored. The replays leave town by a real `Step(Back)`.

## omnis-app
- Shell: `lib.rs` (`AppConfig`), `main.rs` (flags, plugins), `sim.rs` (`PlayState`,
  `ShellCommand::{Save, Load, ToggleAutomap, Pause, Cast, Sheet, Inventory, Look, Quit}`, the
  `shell` system runs whenever a world exists), `input.rs` (keys and the movement pad),
  `ui.rs` (`HELP_*` lines, `build_frame`, `overlay_for`, `RollLog`, `Selected`).
- The tool bar (M7 step 8a; `bevy_ui` over the canvas's `TOOLS` strip since 2026-10-02):
  `tool_bar.rs` (`ToolButton` with `Camp`, seven in a 4+3 grid; `ToolStates` is a resource kept
  every frame by `track` in `UiSet::Model`, so `MinimalPlugins` apps have it; `tool_states`,
  `tool_for` (CAMP is `ShellCommand::Camp`), `ToolPressed`, and `answer`, the one gate: a press on a
  button its state does not make live sends nothing, whatever the widget did), and
  `feathers_tools.rs` (`reconcile`, `sync` puts `InteractionDisabled` on dim buttons, `reports`
  turns `UiId::Tool` reports into `ToolPressed`). Its root is `ui_kit::ToolBar`, not a
  `PanelRoot`: `ui_kit::place` puts it on `TOOLS` shifted by `layout.core`, and the modal
  screens' code (Escape, `screen_text`, one panel at a time) never sees it.
- Screens: `screen.rs` (`View`, `Menu`, `Target`, `click`, `dump_screens`), `screens.rs`
  (overlay painters), `layout.rs` (core 1280×720; `RIGHT_COLUMN`, `TOOLS` y 300..388 (the bar's strip; the canvas paints nothing there), `PAD`,
  `BAND`; menu grid 80×16 at `menu_cell(c, r) = (241 + 6c, 200 + 8r)`), `widget.rs`
  (`WidgetId::{Row, Pad, Member, Stack, Action, Spell, Item}`, `PadState`),
  `panels.rs` (`framed_button`, `pad`), `plan.rs` (viewport and automap paint,
  `REMOTE_OUTLINE`, `PORTAL_MARK`; `paint_row` draws one detail row, portal markers in the block pass), `viewport.rs`, `raster.rs`, `pixel.rs`, `canvas.rs`, `font.rs`, `assets.rs`.
- Menus (Bevy-free model + painter + plugin): pause `menu.rs` (`Pause::ITEMS` seven rows 8..14,
  `Pause::DEBUG = 4`, `debug_available`) and `menus.rs` (`pause_action`, `Actions`); fight
  `combat_menu.rs`/`combat_screen.rs`/`combat.rs` with `spell_menu.rs` and `use_menu.rs`
  (pickers); sheet `sheet_menu.rs`/`sheet_screen.rs`/`sheet.rs`; inventory
  `inventory_menu.rs`/`inventory_screen.rs`/`inventory.rs`; debug `debug_menu.rs`/
  `debug_screen.rs`/`debug.rs` (`DebugPlugin`, feature `devtools`, opens on `OnEnter(PlayState::Debug)`).
- Text: `text.rs` (`Names`, `Line` long ≤100 / short ≤39 cells, `trace_math`, `faces`) imported
  downward by `combat_text.rs` (`event_line` chain: before_fight → round → wound → spell → item →
  sense), `spell_text.rs`, `item_text.rs`, `sense_text.rs`. `look.rs` (`look_command`).
- `bevy_ui` screens (every build since M7 step 1; the canvas creation screen is gone). The kit:
  `ui_model.rs` (Bevy-free: `Payload`, `FONTS`, `fitted_scale`, `scale_cap`, `SCALE_*`, `whole`)
  and `ui_kit.rs` (`UiId`, `UiLabel`, `UiScreen`, one variant per screen, with `name()` for the
  text tree; `Control(UiId)`, `Shown(UiLabel)`, `PanelRoot { screen, shape }`; the scenes `panel`,
  `title`, `row`, `row_label`, `column`, `button`, `message_line`, `dropdown` with `Width`;
  `FontChoice`, `ScaleChoice`, `scale_for`, the systems `scale` and `place`, `set_text`; the five
  observers, which publish `UiReport { id, payload }` and know no screen). A screen is a Bevy-free
  model plus a scenes file: party creation is `creation_menu.rs` (`CreationForm`, `Catalog`, the
  named methods that hold the rules; `set_name` holds 24 characters and the rules' 32 bytes),
  `creation_panel.rs` (`PanelId`, `LabelId`, `apply`, `PanelAction`, `text`, `shape`) and
  `feathers_creation.rs` (scenes per row, `reports` in `UiSet::Dispatch` before
  `menus::CreationFlow`, `reconcile`, `show_scale`, `sync`, `Synced`). `feathers_fonts.rs`
  (`PanelFonts`, `Face`, `wear`: every text under any `PanelRoot`), `feathers_ui.rs`
  (`FeathersUiPlugin`, Escape; it writes `ui.rs`'s `UiPointerCapture`), `capture.rs`
  (`ComposeCapture`, `ComposedSaved`; feature `devtools`), `menus::CreationAsk` (what the panel
  asks of the creation flow), `Menu::Covered` (the backdrop the canvas paints under a panel).
  Fonts under `assets/fonts/{inter,alegreya-sans}`. **A new screen**: a variant in `UiId`,
  `UiLabel` and `UiScreen`, its ids in its Bevy-free model, a `reports` system that reads
  `UiReport`s carrying its ids, a `reconcile` that touches only its own roots; the pattern is in
  `tasks/plans/feathers-experiment.md`. The second screen on the kit is the confirmation before a
  step into or out of a service: `confirm_panel.rs` (`Confirm`, `ConfirmId`, `ask`, `answer`),
  `feathers_confirm.rs` (`reconcile`, `reports`), and in `input.rs` the `Gate` every map key and
  pad click passes (`AskFirst`, `ConfirmAnswer`, `answer_keys`, `settle_answers`,
  `PlayState::Confirm`, `Active::Confirm`). Town events read through `service_text.rs`.
  The third is the service panel (M7 step 7): `PlayState::Service` is what `for_mode` gives
  `Mode::Town` (`Active::Service`; `follow_mode` follows it). `service_panel.rs` (Bevy-free:
  `ServicePanelId`, `ServiceLabelId`, `ServiceForm`, `ServiceAsk`, `apply`, `offer_rows`,
  `note`, `reason`, `money_line`, `shape`) reads `omnis_sim::service_view`;
  `feathers_service.rs` (`ServiceShown`; `look`, `refusals`, `reconcile`, `sync` in
  `UiSet::Model`; `reports` and `escape_leaves` in `UiSet::Dispatch`) sends every offer through
  `input::Gate`, and `confirm_panel::ask` asks before `Service(Leave)`. Refused offers carry
  `InteractionDisabled`; the smith's two lists scroll. Inside, the map's keys and pad are off;
  ITEMS, SPELLS, SHEET and MENU stay live (`tool_bar::tool_states`).
  The fourth is the camp (M7 step 8b): `PlayState::Camp` / `Active::Camp`, a shell overlay over
  the map opened by `ShellCommand::Camp` (CAMP on the bar, live on the map with a member; R) and
  only in `Mode::Explore`; `combat::follow_mode` takes the game out of it when an ambush starts
  a fight. `camp_panel.rs` (Bevy-free: `CampPanelId`, `CampLabelId`, `CampForm` with `fit`,
  `CampAsk`, `apply`, `member_line`, `dice_note`, `food_line`, `long_note`, `shape`) reads
  `omnis_sim::rest_view`; `feathers_camp.rs` (`CampShown`; `look`, `refusals`, `reconcile`,
  `sync` in `UiSet::Model`; `reports` and `escape_closes` in `UiSet::Dispatch`): a slider per
  member who may spend dice, Short rest (dim with nothing chosen), Long rest (dim with its
  reason), Close. The sheet's stats page shows "Hit dice 1/1 d10" on row 4
  (`SheetView.hit_dice`).
- Dev: `dev.rs` (`DevScript`), `socket.rs` (loopback dev socket, `.omnis/dev.addr`; `screenshot`
  takes `ops::ShotTarget::{Canvas, Window}`; `screen.text` is queued by `serve` and answered by
  the exclusive `answer_screen_text`). `ui_text.rs` (every build): `screen_text` (every panel
  under its screen's name) and `panel_text`, the one text-tree walker the tests use too.
- Tests: `tests/common/mod.rs` (`ui_app_saving_to`, `click`, `press`, `seen`, `world`,
  `fighter_draft`, `ask_creation`, `party_by_command`), one file
  per screen; messages are collected by a reader system, never read from `Messages<M>` directly;
  `tests/common/feathers.rs`, `tests/feathers.rs`, `tests/feathers_panel.rs` for the panel,
  `tests/confirm.rs` for the confirmation, `tests/service.rs` for the service panel
  (`feathers::press` sends a key pressed and released, so it can be pressed again).

## omnis-mcp and omnis-cli
`omnis-mcp/src/{tools,schema,bridge,backend,rpc}.rs`: twenty tools, the hand-written `Command`
schema (proven against the Rust types by `tests/schema_proof.rs`), `compact_tiles` for `Visible` and `Sensed`. `omnis-cli/src/{headless,args,schema,bake}.rs`:
`Headless` (a devtools world) is what the MCP `--headless` mode and the tests drive.
`bake.rs`: `BakeSpec.key` (colour key for `Object` crops), `SurfaceSpec.size` in tiles,
`surface_texture`, `standee` (an `Object` upright at the tile's centre).
