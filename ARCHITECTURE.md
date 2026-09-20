# Omnis — Architecture

| | |
|---|---|
| Status | Draft v0.3 (2026-09-20: matches M6 as built; turn budget and tactics as designed; Feathers experiment), reviewed by owner item by item; derived from `PRD.md` v0.5 |
| Date | 2026-09-11 |
| Owner | john@gatewaynode.com |
| Scope | How the system is built. What and why live in `PRD.md`. |

Every section cites the PRD goal, decision, or constraint it serves. When this document and the PRD disagree, the PRD wins and this document is wrong.

---

## 1. Principles

These follow from PRD goals 2, 3, 7 and constraints §11, and from the owner's direction that rules will be tweaked heavily and features will keep arriving.

1. **The simulation is a library.** Rules, world state, combat, exploration, ecosystem, and story run as pure Rust with no Bevy, no I/O, no wall clock, no threads. Input is a command; output is a list of events. Everything else (renderer, editor, MCP, CLI, tests, future co-op) is a client of that library.
2. **Everything is data, including formulas.** Content is RON. Rule formulas are Rhai expressions in RON, evaluated by a strictly sandboxed engine. Changing a number or a formula never requires a rebuild.
3. **Events are the only way out.** The simulation never calls a client. Clients read the event log and query the world. This is what makes replay, tests, MCP, and co-op the same problem.
4. **Boundaries are crates.** Cargo enforces the dependency direction; a crate that must not know Bevy cannot import it.
5. **Thin adapters at every edge.** Bevy is wrapped once, in one crate. MCP is a separate binary that speaks a private protocol to the game. Both can be replaced without touching the simulation.
6. **Untrusted input everywhere.** Packs, saves, socket messages, and expressions are validated with limits before use (PRD §11.3, R8).
7. **Small files, small crates.** Files at or under 1000 lines; a crate does one thing (`CLAUDE.md`).

## 2. System overview

```mermaid
flowchart LR
  subgraph clients [Clients]
    APP[omnis-app<br/>Bevy: viewport, UI, editor]
    CLI[omnis-cli<br/>headless: validate, gen, play scripts]
    MCP[omnis-mcp<br/>stdio MCP bridge]
    TEST[tests]
  end
  subgraph sim [Simulation library, no Bevy]
    SIM[omnis-sim<br/>World, Command, Event, apply, query]
    RULES[omnis-rules]
    GEN[omnis-gen]
    ECO[omnis-eco]
    STORY[omnis-story]
    DATA[omnis-data<br/>schemas, packs, loader, validator]
    EXPR[omnis-expr]
    CORE[omnis-core<br/>ids, int math, rng, calendar]
  end
  APP --> SIM
  CLI --> SIM
  TEST --> SIM
  MCP -. localhost socket .-> APP
  MCP -. in-process headless .-> CLI
  SIM --> RULES & GEN & ECO & STORY
  RULES & GEN & ECO & STORY --> DATA
  DATA --> EXPR
  RULES & ECO & STORY --> EXPR
  DATA & EXPR & SIM --> CORE
```

The game loop, in words: a client turns player input into a `Command`; `omnis-sim::apply` mutates the `World` and returns `Vec<Event>`; the client renders the events and the new state. The MCP bridge is just another client that sends commands and queries over a socket.

## 3. Workspace and crates

Cargo workspace at the repository root. Crates under `crates/`. Names use the `omnis-` prefix; binary names are `omnis`, `omnis-cli`, `omnis-mcp`.

| Crate | Kind | Purpose | Allowed dependencies |
|---|---|---|---|
| `omnis-core` | lib | Typed IDs, integer and fixed-point math, dice, deterministic RNG (own PCG32), calendar and time, ordered collection aliases, error type. | `serde` |
| `omnis-expr` | lib | Sandboxed rule scripting host on Rhai: engine profiles, slot compilation and validation, evaluation with named RNG streams, hot swap. | `rhai`, `serde` |
| `omnis-data` | lib | Schema structs for every content type, pack manifest, pack loader, ID registry and interning, validation with limits, schema versions and migrations, RON read and write. | `core`, `expr`, `serde`, `ron` |
| `omnis-rules` | lib | SRD-derived mechanics as pure functions: character build, checks, attacks, saves, damage and conditions, spell points and components, leveling, rest. | `core`, `expr`, `data` |
| `omnis-gen` | lib | Procedural generation layers producing ordinary map and region data. | `core`, `data` |
| `omnis-eco` | lib | Regional ecosystem state, typed region events, daily tick. | `core`, `expr`, `data` |
| `omnis-story` | lib | Quest graphs, quest templates, static completability check, journal. | `core`, `expr`, `data`, `eco` (types only) |
| `omnis-sim` | lib | The orchestrator: `World`, `Command`, `Event`, `apply`, `query`, save and load, replay. Owns exploration, visibility, combat state machine, towns, party, time. | all of the above |
| `omnis-app` | bin `omnis` | Bevy application: presentation, input, audio, editor, pack asset loading, dev socket server. The only crate that imports Bevy. | `sim` and Bevy |
| `omnis-cli` | bin `omnis-cli` | Headless tool: validate packs, generate worlds, render maps as text, run command scripts, replay saves, dump schemas, bake tileset sprites. Also exposes a library so `omnis-mcp` can run headless. | `sim`, `data`, `core`, `png` (M1: the loader and the bake tool need them directly) |
| `omnis-mcp` | bin `omnis-mcp` | MCP bridge: JSON-RPC over stdio to Claude Code, private protocol to the game socket, or in-process headless via `omnis-cli`. | `serde_json`, `omnis-cli` (lib) |

Rules the dependency graph enforces:
- Nothing below `omnis-app` may depend on Bevy. CI greps `Cargo.lock` paths to prove it.
- `omnis-gen`, `omnis-eco`, `omnis-story`, `omnis-rules` are leaves that never import each other, except `story` reading `eco` types. Cross-cutting flows go through `omnis-sim`.
- Only `omnis-data` reads or writes files. Everything else receives loaded data.

## 4. Simulation model (`omnis-sim`)

Serves PRD goal 7 (determinism), D5 (co-op later), and the MCP requirement.

### 4.1 World

```rust
pub struct World {
    pub schema: u32,                 // save schema version
    pub seed: u64,                   // the world seed every RNG stream derives from (A14)
    pub packs: Vec<PackFingerprint>, // id, version, content hash
    pub rngs: BTreeMap<StreamName, Pcg32>, // every named stream used so far, state and draw count, serialized
    pub clocks: BTreeMap<HolderId, Clock>,       // subjective time per holder (§4.4); no global clock
    pub contacts: BTreeMap<(HolderId, HolderId), Contact>, // arrives with M8 (subjective time)
    pub party: Party,                // up to 6 members (D10), gold, gems, food, inventory
    pub position: Position,          // map, tile, facing
    pub maps: BTreeMap<MapId, MapState>,   // mutable per-map state; static tiles come from data
    pub regions: BTreeMap<RegionId, RegionState>,          // arrives with the ecosystem (M10)
    pub automap: Automap,            // per-map known tiles with layer bits and seen-at time
    pub quests: QuestState,          // arrives with the story engine (M11)
    pub mode: Mode,                  // Explore | Combat(CombatState) | Town(ServiceState) | ...
    pub flags: BTreeMap<FlagId, i64>,
    pub settings: Settings,          // save rule (anywhere, relief, inn only), permadeath (D17), devtools
    pub turn: u64,                   // commands applied so far
    pub log: Vec<Event>,             // recent events for polling clients; not saved, not fingerprinted
}
```

- All collections are ordered (`BTreeMap`, `Vec`); iteration order is part of determinism.
- All numbers are integers. Ratios use `core::Fixed` (i64 with 1/1000 scale) where needed. No `f32` or `f64` anywhere in the simulation crates; CI denies the types with a lint.
- The world is `serde::Serialize + Deserialize`; a save is the world in RON (D15), optionally compressed on disk.
- `World::fingerprint()` hashes the canonical serialization; equal fingerprints on two platforms is the determinism test.

### 4.2 Command and Event

```rust
pub enum Command {
    Step(Direction),            // forward or back
    Turn(Rotation),
    Interact,                   // door, sign, NPC, trigger on the facing tile
    Rest,
    Party(PartyCommand),        // create, reorder, auto_cast { member, spell, on } (M6: a reaction spell's switch; replaced by Tactics, §4.7)
    Service(ServiceCommand),    // inn, temple, trainer, smith, tavern, bank, guild
    Combat(CombatCommand),      // per actor: attack { stack }, cast { spell, target: Stack | Member }, dodge, exchange { with }, run (M4, M6); use (M6b)
    Encounter(EncounterChoice), // attack, bribe, hide, run (M4)
    Cast { caster, spell, target }, // out of combat (M6): healing, buffs, light, mage hand
    Item(ItemCommand),          // M6b: equip, unequip, give, stow, take, use (a spyglass's use is the sense of PRD §7.2, M6c)
    Journal(JournalCommand),
    Dev(DevCommand),            // M6: give item, set hp/points/gold/food/xp/score/condition/flag, teleport, monster hp, kill stack; accepted only when `Settings.devtools` (§12)
}

pub enum Event {
    Moved { from: Position, to: Position },
    Blocked { reason: BlockReason },
    TimeAdvanced { holder: HolderId, minutes: u32, day_rolled: bool },
    Reconciled { a: HolderId, b: HolderId, delta_a: i64, delta_b: i64, era_b: EraId },
    Visible { tiles: Vec<SeenTile> },        // what the party perceives this turn
    PartyChanged,                            // M3: members or their order changed
    // M4 as built (`omnis-sim/src/event.rs`): integers, ids, and roll traces only.
    EncounterCheck { roll: RollTrace, chance: u8, fired: bool },   // the map's random table, every step
    EncounterStarted { source, stacks: Vec<(MonsterId, u8)>, disposition, counts: Vec<RollTrace>,
                       stealth: Option<Roll>, perception: i64, noticed: bool },
    Check { actor: ActorRef, kind: CheckKind, roll: Option<Roll>, dc: i64, success: bool }, // hide, run, flee
    Bribed { cost: u32 },
    CombatStarted { surprised: Surprise },
    Initiative { order: Vec<(ActorRef, i64)>, rolls: Vec<RollTrace> },
    RoundStarted { round }, Turn { actor }, Waited { actor }, Dodging { actor }, Exchanged { a, b },
    AttackResolved { attacker, target, roll: Roll, ac: i64, hit: bool, crit: bool },
    Damage { target, kind: DamageType, rolls: Vec<RollTrace>, raw: i64, amount: i64, adjust: DamageAdjust },
    Down { target: CharacterId }, Wounded { member, failures }, DeathSave { member, roll, result, successes, failures },
    Condition { target, condition, applied: bool },
    Death { target: ActorRef, gold: Option<RollTrace> },
    CombatEnded { outcome: Victory | Fled | Defeat, xp: u32, gold: u32, fallen: Vec<CharacterId> },
    SpellCast { caster, spell, points, components_consumed },   // M6 as built; then Healed { target, rolls, amount, hp },
    EffectApplied { target: Member | Party, spell, caster }, EffectEnded { target, spell, why }, Concentration { caster, spell, ended },
    AutoCast { member, spell, on }, Check { kind: Save(ability) } for a monster's or a concentrating member's save, Dev { command }
    LevelUp { member, level, gains },
    Region(RegionEvent),                     // from omnis-eco
    Quest(QuestEvent),                       // from omnis-story
    Message { key: TextKey, args: Vec<Arg> }, // localized by clients
    Door { map, x, y, facing, open },        // M1 addition: a door changed state
    Saved, Loaded,
}
```

- `apply(&mut World, &Data, Command) -> Result<Vec<Event>, Rejection>`. A `Rejection` is a rule refusal (not your turn, cannot afford, tile blocked) and is not an error; errors are bugs.
- Every dice roll produces a `RollTrace` in the event so a client can show the math and a test can assert it. A `Roll` (M4) is a d20 with its mode (normal, advantage, disadvantage: both dice in the trace, the kept face named), the modifier, the proficiency bonus, and the total.
- Events are appended to `World::log` (bounded ring in release, unbounded in dev) so MCP `events.tail` and replay work.
- Text never appears in events; only keys and arguments. Clients localize from pack `text/`.

### 4.3 Query

Read-only views for clients: `query::party(&World)`, `query::viewport(&World, &Data) -> ViewportModel` (the tiles within visibility depth, with detail-depth cut, see §8.3), `query::automap(map)`, `query::region(id)`, `query::journal()`, `query::path(&World, "party.members[0].hp")` for MCP's generic inspector. Queries never mutate and never roll dice.

### 4.4 Subjective time

Source: `docs/background/introduction.md`. In Toel, time is local to each traveller and converges only briefly when people meet; the standard greeting is a question about how much time has passed for the other person. This is not flavour on top of a global clock. The architecture has no global clock.

**Temporal holders.** Every entity that experiences time carries its own clock.

```rust
pub struct Clock {
    pub elapsed: i64,          // subjective minutes since the holder's origin
    pub era: EraId,            // which age of the world this holder is living in; one era in v1 content
}
pub struct Contact {
    pub other: HolderId,
    pub self_elapsed: i64,     // my clock when we last met
    pub other_elapsed: i64,    // their clock when we last met
}
pub enum HolderId { Party(PartyId), Region(RegionId), Actor(ActorId), Character(CharacterId), Project(ProjectId) }
```

Holders in v1: the party (one clock shared by its members while they travel together), every region (owning its maps, lairs, respawns, and ecosystem state), every named actor (empty in v1 content, filled by NPC agency later), and characters who are separated from the party (a dismissed hireling waiting at an inn ages on their own clock). Construction projects and other player parties are holders the model already admits (§4.4, "What this buys").

**Advancing.** A command advances only the clock of the holder that acted. A step, a rest, or a service advances the party's clock by data-defined minutes and emits `TimeAdvanced { holder, minutes }`. Nothing else in the world moves. The player's calendar (year, day of 360, minute of 1440; shape is data) is a rendering of the party's own elapsed time, and night is a predicate on it.

**Reconciliation.** When two holders interact, their clocks reconcile, and only somewhat. Interactions are explicit in the simulation: the party enters a region, meets an actor, opens a project, or two parties meet. On interaction between `a` (the initiator) and `b`:

1. Look up the last `Contact` between them, or treat `b` as never met.
2. `delta_a` = how much `a` has experienced since that contact.
3. `delta_b` = `reconcile(delta_a, stability_b, coupling_ab, rng)`, a rule expression in data. The v1 base rule is a ratio around 1 with a jitter drawn from the named RNG stream `time:<a>:<b>`, bounded by the region's temporal stability: a stable town might drift by a few percent, a ruin in flux by months. `delta_b` is never negative in v1.
4. `b` catches up by `delta_b`: a region runs its ecosystem catch-up (§7.3), respawns, and project progress for that much time; an actor runs their agency catch-up (later).
5. Both `Contact` records are updated and the simulation emits `Reconciled { a, b, delta_a, delta_b, era_b }`. The greeting in the background text is a `Message` whose arguments are exactly those two deltas.

Holders that are not part of the interaction do not move. A region the party has not visited for a year of subjective time has experienced nothing yet; it experiences its share when the party returns. This is "the world changes without the player" from PRD goal 4, delivered lazily and deterministically.

**Coupling between non-party holders.** Regions declare couplings in data (a road, a trade route, a river). When a region reconciles with the party, it also reconciles once with each coupled region using the same rule, so migration and trade flow across a neighbourhood without a world-wide tick. Coupling depth is one in v1 and is data.

**Eras.** `EraId` on a clock and `BTreeMap<EraId, RegionState>` on a region let a reconciliation land the party in a different age of the same place: the ruin encountered alive, the kingdom that vanished overnight. In v1 content every holder is in the single era `present` and the reconciliation rule never changes era; the data model and the event carry the field so that content and rules can use it later without a save migration.

**Combat and shared clocks.** During an encounter, all combatants share the encounter's clock; reconciliation happens once at encounter start. Party members share the party clock; a member who leaves becomes a holder with a copy of the party's clock at that moment.

**Determinism.** Reconciliation jitter draws from named RNG streams per holder pair, so an added interaction elsewhere does not perturb it. Catch-up of `delta_b` days is bounded: ecosystem catch-up runs in coarse steps (§7.3), and a cap per reconciliation (data, default 10 years) prevents pathological work.

**What this buys.** NPC agency: an actor is a holder with a clock; their off-screen life is catch-up at contact. Multiplayer: each party is a holder; parties reconcile when they meet, and no global tick has to be agreed between players. Construction and travel: a project is a holder whose progress is its reconciled time times a rate; a long journey is the party's clock advancing, then reconciling on arrival. Aging: a character's age is subjective elapsed time, which is why members separated from the party keep their own clocks.

**What it costs.** Every interaction site in the simulation must call reconcile; forgetting one leaves a holder frozen. The interaction points are enumerated in `omnis-sim::time` and tested: region entry, actor meeting, project inspection, party meeting, encounter start.

### 4.5 Combat state machine

As built in M4. `Mode::Encounter(EncounterState)` holds the stacks (monster, initial count, the living's hit points), their disposition, the source (a placement by index, or the map's random table), and the retreat tile; `Mode::Combat(CombatState)` adds the initiative order, the current actor, the round, who was surprised, who dodges this round, and the gold looted so far. Emptied stacks stay in the list so indices are stable; "front" is the first `monster_front_stacks` living stacks. Transitions: `Explore → (a step onto a placement, or the random table fires) → Encounter → (attack, or a failed hide or run) → Combat → (every stack dead | the party fled | every member down) → Explore`; a bribe or a successful hide or run returns to `Explore` from the encounter, and monsters the party fails to notice (their Stealth against its best passive Perception) skip the choice, the party surprised. Each `CombatCommand` is validated in full against whose turn it is and what the acting member can reach (front-row melee against the front stacks; a ranged weapon anywhere) before the first die is rolled, on a copy of the `combat` stream written back only on success, so a rejection never perturbs the stream. Monster turns resolve inside the same command until a member can act or the fight ends, so one player command may produce many actors' worth of events; death saves and the round's minutes are resolved at the round's end. Permadeath (a setting) removes a dead member at the fight's end with their kit into the party inventory; otherwise the member keeps their slot with the `dead` condition for the temple (M7). A wipe ends the fight with `Defeat` and the app offers the last save (D14). Every number is a rule slot or value in `packs/base/data/rules/combat.ron`; Rust rolls every die, Rhai adds and compares.

**Casting as built in M6a.** A spell's `effect` in data (`SpellEffect`: attack, auto-hit, save, heal, buff, reaction, light, utility) is the shape of what it does and `reach` (one, stack, all stacks) how far; the numbers are the `casting.ron` slots (`spell.save_dc`, `spell.save_damage`, `heal.total`, `cantrip.dice`, `concentration.dc`). `CombatCommand::Cast { spell, target }` names a known spell by its index in the caster's list; validation runs on the roller's copy (a pack's point-cost formula may roll) and refuses before any die (unknown, not castable here, not enough points, missing components at the D11 threshold or in the stores, wrong target, dead or downed member, dead stack). Paying takes points and components (from the party's stores, PRD §8.2) and emits `SpellCast`; cantrips scale their dice by level and cost nothing. Attacks roll the casting modifier plus proficiency against armor class; auto-hits and attacks land on the lead individual; a save spell rolls its damage once and every individual of the stack saves from the last index down. Effects (`ActiveEffect`) live on members and on the party with an absolute party-clock expiry (a round is one minute, so the SRD's "1 minute" is ten) or until the bearer's next turn; the clock prunes them. Bless fans out from the target round the marching order; guidance is spent by the next ability check (Hide, Run, Flee, and later Sense roll through one wrapper); a caster holds one concentration spell and damage asks a Constitution save; shield is a reaction the simulation casts for a member who opted in (`PartyCommand::AutoCast`) when a non-critical hit would land that its bonus turns into a miss (a deviation: the SRD casts it on any hit); light lifts the party's visibility depth to at least its own; mage hand toggles the first door straight ahead. `Command::Cast` casts healing, buffs, light and mage hand outside a fight for `cast_minutes` on the `cast` stream; there is no rest yet (M7), so pools only empty, and the debug menu refills them. Deviations named here and in the spell files: healing word is an action (the one-action turn, PRD §8.3), magic missile's darts all hit the lead, spells ignore rows, guidance takes the next check, bless anchors on a member.

**Items as built in M6b.** `Command::Item(ItemCommand)` while exploring: `Equip`, `Unequip`, `Give`, `Stow`, `Take`, `Use`, every item a row of the kit or the stores it names as they stand when the command is applied (the way `spell` is a row of a caster's list), validated in full before anything changes. The equipment rules (`omnis-rules/src/equip.rs`) decide the slot and the hands; a kit that loses the last of a worn item takes it off with an `Unequipped` event, so a slot never names an item the kit lacks (a save is checked for the same). Armor on or off takes `don_armor_minutes` (doffing costs the same for want of a second value); moving items takes no time; a use takes `use_item_minutes` on the `items` stream. `CombatCommand::Use { item, target }` is the turn's action on the fight's dice. A potion's `Heal` goes through `party::heal`, so a downed member gets up; a `Sense` item is refused in a fight and, until sensing lands, on the road. Events: `Equipped`, `Unequipped`, `ItemMoved` (with `ItemPlace`), `ItemUsed`; the item refusals are their own `Rejection` variants. `party.get` lists every kit as rows with the worn slots, the effects and the stores. In the app the inventory overlay (ITEMS, I) shows a pane per member and one for the stores; the fight's USE action opens an item picker like the spell picker.

**Sensing as built in M6c.** A sense item's `SenseSource` in data (a ray geometry with a range, a fidelity ladder, a skill check, a persistence, minutes) is the shape; the `sense.dc` slot (`sensing.ron`) the numbers: `10 + max(0, distance - visibility) + (layer - 1) * 2`. A use on the road (`Item(Use)`, the LOOK button, the L key) walks `visibility::ray` along the facing line (before a wall or a closed door, after an opaque tile, to the map's edge), then the party's best eyes (the best Perception among members who can act) roll one check per knowledge layer through the one check wrapper; a layer's reach is the farthest tile whose difficulty the total meets, and never past the layer below it (a failed layer stops the ladder, D18). Reached tiles are recorded with `layer::REMOTE` and reported in `Event::Sensed { actor, item, checks, tiles }`; `Automap::record` takes the terrain only when `TERRAIN` is carried and the walls and doors only with `STRUCTURE`, `VISITED` sticks, and any direct sighting clears the mark, so the automap shows what was seen and when (D18: remote knowledge is stale). Objects and creatures join the ladder when the automap carries them; a source without a check reaches the whole ray; a fight refuses a look. The app outlines remotely seen tiles with an inset ring and the bridge compacts a look's tiles to a count.

### 4.6 Replay and co-op readiness

A `Replay` is `(initial World fingerprint, pack fingerprints, Vec<Command>)`. Applying the commands to the same initial world must reproduce the final fingerprint; this is a CI test. Networked co-op later is "share the command stream", which is why commands carry no client-side state.

### 4.7 Turn budget and tactics (designed 2026-09-20, not built)

Serves PRD D21–D24, §7.3, §7.9, §8.3. Through M6 a turn is one `CombatCommand` and shield is the only reaction, cast by the simulation for a member who opted in (`auto_cast`). This section is the design that replaces both; nothing in it exists in code yet.

- **Budget.** At the start of a combatant's turn three rule slots are evaluated, `turn.actions`, `turn.bonus_actions`, `turn.reactions`, over class, level, equipped items and active effects; the base pack returns the SRD's one of each. The result is stored on the combat state as `Budget { actions, bonus_actions, reactions }`. Reactions refresh at the start of the combatant's own turn, as in the SRD.
- **Costs.** Every combat action has a `Cost { Action, BonusAction, Reaction }` from data. A spell carries the three fields of D24 (`bonus_action_available`, `preparation_available`, `preparation_required_for_bonus_action`); when a spell may be paid either way the command says which: `CombatCommand::Cast { spell, target, pay }`. Items and class features carry a cost the same way. What preparing a spell costs and how long it holds is open (PRD §14), so `Prepare` is named here and not specified.
- **A turn is several commands.** A member's turn takes commands until `CombatCommand::EndTurn` or until no action and no bonus action is left; monster turns still resolve inside the command that ends the member's turn (`run_until_member`). Validation refuses a command whose cost the budget cannot pay, before the first die (`Rejection::NoActionLeft` and the like). Every command still rolls on the roller's copy of its stream, written back only on success.
- **Triggers are a closed list** the simulation raises at fixed points of resolution: `SpellCast`, `Attacked`, `MemberAttacked`, `MemberWounded`, `MemberDying`, `EnemyFlees`, `EnemyCasts`, `OwnTurn`. Proximity follows PRD §8.3: the same row for allies, the engaged lead stack for enemies, a stack fleeing or a front-row member running or exchanging out for "leaves your reach" (the opportunity attack), any combatant in the fight for ranged triggers.
- **Data on every combatant** (members now; monsters and hirelings hold the same shape, filled later):
  ```rust
  pub struct Tactics {
      pub reactions_on: bool,            // the in-fight switch; flipping it costs nothing
      pub auto: bool,                    // the runbook takes this member's turns
      pub library: Vec<CriteriaSet>,     // every set built so far, offered again in every runbook
      pub runbooks: Vec<Runbook>,
      pub default_runbook: u8,           // exactly one default
  }
  pub struct CriteriaSet { pub name: String, pub action: ActionRef, pub trigger: Trigger, pub when: Criteria }
  pub enum Criteria { Always, All(Vec<Criteria>), Any(Vec<Criteria>), Is(Predicate) }
  pub enum Predicate {                   // integers only; the system offers these, the player composes them
      MonsterCount { monster: MonsterId, cmp: Cmp, n: u16 },
      MonsterShare { monster: MonsterId, cmp: Cmp, percent: u8 },
      Hp { who: Who, cmp: Cmp, percent: u8 }, SpellPoints { who: Who, cmp: Cmp, percent: u8 },
      HasCondition { who: Who, condition: ConditionId }, Row { who: Who, row: Row }, Round { cmp: Cmp, n: u16 },
      WouldChangeOutcome,                // the M6 shield rule, now the player's choice instead of a built-in
  }
  pub struct Runbook {
      pub name: String,
      pub when: Option<Criteria>,        // encounter criteria; the first matching runbook is used, else the default
      pub entries: Vec<(ActionRef, u16)>, // in order of consideration: an action and the one criteria set evaluated for it
  }
  ```
  Names are player text and are validated as input (length, characters); counts and nesting depth are capped in `limits.rs`. There is no script: a criteria tree is data the simulation walks, so tactics add no attack surface (PRD R8).
- **Resolution.** On a trigger the simulation walks combatants in marching order and, for each with `reactions_on` and a reaction left, the active runbook's entries in order; the first entry whose trigger matches, whose criteria hold and whose cost can be paid is validated and resolved like a command, spends the reaction, and emits `Event::Reaction { actor, trigger, action }` ahead of its own events. One trigger fires at most one reaction per combatant. A member with `auto` set is resolved inside `run_until_member` exactly as a monster is: `tactics::choose(world, data, combatant) -> Option<CombatCommand>` walks the runbook on `OwnTurn` entries until the budget is spent, and falls back to the built-in policy (attack the lead stack) when nothing matches. The monsters' present policy becomes their default runbook through the same function.
- **Commands.** `PartyCommand::Tactics(TacticsCommand)` replaces `AutoCast`: `SetReactions { member, on }` and `SetAuto { member, on }` are accepted at any time in a fight and cost nothing; `PutCriteria`, `RemoveCriteria`, `PutRunbook`, `RemoveRunbook`, `SetDefault` are accepted while exploring. Tactics live in the `World`, so a replay reproduces an automated fight from the command log alone, and the MCP and the CLI drive tactics with no code of their own.
- **Save schema 5.** `Character.tactics` replaces `auto_cast`; the migration turns each auto-cast spell into a criteria set (`Attacked`, `WouldChangeOutcome`) in a default runbook, so an M6 save fights as it did. Both golden replays are rebaselined in that commit.
- **App.** A `TacticsPlugin` and a tactics screen beside the character sheet (reached by a button first), the two switches on the fight's action row, and `Event::Reaction` lines in the roll log. The screen needs dropdowns, number inputs, lists and scrolling; its toolkit is decided by the Feathers experiment (A11 as amended).
- **Measured before content.** `tests/measure.rs` gains budget curves and automated parties, so every change to a `turn.*` slot and every bonus-action spell is compared with the SRD baseline over seeds (PRD R12).

## 5. Rule scripting host (`omnis-expr`, Rhai)

Serves the owner's direction that rules will change a lot, PRD goal 2, and D12 (formula is data). Owner decision (A4, revised 2026-09-12): rule formulas are written in Rhai rather than a hand-rolled language. `omnis-expr` keeps its name and its place in the crate graph; its job changes from implementing a language to hosting one under strict configuration. Facts about Rhai below were verified against its repository, book, and crates.io on 2026-09-12.

### 5.1 What `omnis-expr` owns
- **Engine construction.** One `Engine::new_raw()` per profile, with only the packages we choose registered: arithmetic (unary minus, `abs`) and logic (`!`, `min`, `max`), plus our own `clamp`, `floor_div`, and `d(n, sides)`. Binary integer operators and comparisons are built into the evaluator. `new_raw` starts with no other functions, so nothing else enters the sandbox.
- **Profiles.** `Formula` is the only profile in v1: a single expression per slot, no statements. `Script` is the later profile for quest logic (PRD §13, "scripting for mods") with functions, arrays, and maps enabled and higher limits; it is designed for but not built.
- **Slots and inputs.** Every rule slot declares its input names in `omnis-data`; values are integers or booleans. `omnis-expr` compiles each slot's source to an `AST` at pack load and checks that every variable the AST references is a declared input, by walking the AST (Rhai's `internals` feature) or, if that API proves unstable, by a dry-run evaluation with all inputs bound. Unknown identifiers are pack validation errors with file, line, and column.
- **Evaluation.** `eval(slot, inputs, rng_stream) -> Result<i64 | bool, RuleError>` builds a `Scope` from the inputs, binds the RNG stream for `d`, and runs `eval_ast_with_scope`. A `RuleError` is a bug or bad data, never a rejection; it names the slot and the Rhai error.
- **Hot swap.** `set_slot(slot, source)` recompiles and replaces the AST in place (dev builds, via MCP `rules.set`). ASTs are not serializable, so packs and saves carry source text and every load recompiles.

### 5.2 Build features and sandbox limits
Rhai features enabled for the v1 formula build: `only_i64` (one integer type), `no_float` (no floating point exists in the language), `sync` (ASTs and closures are `Send + Sync`, required because compiled rules live in a Bevy resource), `no_time`, `no_custom_syntax`, `no_function`, `no_closure`, `no_module`, `no_index`, `no_object` (no user functions, closures, imports, arrays, or maps in formulas), and `internals` for the AST walk; `default-features = false` drops `ahash/runtime-rng`. The `unchecked` feature is never enabled: integer overflow and division by zero are runtime errors, not wraps. Enabling the `Script` profile later means removing the last five flags in one rebuild; the `Formula` profile keeps enforcing its subset through `disable_symbol` regardless.

Engine limits per profile:

| Limit | Formula | Script (later) |
|---|---|---|
| `disable_symbol` | `fn`, `loop`, `while`, `for`, `do`, `let`, `const`, `import`, `export`, `eval`, `throw`, `try`, `return`, `switch` | `import`, `export`, `eval` |
| string and character literals | refused by the AST walk at compile time (literals are not symbols, and `set_max_string_size(0)` means unlimited) | allowed, 4 KB |
| `set_max_operations` | 10 000 | 1 000 000 |
| `set_max_expr_depths` | 32 (one argument: `no_function` removes the function-body depth) | (64, 32) |
| `set_max_call_levels` | compiled out by `no_function` | 32 |
| `set_max_array_size`, `set_max_map_size` | compiled out by `no_index`, `no_object` | 4 096, 1 024 |

### 5.3 Determinism
- Integers only, checked arithmetic, `only_i64`: results are identical on every platform. `/` truncates toward zero and `%` follows the sign of the dividend, matching Rust; rule authors are told this in the pack documentation.
- `d(n, sides)` draws from the named RNG stream passed in by the caller (§11), so dice in formulas are deterministic and appear in `RollTrace`.
- Rhai's `Map` is a `BTreeMap`, so map iteration is ordered; it is unavailable in the formula build anyway.
- Rhai randomizes its internal function-lookup hashing seed per process unless `set_hashing_seed` is called; `omnis-expr` sets a fixed seed before the first engine is built. This affects only lookup internals, not script results, and is set for hygiene.

### 5.4 Cost and security
- Performance: Rhai is an AST interpreter, published at roughly two to three times slower than CPython on its own benchmarks. A ten-operation formula costs on the order of a microsecond. Combat with fifty rolls per round is negligible. Ecosystem catch-up is the heavy case: a region catching up a subjective year at fifty coarse ticks and twenty rules per tick is about a thousand evaluations, a few milliseconds. Only regions in contact do this work (§4.4).
- Security (R8): the formula build has no I/O, no modules, no functions, no strings, no time, bounded operations and depth, and checked arithmetic. A mod pack can make a rule slow or wrong; it cannot reach the filesystem, the network, or the process. The `Script` profile, when it comes, keeps the no-I/O guarantee and adds only computation.
- Dependency footprint: `rhai` pulls `smallvec`, `thin-vec`, `ahash`, `num-traits`, `once_cell`, `bitflags`, `smartstring`, and the `rhai_codegen` proc-macro crate. `smartstring` is MPL-2.0; it is file-scoped copyleft, compatible with linking into MIT or Apache-2.0 code, and is listed in the attribution file. Several of these crates are already in Bevy's tree and must resolve to Bevy's versions (§12).

### 5.5 Example rules file (data, not code)
Every `data/rules/*.ron` file has one shape: named formula `slots`, plain integer `values`, and integer `tables`. Files merge into one rule set in id order; a later file's slot, value, or table with the same name replaces the earlier one, so a mod can override one formula. Slots are addressed by name everywhere (`rules.get { slot: "spell_points.pool" }`).
```ron
// packs/base/data/rules/casting.ron
(
    schema: 1,
    id: "base:rules:casting",
    slots: {
        "spell_points.pool": (
            inputs: ["level", "cast_mod", "other_mental_mods", "half_caster"],
            expr: "max(level, (if half_caster { level / 2 } else { level }) * cast_mod + other_mental_mods)",
        ),
        "spell_points.cost": (inputs: ["spell_level"], expr: "spell_level"),
    },
    values: {"component_threshold": 5},
)
```

## 6. Data model and packs (`omnis-data`)

Serves PRD goals 2 and 3, D3, D15, §10, §11.3.

### 6.1 Pack layout
```
packs/<pack_id>/
  pack.ron            # manifest: id, version, name, license, attribution, depends, schema
  data/
    races/*.ron  classes/*.ron  backgrounds/*.ron  spells/*.ron  monsters/*.ron
    items/*.ron  conditions/*.ron  tiles/*.ron  maps/*.ron  regions/*.ron
    quests/*.ron  templates/*.ron  eco/*.ron  rules/*.ron  tables/*.ron
  text/<lang>/*.ron   # TextKey -> string with {arg} placeholders
  assets/
    tilesets/  sprites/  ui/  fonts/  audio/
```
- Every data file is one RON struct with a `schema: u32` field first. Migrations are functions `vN -> vN+1` registered per type; loading an old schema runs them in order and reports it.
- IDs are strings `pack:type:name` in files and are interned to `u32` newtypes (`SpellId`, `MonsterId`, ...) in a `Registry` at load. Unknown references are load errors with the referencing file named.
- Later packs override earlier packs by ID. The manifest declares `depends: ["base >= 1.0"]` and load order is a topological sort; cycles are errors.
- The base campaign is `packs/base`. A mod is any other directory in the user's packs folder. The editor writes the same format.

### 6.2 Validation as untrusted input
Applies to packs, saves, and anything crossing the dev socket.
- Path normalization; any component of `..`, absolute paths, or symlinks out of the pack root is rejected.
- Size limits per file (4 MB text), per pack (256 MB), per map (256×256 tiles), per collection (65 536 entries), per string (4 KB).
- Asset extensions whitelisted; images are decoded with dimension caps before upload.
- RON is parsed with a recursion limit. Expressions have the limits in §5.2.
- Every error is collected, not short-circuited; the report lists all of them with file and line. The game never panics on bad data; it refuses the pack.

### 6.3 Attribution
`pack.ron` carries `attribution: [(source, license, text)]`. The base pack carries the SRD 5.1 CC-BY-4.0 notice (PRD §11.2). The credits screen renders every loaded pack's attribution.

## 7. World, generation, ecosystem, story

### 7.1 Map and world graph
- A **map** is a rectangular tile grid up to 256×256 with a tile type per cell, a wall mask per edge (N, E, S, W), objects, triggers, and a kind (outdoor, town, dungeon, special). MM2's 16×16 is a size, not a rule.
- A **region** is an ecosystem unit that owns one outdoor map and any number of sub-maps (towns, dungeons). Regions tile the world in a grid; region edges link to neighbours.
- Maps link through **portals** (stairs, doors, edges, teleporters), each a typed tile trigger.
- A map or region may be `Materialized(data)` or `Pending { seed, params }`. The simulation materializes on first entry by calling `omnis-gen` and stores the result in the save. Nothing else distinguishes generated from authored content (PRD §9.1).

### 7.2 Generation (`omnis-gen`)
Layered pipeline, each layer a pure function `(seed, params, prior layers) -> layer data`, run from a per-layer sub-seed so re-running one layer does not change the others: terrain → hydrology and roads → settlements → dungeons → encounters and loot → quest hooks. Each layer output is ordinary `omnis-data` structs. A `LockMask` per map marks tiles the editor has hand-edited; regeneration of a layer skips locked tiles. Golden tests pin seed → fingerprint on all CI platforms.

### 7.3 Ecosystem (`omnis-eco`)
Serves PRD §9.2 and the NPC-agency forward compatibility requirement.
```rust
pub struct RegionState {
    pub populations: BTreeMap<CreatureGroupId, u32>,
    pub factions: BTreeMap<FactionId, FactionStanding>,
    pub resources: BTreeMap<ResourceId, u32>,
    pub prosperity: u32, pub danger: u32, pub weather: WeatherState,
    pub actors: BTreeSet<ActorId>,     // named entities present; empty in v1 content
    pub stability: u32,                // temporal stability for reconciliation (§4.4); data
    pub couplings: Vec<RegionId>,       // regions that reconcile alongside this one (§4.4)
}
pub enum RegionEvent { PopulationChanged, FactionShift, ResourceChanged, WeatherChanged, ActorMoved, ActorActed, Derived(Outputs) }
```
- State changes only through `apply_region_event`. Rules in `data/eco/*.ron` are expressions over region inputs that emit events. Player actions that touch a region (clearing a lair, trading) are emitted by `omnis-sim` as region events through the same path.
- `tick(region, neighbours, rules, rng, dt_days)` runs ordered phases: environment, populations, factions, actors (no-op in v1), economy, derived outputs. Adding NPC agency later inserts logic into the actors phase without changing the others.
- Ticks are driven by reconciliation (§4.4), not by a global day. `catch_up(region, delta_days)` runs `tick` with `dt_days` up to a data-defined coarse step (default 7) until the delta is consumed, so a region that receives a year runs about fifty coarse ticks, not 360 fine ones. Rules are written per day and scale by `dt_days`; the golden tests include a fine-versus-coarse equivalence check within tolerance.
- Derived outputs (encounter table weights, price multipliers, rumor keys, generated quest availability) are recomputed each tick and read by `omnis-sim` and `omnis-story`.
- Budget: a region catching up a subjective year is about a thousand rule evaluations, a few milliseconds (§5.4); only regions in contact do work (PRD §9.2).

### 7.4 Story (`omnis-story`)
- **Quest graph**: nodes (`Dialogue`, `Choice`, `Check`, `SetFlag`, `Reward`, `Spawn`, `MapChange`, `End`) with edges guarded by expressions over `{flags, party, party clock, region, quest}`. Stored in `data/quests/*.ron`, edited in the editor's graph view.
- **Static check** at pack load: every node reachable from start reaches an `End`; every `Check` has both outcomes wired; every referenced flag, item, monster, and map exists. Failures are validation errors (PRD §9.3).
- **Templates** in `data/templates/*.ron` declare `requires` (bindings: a town, a dungeon within N regions, a creature group with population above X), `instantiate` (the graph to stamp with bindings substituted), and `effects` (region events on completion). `omnis-sim` asks `omnis-story` for candidate templates in a town each tick and instantiates deterministically from the world RNG.
- The journal is a query over `QuestState`.

## 8. Presentation (`omnis-app`)

Serves D2, D16, PRD §7.2, R2, R10. Bevy facts verified against 0.19.1 sources on 2026-09-11.

### 8.1 Structure
- One Bevy `App` with plugins per concern: `SimPlugin` (owns the `World`, applies commands, publishes events), `InputPlugin` (maps keys, the on-screen pad, and gamepad to `Command`), `CursorPlugin` (window size and pointer as a canvas pixel, from window messages), `ViewportPlugin`, `MenusPlugin` (the menu state machines and their key and click dispatch), `CombatPlugin` (the encounter, fight, and defeat screens, the play state following the world's mode, the roll log), `UiPlugin` (composes the frame: menus, location lines, tool pad, pad, party band; hit-tests the pointer; uploads the frame into a canvas sprite), `PixelPlugin` (the fixed internal resolution pipeline, §8.2), `SheetPlugin` (the character sheet's three pages), `InventoryPlugin` (the inventory overlay; item commands apply from it), `PackAssetPlugin`, and under feature `devtools` `DebugPlugin` (the debug menu, opened from the pause overlay or the backtick), `DevPlugin` (scripted commands and a screenshot from the command line) and `DevSocketPlugin`. Planned, not built: `AudioPlugin`, `EditorPlugin` (M5, deferred), `TacticsPlugin` (PRD §7.9, D22–D23).
- Bevy features: `default-features = false, features = ["2d", "png"]` (the game UI is canvas sprites, so `ui` is off; the editor's `bevy_egui` brings its own rendering), plus `audio` when sound arrives. The `3d` group (pbr, gltf) is never enabled. A `dev` feature enables `bevy/dynamic_linking`, `bevy_dev_tools`, and `file_watcher`; it is never shipped.
- App states (`bevy_state`): `Boot → MainMenu → Playing | Editor`, with `SubStates` under `MainMenu`: `Title | NewGame` (Load is an action on the title) and under `Playing`: `CreateParty | Explore | Encounter | Combat | Paused | Defeat` (M4) and the overlays `Debug | Cast | Sheet | Inventory` (M6), joined by `Service | Journal | Tactics` as their milestones arrive. The play state follows the world's mode after every event batch (`PlayState::for_mode`), so a loaded or resumed game lands in the state its mode calls for; `Defeat` is entered on a wipe and left by a load or by quitting to the title. Commands apply in every play state but `Paused`. `OnEnter` builds each screen and `DespawnOnExit` tears it down. Menus are text lists driven by Bevy-free state machines (`menu.rs`), so every transition is unit-tested; a screen only spawns lines and feeds logical key presses.
- The `World` is a Bevy `Resource` wrapped in `SimWorld` (in 0.19 resources are components on singleton entities; `Res` and `ResMut` are unchanged). Only `SimPlugin` systems mutate it, in one ordered system set in `Update`: `collect commands → apply → push events`. All other systems read events from a buffered `Message` queue (`MessageWriter`/`MessageReader`, 0.19's name for the old buffered events) and read the world through `query::*`. Bevy's ECS holds presentation entities only (sprites, UI nodes, sounds); it never holds game state.
- Simulation events are re-published as Bevy messages one to one; observers (`On<E>`) are used only for presentation-internal triggers (a floating number finished, a menu closed).
- Events drive animation. A `Damage` event spawns a floating number; `Moved` starts a step transition; `Visible` updates the viewport model. Presentation may lag the simulation by an animation queue, but the simulation is never blocked by it.

### 8.2 Pixel pipeline
> **Status (2026-09-20, PRD D26):** this section describes the placeholder presentation as built. A pixel-art look is not a goal and nothing here is kept for its sake; the end state is a modern, resolution-independent presentation. What replaces the raster canvas and the bitmap font is decided by the Feathers experiment (A11 as amended) and the open question in PRD §14. Whatever replaces it must keep what the canvas gives today: a screenshot and screen dumps that show the interface, headless tests that drive every widget, and a button before a key for every action.
- A 720-row canvas as wide as the window (PRD D20): the *core* is the 1280-wide layout the constants in `layout.rs` describe, a 960×540 viewport with a 320-px right column beside it (minimap at 8 px a tile, location lines, a 96×40-button pad), every region an expression of the inputs; `canvas.rs` places it on a canvas of any width (`Layout::for_width`): when a 63-cell *wing* fits left of a centred viewport (width ≥ 1716) the viewport is centred, the wing holds the roster, and the 180-px band under both holds the message line, the event log at the band's first column, and the help line; narrower canvases centre the core with the roster in the band beside the log, which at 1280 is the narrow layout exactly. Text is a self-authored 5×7 bitmap font in 6×8 cells (`font.rs`): the menus paint on an 80×16-cell framed box centred in the viewport (`menu_cell`, so the models' row indices are unchanged), the fight screens on the viewport's 160×67 grid, the band on its 22 rows (`band.rs`: a roster row per member with group, name, class, level, HP, SP, AC, and condition, the acting member's row barred and marked while the mouse's selection keeps its highlighted name, an eighteen-row event log with the roll math). The core's painters keep their narrow coordinates and are painted through the core's origin (`Frame::within`; widgets pushed inside land in canvas space), so the viewport-relative constants and their compile-time asserts hold on every width. The pipeline is Bevy's `pixel_grid_snap` pattern: an inner `Camera2d` rendering to an `Image` target on its own render layer with MSAA off, and an outer camera showing that canvas as a sprite at the largest whole multiple of 720 rows that fits the window's *physical* pixels (`layout::fit` also chooses the canvas width at that multiple; `cursor::WindowSize` carries the logical size and the scale factor, so a 4K panel driven at 2× logical still shows three physical pixels a canvas pixel; the rows are letterboxed to whole pixels, the sprite nudged half a pixel for odd bars); the `Layout` resource follows the fit, and a width change resizes the canvas target and the UI image (`Image::resize`, a new GPU texture) and redraws the viewport sprites at the core's origin. `--window small|medium|large|huge` opens a window of 1280×720, 2560×1440, 3840×2160, or 7680×2160 physical pixels; without it the game is borderless fullscreen on the current monitor (a window loses the menu bar and title bar, so on a 1440-row monitor every window falls to 1×; whole multiples are the rule and fractional scaling a horizon). `ImagePlugin::default_nearest()` for all sampling. All game UI is painted Bevy-free into an RGBA raster of the canvas's size (`raster.rs`, `widget.rs`, `screen.rs`, `screens.rs`, `panels.rs`, `band.rs`, `combat_screen.rs`) that `UiPlugin` composes into a scratch frame and uploads into a sprite above the viewport only when it changed. It is pixel exact at every scale, appears in canvas captures and the MCP screenshot, and its widgets are hit-tested from the pointer mapped through the letterbox (`cursor.rs`), so the whole UI is testable headless. The tilesets are baked for the viewport's size (`omnis-cli tileset bake`; `texel_scale: 4` keeps the 16-pixel tiles' look at four canvas pixels a texel) and a test holds every loaded tileset to `layout::VIEWPORT_SIZE`.
- Asset loading: `omnis-data` loads packs to structs outside Bevy's asset system, because packs are validated data, not assets. A small custom `AssetLoader` (0.19 signature: async `load(reader, settings, load_context)`) handles only pack images and audio by pack-relative path into `Handle<Image>` and atlases. No RON goes through Bevy's asset system; Bevy has no generic RON loader and does not need one here. Missing assets resolve to a generated magenta placeholder (PRD §10).

### 8.3 Viewport contract (D16)
- `query::viewport` returns a `ViewportModel`: a forward cone of tiles up to visibility depth, each with terrain, wall mask, objects, monsters, light, and a `distance`.
- The renderer draws rows `0..detail_depth` (fixed, 4–6) from the tileset's per-depth sprite slots: for each depth `d` and lateral offset `o` in `-(d + 1)..=(d + 1)` (clamped to the tileset's width: the canvas edge at distance `z` lies at offset `0.99 z`, so each row's far end shows one tile beyond the diagonal, and row 0 the tiles beside the party), slots `floor`, `ceiling`, `wall_front`, `wall_left`, `wall_right`, `door`, `object`, `monster`. A tileset declares `detail_depth` and `width`; that is the whole art contract, so art scope is bounded (R10). M1 note: each slot also carries its `x`/`y` position on the tileset's declared `viewport` canvas, so baked and hand-drawn slots place themselves; `omnis-cli tileset bake` generates slots from flat textures (`tasks/TODO.md` M1 review). M1 follow-up: two more slot kinds, `block` (solid terrain such as a pillar: its near face plus the side face toward the party; named by `Terrain.block`) and `door_open` (an open door's frame; named by `MapDef.door_open`), both optional in the data.
- Rows beyond detail depth up to visibility depth are drawn as a horizon band: one column per lateral position, a terrain colour swatch plus optional landmark silhouette sprite, height falling with distance. Procedural, not sprite art.
- Visibility depth per tile comes from the simulation (environment, light, weather, abilities), not from the renderer.

### 8.4 Editor
- Lives in the `Editor` app state in the same binary. Edits `omnis-data` structs in memory and writes RON through `omnis-data`; the loader and the writer are the same code path (PRD §10).
- Views: tile map (paint terrain, edges, objects, triggers, lock mask), region (state and rules), quest graph, data tables, procgen panel (generate, regenerate a layer, lock), text keys, and a playtest button that builds a `World` from the in-memory pack at the cursor tile.
- UI toolkit (A11, amended 2026-09-20: `bevy_egui` stays the editor's standing choice until the in-game Feathers experiment reports; if that goes well the editor's toolkit is reconsidered before the editor starts at the end of Phase 1): `bevy_egui` for the editor only, pinned at 0.41.1 for Bevy 0.19. Immediate-mode tables, property panels, docking, and node-graph widgets make the editor views cheap to build and change. Player-facing HUD and menus are canvas sprites (§8.2), keyboard and mouse driven, so the game keeps its pixel look; `bevy_ui` and Feathers are not used (a correction of 2026-09-12: the M3 screens used `bevy_ui` text at native resolution, which could not be sized to the canvas or captured). `bevy_egui` is confined to `EditorPlugin` so a lag at each Bevy release stalls only the editor build, and the editor can be feature-gated off if a release lags badly (R2).

## 9. Dev socket and MCP (`omnis-app` feature `devtools`, `omnis-mcp`)

Serves the owner's requirement to interact with the live game as it is built. Design follows the decision: own minimal MCP, stdio bridge, private socket protocol.

```mermaid
sequenceDiagram
  participant CC as Claude Code
  participant B as omnis-mcp (stdio)
  participant G as omnis (game, devtools)
  CC->>B: initialize / server.discover
  B-->>CC: capabilities, tools
  CC->>B: tools/call sim.command {Step Forward}
  B->>G: {"id":7,"op":"sim.command","args":{...}}\n
  G-->>B: {"id":7,"ok":true,"result":{"events":[...]}}\n
  B-->>CC: content[text json], structuredContent
```

### 9.1 Game side
- `omnis --dev-socket [addr]` (feature `devtools`, on by default in debug builds, compiled out of release builds) listens on `127.0.0.1:0` by default and writes the bound address to `.omnis/dev.addr` in the working directory; the bridge reads that file. Loopback only, one client at a time, no auth beyond loopback in v1; an optional shared token file is a later addition.
- Transport: TCP, newline-delimited JSON, one request per line, one response per line, `{"id", "op", "args"}` → `{"id", "ok", "result" | "error"}`. No async runtime; a Bevy system polls a non-blocking listener each frame, reads complete lines, dispatches on the main thread with full access to the world, and writes responses. Long operations (tick 1000 days) run in bounded slices across frames and report progress.
- Ops mirror MCP tools one to one, so the bridge is a pure translator.

### 9.2 Bridge
- `omnis-mcp` speaks MCP JSON-RPC 2.0 over stdio, one message per line, stdout only for protocol, logs to stderr, exits on stdin EOF.
- **Dual era** (corrected 2026-09-12 from the 2026-07-28 spec pages *Versioning and Compatibility* and *Discovery*). The modern revision has no handshake: every request carries `_meta` with `io.modelcontextprotocol/protocolVersion` and `io.modelcontextprotocol/clientCapabilities`, servers must implement `server/discover` (result: `supportedVersions`, `capabilities`, `instructions`, and serverInfo under `_meta`; tools are still listed by `tools/list`), every result carries `resultType: "complete"`, and an unsupported version is answered with `-32022` listing the supported ones. A dual-era server picks the era per request: modern `_meta` is served statelessly; an `initialize` request selects legacy semantics for the process. `omnis-mcp` does exactly that, accepting legacy `2025-11-25` and earlier and modern `2026-07-28`. Verified 2026-09-12 from `.omnis/mcp.log` after a restart: Claude Code 2.1.269 speaks the **legacy** era, `initialize` with `protocolVersion` `2025-11-25` (client capabilities `roots`, `elicitation`), then `notifications/initialized` and `tools/list`; it starts the stdio server with the project root as working directory and does **not** expand `${CLAUDE_PROJECT_DIR}` in the `command` field nor export it to the process, so `.mcp.json` runs a `sh -c` launcher that falls back to `$PWD`. The modern path stays in the bridge for the day the client moves.
- Registration: `.mcp.json` at project scope with `{"mcpServers": {"omnis": {"command": "${CLAUDE_PROJECT_DIR}/target/debug/omnis-mcp"}}}`.
- Modes: `omnis-mcp` (connect to the running game via `.omnis/dev.addr`) and `omnis-mcp --headless [pack...]` (run the simulation in-process through `omnis-cli`'s library, no window, for fast rule testing and CI-style checks).

### 9.3 Tool set (v1)
| Tool | Purpose |
|---|---|
| `game.status` | mode, party clock and calendar, position, packs, fingerprint |
| `time.clocks`, `time.reconcile` | list holder clocks and contacts; force a reconciliation between two holders (dev) |
| `world.query` | read any path (`party.members[0].hp`) |
| `party.get`, `party.create` | inspect and build a party from data |
| `sim.command` | apply one `Command`, return events with roll traces |
| `combat.get` | the encounter or fight (M4): stacks with hit points, front or back, and reach for the acting member; the order, the round, whose turn |
| `sim.script` | apply a list of commands |
| `events.tail` | last N events |
| `viewport.get`, `map.text` | the viewport model; a map rendered as text with the party marker |
| `automap.get` | known tiles with layers and seen-at |
| `rules.list`, `rules.get`, `rules.set` | inspect and hot-swap expression slots |
| `pack.validate`, `pack.reload` | run the validator; reload packs into the running game |
| `eco.region`, `eco.tick` | region state; advance N days |
| `story.state`, `story.check` | quest state; run the static check |
| `save.write`, `save.read` | snapshot to and from a path |
| `screenshot` | PNG of the window as image content (game mode only) |
| `editor.*` | later: open map, paint, place, lock |

Every tool has a JSON Schema `inputSchema`. The schemas are hand-written (`omnis-mcp/src/schema.rs`), which keeps a schema generator out of the simulation crates' dependency tree and keeps the descriptions written for the agent that reads them. So that the bridge, the socket, and the docs cannot drift, they are proven against the Rust types by a test: one serialized instance of every `Command` variant, nested variants included, validates against the schema, and an exhaustive `match` fails the build when a variant is added without one (owner decision 2026-09-20; until that test lands, tracked in `tasks/TODO.md`, the guard is arm counts and spot checks).

## 10. CLI (`omnis-cli`)

Headless, Bevy-free, fast to compile. Subcommands: `validate <packs>`, `schema dump`, `gen region --seed --params`, `map text <map>`, `play --script <file>` (runs commands, prints events), `replay <save> <commands>` (asserts fingerprint), `bench eco --regions N --days D`. CI uses it for golden and determinism tests. It exposes `omnis_cli::Headless` for the MCP bridge.

## 11. Determinism and testing

Serves PRD goal 7, §11.1, R6, R9, and `CLAUDE.md` verification rules.
- **RNG and seeds** (A14, approved 2026-09-12):
  - **One world seed**, `u64`, fixed at new game. The app layer offers a text seed (hashed with FNV-1a 64) or draws one from OS entropy; the simulation never touches entropy and only ever receives the number. The seed is shown on the new-game and save screens so worlds can be shared, and it is stored in the save.
  - **Named streams.** Every consumer draws from a stream identified by a canonical name. A stream's initial state is a pure function of the world seed and the name: `state = splitmix64(world_seed ^ fnv1a64(name))`, `increment = splitmix64(fnv1a64(name)) | 1`. Stream names in v1: `party`, `combat` (every die of a fight, and the monsters' Stealth at the trigger), `encounter` (the random table's d100 on every step of a map with one, and its count dice), `time:<a>:<b>` (holder IDs in canonical order), `eco:<region>`, `story:<region>`, `gen:<x>:<y>:<layer>`. Adding a stream never perturbs an existing one; adding a draw inside a stream perturbs that stream's later draws only, and golden tests are re-baselined in the same commit.
  - **Stateful streams are persisted.** `World.rngs: BTreeMap<StreamName, Pcg32>` holds every stream that has been used, with its state and draw count, so a loaded save continues exactly. A stream absent from the map is created on first use from the formula above, which is why lazily generated regions and never-met holders cost nothing until touched.
  - **Generation streams are stateless.** `omnis-gen` derives each layer's stream fresh from `gen:<x>:<y>:<layer>` and never persists it, so regenerating a layer is a pure function of seed, coordinates, and parameters regardless of play history. Locked tiles are re-applied after generation.
  - **Dice in formulas** draw from the stream of the calling subsystem, passed in the evaluation context; a combat roll and an ecosystem roll never share a stream.
  - **Every draw is traceable.** `RollTrace` records stream name, draw index, and raw output; a replay divergence therefore names the first stream and index that differed.
  - **Hashing is our own.** FNV-1a 64 and splitmix64 are a few lines each in `omnis-core`. `std::hash::DefaultHasher` and any randomized hasher are banned from the simulation crates because their output is unspecified across Rust versions and, for randomized hashers, across runs.
  - **Later**: multiplayer shares the world seed and gives each party its own `party:<id>` stream.
- **No floats** in simulation crates, enforced by a CI lint. Ordered collections only.
- **Test tiers**:
  1. Unit tests in each leaf crate on real pack data (`packs/test`), no mocks.
  2. Golden tests: seed → fingerprint for gen and eco; a change must be intentional and re-baselined in the same commit.
  3. Replay tests: recorded command scripts under `tests/replays/` reproduce fingerprints; run on macOS and Linux in CI.
  4. Save round-trip: load every fixture save, save, compare fingerprints; migration fixtures for each schema version.
  5. Pack validation corpus: known-bad packs must be rejected with the expected error list.
  6. App tests with `MinimalPlugins` (no window, no GPU): boot to main menu, start a game, step once, no panics; the menu, party, and fight flows driven by clicks on the composed frame's widgets and by logical keys (`tests/common/mod.rs`); the dev socket over loopback. These are the only tests that touch Bevy.
- Every bug fix ships with a failing-then-passing test (`CLAUDE.md`).

## 12. Security

- Packs, saves, socket input: §6.2 limits and normalization; never panic, always report.
- Scripts from data run only in the Rhai formula profile (§5.2): no I/O, no modules, no functions, no strings, bounded operations and depth, checked arithmetic.
- Dev socket: loopback only, compiled out of release, single client, request size cap (1 MB), op allowlist. `Dev` commands are rejected by `apply` (`Rejection::DevOnly`) unless `Settings.devtools` is set, which dev builds of the app do when a game starts and the headless driver always does; a save records it, and a release build never sets it (M6 as built).
- Saves record pack fingerprints; loading with different packs warns and refuses unless forced.
- Dependencies: pinned exact versions in `[workspace.dependencies]`; `Cargo.lock` committed. Policy, in priority order (owner direction 2026-09-12):
  1. **Match Bevy's requirements.** Any crate Bevy already pulls (ron, serde, and so on) is pinned to the version Bevy 0.19 resolves, so the tree stays single-copy. `cargo tree --duplicates` must be empty for those crates; CI checks it.
  2. **N-1 and older than 30 days** for crates Bevy does not pull, per `CLAUDE.md`. Bevy itself is exempt (D9). Owner-granted exceptions are recorded in the §13 table with the date and reason; rhai 1.26.1 is the first.
  3. **When in doubt, audit with Socket** (`depscore`) before adding or bumping, and default to whatever Bevy requires.

## 13. Dependencies (initial)

Verified against crates.io on 2026-09-11. Policy in §12: match Bevy's resolved versions first, N-1 and 30 days for the rest, Socket audit when in doubt. Socket `depscore` audited on 2026-09-12; every crate scored 100 on license, maintenance, and vulnerability and 93 on quality, except smartstring's license score (70, MPL-2.0) and thin-vec's vulnerability score (84). Supply-chain scores: ron 100, bevy_egui 100, smallvec 100, bitflags 100, once_cell 100, thin-vec 100, rhai 1.26.1 71 (owner-reviewed, see row), serde_json 82, serde 81, ahash 82, num-traits 82, thiserror 79, bevy 74. See the rhai row for the 1.25.x anomaly. Resolved on 2026-09-12 when `omnis-expr` first pulled rhai: rhai_codegen 3.2.0, smartstring 1.0.1, thin-vec 0.2.19, no-std-compat 0.4.1 and spin 0.5.2 (from `sync`), const-random 0.1.18 and tiny-keccak 2.0.2 (ahash's compile-time seed), and a second getrandom 0.2.17, already on the duplicate allow list; ahash, smallvec, num-traits, once_cell, and bitflags resolved to Bevy's copies.

| Crate | Pin | Used by | Note |
|---|---|---|---|
| bevy | 0.19.1 | app | D9 exemption; `default-features = false`, features `2d`, `png` (§8.1; `ui` dropped 2026-09-12 with the canvas UI) |
| png | 0.18.1 | cli | Added 2026-09-12 for `tileset bake`; the version Bevy's image stack resolves, so no duplicate |
| serde | 1.0.228 | all | derive |
| serde_json | 1.0.150 | mcp, app devtools | protocol only |
| ron | 0.12.2 | data | Bevy 0.19 pins `ron = "0.12"`; match its resolved version, single copy in the tree (§12) |
| thiserror | 2.0.19 | core | error derives |
| bevy_egui | 0.41.1 | app (editor only) | A11; 0.42.0 fails the 30-day rule as of writing |
| rhai | 1.26.1 | expr | A4. Owner exception to the N-1 and 30-day rule (2026-09-12): 1.26.1 was released 2026-09-10; the owner reviewed the project and release and judged it not compromised. Context: the N-1 minor, 1.25.x, scores 58 on Socket supply chain against 98 for 1.24.0 and 71 for 1.26.1, with no new dependency, build script, or file-set change to explain it; 1.26.1 also contains fixes to the optimizer, function-pointer argument order, and switch matching found while building the Grain VM. The `grain` feature is not enabled. MSRV 1.66, MIT OR Apache-2.0. Re-audit with Socket at 30 days (2026-10-10). |
| smartstring (via rhai) | Bevy's or rhai's resolved | expr | MPL-2.0, file-scoped copyleft; listed in attribution. Socket license score 70 for that reason. |

Deliberately absent: `rand` (own PCG32), `tokio` (no async), any MCP SDK, any procgen or noise crate (own value noise and hashing in `omnis-gen`). Bevy's MSRV is 1.95; the workspace sets `rust-version = "1.95"`.

## 14. Build and CI

- Workspace `Cargo.toml` with `[workspace.dependencies]` pins, `rust-version = "1.95"`, and profiles: `dev` with `opt-level = 1` for workspace crates and `opt-level = 3` for dependencies; `release` with thin LTO and one codegen unit; `bevy/dynamic_linking` in a `dev` feature for the app only. A `.cargo/config.toml` selects a fast linker where available.
- `just` or plain `cargo` aliases: `cargo run -p omnis-app`, `cargo run -p omnis-cli -- validate packs/base`, `cargo test --workspace`.
- CI (GitHub Actions, macOS and Linux; Linux deferred by owner 2026-09-12 until needed): fmt, clippy with `-D warnings`, no-float lint, `cargo test --workspace`, replay fingerprints compared across the two runners, pack validation of `packs/base`.
- Single-copy is enforced relative to Bevy: `scripts/check-duplicates.sh` compares `cargo tree --duplicates` against a committed allow list of the duplicates Bevy's own tree carries, so only duplicates we introduce fail CI. The simulation lint is `scripts/lint-sim.sh`; `omnis-core`, `omnis-rules`, `omnis-gen`, `omnis-eco`, `omnis-story`, and `omnis-sim` are `#![no_std]` so the compiler also excludes the std modules the lint bans.
- Windows builds in CI once Phase 1 is playable.

## 15. Repository layout

```
omnis/
  Cargo.toml              # workspace
  crates/
    omnis-core/  omnis-expr/  omnis-data/  omnis-rules/  omnis-gen/
    omnis-eco/   omnis-story/ omnis-sim/   omnis-app/    omnis-cli/  omnis-mcp/
  packs/
    base/                 # the campaign, authored in the editor
    test/                 # minimal fixtures for tests
  tests/
    replays/  saves/  packs-bad/
  docs/                   # references (SRD, manual), design notes
  tasks/                  # TODO, LESSONS, BUGS, CONTINUITY
  .mcp.json               # omnis-mcp registration for Claude Code
  PRD.md  ARCHITECTURE.md  README.md  CLAUDE.md
```

## 16. Phase mapping

> **Note (2026-09-12):** build order follows the milestones in `tasks/TODO.md`, which front-load `omnis-app` and `omnis-mcp` so there is a testable build early. The crate-to-phase mapping below stays as the long-range map; revisit once the milestones settle.

| PRD phase | Crates built | Exit test |
|---|---|---|
| 0 Foundation | core, expr, data, sim skeleton, cli, mcp (headless mode) | `packs/test` loads and round-trips; expression tests; MCP `game.status` headless |
| 1 Crawler loop plus editor | rules, sim complete, app (viewport, HUD, menus), devtools socket; editor v1 last (deferred M5, after M8 and before M9), re-authoring the phase's hand-written content | Phase 1 done criteria (PRD §13); MCP drives a full dungeon run |
| 2 Procedural world | gen, lazy materialization, editor procgen panel | 100-region seed traversable; golden fingerprints |
| 3 Ecosystem | eco, region events, derived outputs | lair clearance changes neighbours within a week |
| 4 Story | story, graph editor, templates | static check on the main quest; generated quests in every town |
| 5 Release | base pack content, packaging, docs | external playtest; external mod pack |

## 17. Architecture decisions

| # | Decision | Alternatives | Why |
|---|---|---|---|
| A1 | Cargo workspace of small crates with a Bevy-free simulation | Single crate with modules | Cargo enforces the boundary; faster incremental builds; CLI and MCP headless need the sim without Bevy. |
| A2 | Command in, events out; clients never called by the sim | Callbacks, ECS-driven rules | Replay, tests, MCP, and co-op all reduce to the same stream. |
| A3 | Own minimal MCP bridge over stdio, private newline-JSON socket to the game | rmcp SDK in game or bridge | Owner decision; no async runtime in the game; a few hundred lines we own; dual-era handshake because the spec just broke compatibility. |
| A4 | Rhai, integer-only, sandboxed to a formula profile, hosted by `omnis-expr` | In-house expression language (first draft); formulas in Rust; Lua | Owner decision 2026-09-12: a maintained language with a parser, optimizer, and limits already built beats a hand-rolled one; the formula profile keeps the same bounded, deterministic, no-I/O posture. Later `Script` profile serves quest scripting. |
| A5 | Integers and fixed-point only in the simulation | Floats with care | Cross-platform determinism without doubt (R6). |
| A6 | Own PCG32 with named streams | `rand` | Tiny, serializable, stream-per-subsystem isolation, one fewer dependency. |
| A7 | RON for content and saves | TOML, JSON | D15; native enums and nested structs. |
| A8 | Ecosystem state changes only through typed region events | Direct mutation | NPC agency later inserts as an event source (PRD §9.2). |
| A9 | Two-depth viewport: sprite rows to detail depth, procedural horizon band beyond | Single variable depth | D16; bounded art contract (R10). |
| A10 | Game state lives in one serializable `World`, not in Bevy ECS | ECS for game state | Save, replay, and headless become trivial; Bevy churn (R2) cannot reach game state. |
| A11 | `bevy_egui` for the editor, canvas sprites for the game | Feathers everywhere; egui everywhere; egui around the canvas (+17 crates, 2 duplicate versions, antialiased text) | Owner decision (confirmed 2026-09-12): Feathers is new and not mature; the editor needs something that reaches a working state fast without being wrestled with. Tool UI productivity where it matters, pixel UI for players, egui isolated to one plugin. The game UI moved from `bevy_ui` text to canvas sprites with a self-authored bitmap font on 2026-09-12 (§8.2). **Amended by the owner on 2026-09-20 (PRD D26, §11.1): Feathers in game.** The canvas direction seemed safe, but the game needs better interface widgets and a modern look. Start experimenting with Feathers in the game, see how the default styles work, and see if modern fonts can replace the pixel-art based fonts. The experiment is bounded: one in-game screen first (the tactics screen needs dropdowns, number inputs, lists and scrolling, which the canvas toolkit lacks), measured on the `ui` feature's crate count, rendering and scaling on the owner's ultrawide, the MCP screenshot and screen dumps still showing the interface, and headless tests still driving every widget. Facts read from the 0.19.1 sources: `bevy_feathers` and `bevy_ui_widgets` are part of Bevy (no third-party crates, where `bevy_egui` adds 17 and two duplicates), both call themselves experimental, and Feathers' documentation aims it at editors and suggests copying and restyling its widgets for a game. If the in-game integration goes well, the editor's toolkit is reconsidered. |
| A12 | Packs bypass Bevy's asset system; only images and audio go through an `AssetLoader` | RON as Bevy assets | Packs are validated untrusted data with cross-file references; Bevy's loader is per-file and has no RON loader anyway. |
| A13 | No global clock; subjective clocks per holder, reconciled on interaction by a data rule with bounded drift | Global calendar with a world-wide daily tick | Owner direction from `docs/background/introduction.md`; makes NPC agency, multiplayer, construction, and travel the same mechanism; lazy and deterministic. Cost: every interaction site must reconcile. |
| A14 | One world seed; named PCG32 streams derived by FNV-1a and splitmix64; stateful streams persisted in the save, generation streams stateless | Single global RNG; per-entity RNG objects | Approved 2026-09-12. Isolation between subsystems, exact continuation after load, pure regeneration, traceable draws. |
| A15 | Tactics are data in the `World`, walked by the simulation: a closed trigger list, criteria trees of integer predicates, runbooks per combatant, reactions and auto turns resolved inside the command that causes them (§4.7) | Interrupt prompts to the front end; tactics evaluated in the app with the log recording only the chosen commands; player-written Rhai | PRD D21–D23. Reactions happen in the middle of another combatant's command, so they must be resolved in the simulation; keeping auto turns there too means one chooser serves members, hirelings and monsters, replays need nothing but the command log, and every front end gets tactics for free. No script from players (PRD R8). |
