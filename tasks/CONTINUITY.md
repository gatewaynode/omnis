# Continuity notes

Written 2026-10-10, after parity P0 and P1. Rewrite this file every time it is used. Durable knowledge
lives in `tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, not pushed (the owner pushes). Since the adoption (V0–V5, rename `903b82b`)
  and the owner's own `97d7e6a` (the mannequin `.glb` files, now committed by the owner):
  P0a `b11b6b3`, P0b `684f812`, P1 `dbec589`, bribe fix `c9aac59`, and the docs commit after it.
- **P0 done:** `omnis-vector` reads the world only through views. `sim::SimWorld` (world private, methods
  mirror `omnis_sim::api`) and `sim::Views` (`here`, `position`, `party`, `combat`, refreshed by
  `Session::note`). New engine views: Tier 1 `query::door_open`, `MemberView.can_fight` (additive,
  protocol 2). The library is clippy-checked without `test-fixtures`; a field read fails to compile.
- **P1 done (code):** the dev socket in vector (`shell/socket.rs`, `host.rs`, `screen_text.rs`,
  `capture.rs` live capture). Socket commands go through `dispatch` and into the binder's log; a load,
  pack reload or `rules.set` sets `Session::unreplayable`. Headless loopback test `tests/socket.rs`.
- **Not verified by me:** the live `screenshot`. A window started from this agent's session gets no
  window-server connection ("Connection invalid … hiservices", sandboxed or not; `omnis-app` the same)
  and runs no frames. It is Part 2 of `tasks/acceptance/p0-p1.md` for the owner.
- Plan: `tasks/plans/p0-p1.md`; review in `tasks/TODO.md` under P1.

## Next: the presentation layer's architecture (owner, 2026-10-10: "then let's do the architecture for the presentation layer")
Its own plan-mode session; **first confirm the scope with the owner.** My reading: rewrite
`presentation-ARCHITECTURE.md` (still the Phase A experiment's draft, "Omnis Alt", v0.3) into the
architecture of the game client:
- the core/shell split as built after P0/P1 (`SimWorld`, `Views`, the host, the socket)
- a screen model the parity screens P2–P8 share (bevy_ui neon, X15; buttons first; agent-readable text
  via `screen_text`; headless tests of every control)
- the style system's place (colour roles, biosphere palettes in packs, X18, X19)
- the renderer's path from gizmos to meshes with occluding fills (X13, measured X20)
- where terrain height and shaped structures plug in (X10–X12; sim side integers only)
- the test and capture contract
Read cold first: `presentation-PRD.md` §5–§8, §11, §12; `presentation-ARCHITECTURE.md` whole (311 lines);
`ARCHITECTURE.md` §4.9, §8, §9, A17, A18. Edits to `ARCHITECTURE.md`/`PRD.md` need the owner's ask.

## Open, carried over
- Owner acceptance of P0–P1 (`tasks/acceptance/p0-p1.md`), especially the live `screenshot`.
- rhai Socket re-audit: after the migration (P9). `smartstring` RUSTSEC-2026-0249 to add to ARCH §13;
  `ttf-parser` 0.25.1 (Bevy) RUSTSEC-2026-0192.
- Editor's toolkit: open until Editor v1 is planned.
- Bevy 0.20: `tasks/plans/bevy-0.20-migration.md`, not before 2026-11-07.
- Vector's world starts with `Settings::default()` (devtools off), so `time.reconcile` over its socket
  answers DevOnly until P2's new-game settings.
- Unscheduled: `DevCommand::Pass { minutes }`; the `data.*` ops; `scripts/mcp-probe.py`.

## Pins
- Gate `tests passed 675 failed 0 ignored 13`, VERIFY-GREEN (before the bribe fix's +0 tests). Vector: 62
  integration + 28 unit.
- Walk replay `78e57e238dcb93c0`, fight `48d160014c6a627a`; `SAVE_SCHEMA` 7, `PROTOCOL` 2, MCP 24 tools.
- Sentrux: **scan `crates/`** (rules in `crates/.sentrux/rules.toml`): rules pass, quality 8965.

## Process
- Every cargo and gate run: `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first.
- Gate: `scripts/verify.sh > <scratchpad>/gate.txt 2>&1` in the background; no edits while it runs.
- Vector capture: `cargo run -p omnis-vector -- --screenshot .omnis/x.png --size 1600x900 [--walk N]`
  (offscreen, works from the agent's session); starts in the town.
- Vector tests: `common::session` puts the party on the meadow directly; after writing a fixture on a
  `Session`, call `session.refresh()`. `common::views(world, data)` for functions that take `&Views`.
- `pkill`/`pgrep` do not work in the sandbox; stop a background process with TaskStop.
- Tests: one `integration` binary per crate (`scripts/check-test-modules.sh`).
- Unfinished work before a restart or compact is committed as `WIP`, never stashed.

## Carry-over
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` 902, `save_and_replay.rs` 810,
  `tactics_panel.rs` 795, `omnis-vector/src/arena.rs` ~775.
