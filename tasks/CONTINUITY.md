# Continuity notes

Written 2026-10-12, after M8.5 step 1b was accepted (provisionally: the owner will walk the stairs and ladders
once step 3's occlusion exists). Rewrite this file every time it is used. Durable knowledge lives in
`tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes).
- **M8.5 build order:** 1a, 4, 4b, 1b, 2, 3, 5, 6. Accepted: 1a, 4, 4b, 1b (all 2026-10-12). 1b's acceptance:
  "We are going to have to finish the occluding terrain before I can really test the ladders and stairs. Let's
  consider it working well enough to move on." The script was not walked; step 3's acceptance repeats its stair and
  ladder walk.
- **1b commits:** `5e0042f` (levels in data and every read), `f1e0cf5` (joins and `Command::Climb`), `1757521` (level
  names as text keys, owner's yes), `0d5d1f4` (the client), then the docs commit.
- **Owner, 2026-10-12:** "game masters book of" was world building in parallel; ignore it.

## 1b as built (beyond the plan)
- **No ground is `_`.** `.` is the test dungeon's floor. In `map.text`, `_` in a cell means no ground; on an edge it
  is still an open door. The owner was offered a space in the text output instead if it reads badly.
- **No save migration.** `MapState.level_doors` and `Automap.levels` are written only when non-empty; `SAVE_SCHEMA` 7.
- **Joins (`apply.rs`):** `landing` follows a portal, else a stair or hole (`joined`), into `Landing.through`; only a
  step fires. `climb_to` (pub(crate)) allows a climb when either tile is a ladder and the far tile is passable.
- **Client:** `sim::position` reads `here.position.level`; `scene.rs` (own level, neighbours at `NEIGHBOUR_DIM` 0.2,
  ceilings hidden under the level above, `Scene::strokes`); joins drawn in `geometry::joins`; minimap join marks and a
  caption left of the picture; `hud::place`; Actions "Climb up/down"; debug `DebugAct::Level`; `labels::level`
  ("Roof (2)", ground named by the map, `None` on a one-level map).
- **Fixture:** `crates/omnis-sim/tests/packs/levels` (`levels:map:tower`); the hole is at (5, 2) beside the bridge.

## Next
1. **Ask the owner first:** step 2 (layout within a square) or step 3 (occlusion) next? The order says 2, but the
   owner needs 3 to test the stairs and ladders. Do not assume.
2. Plan the chosen step in plan mode, the owner's questions before the plan. Step 2 needs 1b's level for an object;
   step 3 builds its fills under step 4's segments and `Ground` and now `scene.rs` (neighbour levels need fills too).

## Facts
- **Height:** `MapDef.heights`, `Terrain.climb` (default 2), `/` ramps, `MAX_HEIGHT` 4096, `MAX_RAMP` 16. Sight uses
  `WALL` 8 and `EYE` 4. A level's cells carry `base` + height.
- **Pitch (4b):** `look.rs`, `shell::movement::View`, `render::rotation`, setting `level_on_step`.
- **Captures:** scratchpad `shotl.sh name x y facing walk` (the tower as the entry map; level 0 only, walk through a
  stair to go up) and `shot.sh` (the hills). Output in `.omnis/m85/`.
- **Mutation runners:** scratchpad `mutate1b.py` (engine, `levels::` filter), `mutate1bc.py` (client).

## Open, carried over
- **The editor's toolkit is undecided:** A11 says Feathers, presentation-PRD §8 says `bevy_egui`.
- **`assets/README.md` font rows:** the owner's file; report only, never edit.
- **P6 colours:** unconfirmed (step 5 revisits colour roles, including 1b's provisional join colours).
- **Dependencies:** the rhai Socket re-audit (first in the open list; ask the owner when); `smartstring`
  RUSTSEC-2026-0249; `ttf-parser` RUSTSEC-2026-0192; Bevy 0.20 not before 2026-11-07.
- **The owner's asset work** (`.gitignore`, `assets/`) is theirs. Never stage it.

## Pins
- Gate `tests passed 678 failed 0 ignored 12`, VERIFY-GREEN, at `0d5d1f4`.
- Replays: walk `b0220a6eefc5e6e3`, fight `f130927d1f09f19f`, unmoved.
- `SAVE_SCHEMA` 7, `PROTOCOL` 2. MCP command schema `oneOf` 12; schema proof 96 instances, 145 offered.
- Sentrux quality 8953.

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, with `cargo fmt --all` first. Workspace clippy
  with `--all-targets --features omnis-vector/test-fixtures -- -D warnings` before every commit.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1` in the background, with no edits while it runs.
- **Mutations:** `python3 -u` to a file, never under `timeout`. Check that the sources are restored.
- **Commits:** stage by name (`git add` new files, then `git commit -- <paths>`), never `.claude/`. Unfinished
  work before a compact is committed as `WIP`, never stashed.
