# Continuity notes

Written 2026-10-10, after the vector adoption's V0–V5 were built. Rewrite this file every time it is used.
Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`** (cut from `main` at `aa89fff`). Commits on it: V0 `4a73527`, the merge
  `f62738c`, V2 `43bbae2` and `01dd325`, V3 `96d0816`, V4 `b58022a`, V5 (this note's commit). Not pushed;
  the owner pushes.
- **The vector adoption** (plan `tasks/plans/vector-adoption.md`, TODO "Vector adoption"): `omnis-vector` is
  the game client (`just run`), on protocol 2; `omnis-app` stays until parity (`just run-app`), owner's
  choice "after parity". **Owner acceptance open**: `tasks/acceptance/vector-adoption.md` (four parts, all
  in the window).
- Protocol 2 is accepted and closed (Part 3 played by the owner 2026-10-10). alt-PRD v0.3 accepted; PRD v0.8
  (D2, D26 decided, D27) and ARCH A18 fold it in.

## Next
1. The owner's acceptance of the adoption. Then check V5 in `tasks/TODO.md`.
2. Parity, each item planned in plan mode when the owner asks: **P0** (vector reads `World` fields directly,
   30 sites in 13 files, where A17 says views; before or with P1), **P1** dev socket in vector (window-mode
   MCP; `screenshot`/`screen_text`), P2 title/new game/pause/saves, P3 creation, P4 services (replaces the
   town notice), P5 inventory and spells outside fights, P6 camp/sheet/tactics, P7 fight parity (features,
   React, Use picker), P8 debug panel and settings, P9 retire `omnis-app` (binary becomes `omnis`), then the
   rhai Socket re-audit (owner: after the migration).
3. The owner sets the order of alt-PRD §11's visual steps (occluding fills, style system, height and cliffs,
   shaped structures) against parity. Bevy 0.20 not before 2026-11-07.

## Open, carried over
- rhai re-audit deferred by the owner until after the migration. Facts so far (no Socket): 1.26.1 is the
  latest and now 30 days old, checksum matches, `cargo audit` clean; `smartstring` 1.0.1 (via rhai)
  unmaintained, RUSTSEC-2026-0249, to be added to ARCH §13's row; `ttf-parser` 0.25.1 (via Bevy)
  RUSTSEC-2026-0192.
- Two untracked files appeared during the session, not mine: `assets/artist-mannequin-neutral.glb`,
  `assets/artist-mannequin-posed.glb`. Not staged; ask the owner.
- Editor's toolkit: open until Editor v1 is planned (alt-PRD §8 says `bevy_egui`, PRD §11.1 Feathers).
- Unscheduled: `DevCommand::Pass { minutes }`; the `data.*` ops; `scripts/mcp-probe.py`.

## Pins
- Gate `tests passed 670 failed 0 ignored 13`, VERIFY-GREEN, 268 s. `cargo test -p omnis-vector`: 60
  integration + 27 unit.
- Walk replay `8711507745385976768` (`78e57e238dcb93c0`), fight `5247080599556612730` (`48d160014c6a627a`);
  `SAVE_SCHEMA` 7, `PROTOCOL` 2, MCP 24 tools.
- Sentrux: **scan `crates/`** (the rules file is `crates/.sentrux/rules.toml`; scanning the repo root
  finds no rules and a meaningless score): rules pass, quality 8958.

## Process
- Every cargo and gate run: `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first.
- Gate: `scripts/verify.sh > <scratchpad>/gate.txt 2>&1` in the background; no edits while it runs.
- Vector capture: `cargo run -p omnis-vector -- --screenshot .omnis/x.png --size 1600x900 [--walk N]`
  (relative path); it starts in the town and has no flag to start elsewhere.
- Vector tests: `common::session` places the party on the meadow (16, 16, North) directly (does not replay);
  `common::town_session` is the real start and replays. The engine's `Teleport` dev command is refused
  without dev settings.
- Tests: one `integration` binary per crate (`scripts/check-test-modules.sh`).
- Unfinished work before a restart or compact is committed as `WIP`, never stashed.

## Carry-over
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` 902, `save_and_replay.rs` 810,
  `tactics_panel.rs` 795, `omnis-vector/src/arena.rs` ~775.
