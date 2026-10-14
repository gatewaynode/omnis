# Continuity notes

Written 2026-10-14, after the OS upgrade and the editor's direction was settled. Rewrite this file every time it
is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes).
- **After the OS upgrade (Darwin 27):** the toolchain builds and links (both binaries relinked), and the gate is
  `tests passed 712 failed 0 ignored 12`, VERIFY-GREEN.
- **M8.5 build order:** 1a, 4, 4b, 1b, 3, 1c, 2, **editor (M5)**, 5, 6.
  - Accepted: 1a, 4, 4b, 1b, 3, 1c. The hills' crest line is confirmed fixed (`60e8b05`).
- **The editor, settled by the owner 2026-10-14** (`tasks/TODO.md`, "Reassess the editor", checked; M5):
  - `bevy_ui` in the neon style;
  - objects first in step 2 (data and engine, hand-written RON), then the editor, before steps 5 and 6;
  - D4 kept (test pack and fixture maps round-trip and are re-authored in it);
  - a top-down plan of one level with a 3D preview and Playtest;
  - maps plus a raw RON text editor for the other data files, checked by the loader before save;
  - an optional `notes` text on maps and levels (the RON writer drops `//` comments);
  - the build plan is written after step 2; `tasks/plans/editor-v1.md` is superseded for planning (§A and
    §E's kinds, placements, ids, limits and blocking are reusable).
  - Docs amended: PRD D26, §10, §11.1; ARCHITECTURE §8.2, §13, A11; presentation-PRD §2, §8; horizons; BACKLOG.

## Next
1. **Plan step 2** (layout within a square: scenery, items and beings) in plan mode, with the owner's
   questions first: kinds in `data/objects`, placements with stable ids, an offset inside the cell and a
   `level`; limits; `MapState.objects` in the save (`SAVE_SCHEMA` 7 → 8 with a migration, or fields left out
   when empty); blocking movement and sight; the automap; how the client draws them (fills, crests); what the
   editor will need from the schema.
2. Then Editor v1 (M5): write its plan against the finished format.

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
- **`assets/README.md` font rows:** the owner's file; report only, never edit.
- **P6 colours:** unconfirmed. Step 5 revisits the colour roles: joins, fills, the crest line.
- **The automap** still records the forward cone (presentation-PRD §12.5).
- **Dependencies:**
  - the rhai Socket re-audit (ask the owner when);
  - `smartstring` RUSTSEC-2026-0249;
  - `ttf-parser` RUSTSEC-2026-0192;
  - Bevy 0.20, not before 2026-11-07.
- **The owner's asset work** (`.gitignore`, `assets/`) is theirs. Never stage it.

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, with `cargo fmt --all` first. Workspace clippy
  with `--all-targets --features omnis-vector/test-fixtures -- -D warnings` before every commit. Test targets are
  `--test integration`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1` in the background, with no edits while it runs.
- **Mutations:** `python3 -u` to a file (or a sed loop with a backup), never under `timeout`. Check that the sources
  are restored.
- **Commits:** stage by name (`git add` new files, then `git commit -- <paths>`), never `.claude/`. Unfinished
  work before a compact or a restart is committed as `WIP`, never stashed.
