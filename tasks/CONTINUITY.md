# Continuity notes

Written 2026-10-11, after the party column. Rewrite this file every time it is used. Durable knowledge lives in
`tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, not pushed (the owner pushes). After P0–P1 (accepted), the presentation
  architecture v0.4 (accepted) and P2 (`75b7da9`, `4c1f166`, `6c797dd`, docs `aa38c6b`), P3 is built:
  P3a `7b186d6` (engine `creation_view` / `creation.get` / MCP `creation_get`, 25 ops), P3b `b74450b`
  (`Kind::Number` with slider and arrows, panels side by side, `shell/screen_draw.rs`), P3c `5ef3fa2`
  (the CreateParty screen, Start again makes the same party, the load-then-restart seed fix), and the
  P3 docs commit after it.
- **2026-10-11:** P3 fixes from the owner's test (`93fa570`: a visible cursor, the form fits the window; the
  second name confirmed fixed by the owner). Then the party column and the log at the left (owner's
  request): `screen/roster.rs`, `shell/roster.rs`, `arena::layout`'s left column and `party` rectangle;
  acceptance `tasks/acceptance/party-column.md` (accepted 2026-10-11). Gate 726/0/13. The column's Sheet/Inventory/Spells
  open nothing yet: P5 and P6 handle `RosterEffect::Open { tool, member, read_only }` (read-only in a
  fight, owner).
- **Waiting on the owner:** acceptance of P2 (`tasks/acceptance/p2.md`), and P3 (`tasks/acceptance/p3.md`).
- Owner answers for P3 (2026-10-10): the view in Tier 1 + op + MCP tool; Start again = the same party;
  scores by slider plus arrows.

## Next
- P4 (town services and the step confirmation) when the owner asks; plan mode. Read cold:
  presentation-ARCHITECTURE §4.4 (P4 row), §6.6, §6.7 (town and confirm rows); the stopgap
  `screen/service.rs`; `omnis-app`'s town panels; `service_view` in docs/api.md §7.5.
- Not yet built from §6.6: the message line (with P4/P5). The roster band is built as the party column.
- The owner's placeholder SVGs (`assets/*.svg`, `*.svg.kra`, `assets/README.md`) stay unstaged; the
  `omnis-cli` SVG converter belongs to the shaped-structures step unless the owner asks sooner.

## Open, carried over
- rhai Socket re-audit after P9; `smartstring` RUSTSEC-2026-0249 to add to ARCH §13; `ttf-parser` 0.25.1
  RUSTSEC-2026-0192.
- Editor's toolkit: open until Editor v1 is planned.
- Bevy 0.20: `tasks/plans/bevy-0.20-migration.md`, not before 2026-11-07.
- Unscheduled: `DevCommand::Pass { minutes }`; the `data.*` ops; `scripts/mcp-probe.py`.

## Pins
- Gate `tests passed 726 failed 0 ignored 13`, VERIFY-GREEN. Vector: 105 integration + 32 unit.
- Walk replay `78e57e238dcb93c0`, fight `48d160014c6a627a`; `SAVE_SCHEMA` 7, `PROTOCOL` 2, 25 ops, MCP 25
  tools.
- Sentrux: **scan `crates/`**: rules pass, quality 8967.

## Process
- Every cargo and gate run: `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first.
- Gate: `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1`, a new file name each run. No edits while it
  runs.
- A new op: also `omnis-cli/src/schema.rs`'s hand list (and its count in `omnis-cli/tests/headless.rs`),
  `omnis-mcp` tools, `api_doc` macros, `replies.rs`, `vocabulary.rs` VIEWS, and the bridge counts.
- Vector capture: `cargo run -p omnis-vector -- --screenshot .omnis/x.png --size 1600x900 [--screen title|new-game|create-party|pause|actions] [--walk N]`.
- A window started from the agent's session runs no frames; live window checks go to the owner.
- Vector tests: `common::app::{app, app_in_game(save), app_at_title(save)}`; screens.rs has `to_creation`, `keyboard_shell`/`keyboard` (the window's real typing path),
  `type_into`, `slide` (triggers `ValueChange`), `step_choice`; the test shell has no HUD plugin (check
  `Corner` visibility).
- Mutation runs: `python3 -u` to a file, never under `timeout`; grep for each break after.
- Unfinished work before a restart or compact is committed as `WIP`, never stashed.

## Carry-over
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` (omnis-app) 902, `save_and_replay.rs` 810,
  `tactics_panel.rs` 795, `omnis-vector/src/arena.rs` 772, `omnis-vector/tests/screens.rs` 761.
