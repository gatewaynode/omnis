# Continuity notes

Written 2026-10-12, after M8.5 step 1a was built, with its acceptance waiting for the owner. Rewrite this
file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes). `vector-adoption` is merged into `main` (owner,
  2026-10-12: CI green and manual tests passed).
- **TODO was reorganised at the owner's request:**
  - a summary at the top;
  - M8.5's six steps in dependency order: the 3D format, layout within a square, occlusion, elevation drawn,
    the colour scheme with biosphere palettes, then mana data;
  - the biosphere's living things moved to M10;
  - the open work gathered under "Open, outside the milestones".
- **M8.5 step 1a, elevation: built, acceptance waiting.**
  - Plan `tasks/plans/m8.5-elevation.md`, with "As built". Acceptance script `tasks/acceptance/m8.5-1a.md`.
  - Commits: `9364813` (TODO), `6039921` (plan), `0e57065` (data), `f08f7ec` (engine, views, client), then
    docs. `54a8001` is the owner's `docs/locations/` document, committed unedited.
  - Owner answers:
    - height per cell, plus ramp edges;
    - eighths of a cell (a wall is 8);
    - the climb allowance per terrain, set by the terrain entered;
    - drops limited like climbs;
    - ramps up to 16;
    - hills hide, and height extends the reach.
- **Next:**
  - The owner accepts 1a.
  - Then plan **1b, the Z axis**, with the owner: levels above and below, how they join, and overlaps such as a
    bridge over a road or a cave under a hill.
  - The rhai Socket re-audit is still first in the open list. Ask the owner when, since they started M8.5.

## Facts
- **Height in the format:**
  - `MapDef.heights` (rows of integers; none is flat), `Terrain.climb` (default 2), `/` ramp edges.
  - The derived fields are `Cell.{height, ramps, cliffs}`.
  - Limits: `limits::MAX_HEIGHT` 4096 and `MAX_RAMP` 16.
- **Sight (`visibility.rs`):** `WALL` 8 and `EYE` 4. The functions are `line_of_sight` (now pub), `above`, and
  `depth(world, data, map)`.
- **Rule value:** `sight_rise_per_tile` is 8, in `packs/base/data/rules/sensing.ron`.
- **Protocol 2 additions:** `ViewTile.height`, `EdgeView::Ramp`, `BlockReason::Cliff` (the client says "Blocked:
  too steep"), and `/` in `map.text`.
- **Client:**
  - `Views.sight` is the viewport's depth, which the fog uses.
  - The mirror reads `Cell.cliffs`.
  - The client does not draw height yet; that is step 4.
- **Fixture pack:** `crates/omnis-sim/tests/packs/heights` (map `heights:map:hills`). Load it after base and
  test, with `--pack` for the game, the CLI and the bridge.
- **Sight behaviour seen in testing:**
  - The foot of a two-wall drop is hidden by the hilltop's own edge.
  - A one-tile wall is seen around at its end.

## Open, carried over
- **Unchanged since P9:**
  - The editor's toolkit is undecided: A11 says Feathers, presentation-PRD §8 says `bevy_egui`.
  - `assets/README.md`'s font rows: the owner's file, report only.
  - P6's colour calls are unconfirmed.
- **`tasks/BACKLOG.md`:** the colour-deficiency item moved into M8.5 step 5.
- **Dependencies:** the rhai Socket re-audit; `smartstring` RUSTSEC-2026-0249; `ttf-parser`
  RUSTSEC-2026-0192 (Linux only); Bevy 0.20 not before 2026-11-07.
- **The owner's asset work** (`.gitignore`, `assets/`) is theirs. Never stage it.

## Pins
- Gate `tests passed 619 failed 0 ignored 12`, VERIFY-GREEN, at `f08f7ec`.
- Walk replay `b0220a6eefc5e6e3`, fight `f130927d1f09f19f`. `SAVE_SCHEMA` 7, `PROTOCOL` 2, 25 ops, MCP 25
  tools.
- Sentrux (scan `crates/`): rules pass, quality 8928.

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, with `cargo fmt --all` first. The test
  binaries are `--test integration` (omnis-data, omnis-sim, and omnis-vector with `--features test-fixtures`).
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1` in the background, no edits while it
  runs. `docs/api.md` is compiled by a drift test, so it counts as source.
- **Mutations:** `python3 -u` to a file, never under `timeout`. Check the sources are restored (grep for the
  planted strings).
- **Replays:** when a pack's hash moves them, prove the play is unchanged first. Restore `world.packs` to the
  replay's and compare fingerprints. Then `cargo test -p omnis-sim --test integration rebaseline -- --ignored`.
- **Commits:**
  - Stage by name, never `.claude/`.
  - Unfinished work before a compact is committed as `WIP`, never stashed.
  - The owner's world documents are committed as they appear, unedited, each in its own commit.
- **Captures:** make the directory first (`.omnis/m85/`).

## Carry-over
- **Large files:** `bake.rs` 934, `omnis-vector/tests/screens.rs` about 840, `arena.rs` (vector) 830,
  `save_and_replay.rs` 810, `shell/screen.rs` 791, `screen/debug.rs` 747, `map.rs` (data) 714.
