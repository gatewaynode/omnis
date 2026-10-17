# Study: the save model of D28 (characters and interactions over a world the simulation owns)

A study, not a plan to build. It records the owner's direction (2026-10-17, PRD D28 and §7.7), what the code has
today that serves it, how today's state would split, and what must be answered or measured before a plan is
written. Nothing here is decided beyond the owner's answers.

## The direction (owner, 2026-10-17)
- The save centres on the player's characters, not the world.
- The player's interactions are captured and replayed against the world when loading.
- The simulation is in charge of permanent changes to the world through its own state management. The initial
  world and region designs are immutable in game, and minor changes are layered on top until thresholds or story
  points set new base world and region states.

Owner's answers the same day:
- **Replay span:** replay starts from the last base state, not from the new game.
- **World home:** the world's state lives in a world file apart from the character save.
- **A changed world:** left open (PRD §14). The question is what a replay does when the packs changed and an
  interaction no longer fits. One answer skips it and reports it; the other refuses the load unless a migration
  takes the pack to its new version.
- **Order:** the full log view is built now on an in-memory history, then 2c. Step 7 (saves by name) waits for
  this study.

Prompted by: the full log view, "kept in the save", met a save that is the engine's whole `World`, with no place
for anything of the client's. The owner named the coupling.

## What exists today and serves it
- **Determinism by construction (A14).** Every random draw comes from a named, seeded stream (`World.rngs`).
  - `Replay { seed, packs, settings, commands, fingerprint }` (`omnis-sim/src/replay.rs`) already rebuilds a game
    from its commands; the golden walk and fight replays prove it on every gate.
  - The window keeps such a log (`Session::save_log`, `.omnis/vector-session.ron`).
- **Commands name definitions by string id** (P2a–P2c): spells, items and features by string id, members by
  `CharacterId`, objects by placement id. So the interaction journal is name-based already, and does not suffer
  the renumbering step 7 was planned to fix.
  - Positions are still cells, and fixed encounters are still rows in their map file.
- **Events** are by string id too (P2c), and the engine keeps the last 4096 (`World.log`, unsaved).
- **Holders and clocks (M8, D25).** The party and every region keep their own clock, with contacts and
  reconciliation. A region's clock is a natural key for "the region's base state as of when".
- **Eras (§7.8, `EraId` in every `Clock`).** These are carried since M8 and never changed in v1. "A new base
  state" may be an era of a place, or may sit beside it; to be decided.
- **What is already layered:**
  - Object state (`MapState.objects`) and ground piles (`MapState.ground`) are kept only where they differ from
    the map's authored content, so they already have the shape of "layers over an immutable base".
  - Doors, cleared encounters, lore told and the automap are kept by map, in the save.

## How today's `World` would split (first cut, for discussion)
| Field today | Character save | World file: base | World file: layer |
|---|---|---|---|
| `party` (members, kit, stores, conditions, tactics) | yes | | |
| `party_time`, the party's clock | yes | | |
| `automap` (what the party knows) | yes | | |
| `position`, `mode` | yes (where the party stands) | | |
| `settings` (difficulty, the save rule) | yes | | |
| journal of interactions since the base | yes (new) | | |
| region and holder clocks, `contacts` | | as of the base | moved since |
| `maps` (doors, objects, ground, cleared, lore told) | | the authored packs | changes in play |
| `flags` (quest flags) | open: a quest the party carries, or a world fact | | |
| `rngs` (the streams) | open: replay needs the streams' state at the base | | |
| `bus` (signal subscriptions) | | rebuilt from data | |
| `seed`, `packs` | both: the character save names the world and base it replays on | | |
| `turn`, `log` | derived from the journal | | |

These rows are the study's main work. Three are hard:
- **The RNG streams.** A replay from a base must start every stream where it stood at that base. So the base
  records the streams, or the streams are re-derived from the seed and the base's identity.
- **Flags.** Some are the party's (a quest it took) and some are the world's (a bridge burned).
- **Two parties in one world.** Two character saves replaying over one world file must not both own the
  same change (co-op, D5).

## The parts to design
1. **The character save:** its schema, versioned and migrated as the PRD requires.
2. **The interaction journal:**
   - commands as they are, or a narrower player-interaction type;
   - each entry with the party's clock (so the full log can be reworded from it on load);
   - bounded by the next fold.
3. **The world file:**
   - one per world, or one per region;
   - the immutable base (the packs, by name and version);
   - the layers (diffs as `MapState` already keeps them);
   - the fold points, each a new base;
   - written only by the simulation.
4. **The fold rule:**
   - what is "minor" (a layer stays a layer);
   - which thresholds fold (a layer's size, a region's ecosystem change, time);
   - which story points fold (a quest node marks one, PRD §9.3);
   - a fold never loses an interaction a character save still needs: a save older than the fold replays on the
     new base, or keeps its old base.
5. **Loading:** the base, then the layers, then the character save, then the journal replayed. A refused
   interaction is handled by the open question in §14.
6. **What clients read:** views as today. The journal also yields the full log: the client words events
   replayed on load, so a change of language rewords the history.

## What it changes
- **M8.5 step 7 (saves by name, planned):**
  - the name table shrinks to what the character save holds by number (the party's kit and spells are already
    named in commands);
  - pack version and hash in `pack.ron` still stand (the base names its packs);
  - "no saving in a fight" still stands;
  - migrations answer the §14 question if the owner chooses refusal.
- **`SAVE_SCHEMA`, `World::fingerprint`, the replay pins:** the fingerprint splits into the character save's and
  the world's.
- **D17's save rule** (inn-only to anywhere): it governs when a character save may be written; the world file
  is written at folds and on quitting.
- **The editor (M5):** it writes bases. A world in play keeps the base it was folded from.
- **Co-op (D5) and other players' parties (§7.8):** several character saves over one world file.
- **The full log view (built now):** an in-memory history until the journal exists, then read from it.

## Questions for the owner (answered 2026-10-17 unless marked open)
1. **A changed world under a replay (PRD §14): migrate, then report.** On load the packs' declared migrations run
   first (step 7's plan). An interaction that still does not fit is listed in the load check, and the player
   chooses to load with it skipped or to cancel. A skip can move later draws, so the report says the outcome may
   differ from what was played.
2. **Quest flags: both, declared per flag.** A flag's scope is pack data: a party flag (a quest taken, an oath)
   lives in the character save and travels with the party; a world flag (a bridge burned, a lord dead) is a layer
   in the world file and folds.
3. **The world file: a directory per world.** A manifest (packs, seed, each region's current base) and one file per
   region; a fold rewrites one region's file; a region never visited has none (M9's lazy materialization). The
   manifest is written last and checked on load, so a torn write is detected.
4. **Characters between worlds: yes, as a horizon.** The character save is shaped to carry; the import is built
   later. It takes members, levels, kit and party flags; it leaves the journal, automap, clock and position. Kit is
   resolved by name against the new world's packs and what is missing is reported, as in 1.
5. **Folds: visible to the player.** A fold is announced (the world has moved on), part of the fiction; this suits
   story points first. **Open:** the numbers for "minor". Owner (2026-10-17): folds happen at thresholds in the
   larger world simulation, a simplified 4X engine driving the biospheres, settlements, regions and the world, so
   the numbers wait for its mechanics (M10). The save model provides the fold as a mechanism, triggered by a
   story point or by a caller, with no size or time rule of its own.
6. **A save older than the latest fold: keep named bases.** The world directory keeps every region base a character
   save still names and drops a base when no save names it, so loading an older save restores that moment. Disk
   is bounded by the saves kept times the regions they touched.

## Measurements before choosing
- Interactions per hour of play, from the window's session log (`.omnis/vector-session.ron`).
- Time to apply them on load, for an hour and for a long session, against a target load time.
- The size of the layers after a long session, from today's `MapState` diffs.

## Unaffected until then
M8.5 steps 2c, 5 and 6, the editor's plan, and every client screen.
