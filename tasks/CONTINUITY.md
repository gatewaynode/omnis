# Continuity notes

Written 2026-10-10, after the presentation architecture v0.4. Rewrite this file every time it is used.
Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, not pushed (the owner pushes). After the adoption (V0–V5): P0a `b11b6b3`,
  P0b `684f812`, P1 `dbec589`, bribe fix `c9aac59`, docs `fcbe85c`. P0–P1 **accepted by the owner**
  2026-10-10 ("Manually tested").
- **Presentation architecture v0.4 accepted by the owner ("Looks good") and committed.**
  `presentation-ARCHITECTURE.md` rewritten (545 lines); `presentation-PRD.md` header → v0.4; `tasks/TODO.md`
  (acceptance, the v0.4 item with review, P2/P3/P6/P7 annotated); `tasks/acceptance/p0-p1.md` accepted line.
  Plan: `/Users/john/.claude/plans/dreamy-kindling-wozniak.md`.
- **Owner decisions this session:** screens are Bevy-free descriptions (`Screen` + `apply -> Effect`) drawn
  by one shell renderer (VA10); rules come from views, trials and queries, `Data` for labels and catalogues
  only (VA9; engine additions per step in §4.4).

## Next
- Then parity: the owner said "we have a lot of little details to add back in from the old presentation".
  P2 first (plan mode): the screen kit (§6), `labels.rs` (§4.3), `--screen` capture, devtools world in
  debug builds, title/new game/pause/save/load; decide the field widgets (§15.1). §6.7's parity map lists
  each screen's details from `omnis-app`.
- Engine additions (§4.4): P3 point-buy view; P6 known spells with point and action cost; P7 `SpellView`
  action cost and the fight's usable items with refusals. Additive, protocol 2, `docs/api.md` + tests.

## Found in the omnis-app survey (for parity, not fixed in omnis-app)
- Message line shows the raw key `sim:message:nothing_here` (`omnis-app/src/ui.rs:171`, asserted by
  `tests/pointer.rs:478`); saving calls the world's `may_save` not `Here.may_save` (`sim.rs:481`); blocked
  steps shown as `{:?}`; alert colour chosen by text containing "fail"/"forbid" (`ui.rs:313`). The vector
  client must not copy these (presentation-ARCHITECTURE §4.3).

## Open, carried over
- rhai Socket re-audit after P9; `smartstring` RUSTSEC-2026-0249 to add to ARCH §13; `ttf-parser` 0.25.1
  RUSTSEC-2026-0192.
- Editor's toolkit: open until Editor v1 is planned.
- Bevy 0.20: `tasks/plans/bevy-0.20-migration.md`, not before 2026-11-07.
- Unscheduled: `DevCommand::Pass { minutes }`; the `data.*` ops; `scripts/mcp-probe.py`.

## Pins
- Gate `tests passed 675 failed 0 ignored 13`, VERIFY-GREEN. Vector: 62 integration + 28 unit.
- Walk replay `78e57e238dcb93c0`, fight `48d160014c6a627a`; `SAVE_SCHEMA` 7, `PROTOCOL` 2, MCP 24 tools.
- Sentrux: **scan `crates/`** (rules in `crates/.sentrux/rules.toml`): rules pass, quality 8965.

## Process
- Every cargo and gate run: `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first.
- Gate: `scripts/verify.sh > <scratchpad>/gate.txt 2>&1` in the background; no edits while it runs.
- Vector capture: `cargo run -p omnis-vector -- --screenshot .omnis/x.png --size 1600x900 [--walk N]`.
- A window started from the agent's session runs no frames (no window-server connection); live window
  checks go to the owner's acceptance.
- Vector tests: after writing a fixture on a `Session`, call `session.refresh()`.
- `pkill`/`pgrep` do not work in the sandbox; stop a background process with TaskStop.
- Unfinished work before a restart or compact is committed as `WIP`, never stashed.

## Carry-over
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` 902, `save_and_replay.rs` 810,
  `tactics_panel.rs` 795, `omnis-vector/src/arena.rs` 772.
