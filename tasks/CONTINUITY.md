# Continuity notes

Written 2026-10-12, after the owner accepted M8.5 step 4b (camera pitch). 1a accepted too; 1b is being planned. Rewrite
this file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes). `vector-adoption` was merged into `main` on 2026-10-12.
- **M8.5 build order (owner, 2026-10-12): 1a, 4, 4b, 1b, 2, 3, 5, 6.** 4b, camera pitch, was added at step 4's acceptance (the owner may move it). The step numbers are kept. Step 4 moved forward
  after the owner walked the 1a fixture and met cliffs they could not see.
- **1a, elevation in the rules:** accepted 2026-10-12. Acceptance `tasks/acceptance/m8.5-1a.md`.
  - **Fix after the owner's walk (`fc5b0e4`):** the window held the party at a cliff in silence. Now
    `Outcome.bumped` produces "Blocked: too steep", once per contact.
- **4, elevation drawn:** accepted 2026-10-12 ("rough as expected"; no vertical pan makes the hills hard to navigate). Plan `tasks/plans/m8.5-elevation-drawn.md` (with "As built"), acceptance
  `tasks/acceptance/m8.5-4.md`.
  - Owner answers:
    - lines now, with fills in step 3;
    - smooth where walkable;
    - relief with step 5;
    - the minimap shows cliffs and ramps.
  - Commits: `6cf5055`, `5c0a015`, `9e860f3`, `3a57aad`, then docs.
- **4b, camera pitch:** accepted 2026-10-12. Plan `tasks/plans/m8.5-camera-pitch.md` (with "As built"), acceptance
  `tasks/acceptance/m8.5-4b.md`. Pitch in `shell::movement::View` (never in `Pose`), `look.rs` pure, `render::rotation`,
  setting `level_on_step`, Level button, PageUp/PageDown/Home.
- **Next:**
  - Eye height and FOV: the owner raised no change at 4b's acceptance (§12.8 stays open).
  - Then plan **1b, the Z axis**, with the owner: levels above and below, how they join, and overlaps.
  - The rhai Socket re-audit is still first in the open list; ask the owner when.

## Facts
- **Height in the format:** `MapDef.heights`, `Terrain.climb` (default 2), `/` ramp edges.
  - The derived cell fields are `Cell.{height, ramps, cliffs}`; cliffs are directional.
  - Limits: `MAX_HEIGHT` 4096, `MAX_RAMP` 16.
- **Sight (`visibility.rs`):**
  - `WALL` 8, `EYE` 4. The functions are `line_of_sight`, `above` and `depth(world, data, map)`.
  - The rule value `sight_rise_per_tile` is 8 in `packs/base/data/rules/sensing.ron`.
- **Client height (`omnis-vector`):**
  - `ground.rs`:
    - `Ground::from_map`; corners are shared across `joined` edges (no wall, no cliff from either side), each taking
      the group's mean;
    - centres sit at the cell's height;
    - four triangles per cell;
    - `eye`, and `ease_eye` (0.1 s, snapping for a jump of more than 1.0 or under reduced motion).
  - `geometry::extract(map, id, world, &ground)`: `SegKind::Cliff` (hatching every quarter cell) and
    `SegKind::Ramp` (4 treads).
    - Walls run from the lower foot to the higher centre + 1.
    - A copy of the old flat extractor in `tests/geometry.rs` is the reference.
  - `render.rs`: `Lines` holds the `Ground`; `follow` eases y. The colours for Cliff and Ramp are provisional.
  - `minimap.rs`: `CLIFF` and `RAMP` are read from the cells of known tiles.
  - The binder's `Outcome.bumped` and `pressed` feed the cliff line in `shell/movement.rs`.
- **Fixture pack:** `crates/omnis-sim/tests/packs/heights` (map `heights:map:hills`, `entry: None`). Test helper
  `common::heights_data()` in `omnis-vector`.
- **Captures:** `--screenshot` starts at the pack entry.
  - For the fixture I copy the pack into the scratchpad with `entry: Some(...)` and a `start`, then run
    `./target/debug/omnis --seed 1 --pack ... --screenshot .omnis/m85/<name>.png --walk N`.
  - The script is the scratchpad's `shot.sh`.
  - The dev socket doesn't answer from a tool shell (the window doesn't tick).

## Open, carried over
- **Unchanged since P9:**
  - The editor's toolkit is undecided: A11 says Feathers, presentation-PRD §8 says `bevy_egui`.
  - `assets/README.md`'s font rows: the owner's file, report only.
  - P6's colour calls are unconfirmed.
- **Dependencies:** the rhai Socket re-audit; `smartstring` RUSTSEC-2026-0249; `ttf-parser` RUSTSEC-2026-0192
  (Linux only); Bevy 0.20 not before 2026-11-07.
- **The owner's asset work** (`.gitignore`, `assets/`) is theirs. Never stage it.

## Pins
- Gate `tests passed 647 failed 0 ignored 12`, VERIFY-GREEN, at `eeafac7`.
- Walk replay `b0220a6eefc5e6e3`, fight `f130927d1f09f19f`. `SAVE_SCHEMA` 7, `PROTOCOL` 2, 25 ops.
- Sentrux (scan `crates/`): rules pass, quality 8944.

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, with `cargo fmt --all` first. Run vector clippy
  with `--all-targets --features test-fixtures -D warnings` before every commit; the gate catches what is
  missed, but a commit already made has to be fixed forward.
- **Test binaries:** `--test integration` (omnis-data, omnis-sim, and omnis-vector with `--features
  test-fixtures`).
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1` in the background, with no edits while it runs.
- **Mutations:** `python3 -u` to a file, never under `timeout`. Check that the sources are restored.
- **Commits:**
  - Stage by name, never `.claude/`.
  - Unfinished work before a compact is committed as `WIP`, never stashed.
  - The owner's world documents are committed unedited, each in its own commit.

## Carry-over
- **Large files:** `bake.rs` 934, `omnis-vector/tests/screens.rs` about 840, `arena.rs` (vector) 830,
  `save_and_replay.rs` 810, `shell/screen.rs` 791, `screen/debug.rs` 747, `map.rs` (data) 714, `geometry.rs`
  (vector) 443.
