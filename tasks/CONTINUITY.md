# Continuity notes

Written 2026-10-11, after P4 (town services) was built. Rewrite this file every time it is used. Durable
knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, not pushed (the owner pushes).
- **Accepted by the owner 2026-10-11:** P2, P3 and the party column. P0–P1 and presentation-ARCHITECTURE
  v0.4 accepted earlier.
- **P4 is built:** P4a `0e02b5c` (views and words), P4b `e813c59` (the screen), P4c (docs, this commit).
  **Owner acceptance pending**, script `tasks/acceptance/p4.md`. The owner may find things to fix, as in P3.
- **Next: P5** (inventory, give and use, spells outside a fight) once the owner accepts P4. P5 needs plan
  mode and the owner's answers before code.

## What P4 left for later steps
- The screen kit now has: `ControlId::{Offer(u16), Amount, Deposit, Withdraw}`, `Refused::NotDigits`, a
  screen's `apply` may return `Result` (`service::apply`), `Forms.bank`, `Forms::shown(top, base)` (clears
  the outcome on a new top screen and the bank's amount on entering a service).
- **The outcome on a screen:** `carry_out` sends an `Effect::Command` through `Session::ordered`, and puts
  the newest event line (body role) or the refusal (alert role) in `forms.message`. P5's inventory gets this
  for free; the explore message line (§6.6) is still to build, with P5.
- **Scrolling:** every screen panel scrolls (`Scrolls`, `Overflow::scroll_y`, `wheel` over the panel under
  the cursor). The root is capped at 64% of the window while a game is played (`PLAYED_ROOM`), 95% otherwise.
  Nothing marks rows below the fold yet (open, small).
- **The font is ASCII only** (Bevy's default): no "·", "×", typographic quotes or dashes in screen words.
  Pack text with such glyphs would show boxes; not yet checked across the packs.
- **No tool bar** (owner): Sheet, Inventory, Spells open from the party column (`roster::Tool`, dim "Not
  built yet (P5/P6)" until then). Camp, Look, Map join Actions and Menu at the bottom right later.

## Facts
- **The town** (`packs/test/data/maps/town.ron`, 12x5): inn (1,1), temple (4,1), trainer (7,1), guild
  (10,1); smith (1,3), tavern (4,3), bank (7,3); the start (10,2) facing West. `tests/service.rs` has
  `inside(data, site)` and the site constants.
- **The fixed party** starts with 60 gp, 40 food. `--seed N` starts in the town with it.
- **Captures of a state the flags cannot reach**: a temporary `PROBE-TEMP` hook in `src/main.rs`, after
  `Session::with_party` (the screenshot path does not go through `Session::start`), under
  `#[cfg(feature = "test-fixtures")]`, built with `cargo build -p omnis-vector --features test-fixtures` and
  run as `./target/debug/omnis-vector --screenshot .omnis/x.png --size 1600x900`. Remove it and grep `PROBE`.

## Open, carried over
- rhai Socket re-audit after P9; `smartstring` RUSTSEC-2026-0249 to add to ARCH §13; `ttf-parser` 0.25.1
  RUSTSEC-2026-0192.
- Editor's toolkit: open until Editor v1 is planned.
- Bevy 0.20: `tasks/plans/bevy-0.20-migration.md`, not before 2026-11-07.
- Backlog: removing or editing a party member after adding them (TODO, with its open questions).
- Unscheduled: `DevCommand::Pass { minutes }`; the `data.*` ops; `scripts/mcp-probe.py`.
- The owner's asset work (`.gitignore`, `assets/`, now with two untracked `cutout-basic-*.svg`) is theirs.
  Never stage it; commit only named paths (`git commit -- <paths>`, after `git add` for new files).

## Pins
- Gate `tests passed 741 failed 0 ignored 13` at P4b. Vector: 120 integration + 32 unit.
- Walk replay `78e57e238dcb93c0`, fight `48d160014c6a627a`; `SAVE_SCHEMA` 7, `PROTOCOL` 2, 25 ops, MCP 25
  tools. P4 changed no engine code.
- Sentrux: **scan `crates/`**: rules pass, quality 8978.

## Process
- **Cargo:** every cargo and gate run uses `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, with
  `cargo fmt --all` first. The vector clippy lines are in `scripts/verify.sh:9-10`; also run
  `cargo clippy -p omnis-vector --features test-fixtures --tests -- -D warnings`. zsh does not split a
  variable into words: write each clippy line out.
- **Tests:** the integration binary is `--test integration` (`tests/main.rs`), with `--features
  test-fixtures`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1`, a new file name each run, in the
  background. No edits while it runs. The last lines read `tests passed N ...` and `VERIFY-GREEN`.
- **Live window:** a window started from the agent's session runs no frames, so live checks go to the owner.
- **Vector tests:**
  - App shells: `common::app::{app, app_with, app_in_game(save), app_at_title(save), met_with(seed), set,
    key, click}`. The test shell has no `UiPlugin`, so `RelativeCursorPosition` and `Interaction` stay as a
    test sets them.
  - Model shells: `common::{town_session, place, map, id, views}`.
  - `screens.rs` is about 900 lines: new app tests go in their own file.
- **Mutation runs:** a `python3 -u` script writing to a file, never under `timeout`; check the sources are
  restored after (grep each mutated line).
- **Unfinished work** before a restart or compact is committed as `WIP`, never stashed.

## Carry-over
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` (omnis-app) 902, `omnis-vector/tests/screens.rs`
  about 900, `save_and_replay.rs` 810, `tactics_panel.rs` 795, `omnis-vector/src/arena.rs` about 790,
  `omnis-vector/src/shell/screen.rs` about 720, `omnis-vector/tests/service.rs` about 720.
