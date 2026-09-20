# Omnis — Product Requirements Document

| | |
|---|---|
| Status | Draft v0.5, under discussion (v0.5, 2026-09-20: turn budget, declared reactions, tactics runbooks, bonus-action spells, D21–D24; the time model renumbered D25; a modern look and feel, not pixel art, D26) |
| Date | 2026-09-11 |
| Owner | john@gatewaynode.com |
| Companion | `ARCHITECTURE.md` (derived from this document), `README.md` |

This document says what Omnis is, who it is for, why it exists, what it must do, what it must not do, what constrains it, and what could sink it. `ARCHITECTURE.md` says how.

---

## 1. Vision

Omnis is a turn-based, party-based, first-person grid-crawling RPG in the lineage of Might and Magic I and II (1986–1988), rebuilt for the present. It keeps what made those games durable: a large open world you may explore in any order, a party you build from scratch, mass combat against groups of monsters, and a sense that the world is bigger than the story. It adds what they could not do: a living regional ecosystem, a story engine that mixes authored and generated quests, procedural generation of arbitrarily large regions, and an in-game editor and content management system that ships to players and modders as a first-class feature.

Omnis is a modern tactics and strategy re-imagining of the classic era of computer RPGs, not a retro-styled throwback. What it takes from the past is what worked: the structure, the game loop, a first-person viewport with discrete steps and 90° turns. The art of that era is not one of those things. The end goal is an entirely modern look and feel, and no design compromise is made to keep a pixel-art feel (D26). Placeholder art and fonts may come from pixel-art sources while the systems are built; they are placeholders, not a style.

The world is Toel (`docs/background/introduction.md`): a vast, dangerous, rich place settled in waves through portals that open between worlds, humming with reality-twisting forces. Time itself is one of those forces. It runs separately for every traveller and every place, and converges only briefly when they meet; the standard greeting is a question about how much time has passed for the other person. Ruins are sometimes met alive, and towns appear or vanish between visits. That subjective time is a founding rule of the game's world model (§7.8), not a story device layered on top of it.

## 2. Consumers

Ranked by priority for v1.

1. **Players** — people who enjoy classic blobber and dungeon-crawler RPGs (Might and Magic, Wizardry, Bard's Tale, Legend of Grimrock, Etrian Odyssey) and want a large, systemic, replayable world rather than a linear campaign. Single-player, desktop.
2. **Builders and modders** — players who want to make maps, regions, quests, monsters, items, and whole campaigns with the same tools the developers use, and share them. This group is why the editor is in the first milestone rather than a later one.
3. **The developer(s)** — the editor and data pipeline are also the internal production tools. Everything shipped in the base campaign must be buildable with the shipped editor.

Explicit non-consumers for v1: mobile players, browser players, multiplayer groups.

## 3. Motivation

- Modern RPGs in this niche are either faithful clones with the original's limits, or reinterpretations that drop the open world for authored linearity. Neither combines classic structure with systemic depth.
- Procedural generation and simulation are cheap enough today to give a solo or small team a world far larger than any 1980s studio could hand-build, without giving up hand-authored set pieces.
- Modding communities keep games alive for decades. Building the editor first, and making all content data, makes that community possible from day one instead of retrofitting it.
- The D&D 5.1 SRD (CC-BY-4.0) provides a large, legally clean library of stat blocks, spells, items, and conditions that can be adapted as content, which lets effort go into systems rather than re-inventing a bestiary.

## 4. Goals

Ranked. When two conflict, the higher-ranked one wins.

1. **A complete, fun crawler loop.** Create a party, explore, fight, level, shop, rest, save. If this is not good, nothing else matters.
2. **Everything is data.** Maps, monsters, spells, items, quests, regions, ecosystem rules, and the base campaign itself live in human-readable text files that the editor reads and writes. No content is hard-coded.
3. **The editor is a shipped feature.** It runs in the same binary, edits the same data, and is documented for modders.
4. **A world that changes without the player.** Regional ecosystem state (populations, factions, resources, prices) advances on each region's own subjective clock, catching up when the party makes contact, and visibly affects encounters, economy, and quests.
5. **Stories from two sources.** Authored quest graphs with full designer control, plus template-driven generated quests bound to procedurally generated places and to ecosystem state.
6. **Infinite canvas.** Any region can be procedurally generated at any scale, then kept as-is, curated by hand, or overwritten. Hand-authored and generated content coexist in the same map format.
7. **Deterministic, replayable simulation.** The game rules run as a pure function of state, seed, and inputs, separate from rendering and input handling. This makes tests real, bugs reproducible, and networked co-op a future addition instead of a rewrite.
8. **Open.** Source and content are public and permissively licensed.

## 5. Non-goals (v1)

- Real-time or action combat.
- Free movement and camera free look. Movement is tile and 90° turn based, and the viewport only ever shows the facing direction. This does not exclude remote sensing: skills, equipment, and spells that reveal distant tiles into the automap are in scope and are a designed system (§7.2).
- 3D rendering.
- Multiplayer of any kind (but see goal 7).
- Browser or mobile builds.
- Voice acting, cinematics, or licensed music.
- Agent-level simulation of the general population. In v1 the ecosystem is per-region and coarse. Free agency for a limited set of important non-player characters is a long-term goal, scoped when the regional engine exists (§9.2, §13 Later), and the v1 design must not preclude it.
- Language-model-generated content at runtime.
- Reproducing any Might and Magic map, name, monster, item, text, or art.

## 6. Decisions log

Decisions taken during PRD discussion. Changing one of these reopens this document.

| # | Decision | Choice | Rejected alternatives | Rationale |
|---|---|---|---|---|
| D1 | Rules foundation | Hybrid: SRD 5.1 structure and content, MM2 adaptations only where they serve the crawl loop | Pure MM2-style original; pure SRD; SRD content on MM2 structure (first draft, inverted by owner) | The SRD is richer and more familiar to players; MM2 contributes spell points, large party, rows and stacks, resource-based rest, trainer leveling. Where the built game and the SRD disagree, the default direction is toward the SRD (owner, 2026-09-20); Omnis's own departures extend the SRD rather than trim it (D21–D24). See §8. |
| D2 | Presentation | First-person grid crawler, 2D rendering, modern look and feel (D26) | Real 3D grid crawler; top-down/isometric; a retro pixel-art style as a goal (first draft, corrected by the owner on 2026-09-20) | The first-person tile view is part of what worked; 2D is the cheapest art pipeline and Bevy 2D only. The first draft's "pixel art" and "most faithful" were never the owner's intent. |
| D3 | Primary consumers | Players plus a modding community | Solo developer tools only | Editor and formats are public-facing and versioned from the start. |
| D4 | First playable milestone | Crawler loop plus editor | Loop only; loop plus procgen | All base content is built with the shipped editor; proves the tooling early. Order within the phase (owner, 2026-09-19 and 2026-09-20): the loop is built first on hand-written data and the editor closes Phase 1, which is not done until its content has been re-authored in the editor. |
| D5 | Platform and play mode | Desktop single-player (macOS, Linux, Windows); co-op later | Add WASM; single-player only forever | Deterministic input-driven sim keeps co-op possible without rewrite. |
| D6 | Licensing | Open source code, open content | Open engine with closed content; fully proprietary | Maximizes modding; SRD attribution is simple. See §11.2. |
| D7 | Ecosystem engine v1 | Regional simulation on coarse steps of each region's subjective time (D25) | Static world with scheduled events; agent-level sim | Visible consequences at tractable cost. Agent-level simulation for a limited set of important NPCs is a later horizon, not rejected. |
| D8 | Story engine v1 | Authored quest graph plus procedural quest templates | Authored only; LLM-assisted | Deterministic, testable, and binds to procgen and ecosystem. |
| D9 | Engine | Bevy, latest stable (0.19.x at time of writing), explicitly exempt from the N-1 dependency rule in `CLAUDE.md` | Pin to N-1 | Owner decision; Bevy moves fast and back-porting fixes to old versions is not practical. All other dependencies follow the N-1 rule. |
| D10 | Party size | Six party slots, filled by created characters and, when slots are open, hirelings | MM2's eight | Closer to SRD encounter math. Hirelings beyond the six slots are a later sidecar system (§13). |
| D11 | Spell components | Hard component requirement starts at spell level 5; the threshold is configuration, tuned by testing | Fixed threshold | Points are the primary curve; components add scarcity at the top. |
| D12 | Spell point pool | Caster level × casting ability modifier, plus the other two mental modifiers (Int, Wis, Cha) as a flat bonus; half casters use half level; floor of character level | Level + casting modifier (too small); level × sum of all three mental modifiers (too build-dependent) | Within 20 percent of SRD slot capacity from level 5 for focused, dump-stat, and gifted builds, while all three mental scores still matter. Comparison in §8.3. Formula is data. |
| D13 | Aging | Deferred, not rejected | Drop for good | Revisit after v1. |
| D14 | Party wipe | Reload the last save; no other recovery | MM2's fled-member revival | Simple and honest. |
| D15 | Data format | RON for all content and saves | TOML | Bevy-native, expresses Rust enums and nested structs directly. |
| D16 | Viewport and visibility depth | Two numbers. Detail depth is fixed at roughly 4–6 tiles and is what the viewport draws with full sprites. Visibility depth varies by environment and modifiers (3–4 in a dark dungeon, 10–20 on open plains, 1 in fog, 2 in heavy rain), is drawn beyond detail depth as a low-detail horizon band, and feeds the automap as a passive cone-shaped sensing source | Single variable draw depth | Visibility is a property of the world, not the camera, and the sprite pipeline stays bounded (R10). |
| D17 | Saves | Difficulty option (inn-only through save-anywhere); on harder settings some items and magic grant relief | One fixed rule | Player choice of commitment level. |
| D18 | Remote sensing | Active sensing runs layered perception tests per fidelity layer; all remotely sensed knowledge is immediately stale | Flat reveal | Uses SRD perception checks; the automap records what was seen and when. |
| D19 | Title | "Omnis" | — | Working title confirmed. |
| D20 | Display target | The owner's primary display, a 5120×1440 ultrawide, is the target for the first UI, with 16:9 monitors (720p, 1440p, 4K) supported. The canvas is 720 rows tall and as wide as the window at the largest whole multiple of 720 rows that fits, so the layout follows the window's aspect: a glyph pixel covers 2×2 real pixels on the ultrawide and 3×3 at 4K, not the 12×12 of a 320×180 canvas; on a wide canvas the 960×540 3D viewport is centred with the roster to its left and the map to its right; the viewport keeps its coarser texels (16-pixel tiles at four canvas pixels a texel) | Keep the 320×180 canvas at 12× on 4K; a fixed 16:9 canvas with bars at the sides of the ultrawide | Owner decisions (2026-09-13): the display area is one of the things Omnis can improve over the 1980s originals, and the pixel-art feel is not worth sacrificing a better UI. Revised the same day when the fixed 16:9 canvas left half of the primary display empty. Fractional scaling and a canvas that grows in rows are horizons. Since D26 (2026-09-20) the integer-scaled canvas and its bitmap font are the placeholder presentation as built, not a goal. |
| D21 | Turn budget | Each combatant's turn has a budget of actions, bonus actions, and reactions, computed by data rules from class level, items, spells, and effects. The SRD's one of each is the starting point; more than one bonus action and more than one reaction are allowed and arrive with progression | The SRD's fixed single bonus action and single reaction; one action per turn (as built through M6) | Owner decision (2026-09-20): heroes who can face massive hordes, the MM2 feel of mass melee. Extends the SRD's own precedents (Action Surge, Extra Attack, haste) instead of trimming the SRD. See §7.3, §8.2. |
| D22 | Reactions | Declared in advance, never prompted: each character's reactionary tactics name the criteria under which each available reaction (spell, equipment, ability) fires, built from triggers and conditions the system offers. The simulation resolves them at the trigger. Triggers that the SRD ties to miniature proximity map onto rows and stacks (§8.3). Any member's reactions can be switched off at any time in a fight | Interrupt prompts during other combatants' turns; a fixed list of automatic reactions (as built in M6: shield only); dropping grid-dependent reactions such as opportunity attacks (first draft) | A menu-driven fight has no interrupt, and the SRD's own Ready action is a player-declared trigger and response. Closer to the SRD than the first draft. See §7.9. |
| D23 | Tactics runbooks | Every action that can have a trigger can have several named sets of criteria. A runbook names, for each such action, the one set that is evaluated in combat. A character holds a collection of runbooks, exactly one of them the default; earlier criteria sets are offered again whenever a runbook is built. A per-member auto flag lets the runbook take that member's turns, so a fight can be fully automated, fully hand-played, or mixed. Monsters and hirelings hold their own collections | Player-written script; party-wide auto-combat only; propose-and-confirm automation | Removes tedium from mass combat and keeps player-driven combat one switch away. Criteria are data the system offers, never script (R8). How monster and hireling runbooks are distributed and progress is a system, environment, and game-master feature set for later development. See §7.9. |
| D24 | Bonus-action spells | Set explicitly in every spell definition by three fields: `bonus_action_available`, `preparation_available`, `preparation_required_for_bonus_action`. More than one spell in a round is possible within the turn budget, limited by the spell itself | The SRD's rule that a bonus-action spell limits the turn's action to a cantrip; inferring the cost from casting time | Owner decision (2026-09-20): whether a spell can be a bonus action depends on how fast it is cast or whether it can be prepared in advance, and is always stated in data. See §8.3. |
| D25 | Time model | Subjective time: no global clock; every party, region, named actor, separated character, and project keeps its own clock; clocks reconcile only partially on contact, by a data rule with bounded drift | Global calendar with a world-wide daily tick | Owner direction from the world background (2026-09-12). Numbered D20 until 2026-09-20, when the duplicate with the display target was resolved; the more widely cited row kept the number. One mechanism serves NPC agency, multiplayer, construction, travel, and aging. See §7.8. |
| D26 | Look and feel | Entirely modern: modern fonts, modern interface widgets, and a presentation that uses what current displays and hardware can do. Pixel art appears only as placeholder while systems are built. First step: experiment with Feathers in the game, see how its default styles work, and see whether modern fonts can replace the pixel-art fonts (§11.1) | A retro pixel-art presentation as a goal or a constraint (first drafts of §1, D2, §7.2, §11.1) | Owner direction (2026-09-20): Omnis is a modern tactics and strategy take on the classic CRPG, not a throwback. As with the MM2 manual, what is taken from the past is what worked well, and the art was not what worked well. No compromise is made to keep a pixel-art feel. |

## 7. Core experience: the player loop

### 7.1 Party
- The player controls a party, not a character. The party has six slots, filled by created characters and, when slots are open, hirelings (D10).
- Characters are created from SRD race, class, background, and alignment with rolled or point-bought ability scores. They persist across saves and can be swapped in and out at an inn.
- Hirelings are pre-built NPCs recruited in towns for a daily wage; they leave if unpaid.
- Formation matters: front-row characters can melee and be meleed; back-row characters need reach or ranged attacks, and are protected until the front row falls.

### 7.2 Exploration
- The world is a grid of square tiles. The party occupies one tile and faces one of four directions. Movement is one tile forward or backward, or a 90° turn.
- The viewport is a first-person rendering of the tiles ahead, showing walls, doors, terrain, sky or ceiling, and any visible monsters or objects.
- Two depths govern what the player sees (D16). **Detail depth** is fixed at roughly 4–6 tiles: the viewport draws these with full sprites. **Visibility depth** is a property of the environment and the party, not the camera: a dark dungeon allows 3–4 tiles, an open plain 10–20, fog 1, heavy rain 2, modified by light sources, time of day, and character abilities, computed per tile per turn from data. Tiles beyond detail depth but within visibility depth are drawn as a low-detail horizon band of terrain colour and landmark silhouettes, and are recorded to the automap as passive sensing in a forward cone.
- Time advances per step and per action on the party's own clock (§7.8). The party's calendar, with day and night, is a rendering of that clock. Night changes encounters, visibility, and some services.
- Terrain types gate movement by skill or item (mountains, forest, swamp, water, desert). Food and rest are resources.
- **Automap and remote sensing.** The automap is the persistent record of everything the party knows about the world, and it is the game's substitute for free look. Knowledge enters it from several sources:
  - Walking: visited tiles and the tiles within visibility depth in the facing cone, gated by a cartography skill or item as in MM2.
  - Equipment: a spyglass or similar reveals unobstructed tiles along the facing line of sight to a long range; other items may reveal in other shapes.
  - Skills: a scouting or navigation skill widens the recorded radius around the party, indoors or outdoors, by skill rank.
  - Spells: divination spells reveal a large area at once (a radius, a whole 16×16 map, or a region), possibly with extra layers such as secret doors, monsters, or treasure.
  - Purchased or found maps: an item that adds a fixed area to the automap when used.
  - Every source is data with the same shape: a reveal geometry (radius, cone, line-of-sight ray, whole map, whole region), a fidelity (terrain only; walls and doors; objects and triggers; monsters), and a persistence (permanent, or a temporary overlay that expires on the party's clock). New sources are added by content, not code.
  - Active sensing (spyglass, scouting, divination) resolves as layered perception tests: one SRD-style check per fidelity layer (terrain, structure, objects, creatures) against a difficulty set by distance, cover, light, and the target's own concealment. Passing a layer reveals that layer; failing stops there (D18).
  - Revealed knowledge is per party and saved with the game with the party-clock time it was seen. Everything remotely sensed is treated as stale the moment it is recorded; the automap shows what was seen and when, not what is. Ecosystem changes (§9.2) are one reason the two diverge.
  - The viewport never changes for any of these. Remote sensing is read on the automap.
- Outdoor, town, dungeon, and special maps share one format; only their rules and rendering assets differ.

### 7.3 Combat
- Turn-based. Initiative is an SRD initiative roll. Each combatant's turn has a budget of actions, bonus actions, and reactions (D21): the SRD's one of each to begin with, growing with class progression, items, spells, and magic. Movement is replaced by row position (§8.3).
- Encounters are groups: multiple monster types in multiple stacks, potentially dozens of individuals, following MM2 structure (§8.2).
- Actions: attack, cast, use item, dodge, exchange position, run, and SRD class actions. Each spends an action, a bonus action, or a reaction, as its data says.
- Reactions are declared, not prompted (D22): a character's reactionary tactics (§7.9) say what they do when attacked, when a party member is attacked, wounded, or dying, when an enemy casts a spell or flees. The fight never stops to ask. A member's reactions can be switched off at any time, for instance to conserve spell points.
- Any member can be set to auto (D23): their runbook takes their turns until the player takes them back. A fight can be fully automated, fully hand-played, or mixed member by member.
- Resolution is SRD: attack rolls against armor class, damage dice, saving throws against difficulty classes, SRD damage types and resistances.
- Victory grants experience and loot. Defeat is party death unless the party escapes.

### 7.4 Towns and services
- Inn: save, rest, swap party members, hire.
- Temple: heal, cure conditions, resurrect, donate.
- Training: level up for a fee once experience is sufficient.
- Blacksmith: buy, sell, identify, repair equipment.
- Tavern: rumors, food, hireling recruitment, quest hooks.
- Bank: deposit gold and gems; earns interest on the bank's region clock, so the balance grows by however much time that town has experienced when the party returns (§7.8).
- Guilds: buy spells by level for members.
- Every service is a data-defined building placed on a map tile; there is nothing special about "a town" beyond its tiles.

### 7.5 Progression
- Experience is earned in combat and from quests; levels are bought at a trainer.
- Each level grants SRD hit points, class features, and spell points where applicable, and by data rule may widen the turn budget (D21).
- Skills and tool proficiencies come from SRD background and class; crawler abilities such as cartography and wilderness travel are proficiencies, items, or spells (§7.2, §8.1).
- Ability scores change from SRD magic items, ability score improvements at SRD levels, and content-defined shrines. A character's age is their own subjective elapsed time (§7.8); aging effects are not in v1 (§8.3).

### 7.6 Quests
- Quest state is a set of typed flags and counters on the save, never on the map.
- Quests come from authored graphs (§9.3) and generated templates. The player sees both in the same journal.
- Quests may require, reward, or change ecosystem state (§9.2).

### 7.7 Save and load
- A save is the complete world state: party, every map's mutable state, ecosystem state, quest flags, every holder's clock and contact records, and RNG seed and counter.
- Saves are files the player owns. There is no cloud dependency.
- When and where the player may save is a difficulty option ranging from inn-only to anywhere (D17). On harder settings, specific items and spells grant save opportunities. A full party wipe returns the player to the last save with no other recovery (D14).
- Save format is versioned and migratable. A save from build N loads in build N+1.

### 7.8 Subjective time
Time in Toel is local. There is no world clock.
- **Every holder keeps its own clock**: the party (shared by its members while they travel together), every region with its towns, dungeons, lairs, and ecosystem, every named non-player character, any character separated from the party, and, later, construction projects and other players' parties.
- **Only the actor's clock moves.** A step, a rest, or a service advances the party's clock. Nothing else in the world moves until it is met.
- **Clocks reconcile on contact, and only somewhat.** Entering a region, meeting a named character, or two parties meeting compares how much time each has lived since their last contact. The other side catches up by an amount set by a data rule: near equal for stable places, drifting by months for places in flux, never backwards in v1. Both remember the contact. The exchange of "how long has it been for you" is the visible face of this rule.
- **Places connected by roads, rivers, and trade** reconcile with each other when either meets the party, so change spreads through a neighbourhood without a global tick.
- **Eras are part of the model.** A place can exist in more than one age, and a reconciliation may in principle land the party in an older or newer one. V1 content uses a single era and the rule never changes it; the save format carries the field from day one.
- **Consequences the player feels**: a region left for a subjective year has changed by roughly a year when revisited; a bank balance grows by the bank's time, not the party's; a hireling left at an inn has aged on their own; a quest deadline is counted on the clock of whoever set it; rumors report how long ago on the teller's clock.
- **Why this shape**: it makes NPC free agency, multiplayer, construction time, long travel, and aging the same mechanism, and it keeps the simulation lazy and deterministic. The cost is that every place where things meet must reconcile; those places are enumerated and tested in the architecture.

### 7.9 Tactics
Tactics are how a character fights when the player is not choosing for them: every reaction, and every turn of a member set to auto. They exist because the turn budget (D21) and mass combat multiply the choices in a round. Tactics take the routine choices and leave the ones that matter with the player.
- **Criteria sets.** Every action that can have a trigger can have several named sets of criteria for it: a reaction spell, a piece of equipment, a class ability, an opportunity attack, and, for a member on auto, any action at all. A criteria set is a trigger (a spell is cast, the character is attacked, a party member is attacked, wounded, or dying, an enemy flees, an enemy casts a spell, the character's own turn, and so on) plus conditions chosen from what the system offers and combined with "and" and "or": counts and shares of monster kinds ("goblin shaman ≥ 1", "rats > 50%"), hit points and spell points as percentages, conditions present, rows, the round. The system provides the options and the player decides among them; the player never writes script. Criteria sets are kept per character and offered again whenever a runbook is built.
- **Runbooks.** A runbook names, for each such action, the one criteria set that is evaluated in combat, and the order in which actions are considered. A character may hold many runbooks for different combat situations, and exactly one is the default. A runbook may carry encounter criteria of its own ("mostly rats", "goblin shaman present"); at the start of a fight the first runbook whose criteria match is used, otherwise the default.
- **Reactions.** When a trigger occurs, the simulation considers the character's reactions in runbook order and fires the first whose criteria hold and whose cost can be paid, while the turn budget has a reaction left (D22). A per-member switch turns reactions off at any time in a fight, at no cost.
- **Auto.** A per-member flag. On, the runbook chooses that member's turns; off, the player does. It can be flipped at any time in a fight. There is a place for fully automated combat, to remove tedium, with player-driven combat one switch away when it is needed or wanted (D23).
- **The tactics screen** sits beside the character sheet: compose criteria sets, build runbooks, choose the default, set the auto and reactions flags. Like every screen it is reached by a button before a key.
- **Monsters and hirelings** hold their own collections of runbooks. How those are distributed and how they progress is a set of system, environment, and game-master features for later development; the data model gives every combatant a collection from the start so that later work needs no migration.

## 8. Rules: SRD 5.1 structure with MM2 crawler adaptations

The D&D 5.1 SRD is the mechanical spine: ability scores, races, classes, proficiency, attack and save resolution, spells, conditions, damage types, monsters, and items are taken from it as written unless a row in §8.3 says otherwise. Might and Magic II contributes a short list of adaptations, each justified by the dungeon-crawling loop and nothing else. Numbers and tables belong in `data/` and `ARCHITECTURE.md`. MM2 facts are from the C64 manual in `docs/` and describe structure to adapt, not content to copy.

### 8.1 Structural layer (from SRD 5.1)
- **Ability scores**: the six SRD scores with the standard modifier formula, levels 1–20, and the proficiency bonus by level.
- **Races and classes**: the SRD race list with traits, and the SRD class list with the one subclass each the SRD provides. Multiclassing is deferred (§8.3).
- **Backgrounds and skills**: SRD backgrounds, the eighteen skills, and tool proficiencies. These replace MM2's secondary skills; crawler-specific abilities such as cartography, mountaineering, and pathfinding are expressed as SRD skill or tool proficiencies plus items and spells (§7.2). Tools are pack data: Toel's own proficiencies (`docs/background/backgrounds_and_skills.md`) are tool proficiencies, using the SRD tool where one exists (Cartography is cartographer's tools, Orienteering is navigator's tools) and an Omnis tool where none does (Mining, Assaying, Refining). A background keeps the SRD shape of two skills plus tools: the Explorer has Perception and Survival with the two tools.
- **Resolution**: d20 plus modifiers against armor class or a difficulty class; six saving throws; advantage and disadvantage; hit dice; SRD armor class values.
- **Spells**: SRD spell lists by class, spell levels 0–9 including cantrips, concentration, ritual tags, and school. Resource model is adapted (§8.2).
- **Conditions and damage**: the SRD's fifteen conditions and thirteen damage types, with resistance, immunity, and vulnerability as written.
- **Monsters**: SRD stat blocks used as written. Challenge rating and experience values are kept and feed encounter budgeting (§8.2).
- **Items**: SRD weapons, armor, adventuring gear, and magic items, including attunement, rarity, and charges.
- **Alignment**: the SRD nine-way grid. Whether alignment gates places or items is a content decision per pack, not a rule.
- **Rests**: short rests as written (spend hit dice). Long rests are adapted (§8.2).

Everything adapted from the SRD is re-expressed in Omnis's own data format with the required attribution; the SRD text is a design source, not a runtime asset.

### 8.2 Crawler adaptations (from MM2)
Each adaptation names the loop problem it solves. An MM2 mechanic that does not solve one is not adopted. The last three rows are not MM2 mechanics: they extend the SRD on the owner's decision (D21–D23) and are held to the same test.

| Adaptation | MM2 source | Loop problem it solves |
|---|---|---|
| Spell points instead of spell slots | Spell points from the casting stat | Slots per level force rest cycles that break long dungeon runs. Each caster has a pool derived from level and the casting ability modifier (Intelligence, Wisdom, or Charisma per class); a spell costs points by its level; cantrips are free. Known and prepared spell lists collapse into one "known" list per caster. |
| High-level spell scarcity through components | Gem cost on high-level spells | Slots limited 6th-level-and-up spells to one per day; points alone would not. Point cost by spell level is the primary scarcity curve. The hard component requirement begins at spell level 5 by default, and the threshold is configuration tuned by testing (D11). On top of points, a spell may consume a **component list**: an ordered set of item IDs with quantities, all of which must be in the party's inventory and are consumed on cast. Gems are the first component type; rare reagents, compounds, and crafted items follow as content. The SRD's consumed material components (a costly diamond, a jade, and so on) are expressed the same way, so one mechanism covers both sources. This is intended to become a rich subsystem: component gathering, trading, and crafting can hang off it later without changing the spell data format. |
| Party with hirelings | Up to 8 slots, hirelings hired at inns, paid at rest | Six slots (D10), shared by created characters and hirelings hired at inns and paid at rest. Six is near SRD encounter math while still fielding most SRD roles. Encounter budgets scale by the party's total level. A sidecar system for extra hirelings in massive battles is a later horizon. |
| Marching order and engagement | Battle position equals marching order; an engagement marker | The SRD assumes a battle grid, which a first-person tile view lacks. Two rows replace the grid: front-row members can melee and be meleed; back-row members need reach or range. Movement rules are dropped; exchange is an action. Opportunity attacks survive as a reaction to a stack or a member leaving the engagement (§8.3). |
| Monster stacks | Groups of many monsters, lead stack shifts up when killed | Mass combat is what makes an 8-slot party feel necessary. Encounters are stacks of SRD monsters; area spells target stacks. The encounter budget is SRD experience thresholds scaled to the party. |
| Pre-combat options and disposition | Attack, bribe, hide, run; four-level disposition | Gives exploration choices beyond fighting and lets the player tune risk against reward. |
| Overnight rest with food and ambush | Rest costs food, restores fully, may be ambushed | The SRD long rest is free; food, light, and ambush make deep dungeon runs a resource problem. Long rest is the MM2 rest. |
| Levels bought at a trainer | Training grounds | Keeps towns relevant and makes returning to the surface part of the loop. Experience accrues anywhere; the level is granted in town. |
| Guilds and temples sell spells | Mage guild, temple | Spells beyond the ones granted at level-up are bought or found, so gold and exploration both feed the spell list. |
| Calendar and aging | Day and year, aging effects | The party's calendar renders its subjective clock (§7.8); the ecosystem runs on each region's own clock. Aging as a stat effect is deferred past v1 (D13); each character's subjective elapsed time is already tracked, so it can be added without a save migration. |
| Inn-only saves | Sign in at an inn | Turns every dungeon into a commitment. Adopted as the hardest setting of a save difficulty option (D17); easier settings allow save-anywhere; harder settings gain relief through specific items and spells. |
| A turn budget that grows | None (extends the SRD's Action Surge, Extra Attack, haste) | Six heroes against dozens of monsters need more than one action each to make mass melee winnable and worth watching. Actions, bonus actions, and reactions per turn are data rules over class level, items, spells, and effects; the SRD's one of each is the floor (D21). |
| Declared reactions | None (extends the SRD's Ready action) | A menu-driven fight has no moment to interrupt, and the SRD's proximity triggers assume miniatures. Reactions are composed ahead of time as tactics and resolved by the simulation; proximity maps to rows and stacks (D22, §7.9, §8.3). |
| Tactics runbooks and auto | None | The turn budget and stack sizes multiply the choices in a round; routine fights become tedium. Runbooks let any member, or the whole party, fight by the player's stored tactics, with hand play one switch away (D23, §7.9). |

### 8.3 Reconciliation rules
Where the two systems disagree, the SRD wins by default. An MM2 rule overrides only when a row here says so and §8.2 gives the loop reason.

| Friction | SRD | MM2 | Omnis rule |
|---|---|---|---|
| Ability scores | Six scores, modifiers | Seven stats, direct effects | SRD. Speed for initiative is Dexterity; Luck does not exist. |
| Action economy | Action, bonus action, reaction, movement | One action per round | The SRD's action, bonus action, and reaction are kept and extended: a turn budget starts at one of each and grows by data rules (D21). Movement is replaced by row position and the exchange action. Reactions are declared as tactics and resolve without a prompt (D22, §7.9). |
| Reactions and proximity | Triggers tied to reach, five feet, and line of sight on a grid; one reaction per round; the Ready action | None | Every SRD reaction is kept where a trigger can be named without a grid. "Within five feet of you" is the same row for allies and the engaged lead stack for enemies. "Leaves your reach" is a stack fleeing, or a front-row member running or exchanging out of the engagement, which is what provokes an opportunity attack. "Within N feet and visible" is any combatant in the fight. A hit on a stack is a hit on its lead individual. The Ready action is a criteria set like any other. The reaction count comes from the turn budget. |
| Casting time | Action, bonus action, reaction, or longer; a bonus-action spell limits the turn's action to a cantrip | One spell per round | Stated in every spell definition (D24): `bonus_action_available`, `preparation_available`, `preparation_required_for_bonus_action`. The cantrip limit is replaced by the turn budget and the spell's own fields, so more than one spell in a round is possible. Preparation here means readying a spell in advance of casting it; it is not the SRD's daily prepared list, which §8.2 collapsed into the known list. |
| Positioning | Five-foot grid, reach, ranges in feet | Engagement marker | Two party rows and an ordered list of monster stacks. Reach and ranged weapons reach the back row; ranges collapse to "melee" and "ranged". Area spells hit a whole stack or all stacks by data flag. |
| Spell resources | Slots, prepared lists, some consumed material components | Spell points, gems | Spell points per §8.2: pool = caster level × casting ability modifier + the other two mental modifiers, half casters at half level, floor of character level (D12); spell cost = spell level; cantrips free. Candidate formulas are compared against SRD capacity after this table. Every spell carries a point cost and a component list (possibly empty) of item IDs and quantities; the list is data, never a single gem count. V1 content uses points plus gems; SRD consumed components are migrated into the same list. Non-consumed SRD components (verbal, somatic, focus) are dropped. |
| Spell learning | All class spells available when prepared | Some inscribed on level, rest bought or found | Class spells granted at level-up per the SRD's spells-known classes; prepared-list classes gain a fixed number per level and buy or find the rest. |
| Party size and difficulty | Balanced for 4–5, challenge rating | Up to 8, stacks | Six slots (D10). SRD experience thresholds scaled to party size and level; stack sizes chosen by the budget. Challenge rating retained. |
| Rests | Short and long | Overnight rest | Short rest as SRD. Long rest is the MM2 overnight rest with food and ambush. |
| Leveling | Automatic at thresholds | Bought at a trainer | Trainer, per §8.2. Gold cost is data. |
| Skills | Eighteen skills, tools, backgrounds | Two secondary skills per character | SRD. Crawler skills become proficiencies, items, or spells. |
| Conditions and damage types | Fifteen conditions, thirteen types | Seven conditions, nine types | SRD as written. MM2's eradicated and stoned map to the SRD's dead and petrified. |
| Death | Death saves, stabilize, revivify chain | Unconscious, dead, eradicated, revive at temple | SRD death saves in combat. Out-of-combat revival through temple services and SRD spells. A full party wipe reloads the last save (D14). |
| Aging | Absent | Present | Deferred (D13). |
| Multiclassing and feats | Present (one feat in the SRD) | Absent | Deferred. Data model must allow multiclass levels so it can be enabled later. |
| Alignment | Nine-way, no mechanical effect | Three-way, gates places and items | SRD grid; gating is per-pack content. |

**Spell point formula candidates against SRD capacity.** SRD capacity is the sum of slot levels for a full caster. Three builds are shown: a focused caster (casting modifier +3, then +4 at level 8, +5 at level 16; other two mental modifiers +1 and +0), a dump-stat caster (same casting modifier; other two at −1 each), and a mentally gifted caster (same casting modifier; other two at +2 each).

| Formula | Build | L1 | L5 | L10 | L20 |
|---|---|---|---|---|---|
| SRD slot-levels | any | 2 | 16 | 41 | 89 |
| F1: level + casting mod (first draft) | focused | 4 | 8 | 15 | 25 |
| F2: level × (Int + Wis + Cha mods) (owner proposal) | focused | 4 | 20 | 50 | 120 |
| F2 | dump-stat | 1 | 5 | 20 | 60 |
| F2 | gifted | 7 | 35 | 80 | 180 |
| F3 (adopted, D12): level × casting mod + (other two mental mods) | focused | 4 | 16 | 41 | 101 |
| F3 | dump-stat | 1 | 13 | 38 | 98 |
| F3 | gifted | 7 | 19 | 44 | 104 |

Observations. F1 falls to roughly a quarter of SRD capacity. F2 tracks the SRD for a focused build but multiplies the two off-stat modifiers by level, so a dump-stat caster has a third of a focused caster's pool at the same level and a gifted caster twice it, and the sum can reach zero or below; it also makes every SRD caster class dependent on all three mental scores, which SRD class design avoids. F3 keeps the owner's intent that all three mental scores matter, but only the class's casting score scales with level; the other two are a flat bonus. F3 lands within 20 percent of SRD capacity from level 5 onward for all three builds. Whichever formula is chosen, half casters (paladin, ranger) use half their level rounded down, matching SRD multiclass caster-level rules, and the pool has a floor of the character's level.

## 9. World systems

### 9.1 Procedural generation
- Any region can be generated from a seed and a set of parameters (biome mix, danger level, settlement density, dungeon density, size).
- Generated output is ordinary map data. The editor can open it, edit it, and save it. Nothing distinguishes generated from authored tiles after generation.
- Generation is deterministic: same seed and parameters, same output, on every platform.
- Generation is layered: terrain, then rivers and roads, then settlements, then dungeons, then encounters and loot, then quest hooks. Each layer can be re-run independently on a curated map without destroying manual edits in other layers.
- Scale target: a region of 64×64 tiles generates in under one second on a 2020-era laptop; the world may contain thousands of regions, generated lazily on first visit and persisted thereafter.

### 9.2 Ecosystem and environment engine
- The world is divided into regions. Each region has state: populations per creature group, faction presence and attitude, resource levels, prosperity, danger, and weather trend.
- State advances on the region's own subjective clock (§7.8). When the party or a coupled region makes contact, the region catches up by the reconciled amount in coarse steps, applying data-defined rules written per day: predation, breeding, migration, faction conflict, harvest, trade.
- Player actions feed back: clearing a lair reduces a population; over-hunting collapses it; trading raises prosperity; unpaid factions turn hostile.
- Outputs that the player can observe: encounter tables, encounter sizes, shop prices and stock, rumors, available generated quests, and some map changes (a ruined village, a new camp).
- Catch-up is deterministic and cheap: a region catching up a subjective year runs in a few milliseconds, and only regions in contact do any work.
- The editor exposes region state and rules for inspection and override.
- **Forward compatibility with agents.** The long-term goal is that a limited set of important non-player characters (rulers, rivals, quest-givers, notable monsters) act with free agency: they hold goals, move between regions, and change region state on the tick, so the world stays dynamic in single player. The extent is decided when the regional engine exists and its cost is known. To keep that door open, the v1 region model must satisfy three requirements: a region's state is changed only through typed events that any source (rule, player, or future agent) can emit; named entities can be located in a region and referenced by ID from quests and rumors; and the tick is ordered and deterministic so agent actions can be inserted as one more phase.

### 9.3 Story building engine
- **Authored quests** are graphs of nodes (dialogue, choice, check, flag set, reward, spawn, map change) with conditions on quest flags, party state, the relevant holder's clock, and ecosystem state. Deadlines are counted on the clock of the holder that set them (§7.8). Written in data, edited in the editor with a graph view.
- **Generated quests** are instantiated from templates (clear, fetch, escort, deliver, investigate, rumor chain, bounty) that bind to generated or authored places, monsters, and NPCs, and to current ecosystem state. Templates declare what they need and what they change.
- Dialogue is data with variable substitution. No free-text generation at runtime.
- Every quest, authored or generated, is completable, failable, or expirable, and the engine can prove that statically for authored graphs (no dead-end nodes).

## 10. Tools: editor and content management

- The editor lives in the game binary and is opened from the main menu or with a launch flag. It edits the same data files the game loads.
- Views: tile map editor (paint terrain, walls, doors, objects, triggers), region view (ecosystem state and rules), quest graph editor, data table editor (monsters, items, spells, races, classes), procgen panel (generate, regenerate a layer, lock tiles), and a playtest button that drops a party onto the current tile.
- Content is organized as **packs**: a directory with a manifest, data files, and assets. The base campaign is a pack. A mod is a pack. Packs declare dependencies and load order; later packs override earlier ones by ID.
- Every data file has a schema and a version. Loading validates against the schema and reports every error with file and line, never panics.
- Assets (sprites, tilesets, audio) are referenced by pack-relative path. Missing assets render as a visible placeholder, never a crash.
- The editor and the loader are the same code path. If the editor can save it, the game can load it.

## 11. Constraints

### 11.1 Technical
- **Engine**: Bevy, latest stable, explicitly exempt from the N-1 rule (D9). Verified 2026-09-11: 0.19.0 released 2026-06-19, 0.19.1 released 2026-08-13, MSRV 1.95.0, itself edition 2024, MIT OR Apache-2.0. Relevant 0.19 facts: resources are now components on singleton entities; text moved to the Parley engine; `bevy_ui` widgets and Feathers gained text and number inputs, dropdowns, list views, and scrollbars. Feathers in game (owner, 2026-09-20, amending ARCH A11): start experimenting with Feathers in the game, see how the default styles work, and see if modern fonts can replace the pixel-art based fonts; if the in-game integration goes well, the editor's toolkit is reconsidered. Both crates call themselves experimental, and Feathers' own documentation aims it at editors rather than games, so the experiment is bounded and measured; the `bsn!` macro exists but there is no `.bsn` asset loader yet, so scene files are not a content format for Omnis; the `audio` feature no longer implies `2d` or `ui`. No 0.20 date is announced; the stated cadence is three to four months, so expect one migration around Q4 2026.
- **Language**: Rust, edition 2024, stable toolchain (1.98 installed, MSRV follows Bevy). No nightly.
- **Rendering**: Bevy 2D only. The display target for the first UI is the owner's 5120×1440 ultrawide, with 16:9 4K, 1440p, 1080p, and 720p supported (D20). Target 60 fps at 1080p on integrated graphics from 2020 onward remains the performance floor; as built, the placeholder presentation is an integer-scaled 720-row canvas whose width follows the window, so every monitor gets the largest whole multiple of 720 rows that fits and the canvas fills the width at that multiple. That pipeline is not a goal: the end state is a modern, resolution-independent presentation (D26), and nothing is kept for the sake of a pixel-art feel.
- **Determinism**: the simulation crate has no dependency on rendering, input devices, wall-clock time, or floating-point-sensitive math where cross-platform determinism matters (use integer or fixed-point for rules). All randomness comes from seeded, counted generators stored in the save.
- **Data formats**: RON for all content and saves (D15); binary only for assets. Formats are versioned and migratable. Large saves may be compressed on disk, but the uncompressed form is always plain RON.
- **Dependencies**: Bevy and its ecosystem aside, follow `CLAUDE.md`: pin versions, prefer N-1, nothing under 30 days old, minimize count, prefer own helpers. Audit with Socket before adding.
- **Code shape**: files at or under 1000 lines; composition of small crates or modules; simulation, content, editor, and presentation are separable.
- **Testing**: rules and generation are tested as pure functions with real data, not mocks. Golden tests for procgen seeds. Save round-trip tests.

### 11.2 Legal
- **Might and Magic** is Ubisoft property. Omnis is inspired by its mechanics and structure. It must not reproduce maps, names, monsters, items, text, art, music, or fonts from any Might and Magic title. The C64 manual in `docs/` is a design reference only and is not redistributed in releases.
- **SRD 5.1** is CC-BY-4.0. Any content adapted from it ships with the required attribution text (see `docs/dnd.srd.5.1/Legal.md`) in the base pack and in the game's credits. Product identity terms excluded from the SRD are not used.
- **Omnis code**: MIT OR Apache-2.0, matching Bevy. **Omnis content**: CC-BY-4.0. Contributed mods keep their authors' licenses.
- Fonts, sprites, and audio must be original or under licenses compatible with the above, with per-asset attribution recorded in the pack manifest.

### 11.3 Project
- Small team, possibly one person, with agentic assistance. Scope is controlled by the phase plan in §13, not by ambition.
- Everything in `data/` for the base campaign is authored with the shipped editor. Hand-editing data files is allowed; bypassing the format is not.
- Security posture from `CLAUDE.md` applies: validate all loaded data, treat mod packs as untrusted input (path traversal, size limits, no code execution), filter output.

## 12. Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R1 | Scope: four large subsystems plus an editor on top of a full RPG | High | Fatal | Phase plan (§13); each phase ships a playable build; no phase starts until the prior one's success criteria are met. |
| R2 | Bevy churn: breaking changes every release, ecosystem crates lag | High | Medium | Isolate Bevy behind thin presentation and input layers; the simulation crate has no Bevy dependency; budget one migration per release. |
| R3 | Crawler adaptations (spell points, six-slot party, stacks, no grid) unbalance SRD content that was tuned for slots, 4–5 characters, and a grid | Medium | High | Keep adaptations to the §8.2 list; decide reconciliation rules (§8.3) before content authoring; automated encounter simulations over the SRD bestiary; small playable slice before scaling content. |
| R4 | IP claim from the Might and Magic rights holder | Low | High | §11.2 constraints; review all names and art before release; no "Might and Magic" in the title or marketing. |
| R5 | Editor-first delays the fun | Medium | Medium | The editor's first version is minimal (tile paint, place object, set trigger, playtest). Fancier views follow content needs. |
| R6 | Determinism drifts (float math, hash ordering, platform differences) | Medium | High | Rules use integer math; ordered collections in the sim; cross-platform golden tests in CI. |
| R7 | Procgen output is bland | Medium | Medium | Layered generation with authored set-piece insertion; ecosystem and quests give generated places meaning. |
| R8 | Mod packs as attack surface | Medium | Medium | Packs are data plus rule formulas; strict schema validation; size and path limits; formulas run in a sandboxed integer-only scripting engine with no I/O, no functions, and bounded cost (architecture §5); general scripting for mods stays deferred. |
| R9 | Save incompatibility across builds | Medium | Medium | Versioned formats with migrations and round-trip tests from day one. |
| R10 | Art pipeline for a first-person viewport (wall permutations, distance layers) is larger than expected | Medium | Medium | Fixed detail depth (D16) and a strict tileset contract; the horizon band is procedural, not sprite art; procedural placeholder art so systems can be built before art exists. |
| R11 | Subjective time confuses players or breaks quest logic (a deadline that means different things to two holders) | Medium | Medium | Every clock-dependent rule names its holder; the journal shows deadlines on the party's clock with the drift range; a debug view of all clocks in dev builds; playtest the greeting exchange early. |
| R12 | A growing turn budget and automation trivialize combat: spell points spent through several bonus actions in one turn, or runbooks that solve every fight unattended | Medium | High | The budget is data and starts at the SRD's one of each; every budget curve and bonus-action spell is measured over seeds against the SRD baseline before content ships (R3's simulations); a spell opts in to the bonus action explicitly (D24); monsters fight by runbooks too (D23). |
| R13 | A modern look and feel (D26) costs more art and interface work than the placeholder pipeline, for a small team | Medium | Medium | Placeholders keep the systems moving; Bevy's own interface crates are tried before anything is built by hand; the art direction is decided before content scales (§14); the tileset contract (R10) bounds what the viewport needs whatever the style. |

## 13. Phased scope and success criteria

Each phase produces a runnable build. Criteria are testable.

> **Note (2026-09-12):** implementation priorities differ from this phase order. Development runs as vertical-slice milestones in `tasks/TODO.md` (a playable app first, then one system at a time across the full scope) so the owner can test from the start. The phases below remain the long-range scope and exit criteria; revisit this section once the milestones settle.

### Phase 0 — Foundation
- Workspace layout, simulation crate with no Bevy dependency, data schema and loader, save round-trip, CI with tests on macOS and Linux.
- **Done when**: a data pack loads, validates, and round-trips through save and load with no loss; a golden procgen seed produces identical bytes on both CI platforms.

### Phase 1 — Crawler loop plus editor (first playable, D4)
- Party creation, one town with all services, one multi-level dungeon, outdoor area connecting them, movement, viewport rendering, turn-based mass combat with the turn budget and declared reactions (D21, D22), spells, items, leveling, conditions, subjective clocks with reconciliation on region entry, save and load.
- Editor: tile paint, objects, triggers, data tables, playtest. It is the last work of the phase: the loop is built first on hand-written data, then all Phase 1 content is re-authored in the editor.
- **Done when**: a new player can create a party, clear the dungeon, and return to town in under two hours without hitting a crash, a soft-lock, or an untestable rule; every rule has a test; the content has been re-authored in the editor and round-trips through it with no loss.

### Phase 2 — Procedural world
- Region generation for all layers; lazy generation on first visit; editor procgen panel with layer regeneration and tile locking.
- **Done when**: a seed produces a traversable world of at least 100 regions with towns and dungeons the player can complete; regenerating one layer preserves manual edits in the others.

### Phase 3 — Ecosystem
- Region state, tick rules, feedback from player actions, observable outputs in encounters, prices, rumors.
- **Done when**: clearing a lair measurably changes encounter tables and prices in coupled neighbouring regions on the party's next contact after a subjective week or more has passed, and the effect is visible to the player through rumors that report the teller's own elapsed time.

### Phase 4 — Story engine
- Authored quest graph format and editor view; generated quest templates bound to procgen and ecosystem; journal.
- **Done when**: the base campaign's main quest is authored as a graph with static completability proof, and generated quests appear in every generated town.

### Phase 5 — Base campaign and release
- A full world, opening story, balancing pass, mod pack documentation, attribution and licensing audit, packaged builds.
- **Done when**: external playtesters complete the opening act; a modder outside the team ships a working pack from the documentation alone.

### Later
- Free agency for a limited set of important non-player characters on top of the regional ecosystem (§9.2), extent set by measured cost. A sidecar party system for hirelings in massive battles (D10). Aging as an optional rule (D13). Networked co-op on the deterministic sim. Additional platforms. General scripting for mods (quest logic beyond the graph), using the same sandboxed engine the rule formulas already run on, if data-only proves insufficient.

## 14. Open questions

Answered questions have moved to the decisions log (D10–D19). Remaining:

- **Exact detail depth and internal resolution:** a 720-row canvas, 1280 wide or as wide as the window, with a 960×540 viewport since 2026-09-13 (D20): 2× on the 5120×1440 ultrawide with the viewport centred, 3× at 4K, 1× at 1080p with 180-px bars above and below. Detail depth is tileset data within D16's range: 6 in the placeholder dungeon set and 4 outdoors, with the 16×16 placeholder tiles, until the final art direction.
- **Component economy:** how gems and other reagents enter the world (found only, or also ecosystem-driven trade and crafting), and whether the highest spell levels also need a per-day cap on top of components.
- **Preparation (D24):** what preparing a spell costs (an action in a fight, minutes outside one, spell points paid when preparing or when releasing), how many prepared spells a caster can hold, whether holding one needs concentration as the SRD's readied spell does, and whether a prepared spell survives from one fight to the next.
- **Turn-budget curves (D21):** which classes, items, and spells widen the budget, at which levels, and with what ceiling. Decided with numbers from the encounter simulations (R12).
- **Runbook scope and timing (D23):** whether a party-level switch changes all six members' runbooks at once, and in which milestone auto play and runbooks land relative to the turn budget and declared reactions.
- **Modern presentation (D26):** what modern means for the viewport (higher-resolution 2D art on the same tile contract, smooth rather than integer scaling, or reopening the 3D non-goal), the art direction, and who makes the art.
- **Non-goal horizons:** browser builds, 3D rendering, and language-model-generated content are listed as v1 non-goals. Owner to state which are rejected outright and which are deferred.
