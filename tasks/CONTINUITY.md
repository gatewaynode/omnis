# Continuity notes

Written 2026-10-11, before the compact that precedes P4's implementation. Rewrite this file every time it
is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, not pushed (the owner pushes).
- **Accepted by the owner 2026-10-11:** P2, P3 and the party column (log at the left, selectable party
  column at the right, `df2f952`). P0–P1 and presentation-ARCHITECTURE v0.4 accepted earlier.
- **P4 is planned and approved, not started.** The plan is `tasks/plans/p4.md` (commit `514f869`): read it
  in full first. The owner's answers (2026-10-11):
  - **No tool bar.** Sheet, Inventory and Spells come from the party column; Camp, Look and Map join
    Actions and Menu when their steps land.
  - **The bank amount is a typed field.** "Gold pieces", digits only.
  - **No step confirmation, entering or leaving.** The §6.7 "Step confirmation" row is dropped.
- The column's Sheet, Inventory and Spells open nothing yet. P5 and P6 handle
  `RosterEffect::Open { tool, member, read_only }`; it is read-only in a fight (owner).

## When you resume (after the catchup)
1. Read `tasks/plans/p4.md`.
2. Read these sources cold:
   - `crates/omnis-vector/src/screen/service.rs`, the stopgap the plan rewrites.
   - `omnis-sim/src/service_view.rs` and `service.rs` (`ServiceCommand`).
   - `omnis-app/src/service_panel.rs` and `service_text.rs`, the words to port.
   - `omnis-vector/src/shell/text.rs` and `src/shell/screen.rs` (`carry_out`, the `TooLong` field rewrite).
   - `tests/shell.rs::a_service_shows_the_town_notice_and_leave_steps_back_out`, which is replaced.
3. Build P4a, then P4b, then P4c, committing each by name.

## Facts for P4
- **The town** (`packs/test/data/maps/town.ron`, 12x5) has a site tile inside each door:
  - inn (1,1), temple (4,1), trainer (7,1), guild (10,1)
  - smith (1,3), tavern (4,3), bank (7,3)
  - the portal to the meadow at (11,2)
  
  The services are `base:service:<kind>`. Prices are in `packs/base/data/rules/services.ron`.
- **The start** is (10,2) facing West. The stopgap test turns right, Interacts, and steps into the guild.
- **Entering is engine-side** (`service::enter_here` on a step or Interact). The binder freezes in Town
  mode, and `nav::base` is Service while `here.service` is set.
- **The service screens only render.** The engine's `service_view` lists every offer with its command,
  member, price, pays, refusal and subject. Leave is last. The bank has no offers (Deposit and Withdraw take
  an amount).
- **`Session::order`** notes events into `Session.lines` (the HUD, last 8). Service events have no line yet.

## Open, carried over
- rhai Socket re-audit after P9; `smartstring` RUSTSEC-2026-0249 to add to ARCH §13; `ttf-parser` 0.25.1
  RUSTSEC-2026-0192.
- Editor's toolkit: open until Editor v1 is planned.
- Bevy 0.20: `tasks/plans/bevy-0.20-migration.md`, not before 2026-11-07.
- Unscheduled: `DevCommand::Pass { minutes }`; the `data.*` ops; `scripts/mcp-probe.py`.
- The owner's asset work (`.gitignore`, `assets/` deletions, some staged, and new asset folders) is theirs.
  Never stage it; commit only named paths (`git commit -- <paths>`, after `git add` for new files).

## Pins
- Gate `tests passed 726 failed 0 ignored 13`, VERIFY-GREEN. Vector: 105 integration + 32 unit.
- Walk replay `78e57e238dcb93c0`, fight `48d160014c6a627a`; `SAVE_SCHEMA` 7, `PROTOCOL` 2, 25 ops, MCP 25
  tools.
- Sentrux: **scan `crates/`**: rules pass, quality 8967.

## Process
- **Cargo:** every cargo and gate run uses `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, with
  `cargo fmt --all` first. The two vector clippy lines are in `scripts/verify.sh:9-10`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1`, a new file name each run, run in the
  background. No edits while it runs.
- **Captures:**
  - `cargo run -p omnis-vector -- --screenshot .omnis/x.png --size 1600x900 [--screen ...] [--walk N]`.
  - To reach a state the flags can't, use a temporary hook marked `PROBE-TEMP` (an env var, under
    `--features test-fixtures` for world access). Remove it before committing and grep for `PROBE`.
- **Live window:** a window started from the agent's session runs no frames, so live window checks go to
  the owner.
- **Vector tests:**
  - App shells: `common::app::{app, app_in_game(save), app_at_title(save), met_with(seed)}`;
    `common::fight::{met, turn_of(name)}`.
  - `screens.rs` has `to_creation`, `keyboard_shell`/`keyboard` (the window's real typing path),
    `type_into`, `slide` and `step_choice`. `column.rs` has `press_side`.
  - The test shell has no HUD plugin: check `Corner` visibility instead.
  - `screens.rs` is about 900 lines, so new app tests go in their own file.
- **Mutation runs:** a `python3 -u` script writing to a file, never under `timeout`. Check the sources
  are restored after.
- **Unfinished work** before a restart or compact is committed as `WIP`, never stashed.

## Carry-over
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` (omnis-app) 902, `omnis-vector/tests/screens.rs`
  about 900, `save_and_replay.rs` 810, `tactics_panel.rs` 795, `omnis-vector/src/arena.rs` about 790.
