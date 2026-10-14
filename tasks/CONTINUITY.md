# Continuity notes

Written 2026-10-14, before a hard restart for an OS upgrade. Rewrite this file every time it is used. Durable
knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, clean, not pushed (the owner pushes).
- **M8.5 build order:** 1a, 4, 4b, 1b, 3, 2, 5, 6, with 1c added before 2.
  - Accepted: 1a, 4, 4b, 1b, 3.
  - **1c (a stair's direction) accepted 2026-10-14:** "Manually tested stair, hills and ladders, acceptable
    functionality" (`f7f55be`).
- **The hills' crests, fixed and confirmed (2026-10-14):**
  - The finding: a rise's top had no line, so the black ground against the black sky read as a void.
  - The fix (`60e8b05`): `Ground::crests(eye, reach)` in `omnis-vector/src/ground.rs` returns each edge of the
    ground's triangles between one that faces the eye and one that faces away. That covers diagonals inside a cell,
    and sides only where the ground runs on (both cells carry the side at the same heights).
  - `render::draw` draws them each frame from the camera's `Transform`, in a provisional pale `CREST` colour,
    whether or not the grid is shown.
  - Tests in `tests/ground.rs`: none on flat ground; the terrace's lip from above and below; one along a side; none
    along a split. Mutations 4 of 4 caught.
  - The owner confirmed it fixed in the window.
- **Gate:** `tests passed 712 failed 0 ignored 12`, VERIFY-GREEN, at `60e8b05`. Sentrux rules pass, quality 8960.

## Next
1. **Held: reassess the editor** before step 2. The owner (2026-10-14): "We should stop and reassess the editor
   decisions and directions before we commit to that path", then "hold the editor decisions for later".
   - The review's findings and the three open options are in `tasks/TODO.md`, under "Open, outside the milestones",
     in the item "Reassess the editor".
   - Ask the owner when to take it up. Do not plan step 2 until it is done.
2. **Then step 2** (layout within a square), planned in plan mode with the owner's questions first.
   - It would take the parts of the editor plan's §E that still fit.
   - Objects that block sight may need fills (`fill.rs`) and crests.

## Built in M8.5 so far (for orientation)
- **Height (1a, 4):** heights per cell in eighths, ramps `/`, cliffs derived at load, and the ground drawn smooth
  where walkable (`ground.rs`), with cliffs hatched.
- **Levels (1b):** `MapDef.levels`, `_` for no ground, and joins (stairs, ladders, holes).
- **Solid fills (3):** `fill.rs`, drawn under the gizmo lines.
- **A stair's direction (1c):**
  - `MapDef.stairs: [(level, x, y, rise)]`, required for every stair (`omnis-data/src/stairs.rs`);
  - a stair up is taken from its foot, a stair down from its head; from the side it is an ordinary step
    (`apply::joined`).
- **Camera pitch (4b):** kept in `shell::movement::View`.

## Facts
- **Fixtures:**
  - heights: `crates/omnis-sim/tests/packs/heights` (`heights:map:hills`);
  - levels: `crates/omnis-sim/tests/packs/levels` (`levels:map:tower`; its stair is at (2, 0) rising west,
    and the courtyard stair at (4, 3) rises north).
- **Captures:** scripts in the session scratchpad, which an OS restart may wipe. Output goes to `.omnis/m85/`.
  - To rebuild them: copy a fixture pack, set `entry: Some("<map>")` in its `pack.ron` and the map's `start`.
  - Then run `target/debug/omnis --seed 1 --pack packs/base --pack packs/test --pack <copy> --screenshot
    <png> --walk 0`. `--walk` needs a frame count.
  - `--timing` (with `--screenshot`) reports frame times.
- **Pins:**
  - replays unmoved: walk `b0220a6eefc5e6e3`, fight `f130927d1f09f19f`;
  - `SAVE_SCHEMA` 7, `PROTOCOL` 2;
  - MCP command schema `oneOf` 12; schema proof 96 instances, 145 offered.

## Open, carried over
- **The editor reassessment** (above), including its toolkit (A11 says Feathers, presentation-PRD §8 says
  `bevy_egui`).
- **`assets/README.md` font rows:** the owner's file; report only, never edit.
- **P6 colours:** unconfirmed. Step 5 revisits the colour roles: joins, fills, the crest line.
- **The automap** still records the forward cone (presentation-PRD §12.5).
- **Dependencies:**
  - the rhai Socket re-audit (ask the owner when);
  - `smartstring` RUSTSEC-2026-0249;
  - `ttf-parser` RUSTSEC-2026-0192;
  - Bevy 0.20, not before 2026-11-07.
- **The owner's asset work** (`.gitignore`, `assets/`) is theirs. Never stage it.

## After the OS upgrade
- **Check the toolchain still builds:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo build`.
  - Command Line Tools may need reinstalling (`xcode-select --install`).
  - Then run the gate once to confirm 712 tests.
- **The scratchpad** (gate logs, mutation runners, capture scripts) is temporary and may be gone. Nothing in it is
  needed; rebuild the capture script from the Facts above.

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, with `cargo fmt --all` first. Workspace clippy
  with `--all-targets --features omnis-vector/test-fixtures -- -D warnings` before every commit. Test targets are
  `--test integration`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1` in the background, with no edits while it runs.
- **Mutations:** `python3 -u` to a file (or a sed loop with a backup), never under `timeout`. Check that the sources
  are restored.
- **Commits:** stage by name (`git add` new files, then `git commit -- <paths>`), never `.claude/`. Unfinished
  work before a compact or a restart is committed as `WIP`, never stashed.
