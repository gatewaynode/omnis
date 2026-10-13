# Continuity notes

Written 2026-10-12, after M8.5 step 1c (a stair's direction) was built, waiting for the owner's acceptance. Step 3 is accepted. Rewrite this
file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes).
- **M8.5 build order:** 1a, 4, 4b, 1b, 3, 2, 5, 6. The owner moved 3 before 2 on 2026-10-12.
  - Accepted: 1a, 4, 4b, 1b. 1b's acceptance was provisional: its stair and ladder walk is repeated in step 3's
    script.
  - Step 3 accepted 2026-10-12 ("tasks/acceptance/m8.5-3.md accepted"), with 1b's walk in it. No window fps
    reading was recorded.
- **Step 3 commits:**
  - `a86204e`, the plan;
  - `b46a54a`, `--timing`;
  - `3e9e1e6`, the fills;
  - then the docs commit.
- **The owner's answers for step 3:**
  - fills first under the gizmo lines, then measure;
  - the levels above and below solid, hidden where covered;
  - the automap stays on the cone (presentation-PRD §12.5 still open).

## Step 3 as built
- **`src/fill.rs`** (Bevy-free). `fills(map, map_id, level, world, ground) -> Vec<Tri>` fills:
  - each cell's ground, as `Ground::height`'s four triangles;
  - wall faces, from their feet to their top;
  - doors (an open one leaves its opening under the lintel);
  - blocks and ceilings;
  - cliff faces.
- **Openings where levels join:** no ceiling over an Up stair or a ladder with a level above; no floor under a Down
  stair or a ladder with a level below; a hole keeps only its rim. Stairs and ladders get no fill.
- **Shared with the lines:** the walk over edges and splits lives in `geometry::edges` and `geometry::splits`, so
  `extract` and `fills` agree. The door constants are `DOOR` and `door_head`.
- **Scene and renderer:**
  - `Scene::fills` holds the party's level and its neighbours.
  - The renderer draws them as one `Mesh3d` (`render::mesh`), in an unlit black material with `cull_mode: None`.
  - `LINE_DEPTH_BIAS` is −0.002 on the gizmo config.
- **1b bugs fixed, red first:**
  - `Ground::patch` is `None` on a tile with no ground; walls had reached down to 0.
  - `geometry::flight`: a stair pair draws one flight.
- **Timing (`--timing`):** about 2 ms a frame before and after, at both sizes. So line meshes are deferred, and
  presentation-ARCHITECTURE §15.3 stays open. The offscreen numbers may understate the GPU's share.

## Next
1. **The owner walks `tasks/acceptance/m8.5-1c.md`** (a stair's direction). Fix whatever they find, each fix with a
   test written red first.
2. **Then step 2** (layout within a square), planned in plan mode with the owner's questions first. It brings forward
   the object schema from `tasks/plans/editor-v1.md` §E. Objects that block sight may need fills (`fill.rs`).

## 1c as built
- **Data:**
  - `MapDef.stairs: Vec<StairDef { level, x, y, rise }>`;
  - `omnis-data/src/stairs.rs`: `check` fills `LevelData.rises`;
  - `MapData::rise(level, x, y)` reads it.
  - A stair error discards the levels, as a join error does, so the corpus has its own `stairs.ron`.
- **Engine:** `apply::joined(map, at, join, heading)`: Up fires when `heading == rise`, Down when
  `heading == rise.opposite()`.
- **Client:**
  - `geometry::stair_at(rise, cell, t, s)` gives the treads and the slab;
  - `fill::stair` draws the slab;
  - the minimap's chevron is turned to the rise, and a stair down has a bar at its foot.
- **Fixture:** the tower's stair is at (2, 0), rising west, its foot (3, 0) through an east opening; the courtyard
  stair rises north.

## Facts
- **Height:** `MapDef.heights`; `Terrain.climb` (default 2); `/` ramps; `MAX_HEIGHT` 4096; `MAX_RAMP` 16. Sight uses
  `WALL` 8 and `EYE` 4. A level's cells carry `base` plus height.
- **Levels:** `_` is no ground. Joins are Up, Down, Ladder and Hole. Fixture `crates/omnis-sim/tests/packs/levels`
  (`levels:map:tower`, with the hole at (5, 2)).
- **Captures** (scripts in the scratchpad, output in `.omnis/m85/`):
  - `shotl.sh name x y facing walk` puts the party on the tower, level 0 only;
  - `shot.sh` puts it on the hills;
  - `time.sh label` runs the release timings.
- **Mutation runners** (scratchpad): `mutate1c.py` (1c, three crates), `mutate3.py` (step 3), `mutate1bc.py` (client), `mutate1b.py` (engine).

## Open, carried over
- **The editor's toolkit is undecided:** A11 says Feathers, presentation-PRD §8 says `bevy_egui`.
- **`assets/README.md` font rows:** the owner's file; report only, never edit.
- **P6 colours:** unconfirmed. Step 5 revisits the colour roles, including the join colours and the fill colour.
- **Dependencies:**
  - the rhai Socket re-audit (ask the owner when);
  - `smartstring` RUSTSEC-2026-0249;
  - `ttf-parser` RUSTSEC-2026-0192;
  - Bevy 0.20, not before 2026-11-07.
- **The owner's asset work** (`.gitignore`, `assets/`) is theirs. Never stage it.

## Pins
- Gate `tests passed 708 failed 0 ignored 12`, VERIFY-GREEN, at `c679eb1`.
- Replays unmoved: walk `b0220a6eefc5e6e3`, fight `f130927d1f09f19f`.
- `SAVE_SCHEMA` 7, `PROTOCOL` 2. MCP command schema `oneOf` 12; schema proof 96 instances, 145 offered.
- Sentrux quality 8957.

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, with `cargo fmt --all` first. Workspace clippy
  with `--all-targets --features omnis-vector/test-fixtures -- -D warnings` before every commit. The vector crate's
  test target is `integration` (`--test integration`).
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1` in the background, with no edits while it runs.
- **Mutations:** `python3 -u` to a file, never under `timeout`. Check that the sources are restored.
- **Commits:** stage by name (`git add` new files, then `git commit -- <paths>`), never `.claude/`. Unfinished
  work before a compact is committed as `WIP`, never stashed.
