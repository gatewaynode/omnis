# Continuity notes

Written 2026-10-11, after P8 was accepted, before a compact. Rewrite this file every time it is used.
Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, not pushed (the owner pushes).
- **Accepted by the owner:** P2, P3, the party column, P4, P5, P6, P7, P8 (all 2026-10-11; P8: "P8 accepted,
  prepare for a compact").
- **P8 built and accepted** (`tasks/acceptance/p8.md`). Plan `tasks/plans/p8.md` with "As built".
  - Commits: `03b8000` (plan), `353e1ad` (P8a settings), `b3272aa` (P8b font and scale), `1773569` (P8c debug), then P8d (docs).
  - Owner answers:
    - Debug in pages: Member, Party, World, Fight.
    - Quality is one preset.
    - Reduced motion snaps the 90-degree turns and the drift.
    - Settings wait for Apply and open from the title and the pause.
    - Fonts: built-in, Inter, Alegreya Sans.
    - The scale reaches everything, 50% to 200%.
    - Debug opens from the pause and with backtick.
  - Beyond the plan: `UiScale`, held to what the window holds (`settings::fits`: 120% at 1600×900, 200% at 2560×1440), with "N% fits this window" on the screen. The fight is laid out at `arena::zoom` and its lines and clicks read at the zoom. The plan had said "no UiScale".
- **Next:**
  - P9: retire `omnis-app` (TODO line). Plan it in plan mode: explore first, then ask the owner before
    writing the plan. Wait for the owner to say to move on.
  - Then the rhai Socket re-audit (the owner: after the migration).

## Facts
- **Settings words:**
  - "SETTINGS"; "Reduced motion < off >", "Floor grid < shown >", "Quality < high >", "Font < built-in >", "Interface scale < 100% >".
  - "Apply: nothing changed"; "Settings saved to .omnis/settings.ron".
  - The file: `(reduced_motion, floor_grid, quality: "low|medium|high", font: "built-in|inter|alegreya", scale)`.
- **Debug words:**
  - "DEBUG"; "Page < Member >", "Member < Brenna >"; "Hit points [12] Set of 12"; "Condition < Blinded > On / off off".
  - "The packs declare no flags"; "Map < test:map:town (12 by 5) >"; "No fight is on"; "type a number".
  - Pause's Debug, when dim: "Devtools are off in this game" or "This build has no debug panel".
  - Dev lines: "dev: Brenna hp 5", "dev: Spyglass x3 to Durin", "dev: gold 1 gp 5 sp 0 cp", "dev: teleport to Test Town (3, 4) facing north", "dev: stack 1 slain".
- **Quality:** low is no bloom and 1 px lines; medium is bloom 0.15 and 1.5 px; high is 0.25 and 2 px (the old look).
- **Test worlds:** `common::town_session` uses `Settings::default()`, so devtools are off. `Session::start(&config())` and `met_with` use `game_settings`, so devtools are on.

## Open, carried over
- **From P8:**
  - At 200% the party column's entries wrap.
  - `assets/README.md` (the owner's) still says the fonts are Feathers-only. Tell the owner; do not edit it.
  - A flag's Set is not reachable with the shipped packs.
- **From P6, unconfirmed calls:** down members red; dead members dim, with "dead" in red.
- **Dependencies and advisories:**
  - The rhai Socket re-audit after P9.
  - `smartstring` RUSTSEC-2026-0249 to add to ARCH §13.
  - `ttf-parser` 0.25.1 RUSTSEC-2026-0192.
  - Bevy 0.20 not before 2026-11-07.
- **Backlog:** removing or editing a party member. Unscheduled: `DevCommand::Pass`; the `data.*` ops; `scripts/mcp-probe.py`.
- **Small:**
  - Pack text is not checked for non-ASCII.
  - The health colours are not checked for colour deficiency.
  - The picture window draws every monster as the rat.
- The owner's asset work (`.gitignore`, `assets/`, two untracked `cutout-basic-*.svg`) is theirs. Never stage it.

## Pins
- Gate `tests passed 827 failed 0 ignored 13`, VERIFY-GREEN at `1773569` (docs since: `fdd9529`, the acceptance commit). Vector 195 integration + 37 unit.
- Walk replay `78e57e238dcb93c0`, fight `48d160014c6a627a`; `SAVE_SCHEMA` 7, `PROTOCOL` 2, 25 ops, MCP 25 tools.
- Sentrux: scan `crates/`: rules pass, quality 8995.

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, with `cargo fmt --all` first. Vector clippy, each line written out (zsh does not split a variable):
  - `--lib`;
  - `--no-default-features --lib`;
  - `--features test-fixtures --tests`.
- **Tests:**
  - `--test integration` with `--features test-fixtures` (`tests/main.rs` lists the files).
  - App helpers: `common::app::{press_ui, tap, click, shell, config}`.
  - A test that applies settings names its own file (`Config.settings`); the default test path is `.omnis/test-saves/settings-none.ron`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1` in the background, a new file each run, no edits while it runs.
- **Captures:**
  - Settings need no hook: write a file and pass `--settings .omnis/p8/x.ron` with `--screenshot`.
  - A fight or an open screen needs a temporary `PROBE-TEMP` hook in `shell/screen.rs::open_start`: put the party at dungeon (3, 6) facing South, then `--walk 80`.
  - A screen opened in `open_start` pauses the game, so the walk never happens. Open it from `route` once `views.combat` is set.
  - Fill the HUD's lines with `session.say`.
  - Back the file up to the scratchpad first, restore it, then grep `PROBE`.
  - Build with `--features test-fixtures`.
- **Mutations:** `python3 -u` to a file, never under `timeout`; check that the sources are restored. A mutation is missed when the test's value equals the world's ("7" against 7): type "007".
- **Commits:** unfinished work before a compact is committed as `WIP`, never stashed. Stage by name, never `.claude/`.
- **`cargo fmt` reflows lines:** a Python `replace` written before formatting may silently miss; assert counts.

## Carry-over
- **Large files:** `plan.rs` 980, `bake.rs` 934, `screen.rs` (omnis-app) 902, `omnis-vector/tests/screens.rs` about 840, `save_and_replay.rs` 810, `arena.rs` (vector) 830, `shell/screen.rs` (vector) 791, `screen/debug.rs` 747, `screen/mod.rs` (vector) 728, `combat_menu.rs` 719.
