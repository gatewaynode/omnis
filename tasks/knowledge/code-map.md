# Code map

Where each system lives, by crate and file, as of M6 (2026-09-20). Behaviour is described in
ARCHITECTURE.md §4.5; this file is the index into the code. Verify a name before leaning on it.

## Crates
`omnis-core` (ids, `Fixed`, dice, `Pcg32` streams, geometry, time) → `omnis-expr` (Rhai, `no_float`,
`only_i64`, checked) → `omnis-data` (the pack loader; the only file I/O) → `omnis-rules`
(characters, attacks, conditions, spells, effects, equipment) → `omnis-sim` (the world) →
`omnis-app` (Bevy), `omnis-cli` (headless), `omnis-mcp` (the bridge). The app reaches data through
`omnis_sim::omnis_data`; the sim reaches `Value` through `omnis_data::omnis_expr::Value`.

## omnis-data
`spell.rs` (`SpellEffect`, `Reach`), `item.rs` (`ItemKind`, `Slot`, `UseEffect::{Heal, Sense}`,
`consumable`, `description`), `sense.rs` (`SenseSource { geometry: Geometry::Ray, fidelity, check,
persistence, minutes }`, `Fidelity::rank()`), `rules.rs` (slots and values), `content.rs`
(cross-file checks such as the component threshold), `limits.rs`, `loader.rs`, `registry.rs`.
Base pack rules: `packs/base/data/rules/{casting,combat,creation,items,leveling,sensing}.ron`.

## omnis-rules
`character.rs` (`create`, `starting_kit`), `stats.rs` (checks, saves, pools), `attack.rs`
(`attack_roll`, `rejudge`), `spell.rs` (casting ability, save DC, monster saves, heal rolls,
cantrip dice), `effect.rs` (`ActiveEffect`, `Expiry`, `BuffOn`, `Roll.bonus`), `equip.rs`
(`can_equip`, `equip`, `unequip`, `auto_equip`, `armor_class`, `weapons`), `condition.rs`.

## omnis-sim
- Entry: `command.rs` (`Command::{Step, Turn, Interact, Party, Encounter, Combat, Cast, Item, Dev}`,
  words and `parse_script`, `Rejection` with `fmt_play`/`fmt_items`/`fmt_magic`), `apply.rs`
  (`match (mode, command)`; `advance()` is the only clock writer; effects pruned there),
  `event.rs` (ids and roll traces only; `ItemPlace`, `LayerCheck`, `SensedTile`).
- World: `world.rs` (`World`, `Settings { save_rule, permadeath, devtools }`, `Automap`,
  `Known`, `layer::{TERRAIN 1, STRUCTURE 2, VISITED 4, REMOTE 8}`, layer-aware `record`),
  `party.rs` (`heal`, `bury`), `visibility.rs` (`depth`, `cone`, `ray`), `query.rs`, `view.rs`,
  `migrate.rs`, `replay.rs`, `ops.rs` (`PartyView`, `MemberView`, `ItemView`, `party_view`).
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
  `party_of`), `measure.rs` ignored.

## omnis-app
- Shell: `lib.rs` (`AppConfig`), `main.rs` (flags, plugins), `sim.rs` (`PlayState`,
  `ShellCommand::{Save, Load, ToggleAutomap, Pause, Cast, Sheet, Inventory, Look, Quit}`, the
  `shell` system runs whenever a world exists), `input.rs` (keys, `tool_for`, `map_tools`),
  `ui.rs` (`HELP_*` lines, `build_frame`, `tool_states`, `overlay_for`, `RollLog`, `Selected`).
- Screens: `screen.rs` (`View`, `Menu`, `Target`, `click`, `dump_screens`), `screens.rs`
  (overlay painters), `layout.rs` (core 1280×720; `RIGHT_COLUMN`, `TOOLS` y 300..388, `PAD`,
  `BAND`; menu grid 80×16 at `menu_cell(c, r) = (241 + 6c, 200 + 8r)`), `widget.rs`
  (`WidgetId::{Row, Skill, Pad, Tool, Member, Stack, Action, Spell, Item}`, `ToolButton`),
  `panels.rs` (`framed_button`, `pad`, `tools`), `plan.rs` (viewport and automap paint,
  `REMOTE_OUTLINE`), `viewport.rs`, `raster.rs`, `pixel.rs`, `canvas.rs`, `font.rs`, `assets.rs`.
- Menus (Bevy-free model + painter + plugin): pause `menu.rs` (`Pause::ITEMS` seven rows 8..14,
  `Pause::DEBUG = 4`, `debug_available`) and `menus.rs` (`pause_action`, `Actions`); fight
  `combat_menu.rs`/`combat_screen.rs`/`combat.rs` with `spell_menu.rs` and `use_menu.rs`
  (pickers); sheet `sheet_menu.rs`/`sheet_screen.rs`/`sheet.rs`; inventory
  `inventory_menu.rs`/`inventory_screen.rs`/`inventory.rs`; debug `debug_menu.rs`/
  `debug_screen.rs`/`debug.rs` (`DebugPlugin`, feature `devtools`, opens on `OnEnter(PlayState::Debug)`).
- Text: `text.rs` (`Names`, `Line` long ≤100 / short ≤39 cells, `trace_math`, `faces`) imported
  downward by `combat_text.rs` (`event_line` chain: before_fight → round → wound → spell → item →
  sense), `spell_text.rs`, `item_text.rs`, `sense_text.rs`. `look.rs` (`look_command`).
- Dev: `dev.rs` (`DevScript`), `socket.rs` (loopback dev socket, `.omnis/dev.addr`).
- Tests: `tests/common/mod.rs` (`ui_app_saving_to`, `click`, `press`, `seen`, `world`), one file
  per screen; messages are collected by a reader system, never read from `Messages<M>` directly.

## omnis-mcp and omnis-cli
`omnis-mcp/src/{tools,schema,bridge,backend,rpc}.rs`: eighteen tools, the hand-written `Command`
schema, `compact_tiles` for `Visible` and `Sensed`. `omnis-cli/src/{headless,args,schema,bake}.rs`:
`Headless` (a devtools world) is what the MCP `--headless` mode and the tests drive.
