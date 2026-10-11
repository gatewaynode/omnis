# Continuity notes

Written 2026-10-11, after P9 was built, before the owner's acceptance. Rewrite this file every time it is used.
Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, not pushed (the owner pushes).
- **Accepted by the owner:** P2 to P8 (all 2026-10-11). **P9 is built and awaits acceptance** (`tasks/acceptance/p9.md`).
- **P9: `omnis-app` retired.** Plan `tasks/plans/p9.md`, with "As built".
  - Commits: `d165940` (plan), `736558c` (P9a), `1c017db` (P9b), then P9c (docs).
  - Owner answers:
    - Look and Map go to the backlog.
    - The binary is `omnis`; the crate stays `omnis-vector`.
    - Only `--frame-stats` comes across from the old flags.
    - ARCHITECTURE.md and PRD.md are edited, with the decision records marked.
    - The clock line's date, found during P9, goes to the backlog.
  - P9a: `[[bin]] name = "omnis"`; `--frame-stats` (vsync off, `LogDiagnosticsPlugin`); the error prefix `omnis:` and
    the window title "Omnis". Tests ported: the socket's `party.create` and `rules.set`, and `tests/game_io.rs` (the
    save's size cap and link refusal, a missing pack).
  - P9b: the crate deleted. The lock went from 544 to 534 packages: `omnis-app`, and nine packages from its local
    `dev` feature (`bevy_dylib`, the `notify` family, `mio`). `bevy_feathers` is no longer built but stays in the lock
    as an optional dependency of `bevy_internal`. Also: `verify.sh`, CI, justfile, `lint-sim.sh`; vector comments
    say "the retired client"; the MCP screenshot and screen-text wording.
  - P9c: the docs (ARCHITECTURE status, §2, §3, §4.7, §8 shrunk, §9, §11, §13, §14, §15, A11, A18; PRD status;
    presentation-PRD and presentation-ARCHITECTURE; README; api.md; knowledge; ATTRIBUTION's fonts), historical notes
    on six acceptance scripts, and the P9 acceptance script.
- **Next:**
  - The owner's P9 acceptance.
  - Then the rhai Socket re-audit (TODO).
  - Then the owner sets the order of: Look and Map, the clock line's date, the presentation-PRD §11 visual steps,
    and Bevy 0.20 (not before 2026-11-07).

## Facts
- `cargo run -p omnis-vector` and `just run` start `target/debug/omnis`. There is no `just run-app`.
- `--frame-stats` prints `bevy_diagnostic` `fps` and `frame_time` lines about once a second (checked offscreen).
- The HUD shows "Day 1 00:04" (from `Here.age`). The retired client's date line, "Year 1 day 40 14:20 night  age
  2y 40d", is in the backlog. Its source is at `git show 736558c:crates/omnis-app/src/panels.rs` (`clock_text`).
- The old crate's code is in git at `736558c` (the commit before its deletion).
- `crates/.sentrux/rules.toml` is gitignored. Its `clients` layer and three boundaries now name `omnis-vector`, but
  only on this machine.
- An offscreen `--screenshot` needs its directory to exist; without one it logs an I/O error and exits 0 (backlog).

## Open, carried over
- **From P9:** the editor's toolkit is open. A11 chose Feathers, and presentation-PRD §8 says `bevy_egui`; ARCHITECTURE
  §8.2 says so. Decide it when the editor is planned.
- **From P8:**
  - At 200% the party column's entries wrap.
  - `assets/README.md` (the owner's) still says the fonts are Feathers-only. Tell the owner; do not edit it.
    `ATTRIBUTION.md` is now correct.
  - A flag's Set is not reachable with the shipped packs.
- **From P6, unconfirmed calls:** down members red; dead members dim, with "dead" in red.
- **Dependencies and advisories:**
  - The rhai Socket re-audit (next).
  - `smartstring` RUSTSEC-2026-0249 to add to ARCH §13.
  - `ttf-parser` 0.25.1 RUSTSEC-2026-0192.
  - Bevy 0.20 not before 2026-11-07.
- **Assets no client uses since P9** (the owner's, report only): `Inter-Bold.ttf`, `AlegreyaSans-Bold.ttf`, and the
  baked tileset PNGs (the tileset data is still loaded and validated).
- **Backlog:**
  - Look and Map.
  - The clock line's date.
  - The screenshot directory.
  - Removing or editing a party member.
  - Unscheduled: `DevCommand::Pass`; the `data.*` ops; `scripts/mcp-probe.py`.
- **Small:**
  - Pack text is not checked for non-ASCII.
  - The health colours are not checked for colour deficiency.
  - The picture window draws every monster as the rat.
- The owner's asset work (`.gitignore`, `assets/`) is theirs. Never stage it. (The two `cutout-basic-*.svg` files
  were committed by the owner in `c660d8d`.)

## Pins
- Gate `tests passed 604 failed 0 ignored 12`, VERIFY-GREEN at `1c017db` (830 at `736558c`, before the deletion).
- Walk replay `78e57e238dcb93c0`, fight `48d160014c6a627a`; `SAVE_SCHEMA` 7, `PROTOCOL` 2, 25 ops, MCP 25 tools.
- Sentrux: scan `crates/`: rules pass, quality 8929. The drop from 8995 is all in the equality score, because the
  old crate's files left.

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, with `cargo fmt --all` first. The vector clippy
  lines, each written out (zsh does not split a variable):
  - `--lib`;
  - `--no-default-features --lib`;
  - `--features test-fixtures --tests`.
- **Tests:**
  - `--test integration` with `--features test-fixtures` (`tests/main.rs` lists the files); the session's parse tests
    are `--lib`.
  - App helpers: `common::app::{press_ui, tap, click, shell, config}`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1` in the background, a new file each run, no edits
  while it runs.
- **Captures:**
  - `cargo run -p omnis-vector -- --screenshot .omnis/p9/x.png --walk 60`. Make the directory first.
  - A fight or an open screen needs a temporary `PROBE-TEMP` hook (see the P8 notes in `tasks/plans/p8.md`).
- **Mutations:** `python3 -u` to a file, never under `timeout`; check that the sources are restored.
- **Commits:**
  - Unfinished work before a compact is committed as `WIP`, never stashed. Stage by name, never `.claude/`.
  - `git commit -- <path>` fails on a deleted directory. Stage by name and then commit without paths, after checking
    what is staged.
  - `git add` on a gitignored path warns but stages the rest.
- **`cargo fmt` reflows lines:** a Python `replace` written before formatting may silently miss; assert counts.

## Carry-over
- **Large files:** `bake.rs` 934, `omnis-vector/tests/screens.rs` about 840, `save_and_replay.rs` 810,
  `arena.rs` (vector) 830, `shell/screen.rs` (vector) 791, `screen/debug.rs` 747, `screen/mod.rs` (vector) 728,
  `combat_menu.rs` (vector) 719.
