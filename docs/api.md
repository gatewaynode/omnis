# The Omnis engine API

Reference for developers on other tracks (an alternate UI, external tools, test harnesses) who
code against the simulation while it is still being built. It describes the API **as built**,
protocol version 2. The rules behind it are in `ARCHITECTURE.md` §4.9; when this document and
the code disagree, the code wins and this document has a bug.

`crates/omnis-mcp/tests/api_doc.rs` fails when this document misses an op, a reply tag, an error
kind, a command, a dev command, an event or a rejection. `crates/omnis-mcp/tests/vocabulary.rs`
fails when a field name on the wire carries two meanings (§4).

## 1. Overview

Omnis's simulation (`omnis-sim`) is a deterministic, turn-based engine for an SRD 5.1 variant:
a party of up to six walks a grid, meets monsters, fights in rounds, casts, rests, trades and
levels. All state lives in one `World`; it changes only when a `Command` is applied, and every
change is reported as a list of `Event`s.

The API has two tiers that share one version:

| Tier | What | Use it when |
|---|---|---|
| 1 | The Rust library, `omnis_sim::api` | Your client is Rust and runs in the same process (the app, the CLI, a test, another Rust front end). |
| 2 | The JSON op protocol, `omnis_sim::ops` | Anything else: another language, another process, a script, an agent. |

**Version.** `ops::PROTOCOL` is `2`. A client calls `game.status` first and checks the
`protocol` field of the reply. See §12 and §13 for what moves it.

**Not API.** Anything not re-exported from `crates/omnis-sim/src/api.rs` is internal and may
change without notice. In particular:

- `World`'s fields are public for the simulation's own tests and tools, never for clients. Read
  state through the views (§7). The app is held to this by the compiler (its `SimWorld` keeps
  the world private).
- `world.query` reads the world's serialized shape by path. It is a debugging aid; its paths
  change whenever the world's layout does, with no version bump.

## 2. Quick start, Tier 1 (Rust)

```rust
use omnis_sim::api::*;
use std::path::Path;

let data = load_packs(&[Path::new("packs/base"), Path::new("packs/test")])?; // Result<Data, LoadReport>
let mut world = World::new(&data, 1, Settings::default())?;                  // seed 1
match apply(&mut world, &data, Command::Step(Direction::Forward)) {
    Ok(events) => { /* Vec<Event>, in the order they happened */ }
    Err(rejection) => { /* a Rejection: the rules refused; the world is unchanged */ }
}
let place: Here = here(&world, &data);        // mode, position (a Place), date, may_save, ...
let party: PartyView = party_view(&world, &data);
```

- `apply(&mut World, &Data, Command) -> Result<Vec<Event>, Rejection>`. A command is validated
  in full before anything changes, so a `Rejection` leaves the world and its random streams
  untouched. A `Rejection` is a rule refusal (not your turn, cannot afford, a wall), not an error.
  Bad pack data found mid-command surfaces as `Rejection::Rule`, never a panic.
- `World::new(&Data, seed, Settings)` fails only with `NewGameError::NoEntryMap`.
- `World::to_ron`, `World::from_ron(text, &Data, force)` and `World::fingerprint` save, load
  (migrating older schemas) and hash the world. `Replay::record` and `Replay::check` re-run a
  command list and compare fingerprints.
- The new world's party is empty; create members with `Command::Party(PartyCommand::Create(..))`.
- `Dev` commands are refused (`DevOnly`) unless `Settings.devtools` is true.
- Tier 1 also has `dispatch(&mut World, &Data, &Op) -> Result<Reply, OpError>`, the Tier 2
  protocol in process without JSON; it refuses the six host ops with `HostOnly`.

## 3. Quick start, Tier 2 (JSON)

### Transports

| Transport | Where | Notes |
|---|---|---|
| Dev socket | The game binary, feature `devtools` (on in debug builds, compiled out of release) | TCP on loopback (`127.0.0.1:0` by default, any free port). The bound address is written to `.omnis/dev.addr` in the working directory. Newline-delimited JSON: one request per line, one reply per line. A line longer than 1 MiB (`MAX_LINE`) drops the client. One client at a time; a second is told `another client is connected` and closed. No auth beyond loopback. |
| `omnis_cli::Headless` | In process, Rust | `Headless::new(packs, seed)` then `handle(&Op) -> Result<Reply, OpError>`. No window: `screenshot` and `screen.text` answer `Failed`. Its world has `devtools` on. |
| MCP bridge `omnis-mcp` | stdio, JSON-RPC 2.0 | One MCP tool per op; the tool name is the op name with `_` for `.` (`game.status` is `game_status`). `omnis-mcp` talks to the running game through `.omnis/dev.addr`; `omnis-mcp --headless [--pack <dir>]... [--seed <n>]` runs a `Headless` world in process. Tool descriptions and input schemas: `crates/omnis-mcp/src/tools.rs`, `schema.rs`. |

Socket envelope: request `{"id": <any JSON>, "op": "<name>", "args": {...}}`; reply
`{"id": <same>, "ok": true, "result": <Reply>}` or `{"id": <same>, "ok": false, "error": <OpError>}`.
`args` is omitted for ops that take none (`{}` or `null` is accepted too). A request that is not
JSON, not an object or not an op is answered with `BadRequest`; one sent before a game is
running is answered `BadRequest` too.

### A real exchange

Captured from a `Headless` world on `packs/base` and `packs/test`, seed 1 (long `Visible` tile
lists shortened with `...`).

```text
> {"id":1,"op":"game.status"}
< {"id":1,"ok":true,"result":{"clock":{"day":0,"elapsed":0,"era":0,"minute":0},"date":{"day":0,"era":0,"minute":0,"minutes":0,"night":true,"year":0},"fingerprint":"3412d45a770d0c6b","groups_cleared":[0,0],"may_save":true,"mode":"Explore","packs":[{"hash":10297398543596196832,"id":"base","version":"0.1.0"},{"hash":9493551322087149983,"id":"test","version":"0.1.0"}],"position":{"facing":"West","map":"test:map:town","x":10,"y":2},"protocol":2,"reply":"status","seed":1,"service":null,"settings":{"devtools":true,"permadeath":false,"save_rule":"Anywhere"},"turn":0}}

> {"id":3,"op":"party.create","args":{"character":{"name":"Wren","race":"base:race:human","class":"base:class:cleric","background":"base:background:acolyte","alignment":"LawfulGood","scores":[15,14,13,12,10,8],"skills":["History","Medicine"]}}}
< {"id":3,"ok":true,"result":{"events":["PartyChanged",{"Visible":{"tiles":[{"depth":0,"offset":0,"x":10,"y":2}, ...]}}],"reply":"events"}}

> {"id":4,"op":"sim.command","args":{"command":{"Turn":"Left"}}}
> {"id":5,"op":"sim.command","args":{"command":{"Step":"Forward"}}}
< {"id":5,"ok":true,"result":{"events":[{"Moved":{"from":{"facing":"South","map":"test:map:town","x":10,"y":2},"to":{"facing":"South","map":"test:map:town","x":10,"y":3}}},{"TimeAdvanced":{"day_rolled":false,"holder":"party:0","minutes":1}},{"Visible":{"tiles":[...]}}],"reply":"events"}}

> {"id":6,"op":"sim.command","args":{"command":{"Encounter":"Attack"}}}
< {"error":{"kind":"Rejected","rejection":"WrongMode"},"id":6,"ok":false}

> {"id":11,"op":"combat.get"}
< {"error":{"kind":"NoEncounter"},"id":11,"ok":false}
```

An attack in a fight, showing roll traces (the first hit in the golden fight replay,
`crates/omnis-sim/tests/replays/fight.ron`):

```json
{"AttackResolved":{"ac":12,"attacker":{"Member":0},"crit":false,"hit":true,
  "roll":{"bonus":null,"face":17,"mode":"Normal","modifier":3,"proficiency":2,"total":22,
          "trace":{"dice":{"count":1,"modifier":0,"sides":20},"rolls":[{"draw":7,"raw":3255901076,"value":17}],"stream":"combat","total":17}},
  "target":{"Monster":{"index":0,"stack":0}}}}
{"Damage":{"adjust":"None","amount":8,"kind":"Slashing","raw":8,
  "rolls":[{"dice":{"count":1,"modifier":0,"sides":8},"rolls":[{"draw":8,"raw":3380988940,"value":5}],"stream":"combat","total":5}],
  "target":{"Monster":{"index":0,"stack":0}}}}
```

## 4. Wire forms

All JSON is serde's derived form of the Rust types; field names are the Rust field names.

**Names (protocol 2).** One field name has one meaning, in commands, events, views and
rejections alike (ARCHITECTURE.md §4.9):

1. **A member is named by its `CharacterId`** wherever it appears: `member`, `caster`, `with`,
   `giver`, `receiver`; `{"Member": id}` in `Target`, `ActorRef`, `EffectTarget` and
   `ItemPlace`; `Reorder.order`. No command addresses a marching-order slot, so a command means
   the same member after a reorder. `MemberView.member` is the id to send.
2. **A definition is named by its string id** (`pack:kind:name`; a feature by its name key)
   wherever it appears: `spell`, `item` (an item kind; a kit and the stores count by kind),
   `monster`, `map`, `service`, `condition`, `class`, `feature`; `ActionRef::{Spell, Item}`; the
   criteria's monster and condition. No registry number crosses the protocol. A number no pack
   names shows as `#n`.
3. **A thing with no identity is named by its place in a named list**: `stack` (a stack in the
   fight), `stack` with `index` (one of a stack's living: `{"Monster": {"stack", "index"}}`,
   `SetMonsterHp`), `entry` (a runbook entry; a map's rest event), `rumor` (a tavern's rumor).
   There is no other `index` field, and `row` names only the front or back row (`Predicate::Row`).
   Lists in views are in marching or list order.
4. **A place on a map is a `Place`**, `{"map": "<string id>", "x", "y", "facing"}`, in events,
   views and `Status`.
5. **Left numeric by design:** `era` (a count, not a definition), `ActorRef::Stack` and
   `Monster` (positions), `ViewTile.terrain` and `Known.terrain` (rows of the map's terrains),
   `EncounterSource::Fixed` (a row of the map's placements), `world.query` paths (the saved
   world), and Tier 1's `automap` and `map_text` (which take a `MapId`), `step_lands` (a
   `Position`) and `site_ahead` (a `ServiceId`). The `World`'s own fields keep
   registry numbers; they are not API.

The vocabulary test collects every key that every command, the golden replays' events and every
view put on the wire, and fails when a key carries two JSON types, a definition key is not a
string, a member key holds a number no member has had, or `caster_id`, `at`, a numeric `row` or
a stray `index` appears.

| Type | Form | Example |
|---|---|---|
| `Op` | adjacently tagged, `{"op", "args"}` | `{"op":"events.tail","args":{"count":8}}` |
| `Reply` | internally tagged: the payload's fields plus `"reply": "<snake_case variant>"` | `{"reply":"events","events":[...]}` |
| `OpError` | internally tagged on `kind` (the variant name as written) | `{"kind":"UnknownMap","map":"nope"}` |
| `Command`, `Event`, `Rejection` and their nested enums | externally tagged: `{"Variant": {...}}` for a struct variant, `{"Variant": value}` for a newtype, the bare string for a unit variant | `{"Combat":{"Attack":{"stack":0}}}`, `{"Step":"Forward"}`, `"Interact"`, `"WrongMode"` |
| tuples, `(A, B)` | arrays | `"order":[[{"Stack":0},12],[{"Member":0},8]]` |
| `Option<T>` | `null` or the value | `"service":null` |

Common value types:

| Type | Wire | Meaning |
|---|---|---|
| `Place` | `{"map": "<string id>", "x", "y", "facing"}` | `facing` is `North` (−y), `East` (+x), `South` (+y), `West` (−x). |
| String id | `"pack:kind:name"`, e.g. `"base:spell:bless"` | A definition in the loaded packs; `#n` for a number no pack names. Tier 1 maps ids to registry numbers through `Data::registry`. |
| `CharacterId` | number | A member's stable identity, `MemberView.member`; kept through reorders, deaths and saves. |
| Holder | `"party:0"` or a region's string id | Who owns a clock, in events and `TimeView`. |
| `Direction` | `Forward`, `Back`, `Left`, `Right` | Relative to the facing; `Left`/`Right` sidestep. |
| `Rotation` | `Left`, `Right`, `Around` | |
| `RollTrace` | `{"stream", "dice": {"count","sides","modifier"}, "rolls": [{"draw","raw","value"}], "total"}` | One dice roll: the random stream, the draw count and raw value of each die, the face, the total. |
| `Roll` | `{"trace", "mode", "face", "modifier", "proficiency", "bonus", "total"}` | A d20 check or attack. `mode` is `Normal`, `Advantage` or `Disadvantage` (then `trace` holds both dice and `face` is the kept one); `bonus` is a buff die (bless, guidance). |
| `Edges` | number, bits `1` North, `2` East, `4` South, `8` West | Walls or doors on a tile's sides. |
| `Settings` | `{"save_rule", "permadeath", "devtools"}` | `save_rule` is `Anywhere`, `Relief` (inns and granted saves) or `InnOnly`. |
| `ModeKind` | `Explore`, `Encounter`, `Combat`, `Town` | What the party is doing; `Town` means inside a service. |

**Units.** Money is in copper pieces everywhere (100 cp = 1 gp) except `Death.gold`, which is
the whole gold pieces as rolled. Time is in minutes. `company` and `shared` time are per mille.

## 5. Ops

24 ops. "Host" ops touch files, packs or the window; `dispatch` refuses them (`HostOnly`) and
every host (the socket, `Headless`) implements them around the I/O-free rules in `ops.rs`
(`save_text`, `load_text`, `check_reload`, `rules_set`). `BadRequest` can answer any op on the
socket and the bridge (malformed request).

| Op | Args | Reply | Errors | Host |
|---|---|---|---|---|
| `game.status` | none | `status` | `Failed` | |
| `world.query` | `path` (string) | `value` | `TooLong` | |
| `sim.command` | `command` (`Command`) | `events` | `Rejected` | |
| `sim.script` | `commands` (`[Command]`, at most `MAX_SCRIPT`) | `script` | `TooMany` | |
| `events.tail` | `count` (default 32, capped at 4096) | `events` | | |
| `viewport.get` | none | `viewport` | `UnknownMap` | |
| `map.text` | `map` (optional id; the party's map when absent) | `text` | `TooLong`, `UnknownMap` | |
| `automap.get` | `map` (optional id) | `automap` | `TooLong`, `UnknownMap` | |
| `party.get` | none | `party` | | |
| `party.create` | `character` (`Draft`) | `events` | `Rejected` | |
| `combat.get` | none | `combat` | `NoEncounter` | |
| `service.get` | none | `service` | `NoService` | |
| `rest.get` | none | `rest` | | |
| `cast.get` | none | `casts` | | |
| `time.clocks` | none | `time` | | |
| `time.reconcile` | `region` (`pack:region:name`) | `events` | `Rejected` | dev only |
| `rules.list` | none | `rules` | | |
| `rules.get` | `slot` (e.g. `spell_points.pool`) | `rule` | `TooLong`, `UnknownSlot` | |
| `rules.set` | `slot`, `source` | `rule` | `TooLong`, `UnknownSlot`, `BadRequest` (a formula that does not compile) | host |
| `save.write` | `path` (relative `.ron`) | `written` | `TooLong`, `Failed` (bad path, the save rule forbids saving here, I/O) | host |
| `save.read` | `path`, `force` (default false) | `status` | `TooLong`, `Failed` (bad path, unreadable, unknown schema, other packs without `force`, inconsistent save) | host |
| `pack.reload` | none | `done` | `Failed` (packs do not load, or no longer hold the party's tile; nothing changes) | host |
| `screenshot` | `path` (optional `.png`, default `.omnis/screenshot.png`), `target` (`canvas` default, or `window`) | `written` | `TooLong`, `Failed` (headless, bad path) | host, game only |
| `screen.text` | none | `text` | `Failed` (headless) | host, game only |

Notes:

- `sim.command` and `sim.script` are the only ops that change the world, with `party.create`
  (sugar for `Command::Party(PartyCommand::Create(..))`), `time.reconcile` (sugar for
  `Command::Dev(DevCommand::Reconcile { .. })`, so a replay holds it; refused with `DevOnly`
  unless the world's settings allow dev commands), `save.read`, `pack.reload` and `rules.set`.
- `sim.script` stops at the first rejection and still answers `script` with the events so far,
  `applied` and `rejected`; it is not an `OpError`.
- `events.tail` reads `World::log`, the last 4096 events (`LOG_CAPACITY`). The log is not saved:
  it starts empty after `save.read`.
- `world.query` is outside the contract (§1). `value` is `null` when the path does not exist.
- The MCP tool descriptions (`crates/omnis-mcp/src/tools.rs`) say the same for an agent.

## 6. Replies

| Tag (`reply`) | Variant | Payload | Answers |
|---|---|---|---|
| `status` | `Reply::Status(Status)` | `Status`'s fields, flattened | `game.status`, `save.read` |
| `viewport` | `Reply::Viewport(ViewportModel)` | `ViewportModel`'s fields, flattened | `viewport.get` |
| `script` | `Reply::Script` | `applied` (count), `events` (`[Event]`), `rejected` (`Rejection` or null) | `sim.script` |
| `events` | `Reply::Events` | `events` (`[Event]`) | `sim.command`, `party.create`, `time.reconcile`, `events.tail` |
| `automap` | `Reply::Automap` | `map` (string id), `tiles` (`[KnownTile]`, in `(x, y)` order: column by column) | `automap.get` |
| `text` | `Reply::Text` | `text` (string) | `map.text`, `screen.text` |
| `written` | `Reply::Written` | `path` (string) | `save.write`, `screenshot` |
| `party` | `Reply::Party` | `party` (`PartyView`) | `party.get` |
| `rules` | `Reply::Rules` | `rules` (`RulesView`: `slots` `[SlotView]`, `values` map name → integer, `tables` map name → `[integer]`) | `rules.list` |
| `rule` | `Reply::Rule` | `rule` (`SlotView`: `name`, `inputs` `[string]`, `source`) | `rules.get`, `rules.set` |
| `combat` | `Reply::Combat` | `combat` (`CombatView`) | `combat.get` |
| `service` | `Reply::Service` | `view` (`ServiceView`) | `service.get` |
| `time` | `Reply::Time` | `time` (`TimeView`) | `time.clocks` |
| `rest` | `Reply::Rest` | `rest` (`RestView`) | `rest.get` |
| `casts` | `Reply::Casts` | `casts` (`[CastView]`) | `cast.get` |
| `value` | `Reply::Value` | `value` (string or null) | `world.query` |
| `done` | `Reply::Done` | nothing | `pack.reload` |

`KnownTile` is `{"x", "y", "known": Known}` (see §7.9). `map.text` renders the map in the pack
map layout (`2h+1` lines of `2w+1` characters): walls `-` `|`, closed doors `=` `:`, open doors
`_` `'`, a portal `*`, the party `^` `>` `v` `<`.

## 7. Views

Every view is a pure read: **views never mutate the world and never consume a die**. A view
that quotes a price or a cost rolls on a copy of the stream. Tier 1 functions take
`(&World, &Data)` unless noted. Views name members by `CharacterId`, definitions by string id
and places by `Place` (§4); lists are in marching or list order.

| Function | Returns | Op |
|---|---|---|
| `here` | `Here` | (inside `game.status`) |
| `status` | `Result<Status, OpError>` | `game.status` |
| `party_view` | `PartyView` | `party.get` |
| `combat_view` | `Option<CombatView>` (none outside an encounter or fight) | `combat.get` |
| `service_view` | `Option<ServiceView>` (none outside a service) | `service.get` |
| `rest_view` | `RestView` | `rest.get` |
| `time_view` | `TimeView` | `time.clocks` |
| `cast_view` | `Vec<CastView>` | `cast.get` |
| `viewport` | `Option<ViewportModel>` (none when the party's map is not loaded) | `viewport.get` |
| `automap(&World, MapId)` | `Option<&BTreeMap<(u16, u16), Known>>` | `automap.get` |
| `map_text(&World, &Data, MapId)` | `Option<String>` | `map.text` |
| `flags` | `Vec<(String, i64)>`, every declared flag with its value (0 when never set) | none |
| `step_lands(&World, &Data, Direction)` | `Option<Position>`: where a step would leave the party (through a portal, at its far end); a Tier 1 `Position` with its `MapId` | none |
| `site_ahead(&World, &Data, Direction)` | `Option<ServiceId>`: the service a step would enter (a registry number) | none |

`automap`, `map_text`, `step_lands` and `site_ahead` are Tier 1 only and keep registry numbers;
the ops answer with string ids.

### 7.1 `Here` and `Status`

`Here`, the cheap read a client makes every frame:

| Field | Meaning |
|---|---|
| `mode` | `ModeKind`. |
| `turn` | Commands applied. |
| `position` | `Place`. |
| `service` | The service id the party is inside, or null. |
| `age` | The party's age in minutes; never reverses. |
| `date` | `DateView`, the date the party believes. |
| `may_save` | Whether the save rule allows a save here. |
| `seed` | The world seed. |
| `settings` | `Settings`, fixed when the game started. |

`Status` (`game.status`): `protocol` (`PROTOCOL`), `mode`, `turn`, `position` (`Place`), `clock`
(`ClockView`: `elapsed` minutes since the party's origin, `day`, `minute` of the day, `era`),
`date`, `packs` (`[PackFingerprint]`: `id`, `version`, `hash`), `fingerprint` (the world hash
as 16 hex digits; serializes the world, so not a per-frame read), `service`, `groups_cleared`
(`[cleared, total]` of the map's `once` encounter groups), `may_save`, `seed`, `settings`.

### 7.2 `PartyView`, `MemberView`, `ItemView`, `EffectView`

`PartyView`:

| Field | Meaning |
|---|---|
| `members` | `[MemberView]` in marching order. |
| `slots` | Party slots the rules allow. |
| `front_row` | How many members stand in the front row. |
| `gold` | Purse, copper. |
| `gems` | Gems; never written (components are items in the stores), kept for the shape. |
| `food` | Food units in the stores. |
| `inventory` | The stores, `[ItemView]`, in the order they were filled. |
| `effects` | `[EffectView]` on the whole party (light). |
| `bank` | Copper in the bank. |
| `last_long_rest` | Party clock `elapsed` when the last long rest ended, or null. |
| `long_rest_wait` | Minutes before a long rest may begin; 0 when it may. |

`MemberView`:

| Field | Meaning |
|---|---|
| `member` | `CharacterId`: what every command and event names the member by. |
| `name`, `race`, `class`, `background` | Display name; race, class and background ids. |
| `alignment` | e.g. `LawfulGood`. |
| `level`, `xp`, `next_xp` | Level, experience, experience the next level needs (null at the top). |
| `ready` | Experience has reached a level no trainer has granted yet. |
| `hp`, `hp_max` | Hit points. |
| `spell_points`, `spell_points_max` | Spell points (this variant's casting pool). |
| `ac` | Armor class. |
| `scores`, `modifiers` | The six ability scores and their modifiers, SRD order (Str, Dex, Con, Int, Wis, Cha). |
| `proficiency` | Proficiency bonus. |
| `saves` | `[[Ability, bonus]]`, the class's saving throws. |
| `skills` | `[[Skill, bonus]]`, proficient skills, SRD order. |
| `casting` | The casting ability, or null. |
| `in_front` | In the front row. |
| `conditions` | Condition ids in effect. |
| `down`, `dead` | At zero hit points; dead (carries the pack's `dead` condition). |
| `death_saves` | `{"successes", "failures", "stable"}`. |
| `spells` | Known spell ids, in the order the member learned them; the `spell` that `Cast` takes. |
| `equipment` | The kit, `[ItemView]`, in the order it was filled. |
| `worn` | `[[EquipSlot, item id]]`; slots `MainHand`, `OffHand`, `Ranged`, `Body`. |
| `effects` | `[EffectView]` on the member. |
| `hit_dice`, `hit_dice_left`, `hit_die` | Hit dice in all (one per level), left for short rests, faces of the class's die. |
| `spell_picks` | Spells owed by levels, chosen at a trainer. |
| `tactics` | `TacticsView`. |
| `age_years` | Age at creation plus the years the party has lived since. |

`ItemView` (one item kind in a kit or the stores): `item` (the item id, the `item` the item and
service commands take), `name` (text key), `count`, `slot` (`EquipSlot` or null), `usable` (Use
does something), `consumable` (a use spends one), `equipped` (worn or wielded by the kit's
owner; always false in the stores).

`EffectView`: `spell` (id), `caster` (`CharacterId`), `minutes_left` (party-clock minutes, or
null when it ends at the bearer's next turn).

### 7.3 `TacticsView`, `ReactionView`, `AnswerView`

`TacticsView`: `reactions_on` (the in-fight switch), `auto` (the runbook takes the member's
turns; stored, not built), `reactions` (`[ReactionView]`, tried in order), `answers`
(`[AnswerView]`, what could be declared: the weapon and each known spell).

`ReactionView`: `entry` (its entry in the default runbook, the `entry` that `PutReaction` and
`RemoveReaction` take), `name` (the criteria set's name), `action` (`ActionRef`: `"Attack"`,
`{"Spell": "<spell id>"}`, `{"Item": "<item id>"}`, `{"Feature": "<name key>"}`), `action_name`
(the action's id; `attack` for the weapon), `trigger` (`Trigger`), `when` (`Criteria`:
`"Always"`, `{"All": [..]}`, `{"Any": [..]}`, `{"Is": Predicate}`).

`Predicate` (in `omnis-rules/src/tactics.rs`): `MonsterCount { monster, cmp, n }`,
`MonsterShare { monster, cmp, percent }` (`monster` a monster id), `Hp { who, cmp, percent }`,
`SpellPoints { who, cmp, percent }`, `HasCondition { who, condition }` (a condition id),
`Row { who, row }` (`Front` or `Back`), `Round { cmp, n }`, `WouldChangeOutcome`. `cmp` is
`Lt`, `Le`, `Eq`, `Ge`, `Gt`; `who` is `Me` or `Subject` (the member the trigger is about).

`AnswerView`: `action` (`ActionRef`), `name` (the action's id), `triggers` (`[Trigger]` it can
answer).

`Trigger` is one of `SpellCast`, `Attacked`, `MemberAttacked`, `MemberWounded`, `MemberDying`,
`EnemyFlees`, `EnemyCasts`, `OwnTurn` (see §14 for which are live).

### 7.4 `CombatView`, `StackView`, `SpellView`, `FeatureView`, `FighterView`, `ChoiceKind`

`CombatView` (the encounter before the choice, or the fight):

| Field | Meaning |
|---|---|
| `phase` | `Encounter` or `Combat`. |
| `source` | `EncounterSource`: `{"Fixed": n}` (the placement's position in the map file), `Random`, `Ambush` (at rest). |
| `disposition` | `Hostile`, `Wary`, `Neutral`, `Friendly`. |
| `stacks` | `[StackView]`, in encounter order. |
| `retreat` | `Place` that Run and Flee put the party on. |
| `round` | From 1; 0 before the fight. |
| `current` | `ActorRef` whose turn it is, or null. |
| `order` | `[[ActorRef, initiative total]]`, highest first. |
| `surprised` | `None`, `Party`, `Monsters`. |
| `dodging`, `hidden` | `[CharacterId]` dodging this round; hidden (advantage on the next attack). |
| `gold` | Copper looted so far. |
| `front_row`, `monster_front_stacks` | Members in the front row; stacks in front. |
| `spells` | The acting member's `[SpellView]`; empty when no member acts. |
| `budget` | `{"actions", "bonus_actions"}` the acting member's turn has left. |
| `reactions` | `[[ActorRef, left]]` reactions left this round. |
| `members` | `[FighterView]` in marching order; empty before the fight. |
| `bribe` | Copper a bribe would cost, before the fight, when the monsters take one. |

`ActorRef` is `{"Member": CharacterId}`, `{"Stack": n}` or `{"Monster": {"stack", "index"}}`
(`index` among the stack's living at that moment).

`StackView`: `stack` (the stack's position in the encounter, the `stack` that `Attack` and
`Target::Stack` take), `monster` (id), `name` (text key), `initial` (count at the start), `hps`
(`[hp]` of the living), `ac`, `in_front`, `alive`, `reachable` (the acting member can reach
it), `points_left` (spell points per living caster; empty for non-casters), `shielded` (the
living whose Shield is up, by their place among the living), `refusal` (`Rejection` the acting
member would get attacking it, or null).

`SpellView`: `spell` (the spell id, the `spell` that `Cast` takes), `name` (text key), `level`
(0 cantrip), `cost` (points), `reach` (`One`, `Stack`, `AllStacks`), `targets_members`,
`blocked` (`Rejection` for paying with the action, or null), `bonus` (`Rejection` for paying
with the bonus action, or null).

`FighterView`: `member` (`CharacterId`), `reactions_on`, `features` (`[FeatureView]`).

`FeatureView`: `feature` (the feature's name key, the `feature` that `Feature` takes),
`pay` (`Action`, `BonusAction`, `Reaction`, `Free`), `uses_left` (null at will), `blocked`
(`Rejection` or null; `NotYourTurn` off the member's turn), `choices` (`[ChoiceKind]`).

`ChoiceKind`: `Plain` (send `FeatureChoice::None`), `Exchange` (send
`FeatureChoice::Exchange { with }`, `with` a `CharacterId`), `Hide` (send
`FeatureChoice::Hide`).

### 7.5 `ServiceView`, `OfferView`

`ServiceView`: `service` (id), `name` (text key), `kind` (`Inn`, `Temple`, `Trainer`, `Smith`,
`Tavern`, `Bank`, `Guild`), `gold`, `bank` (copper), `food`, `offers` (`[OfferView]`, leaving
last).

`OfferView`: `command` (the exact `ServiceCommand` that asks for it, ready to send; counts are
1), `member` (`CharacterId` at the temple, the trainer and for spells, or null), `price`
(copper, 0 when free; null for a sale or when refused before a price), `pays` (copper a sale
brings), `refusal` (`Rejection` or null), `subject` (the item or spell id the command names, or
null). A smith offers each stocked item to buy and each item kind in the stores to sell; a
temple heals, cures and raises each member; a trainer trains each member and offers each spell
pick; a guild or temple offers its spells to each member. A bank lists only leaving; deposits
and withdrawals are sent with an amount.

### 7.6 `RestView`, `CampMember`

`RestView`: `refusal` (why no rest may begin here: inside a service, in a fight; or null),
`members` (`[CampMember]` in marching order), `long_food` (food a long rest eats), `food` (food
in the stores), `long_refusal` (why the long rest would be refused, or null).

`CampMember`: `hp`, `hp_max`, `hit_dice_left`, `hit_dice` (one per level), `die` (faces),
`spendable` (hit dice a short rest may spend now; 0 when dead or at full hit points). It has
no member field: the list is in the order of `PartyView.members`.

### 7.7 `TimeView`, `DateView`, `HolderClock`, `ContactView`

Time is subjective per holder (ARCHITECTURE.md §4.4): there is no global clock.

`TimeView`: `age` (the party's age, minutes), `shared` (its shared time: minutes lived ×
the region's company, over 1000), `date` (`DateView`), `clocks` (`[HolderClock]`, the party's
first), `contacts` (`[ContactView]`).

`DateView`: `minutes` (on the calendar), `year`, `day` (of the year, from 0), `minute` (of the
day), `night`, `era`.

`HolderClock`: `holder` (`party:0` or a region id), `elapsed` (minutes since its origin), `era`.

`ContactView`: `holder`, `other`, `self_elapsed` and `other_elapsed` (both clocks when they last
met).

### 7.8 `CastView`

One entry per spell a member may cast outside a fight, in marching order, then the order the
member learned them: `caster` (`CharacterId`, `Command::Cast`'s `caster`), `spell` (the spell
id, `Command::Cast`'s `spell`), `name` (text key), `cost` (points, 0 for a cantrip),
`targets_members`, `refusal` (`Rejection` or null).

### 7.9 `ViewportModel`, `ViewTile`, `EdgeView`, `Known`, `layer`

`ViewportModel` (what the renderer draws, ARCHITECTURE.md §8.3): `map` (the map's string id),
`tileset` (its tileset's string id), `facing`, `detail_depth` (rows drawn with sprites),
`visibility_depth` (rows visible at all), `tiles` (`[ViewTile]`, nearest first).

`ViewTile`: `depth` (tiles ahead), `offset` (tiles right of the facing line; negative is left),
`x`, `y` (map cell), `terrain` (a number into the map's terrains; §4), `front`, `left`, `right`
(`EdgeView`: `"Open"`, `"Wall"`, `{"Door": {"open": bool}}`, as seen from the party).

`Known` (one automap tile): `terrain`, `walls` and `doors` (`Edges` bits), `layers` (`layer`
bits), `seen_at` (party clock when last seen; the automap shows what was seen and when, not what
is). `layer::TERRAIN` = 1, `layer::STRUCTURE` = 2 (walls and doors), `layer::VISITED` = 4,
`layer::REMOTE` = 8 (the latest knowledge came from afar; a direct sighting clears it).

## 8. Commands

`Command` (11 variants). Commands name members by `CharacterId` (`MemberView.member`) and
definitions by string id (a feature by its name key); a stack is named by its position in the
encounter (`stack`). No command addresses a marching-order slot or a list row, so a command
means the same thing after a reorder or a change to a list.

| Variant | Fields | Meaning |
|---|---|---|
| `Step` | `Direction` | Move one tile relative to the facing, without turning. |
| `Turn` | `Rotation` | Turn in place. |
| `Interact` | | Use the facing edge or tile: a door, a site (enters its service). |
| `Party` | `PartyCommand` | Build, reorder or set tactics. |
| `Encounter` | `EncounterChoice` | Choose what to do about the monsters ahead. |
| `Combat` | `CombatCommand` | Act on the acting member's turn. |
| `Cast` | `caster` (`CharacterId`), `spell` (spell id; the caster must know it), `target` (`Target`) | Cast outside a fight: healing, a buff, light, mage hand (`cast.get` lists what may be cast). `target` is ignored by light and mage hand. |
| `Item` | `ItemCommand` | Equip, unequip, give, stow, take or use outside a fight. |
| `Service` | `ServiceCommand` | Act inside a service. |
| `Rest` | `RestCommand` | Rest outside a service. |
| `Dev` | `DevCommand` | A debugging edit; refused with `DevOnly` unless `Settings.devtools`. |

`PartyCommand`: `Create(Draft)` (a `Draft` is `name`, `race`, `class`, `background` ids,
`alignment`, `scores` (six point-buy scores, SRD order), `skills` (picked from the class list)),
`Reorder { order }` (`[CharacterId]`: every member once, front first),
`Tactics(TacticsCommand)`.

`TacticsCommand`: `SetReactions { member, on }` (the switch; any time),
`PutReaction { member, entry, set }` (declare a `CriteriaSet` `{name, action, trigger, when}`
naming spells, items, monsters and conditions by string id; replaces runbook entry `entry`, or
appends when null), `RemoveReaction { member, entry }` (the library keeps the set).

`EncounterChoice`: `Attack` (fight), `Bribe` (pay `CombatView.bribe`; free for a friendly
group), `Hide` (Stealth against their passive Perception), `Run` (back to `retreat` on
Dexterity).

`CombatCommand`: `Attack { stack }` (the best weapon that reaches it),
`Cast { spell, target, pay }` (`spell` a known spell's id; `pay` defaults to `Action`),
`Use { item, receiver }` (an item id in the acting member's kit, a potion; `receiver` a
`CharacterId`, or null for the user), `Dodge`, `Exchange { with }` (swap marching places with
member `with`), `Run` (the whole party tries to flee), `Feature { feature, choice }` (`feature`
the name key `FeatureView.feature` shows; `choice` defaults to `None`), `EndTurn`.

`Target`: `{"Stack": n}` or `{"Member": CharacterId}`. `Pay`: `Action`, `BonusAction`.
`FeatureChoice`: `None`, `Exchange { with }` (Cunning Action's swap, `with` a `CharacterId`),
`Hide`.

`ItemCommand` (`member`, `giver`, `receiver` are `CharacterId`s; `item` an item id; a kit and
the stores count by kind): `Equip { member, item }`,
`Unequip { member, slot }` (`EquipSlot`), `Give { giver, receiver, item, count }` (kit to kit),
`Stow { member, item, count }` (kit to stores), `Take { member, item, count }` (stores to kit),
`Use { member, item, receiver }` (`receiver` null for the user).

`ServiceCommand` (`member` a `CharacterId`; `item` and `spell` string ids): `Leave` (any),
`Room` (inn: a long rest), `Rumor` (tavern), `BuyFood { count }` (tavern), `Heal { member }`,
`Cure { member }`, `Raise { member }` (temple), `Buy { item, count }` (smith: an item it
stocks), `Sell { item, count }` (smith: an item in the stores), `Deposit { amount }`,
`Withdraw { amount }` (bank, copper), `Train { member }` (trainer: the next level),
`Choose { member, spell }` (trainer: a pick from the member's class list, free),
`Learn { member, spell }` (guild or temple: a spell it teaches, bought).

`RestCommand`: `Short { spend }` (an hour; `spend` is `[HitDiceSpend]`, each
`{"member": CharacterId, "count"}`; members not named spend none, a member named twice is
`MemberTwice`; the dice are rolled in marching order), `Long` (the night; eats food; once per
24 hours).

`DevCommand` (13 variants; every one is raised back as `Event::Dev`):

| Variant | Fields | Meaning |
|---|---|---|
| `GiveItem` | `member` (`CharacterId`, or null for the stores), `item` (id), `count` | Add items. |
| `SetHp` | `member`, `hp` | Hit points; 0 downs, above 0 clears down and dead. |
| `SetSpellPoints` | `member`, `points` | Spell points, may pass the maximum. |
| `SetGold` | `gold` | The purse, copper. |
| `SetFood` | `food` | Food units. |
| `SetXp` | `member`, `xp` | Experience; levels reached are granted at once, free. |
| `SetScore` | `member`, `ability`, `score` | One ability score, any `u8` (Con and mental scores recompute their pools). |
| `SetCondition` | `member`, `condition` (id), `applied` | A condition on or off, raw. |
| `SetFlag` | `flag` (id), `value` | A world flag. |
| `Teleport` | `map` (id), `x`, `y`, `facing` | Explore only; no time passes, no encounter triggers. |
| `SetMonsterHp` | `stack`, `index` (among the stack's living, as in `ActorRef::Monster`), `hp` | Fights only; 0 removes it without gold. |
| `KillStack` | `stack` | Fights only; the stack dies without gold. |
| `Reconcile` | `region` (id) | Outside a fight: meet a region as on entering it (`time.reconcile`). |

**Script words.** A typing aid for the CLI's `play --script`, the app's dev script and tests
(`word.rs`). `parse_script(text) -> Result<Vec<Word>, ScriptError>` reads a text script: words
separated by whitespace or commas, `#` starts a comment; it checks each word's syntax only.
Words count as the player sees the screen: members by marching-order slot, spells, items and
features by their position in the list shown. `Word::command(&World, &Data) -> Option<Command>`
resolves a word against the world when it is applied, into a command that names identities and
string ids: a slot means whoever stands there at that moment, a position whatever that list
holds then; `None` when the slot or position is empty or the word does not fit the world's
state. The words: `forward` `back` `left` `right` `turn-left` `turn-right` `around` `use`;
before a fight `fight` `bribe` `hide` `run`; in one `attack` (the first stack) `attack-N`
(stack `N`) `cast-N-M` (the acting member's known spell `N` at stack `M`) `cast-N-mM` (at the
member in slot `M`), either with `-bonus`, `use-item-N` (kit item `N`, on the user)
`use-item-N-mM` `dodge` `swap-N` `flee` `feature-F` `feature-F-W` (Cunning Action's swap with
slot `W`) `feature-F-hide` `end`; any time `react-M-on` `react-M-off`; in a service `leave`
`room` `rumor` `food-N` `heal-M` `cure-M` `raise-M` `buy-R[-N]` (stock item `R`) `sell-R[-N]`
(stores item `R`) `deposit-N` `withdraw-N` (copper) `train-M` `choose-M-R` (class-list spell
`R`) `learn-M-R` (the service's spell `R`); outside one `rest` `short-rest`
`short-rest-A-B-…` (hit dice per member in marching order). Party creation, items, `Cast`
outside a fight and `Dev` have no words. `Command::word` (`command.rs`) gives a command's bare
verb for logs (`party`, `item`, `cast`, `dev` for those without a word of their own).

## 9. Events

`Event` has 62 variants. Events carry ids, numbers and roll traces, never text: a member is its
`CharacterId`, a definition its string id, a place a `Place`. Most commands end with a
`Visible` event. Groups as in ARCHITECTURE.md §4.2.

### Exploration and time

| Event | Fields | Meaning |
|---|---|---|
| `Moved` | `from`, `to` (`Place`) | The party moved, possibly through a portal. |
| `Blocked` | `reason` (`Wall`, `ClosedDoor`, `Impassable`, `MapEdge`) | A step did not happen; the turn was taken. |
| `Visible` | `tiles` (`[{x, y, depth, offset}]`) | What the party perceives after the command, nearest first. |
| `Door` | `map` (id), `x`, `y`, `facing`, `open` | A door on the party's tile, on edge `facing`, changed state. |
| `Message` | `key` (`MessageKey`: `NothingHere`) | A message for the player; text under `sim:message:<name>`. |
| `PartyChanged` | | Members or their order changed. |
| `TimeAdvanced` | `holder` (`"party:0"` or a region id), `minutes`, `day_rolled` | A clock advanced. |
| `Reconciled` | `a`, `b` (holders, named as `holder`), `delta_a`, `delta_b` (minutes), `era_b` | Two holders met; `b` caught up by `delta_b` for the `delta_a` that `a` lived. |
| `SignalsDropped` | `count` | The signal bus dropped signals past its depth or budget. |

### Encounters

| Event | Fields | Meaning |
|---|---|---|
| `EncounterCheck` | `roll` (d100 `RollTrace`), `chance`, `fired` | A step rolled for a random encounter. |
| `EncounterStarted` | `source`, `stacks` (`[[monster id, count]]`), `disposition`, `counts` (`[RollTrace]`), `stealth` (`Roll` or null), `perception`, `noticed` | Monsters stand before the party; unnoticed means the party is surprised. |
| `Check` | `actor` (`ActorRef`), `kind` (`Stealth`, `Hide`, `Run`, `Flee`, `{"Save": Ability}`), `roll` (`Roll` or null), `dc`, `success` | A check against a difficulty. |
| `Bribed` | `cost` (copper) | The monsters took the money and left. |

### Fights

| Event | Fields | Meaning |
|---|---|---|
| `CombatStarted` | `surprised` | The fight is on. |
| `Initiative` | `order` (`[[ActorRef, total]]`), `rolls` (`[RollTrace]`: members in marching order, then stacks) | Initiative order, highest first. |
| `RoundStarted` | `round` | A round began (from 1). |
| `Turn` | `actor` | A turn began; on a member's turn the engine waits for a command. |
| `Waited` | `actor` | A stack could do nothing from where it stands. |
| `Dodging` | `actor` | A member dodges until the round ends. |
| `Exchanged` | `member`, `with` (`CharacterId`s) | The acting member and `with` swapped marching places. |
| `AttackResolved` | `attacker`, `target` (`ActorRef`), `roll` (`Roll`), `ac`, `hit`, `crit` | An attack roll. |
| `Damage` | `target` (`ActorRef`), `kind` (`DamageType`), `rolls`, `raw`, `amount`, `adjust` (`None`, `Resisted`, `Vulnerable`, `Immune`) | Damage dealt, before and after defenses. |
| `Down` | `target` (`CharacterId`) | A member fell to 0 hit points. |
| `Wounded` | `member`, `failures` | Damage at 0 hit points: a failed death save (two on a crit). |
| `DeathSave` | `member`, `roll` (`RollTrace`), `result` (`Success`, `Failure`, `Stable`, `Revived`, `Died`), `successes`, `failures` | A death saving throw at the end of a round. |
| `Condition` | `target` (`ActorRef`), `condition` (id), `applied` | A condition came or went. |
| `Death` | `target` (`ActorRef`), `gold` (`RollTrace` of whole gold pieces, or null) | A combatant died; the gold is looted as copper. |
| `CombatEnded` | `outcome` (`Victory`, `Fled`, `Defeat`), `xp` (each survivor), `gold` (copper), `fallen` (`[CharacterId]` removed by permadeath) | The fight is over. |
| `FeatureUsed` | `member`, `feature` (name key) | A class feature was used; its effect follows. |
| `OpportunityAttack` | `stack`, `member` | A front stack swings at a member leaving; attack events follow. |
| `MonsterCast` | `caster` (`ActorRef`), `spell` (id) | A monster cast; what it did follows. |
| `ShieldStops` | `target` (`ActorRef`) | Shield stopped a Magic Missile. |
| `ReactionsSwitched` | `member`, `on` | A member's reactions switch changed. |
| `TacticsChanged` | `member` | A member's declared reactions changed. |
| `Reaction` | `actor` (`CharacterId`), `trigger`, `action` (`ActionRef`, by string ids) | A declared reaction fired; its events follow. |

### Casting

| Event | Fields | Meaning |
|---|---|---|
| `SpellCast` | `caster` (`CharacterId`), `spell` (id), `points`, `components_consumed` (`[[item id, count]]`) | A member cast; what it did follows. |
| `Healed` | `target` (`CharacterId`), `rolls`, `amount` (before the cap), `hp` (after) | Hit points regained. |
| `EffectApplied` | `target` (`{"Member": CharacterId}` or `"Party"`), `spell` (id), `caster` | An effect settled. |
| `EffectEnded` | `target`, `spell`, `why` (`Expired`, `Consumed`, `Concentration`, `TurnBegan`, `FightOver`) | An effect ended. |
| `Concentration` | `caster`, `spell`, `ended` (always true) | Concentration ended. |

### Items and sensing

| Event | Fields | Meaning |
|---|---|---|
| `Equipped` | `member`, `slot`, `item` (id) | Worn or wielded. |
| `Unequipped` | `member`, `slot`, `item` | Taken off, or it left the kit. |
| `ItemMoved` | `item`, `count`, `from`, `to` (`{"Member": CharacterId}` or `"Stores"`) | Items moved. |
| `ItemUsed` | `member`, `item`, `receiver` (`CharacterId` or null), `consumed` | An item was used; what it did follows. |
| `Sensed` | `actor` (`CharacterId`), `item`, `checks` (`[{layer, roll, reach}]`), `tiles` (`[{x, y, layers}]`) | A member looked from afar (a spyglass); the automap now carries the tiles as remote. |

### Town, rest and progression

| Event | Fields | Meaning |
|---|---|---|
| `ServiceEntered` | `service` (id) | The party went into the service on its tile. |
| `ServiceLeft` | `service` | The party came out. |
| `RoomTaken` | `cost` | A night at the inn (a long rest); `Healed` follows. |
| `FoodBought` | `count`, `cost` | Food into the larder. |
| `Rumor` | `service`, `rumor` (the rumor's position in the tavern's rumors, file order), `ago` (minutes on the region's clock) | A rumor heard. |
| `Treated` | `member`, `cost` | A temple healed or cured; `Healed` and `Condition` follow. |
| `Raised` | `member`, `cost` | A temple raised the dead at 1 hit point. |
| `Bought` | `item` (id), `count`, `cost` | Into the stores. |
| `Sold` | `item`, `count`, `price` | Out of the stores. |
| `Banked` | `amount`, `deposit` | Copper between purse and bank. |
| `Rested` | `long`, `minutes`, `food` | A rest ran its course; restoration follows. |
| `HitDiceSpent` | `member`, `dice` | Hit dice spent on a short rest; `Healed` follows. |
| `RestInterrupted` | `minutes` | Monsters came upon the resting party; the encounter follows; nothing restored. |
| `RestEvent` | `map` (id), `entry` (its position in the map's rest events, file order) | A rest event happened; it changes nothing yet. |
| `LevelUp` | `member`, `level`, `cost`, `gains` (`{hp, spell_points, picks, proficiency, features}`) | A trainer (or `SetXp`, cost 0) granted a level. |
| `SpellLearned` | `member`, `spell` (id), `cost` | A spell went onto the member's list (a pick costs 0). |

### Dev

| Event | Fields | Meaning |
|---|---|---|
| `Dev` | `command` (`DevCommand`) | A debugging edit was applied; what it caused follows. |

## 10. Rejections and errors

`Rejection` (62 variants) is a rule refusal; the world is unchanged. Over Tier 2 it arrives as
`{"kind":"Rejected","rejection": ...}`, in `Reply::Script.rejected`, or inside a view
(`refusal`, `blocked`, `bonus`, `long_refusal`). A rejection names members by `CharacterId`
(`member`), definitions by string id (`spell`, `item`, `feature`, `id`) and a stack by
`stack`.

| Rejection | Fields | Meaning |
|---|---|---|
| `WrongMode` | | The command does not apply in the current mode. |
| `PartyFull` | | Every party slot is taken. |
| `Character` | `CreationError` | The draft does not make a character (`Name`, `UnknownRace`, `UnknownClass`, `UnknownBackground`, `ScoreRange`, `Points`, `Skills`, `Rule`). |
| `BadOrder` | | The order does not list every member once. |
| `NotYourTurn` | | The fight is not waiting on a (living) member. |
| `NoSuchStack` | `stack` | No stack has that number. |
| `StackDead` | `stack` | Nobody in that stack still stands. |
| `OutOfReach` | `stack` | A front-row member without a ranged weapon cannot reach a stack behind the front. |
| `NeedsRangedWeapon` | | A back-row member needs a ranged weapon to attack. |
| `NoSuchMember` | `member` | No member has that identity. |
| `SameMember` | | A member cannot exchange with themselves. |
| `MemberTwice` | `member` | A list names the same member twice (a short rest's hit dice). |
| `CannotAfford` | `cost`, `gold` | The party cannot pay (copper). |
| `UnknownSpell` | `spell` | The caster knows no spell by that id. |
| `NotCastable` | `spell` | The spell has no effect the engine can cast here yet. |
| `NotEnoughPoints` | `need`, `have` | The caster's pool is short. |
| `MissingComponents` | `spell` | The components are not in the stores. |
| `WrongTarget` | | A stack for a helping spell, or a member for a harmful one. |
| `MemberDead` | `member` | The member is dead. |
| `MemberDown` | `member` | The member is at 0 hit points and cannot act. |
| `NotEnough` | `item`, `have` | Fewer of an item than needed. |
| `UnknownItem` | `item` | The kit holds no item of that id. |
| `NotInStores` | `item` | The stores hold no item of that id. |
| `NotCarried` | | The member does not carry the item. |
| `NotEquippable` | | The item has no slot. |
| `HandsFull` | | A two-handed weapon and a shield cannot both be held. |
| `SlotEmpty` | `slot` | Nothing is in that slot. |
| `NotUsable` | | The item does nothing when used. |
| `NotUsableHere` | | The item is not used from a fight. |
| `TargetDead` | `member` | The member the item goes to is dead. |
| `ZeroCount` | | A count of zero moves nothing. |
| `DevOnly` | | A `Dev` command in a world whose settings do not allow them. |
| `UnknownId` | `id` | No loaded pack defines that id. |
| `OutOfRange` | | A number out of range: a `GiveItem` count of zero, a total that would overflow (food, purse, bank, stores), an individual past the end of its stack (`SetMonsterHp`). |
| `OffMap` | `x`, `y` | The tile is not on the map. |
| `NotOffered` | | This service does not do that, or does not stock that item. |
| `NothingToTreat` | `member` | Full hit points, or no condition to cure. |
| `NotDead` | `member` | Only the dead are raised. |
| `BankShort` | `amount`, `bank` | The bank holds less than the withdrawal (copper). |
| `RestTooSoon` | `minutes` | The last long rest ended too recently (`minutes` until allowed). |
| `NoFood` | `need`, `have` | Less food than a long rest eats. |
| `NoHitDice` | `member`, `left` | The member has fewer hit dice left than asked. |
| `NotReady` | `member`, `xp`, `needed` | Experience has not reached the next level. |
| `MaxLevel` | `member` | The member is at the highest level. |
| `NoPicks` | `member` | No spell picks left to choose. |
| `NoSuchSpell` | `spell` | The spell is not on the list the command names (the class list, the service's spells). |
| `NotOnList` | `member` | The spell is not on the member's class list. |
| `CantripNotLearned` | | Cantrips come with the class; none is picked or bought. |
| `SpellTooHigh` | `level`, `max` | Above the highest level the member may learn. |
| `AlreadyKnown` | `member` | The spell is already on the member's list. |
| `NoActionLeft` | | The turn's action is spent. |
| `NoBonusActionLeft` | | The turn's bonus action is spent. |
| `ReactionOnly` | | That costs a reaction; only a declared reaction pays for it. |
| `NotABonusAction` | `spell` | The spell cannot be paid with the bonus action. |
| `NeedsPreparation` | `spell` | The spell takes the bonus action only once readied; readying is not built. |
| `NoSuchFeature` | `feature` | The member has no feature with effect by that name key. |
| `NoUsesLeft` | `feature` | The feature's uses are spent until a rest. |
| `WrongChoice` | `feature` | The feature does not do what was asked. |
| `Tactics` | `TacticsFault` | The tactics' shape is refused (`Name`, `TooDeep`, `TooMany`, `Percent`, `NoSuchSet`, `NoDefault`). |
| `CannotReact` | | No such reaction, or it cannot answer that trigger. |
| `NoSuchEntry` | `entry` | The default runbook has no entry there. |
| `Rule` | `RuleError` | A rule formula failed while resolving: bad pack data, reported instead of a panic. |

`OpError` (10 kinds). The world is unchanged unless the message says otherwise.

| Kind | Fields | Meaning |
|---|---|---|
| `Rejected` | `rejection` | The rules refused the command. |
| `UnknownMap` | `map` (the id given) | No such map is loaded. |
| `UnknownSlot` | `slot` | No rule slot has that name. |
| `TooLong` | `limit` (bytes) | A string argument is over `MAX_STRING_BYTES`. |
| `TooMany` | `limit` | A script is over `MAX_SCRIPT` commands. |
| `HostOnly` | | Sent to `dispatch`; the op needs a host. |
| `NoEncounter` | | No encounter or fight is in progress. |
| `NoService` | | The party is not inside a service. |
| `BadRequest` | `message` | Malformed request: not JSON, not an op, a line too long, another client connected, no game running; also a `rules.set` that does not compile. |
| `Failed` | `message` | The host or the data layer failed; the message (English) says how. |

## 11. Limits

| Limit | Value | Where |
|---|---|---|
| `MAX_SCRIPT` | 10,000 commands per `sim.script` | `omnis-sim/src/ops.rs` |
| `MAX_LINE` | 1 MiB per request line on the dev socket; a longer line drops the client | `omnis-app/src/socket.rs` |
| `omnis_data::limits::MAX_STRING_BYTES` | 4 KiB per string argument (`TooLong`) | `omnis-data/src/limits.rs` |
| `MAX_FILE_BYTES` | 4 MiB per file read (saves included); symlinks refused | `omnis-data/src/limits.rs`, `ron_io::read_text` |
| `LOG_CAPACITY` | 4096 events kept for `events.tail` | `omnis-sim/src/lib.rs` |
| Paths | Relative; only ASCII letters, digits, `.`, `_`, `-`, `/`; no `.` or `..` components; the extension checked (`.ron` for saves, `.png` for screenshots) | `ops::client_path` |

Every argument from a client is untrusted (ARCHITECTURE.md §6.2) and bounded before it is
looked at.

## 12. Versions

| Version | Value | Covers | Moves when |
|---|---|---|---|
| `ops::PROTOCOL` | 2 | Both API tiers | A rename, a removal, or a change of meaning or units (§13). |
| `SAVE_SCHEMA` | 7 | Save files (`World::to_ron`) | The saved world's shape changes; older schemas (1 to 6) migrate on load. |
| `omnis_data::SCHEMA` | 1 | Pack data files | The pack file format changes; the loader refuses other schemas. |
| Pack fingerprints | `PackFingerprint { id, version, hash }` per pack, in `Status.packs` | The content a world runs on | Any edit to a pack's data or text files (FNV-1a 64 over them). A save made on other packs is refused (`save.read` `Failed`) unless `force`. |

The three versions move independently.

## 13. Stability rules

From ARCHITECTURE.md §4.9:

- **Additions do not bump `PROTOCOL`**: a new op, a new optional field (`serde(default)`), a new
  `Event`, `Command`, `Rejection` or `Reply` variant.
- **Clients must ignore** fields they do not know and variants they do not handle (an unknown
  event is skipped, an unknown rejection shown generically).
- **A rename, a removal, or a change of meaning or units bumps `PROTOCOL`**, and the changelog
  below names the change and the migration.
- Saves and pack data are versioned separately (§12).

## 14. Known gaps

As of protocol 2:

- **No pack definitions or text over Tier 2.** No op exposes pack data or the `text/` tables.
  Commands, events and views name definitions by string id and carry text keys
  (`base:text:item.mace.name`), not display text, and no op turns an id or a key into a label or
  a definition (the planned `data.*` ops). `ViewTile.terrain` indexes terrains no op lists. Only
  Tier 1 has `Data`.
- **Tier 1 reads without an op**: `flags`, `step_lands`, `site_ahead`. `here` is reachable only
  inside `game.status`, which also serializes the world for its fingerprint.
- **No push.** Clients poll (`events.tail`); events from a command go to whoever sent it. The
  event log is not saved, so it is empty after `save.read`.
- **No sliced long ops.** Every op answers within one frame; ops that take long (an ecosystem
  tick, M10) and their progress reports are not built.
- **`world.query` is debug-only** and outside the contract.
- **Later systems.** Region events (M10, `omnis-eco`) and quest events (M11, `omnis-story`) do
  not exist yet, nor the ops `pack.validate`, `eco.region`, `eco.tick`, `story.state`,
  `story.check`, `editor.*`.
- **Triggers.** `SpellCast` and `EnemyCasts` are raised, but no action can answer them (only
  reaction spells answer, and only `Attacked`, `MemberAttacked`, `MemberWounded`,
  `MemberDying`). `EnemyFlees` and `OwnTurn` have no source; `TacticsView.auto` is stored, not
  built. `Surprise::Monsters` is reserved and never produced.
- **Stubs.** `RestEvent` changes nothing yet. `MessageKey` has one key, `NothingHere`.
- **Transport.** The dev socket has no authentication beyond loopback, one client, and exists
  only in `devtools` builds. `Failed` and `BadRequest` messages are English text, not keys.
- **No reply schemas.** Inputs have hand-written JSON Schemas in `omnis-mcp/src/schema.rs`,
  proven against the `Command` type; replies have none beyond this document.

## Changelog

### Protocol 2 (2026-10-10)

One meaning per field name (§4; owner, 2026-10-08: "identities everywhere"). Under protocol 1,
casting Bless with `{"Cast":{"caster":2,"spell":3,...}}` raised `{"SpellCast":{"caster":2,
"spell":1}}`: in the command `caster` was a marching-order slot and `spell` a row of the
caster's list, in the event an identity and a registry number, and in the view a string id.
Saves (`SAVE_SCHEMA` 7) and replays are unchanged: the world keeps its registry numbers and
the wire types convert at the boundary.

| Where | Protocol 1 | Protocol 2 |
|---|---|---|
| Every command, rejection and view naming a member (`member`, `caster`, `with`, `Target::Member`, `Reorder.order`, `NoSuchMember`, the member refusals, `FighterView`, `OfferView.member`, `CastView.caster`) | Marching-order slot (`index`, `member`) | `CharacterId`; the member refusals' `index` and `FighterView.index` are `member`; `MemberView.index` and `CastView.caster_id` removed |
| `MemberView.id` | `CharacterId` | `member` |
| `ItemCommand::Give { from, to }` | slots | `{ giver, receiver }`, ids |
| `ItemCommand::Use.target`, `CombatCommand::Use.target`, `ItemUsed.target` | slot / id | `receiver`, id |
| `RestCommand::Short { dice }` | `[count]` by slot | `{ spend: [{member, count}] }`; `MemberTwice` new |
| `Cast.spell`, `CombatCommand::Cast.spell`, `Choose.spell`, `Learn.spell` | row of a list | spell id |
| Item commands' `item`, `Buy.item`, `Sell.item`, `Take.item` | row of the kit, stock or stores | item id (a kit and the stores count by kind) |
| `CombatCommand::Feature.feature` | row | the feature's name key |
| `PutReaction.at`, `RemoveReaction.at`, `ReactionView.index` | `at`, `index` | `entry` |
| `PutReaction.set` (`CriteriaSet`), `ReactionView.action`, `Reaction.action` | `ActionRef` and criteria with registry numbers | string ids |
| `NoSuchSpell.row`, `UnknownSpell`, `NotEnough.item`, the spell and feature refusals | rows, registry numbers | `spell`, `item`, `feature` ids; `NoSuchEntry { entry }` |
| `SpellView.{index,id}`, `FeatureView.{index,name}`, `ItemView.{index,id}`, `CastView.{spell,id}`, `OfferView.row` | a row, beside the id (a feature's `name`) | rows removed; the id is `SpellView.spell`, `FeatureView.feature`, `ItemView.item`, `CastView.spell`, the ids commands take |
| Every event field naming a spell, item, monster, condition, service or map (`SpellCast`, `EffectApplied`, `EncounterStarted.stacks`, `components_consumed`, `Bought`, `Condition`, `Door.map`, ...) | registry number | string id |
| `Moved.{from,to}`, `Status.position`, `Here.position`, `CombatView.retreat` | `Position` with a `MapId` | `Place` with the map's string id; `Status.map` and `Here.map` removed |
| `ViewportModel.map`, `ViewportModel.tileset` | registry numbers | string ids |
| `TimeAdvanced.holder`, `Reconciled.{a,b}` | `HolderId` (`{"Party":0}`) | `"party:0"` or the region's id |
| `Exchanged { a, b }` | slots | `{ member, with }`, ids |
| `Rumor.index`, `RestEvent.index` | `index` | `rumor`, `entry` |
| `StackView.index` | `index` | `stack` |
| `StackView.hp`, `StackView.front`, `MemberView.front` | `hp`, `front` | `hps`, `in_front` |
| `MemberView.equipped` | `[[slot, item]]` | `worn` |
| `FeatureView.cost` | `Cost` | `pay` |
| `CampMember.dice`, `CampMember.dice_left` | counts | `hit_dice`, `hit_dice_left` |
| `RestView.long` | `Rejection` or null | `long_refusal` |
| `Reply::Service.service` | `ServiceView` | `view` |
| A die in a `RollTrace` | `{index, raw, value}` | `{draw, raw, value}` |

**Migration.** Read members' ids from `party.get` (`MemberView.member`) and send them wherever a
member is named; send the string ids the views list (`MemberView.spells`, `ItemView.item`,
`SpellView.spell`, `FeatureView.feature`, `OfferView.command`) instead of list rows. Read
places from `position.map` and events' `Place`s. Script words (`cast-1-2`, `heal-0`) still count
slots and rows as the player sees them, and resolve against the world when applied.

### Protocol 1 (2026-10-07)

The baseline, established in M8 step 8:

- `ops::PROTOCOL` = 1, reported as `Status.protocol` and in the MCP server instructions.
- `Reply` is tagged inside (`"reply": "<snake_case>"`), so a client decodes a reply without
  knowing the op it sent.
- New ops `rest.get` (`RestView`) and `cast.get` (`[CastView]`), bringing the ops to 24.
- `Status` gained `protocol`, `may_save`, `seed` and `settings`; `query::here` (`Here`) is the
  cheap per-frame read `Status` is built from.
- `MemberView.effects` and `PartyView.effects` became `[EffectView]` rows (`spell`, `caster`,
  `minutes_left`) instead of spell ids. `MemberView` gained `background`, `alignment`,
  `age_years`, `proficiency`, `next_xp`, `hit_die`, `saves`, `skills`, `casting` and
  `modifiers`.
- `StackView.refusal`, `CombatView.bribe`, `FeatureView.choices` (`ChoiceKind`) and
  `OfferView.subject` added.
- The host ops' rules (`save_text`, `load_text`, `check_reload`, `rules_set`) live once in
  `ops.rs`; every host reads saves with the loader's limits (no symlinks, `MAX_FILE_BYTES`).
- `omnis_sim::api` re-exports exactly the Tier 1 contract, including `EdgeView`, `ViewTile` and the types inside replies (`ClockView`, `KnownTile`, `SlotView`, `RulesView`, `ShotTarget`, `Edges`).
- `rules.set` with an unknown slot answers `UnknownSlot`, as `rules.get` does (it answered `BadRequest`; B5).
