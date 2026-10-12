# Continuity notes

Written 2026-10-12, mid M8.5 step 1b (the Z axis). Stage 1 of 1b is committed (`5e0042f`); joins are not
walked yet. Rewrite this file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes).
- **M8.5 build order:** 1a, 4, 4b, 1b, 2, 3, 5, 6. Accepted so far: 1a, 4 and 4b (all on 2026-10-12).
- **1b plan:** `tasks/plans/m8.5-levels.md`, approved. The owner's answers:
  - layers in one map (chosen over stacked maps);
  - joins: stairs (paired), ladders and shafts (`Command::Climb`), holes (one way);
  - neighbouring levels drawn dimmer, with rules per level;
  - a new fixture pack; the shipped maps stay as they are.
- **Owner's mid-turn message "game masters book of":** it looked cut off. I said so and carried on. Ask what they
  meant.

## 1b, done in `5e0042f` (gate 650, VERIFY-GREEN)
- **Core:**
  - `omnis_core::Level(i8)` (`GROUND`, `is_ground`), exported from `omnis_sim::api`.
  - `Position.level` and `Place.level`, both `serde(default, skip_serializing_if = is_ground)`.
- **Data:**
  - **New module `omnis-data/src/levels.rs`:**
    - `LevelDef { level, name, base, layout, heights }`;
    - `LevelData { level, name, cells: Vec<Option<Cell>> }`;
    - `NO_GROUND = '_'` (not `.`, which the test dungeon's floor uses);
    - `parse(def)`, which returns every level, lowest first, with level 0 among them;
    - `beyond(levels, level, up)`;
    - join landing checks: "has no level above/below", "lands where there is no ground", "lands on a tile the
      party cannot enter", and for a ladder "leads to no ground above or below";
    - number checks, `MAX_LEVELS` 16 in `limits.rs`.
  - **`map.rs`:**
    - the grid parsing moved out to `levels.rs`;
    - `Join { Up, Down, Ladder, Hole }` and `Terrain.join`;
    - `MapDef.levels`;
    - `Portal.level` and `to_level`;
    - `_` is refused as a terrain glyph;
    - placements on a missing level are refused.
  - **`loader.rs`:**
    - `MapData.levels` replaces `cells`;
    - `cell(level, x, y)`, `inside`, `level()` and `beyond()`;
    - `portal_at`, `encounter_at`, `site_at` and `marker_at` take a level;
    - `ResolvedPortal.level` and `to_level`, with a check that `to_level` exists.
  - `FixedEncounter`/`ResolvedEncounter.level` and `Site`/`ResolvedSite.level`.
  - Unit tests are in `levels.rs`. The bad-pack corpus has a new `broken/data/maps/levels.ron`.
- **Engine:**
  - Everything reads `pos.level`: `landing` (`BlockReason::NoGround` for a step into no ground), doors, visits,
    the cone, sense, rest, services and the load check.
  - Sight is per level through a private `Sheet`. Cells with no ground are transparent, with `NO_FLOOR =
    -(1 << 40)`.
- **Save (a departure from the plan, to report):**
  - **No `SAVE_SCHEMA` bump.** Other levels go in new fields that are left unwritten when empty:
    - `MapState.level_doors: BTreeSet<(Level, u16, u16, Facing)>`, with `toggle_door` and `door_open(level, ...)`;
    - `Automap.levels: BTreeMap<(MapId, Level), tiles>`, with `record(map, level, ...)` and `tiles(map, level)`.
  - Saves and replays on level 0 come out byte-identical: the fingerprints did not move and nothing was
    rebaselined. The plan's v7→v8 migration is therefore not needed.
- **API (protocol 2, additive):**
  - `ViewportModel.level` and `level_name`;
  - `Op::MapText` and `Op::AutomapGet` take `level: Option<Level>`, chosen by `ops::level_of` (the party's level on
    its map, else the ground);
  - `Reply::Automap.level`;
  - `Event::Door.level`;
  - `DevCommand::Teleport.level`;
  - `query::door_open`, `automap` and `map_text` take a level;
  - `map_text` draws `_` for no ground;
  - the MCP schema: `Level` as an integer, `Teleport` with an optional `level`, and a `level` field on the `map_text`
    and `automap_get` tools.
- **Client:**
  - `collide::blocks(data, world, map, level, x, y, heading)`, mirroring `NoGround`;
  - `Ground::from_map(map, level)`, `joined(map, level, ...)` and `extract(map, map_id, level, world, ground)`;
  - `minimap::paint(map, level, known, pose)`;
  - the `Lines` and minimap caches are keyed by `(MapId, Level)`;
  - `SimWorld::automap(map, level)` and `door_open(map, level, ...)`;
  - text "Blocked: no ground";
  - **The debug Go still sends `Level::GROUND` (a placeholder until the client stage).**

## 1b, still to do (order)
1. **Engine joins (`apply.rs`):**
   - A step onto `Up`, `Down` or `Hole` gives `Moved` onto the cell, then a second `Moved` to the same (x, y) on the
     next level (`map.beyond`). Arriving never fires the far join, because only a step fires.
   - Add `Command::Climb(Vertical)` for a ladder: the landing must be ground the party can enter, else a refusal. It
     costs the destination terrain's `step_minutes` and is valid while exploring only.
   - Then: the vocabulary and docs tests, the MCP command schema, and `docs/api.md`.
   - Tests in `crates/omnis-sim/tests/levels.rs`.
2. **The fixture `crates/omnis-sim/tests/packs/levels`:** `levels:map:tower`, 7×5. The plan's table gives:
   - a cellar at −1;
   - the ground with a road on row 2;
   - an upper floor at +1 with the bridge (column 4, rows 1–3), the hole (4, 2) and a stair down (4, 3);
   - a roof at +2.
   It also needs a region file, as `packs/heights` has.
3. **The client:**
   - neighbour levels drawn dimmer at absolute heights;
   - the join `SegKind`s `Stair`, `Ladder` and `Hole`;
   - minimap marks and the level name;
   - the HUD's level;
   - Actions "Climb up" and "Climb down";
   - a debug Level choice;
   - the ceiling grid hidden under the level above;
   - the agreement test on the fixture's levels.
4. **Mutations (8 in the plan), Sentrux, captures.**
5. **Docs:**
   - ARCHITECTURE §7.1 and §6.2, sensing, the save note;
   - presentation-PRD X10 and X11;
   - presentation-ARCHITECTURE §5.6, §8.4, §10.1 and §10.2;
   - `docs/api.md` and its changelog;
   - the editor-v1 note and the M9 line;
   - acceptance `tasks/acceptance/m8.5-1b.md`;
   - the TODO review.

## Facts
- **Height:** `MapDef.heights`, `Terrain.climb` (default 2), `/` ramp edges, `MAX_HEIGHT` 4096, `MAX_RAMP` 16.
  Sight uses `WALL` 8 and `EYE` 4.
- **Client height (step 4):** `ground.rs` (corners shared across joined edges), `geometry.rs` (Cliff and Ramp),
  the camera's eye eased in `render.rs`.
- **Pitch (step 4b):**
  - `look.rs`;
  - `shell::movement::View { pitch, levelling, seen }`;
  - `render::rotation`;
  - the setting `level_on_step`;
  - the Level button, PageUp, PageDown and Home.
- **Fixtures:** `crates/omnis-sim/tests/packs/heights` (`heights:map:hills`). Captures use the scratchpad's
  `shot.sh` (a pack copy with `entry` and `start`, then `--screenshot`).
- **Scratchpad helpers:** `addlevel.py`, `levelfix.py` and `vfix.py` (regex call-site fixers), and the
  `mutate4b.py` pattern for mutation runs.

## Open, carried over
- **The editor's toolkit is undecided:** A11 says Feathers, presentation-PRD §8 says `bevy_egui`.
- **`assets/README.md` font rows:** the owner's file; report only, never edit.
- **P6 colours:** unconfirmed.
- **Dependencies:**
  - the rhai Socket re-audit (still first in the open list; ask the owner when);
  - `smartstring` RUSTSEC-2026-0249;
  - `ttf-parser` RUSTSEC-2026-0192;
  - Bevy 0.20, not before 2026-11-07.
- **The owner's asset work** (`.gitignore`, `assets/`) is theirs. Never stage it.

## Pins
- Gate `tests passed 650 failed 0 ignored 12`, VERIFY-GREEN, at `5e0042f`.
- Replays: walk `b0220a6eefc5e6e3`, fight `f130927d1f09f19f`, unmoved.
- `SAVE_SCHEMA` 7, `PROTOCOL` 2.
- Sentrux quality 8944 (at 4b).

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, with `cargo fmt --all` first. Workspace clippy
  with `--all-targets --features omnis-vector/test-fixtures -- -D warnings` before every commit.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1` in the background, with no edits while it runs.
- **Mutations:** `python3 -u` to a file, never under `timeout`. Check that the sources are restored.
- **Commits:**
  - Stage by name (`git add` new files, then `git commit -- <paths>`), never `.claude/`.
  - Unfinished work before a compact is committed as `WIP`, never stashed.
