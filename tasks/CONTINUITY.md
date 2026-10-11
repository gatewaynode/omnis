# Continuity notes

Written 2026-10-11, after P4a and before the compact that precedes P4b. Rewrite this file every time it
is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, not pushed (the owner pushes).
- **Accepted by the owner 2026-10-11:** P2, P3 and the party column. P0–P1 and presentation-ARCHITECTURE
  v0.4 accepted earlier.
- **P4 (town services), plan `tasks/plans/p4.md`, approved.** Owner's answers: no tool bar (Sheet, Inventory,
  Spells come from the party column); the bank amount is a typed field; **no step confirmation**, entering
  or leaving.
  - **P4a is done, `0e02b5c`.** Vector 111 integration + 32 unit, three clippy lines clean, Sentrux 8975.
    The full gate was not run (the plan runs it before the P4b commit).
  - **P4b is next.** Then P4c (docs, acceptance, continuity).

## What P4a gave (use it in P4b)
- `Views.service: Option<ServiceView>` (`sim.rs`, via `SimWorld::service`).
- `labels.rs`:
  - `offer_label(data, &PartyView, &OfferView) -> String`;
  - `offer_caption(&ServiceCommand) -> &'static str`;
  - `offer_note(&OfferView) -> String` (price, "free", "pays X"; a refused purchase keeps its price);
  - `service_reason(&Rejection) -> String`, for the dim button's `why_not`;
  - `ago`, `rumor`, `coins`.
- `shell::text::describe(event, &Names, &data)` gives the town lines. `Session::note` passes `self.names`.
  `rolllog::Names::member` is now pub.
- `tests/service.rs` (registered in `tests/main.rs`): `inside(data, (x, y)) -> SimWorld` places the
  party on a site tile and Interacts, with `SMITH (1,3)`, `TEMPLE (4,1)`, `TRAINER (7,1)` and `TAVERN (4,3)`
  defined; `rows(data, views)`. P4b's model tests go in this file.

## When you resume (after the catchup)
1. Read `tasks/plans/p4.md`, the P4b section.
2. Read these cold:
   - `crates/omnis-vector/src/screen/service.rs`, the stopgap to rewrite;
   - `src/screen/mod.rs` (`ControlId`, `Forms`, `accepted`, `Effect`);
   - `src/screen/nav.rs` (`escape`; Esc in a service must leave);
   - `src/shell/screen.rs` (`carry_out`, the `TooLong` refused-text rewrite, `forms.message`);
   - `src/shell/screen_draw.rs` (`rows`, panels; the scrolling goes here);
   - `tests/shell.rs::a_service_shows_the_town_notice_and_leave_steps_back_out`, which is replaced;
   - `omnis-app/src/service_panel.rs` `apply` and the per-kind lists in `feathers_service.rs`.
3. Build P4b. Run the full gate to a new file, then Sentrux, then the mutations in the plan, then commit.

## Facts for P4b
- **The bank amount: 8 digits, not the plan's 9.** `Deposit`/`Withdraw { amount }` are `u32` copper, so the
  most gold is `u32::MAX / 100` = 42,949,672 (the other client's `AMOUNT_MAX_GP`). Refuse a larger number,
  or clamp it, and test it. Say so in P4c's docs.
- **The town** (`packs/test/data/maps/town.ron`, 12x5):
  - inn (1,1), temple (4,1), trainer (7,1), guild (10,1);
  - smith (1,3), tavern (4,3), bank (7,3);
  - the portal at (11,2).

  The start is (10,2) facing West. Services are `base:service:<kind>`.
- **Entering is engine-side** (a step onto a site tile, or Interact). The binder freezes in Town mode, and
  `nav::base` is Service while `here.service` is set.
- **`service_view`:** the offers in order by kind, with Leave last. The bank has no offers. Temple offers
  are Heal/Cure/Raise per member, then Learn. Trainer offers are Train per member, then Choose.
- **The fixed party:** Brenna (fighter), Durin (cleric), Ilvara (wizard), Pip (rogue). The start gold is
  modest; the tests set `world.party.gold` through the fixture.

## Open, carried over
- rhai Socket re-audit after P9; `smartstring` RUSTSEC-2026-0249 to add to ARCH §13; `ttf-parser` 0.25.1
  RUSTSEC-2026-0192.
- Editor's toolkit: open until Editor v1 is planned.
- Bevy 0.20: `tasks/plans/bevy-0.20-migration.md`, not before 2026-11-07.
- Backlog: removing or editing a party member after adding them.
- Unscheduled: `DevCommand::Pass { minutes }`; the `data.*` ops; `scripts/mcp-probe.py`.
- The owner's asset work (`.gitignore`, `assets/`) is theirs. Never stage it; commit only named paths
  (`git commit -- <paths>`, after `git add` for new files).

## Pins
- Gate `tests passed 726 failed 0 ignored 13` at the party column. P4a adds 6 vector tests, so the next
  gate should read 732 / 0 / 13. Vector: 111 integration + 32 unit.
- Walk replay `78e57e238dcb93c0`, fight `48d160014c6a627a`; `SAVE_SCHEMA` 7, `PROTOCOL` 2, 25 ops, MCP 25
  tools. P4 is client only: none of these move.
- Sentrux: **scan `crates/`**: rules pass, quality 8975.

## Process
- **Cargo:** every cargo and gate run uses `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, with
  `cargo fmt --all` first. The vector clippy lines are in `scripts/verify.sh:9-10`.
- **Tests:** the integration binary is `--test integration` (`tests/main.rs`), with `--features
  test-fixtures`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1`, a new file name each run, in the
  background. No edits while it runs.
- **Captures:**
  - `cargo run -p omnis-vector -- --screenshot .omnis/x.png --size 1600x900 [--screen ...] [--walk N]`.
  - To reach a state the flags can't (a party on a site tile), use a temporary `PROBE-TEMP` hook (an env
    var, under `--features test-fixtures`). Remove it before committing and grep for `PROBE`.
- **Live window:** a window started from the agent's session runs no frames, so live checks go to the owner.
- **Vector tests:**
  - App shells: `common::app::{app, app_in_game(save), app_at_title(save), met_with(seed)}`.
  - Model shells: `common::{town_session, place, map, id, views}`.
  - `screens.rs` is about 900 lines, so new app tests go in their own file (for example `tests/services.rs`
    or in `tests/service.rs`).
  - The test shell has no HUD plugin: check `Corner` visibility.
- **Mutation runs:** a `python3 -u` script writing to a file, never under `timeout`. Check the sources are
  restored after.
- **Unfinished work** before a restart or compact is committed as `WIP`, never stashed.

## Carry-over
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` (omnis-app) 902, `omnis-vector/tests/screens.rs`
  about 900, `save_and_replay.rs` 810, `tactics_panel.rs` 795, `omnis-vector/src/arena.rs` about 790.
