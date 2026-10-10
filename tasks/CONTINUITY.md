# Continuity notes

Written 2026-10-10, before a compact. Rewrite this file every time it is used. Durable knowledge lives in
`tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`** (cut from `main` at `aa89fff`): V0 `4a73527`, merge `f62738c`, V2 `43bbae2`
  `01dd325`, V3 `96d0816`, V4 `b58022a`, V5 `ff21955`, and the rename plus this note (one commit after).
  Not pushed; the owner pushes.
- **The vector adoption is done** (plan `tasks/plans/vector-adoption.md`, TODO "Vector adoption", V0–V5
  checked). `omnis-vector` is the game client (`just run`) on protocol 2; `omnis-app` stays until parity
  (`just run-app`). Owner's manual test 2026-10-10: "close to the experiment's level of functionality"
  (no gap named; ask if one comes up).
- **Renamed at the owner's request:** `presentation-PRD.md` (was `alt-PRD.md`, v0.3, accepted) and
  `presentation-ARCHITECTURE.md` (was `alt-ARCHITECTURE.md`, v0.3). PRD D27 and ARCH A18 point to them.
  `tasks/alt-TODO.md` keeps its name as the experiment's history (Phases A and B).
- The owner's `.glb` files in `assets/` (`artist-mannequin-neutral.glb`, `artist-mannequin-posed.glb`) are
  theirs; they add them if they turn out. Never stage them.

## Next: pick up where the experiment left off (owner, 2026-10-10)
Read cold first: `presentation-PRD.md` §11 "Next" and §12, `presentation-ARCHITECTURE.md` (§1 principles,
§3 dependencies, §5–§6 the 3D view and shell, §9 the fight screen), `tasks/alt-TODO.md` (Phases A–B as
built). The experiment's own next steps, in its order (§11; step 1, the merge, is done):
2. **Occluding fills** (X13): from gizmos to line meshes over dark solid surfaces, measured (X20).
3. **The style system** (X18, X19): colour roles, biosphere palettes in pack data, non-colour channels, on
   the 3D view, the fight screen and the HUD.
4. **Terrain height and cliffs** (X10, X11): data shape with the mechanics side, relief, contour grid,
   camera following the ground. Needs `omnis-data`/`omnis-sim` (integers only; lint rules stand).
5. **Shaped structures** (X12): shape generators per terrain kind, then an authored set piece.
6. **Interface parity** (X14, X15) = `tasks/TODO.md` P0–P9 (P0: vector reads `World` fields directly in
   30 places, A17 says views; P1: dev socket in vector for window-mode MCP).
The art update (animated figures, the fight screen's scale on large windows, from B6) fits with 3 and 6.
**First question for the owner after the compact:** which of these comes first (the experiment's order
puts occluding fills next; parity P0/P1 would give agents MCP capture of the vector client). Then plan
mode. Open questions that block some steps: §12 (height's data shape, biosphere assignment, the role set,
the set-piece format, automap sensing, the performance floor).

## Open, carried over
- rhai Socket re-audit: deferred by the owner until after the migration (P9). Facts: 1.26.1 latest and 30
  days old, checksum matches, `cargo audit` clean; `smartstring` 1.0.1 unmaintained, RUSTSEC-2026-0249, to
  add to ARCH §13's row; `ttf-parser` 0.25.1 (Bevy) RUSTSEC-2026-0192.
- Editor's toolkit: open until Editor v1 is planned (presentation-PRD §8 `bevy_egui`; PRD §11.1 Feathers).
- Bevy 0.20 migration: `tasks/plans/bevy-0.20-migration.md`, not before 2026-11-07.
- Unscheduled: `DevCommand::Pass { minutes }`; the `data.*` ops; `scripts/mcp-probe.py`.

## Pins
- Gate `tests passed 670 failed 0 ignored 13`, VERIFY-GREEN, 268 s. `cargo test -p omnis-vector`: 60
  integration + 27 unit.
- Walk replay `8711507745385976768` (`78e57e238dcb93c0`), fight `5247080599556612730` (`48d160014c6a627a`);
  `SAVE_SCHEMA` 7, `PROTOCOL` 2, MCP 24 tools.
- Sentrux: **scan `crates/`** (rules in `crates/.sentrux/rules.toml`; the repo root has none): rules pass,
  quality 8958.

## Process
- Every cargo and gate run: `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first.
- Gate: `scripts/verify.sh > <scratchpad>/gate.txt 2>&1` in the background; no edits while it runs.
- Vector capture: `cargo run -p omnis-vector -- --screenshot .omnis/x.png --size 1600x900 [--walk N]`
  (relative path); starts in the town; no flag to start elsewhere. Read the PNG to judge it.
- Vector tests: `common::session` puts the party on the meadow (16, 16, North) directly (no replay);
  `common::town_session[_seeded]` is the real start and replays. `Teleport` needs dev settings.
- Vector is not a simulation crate (floats and Bevy allowed) but never passes a float into the simulation:
  commands in, queries and events out (presentation-PRD §9).
- Tests: one `integration` binary per crate (`scripts/check-test-modules.sh`).
- The clone `/Users/john/code/omnis-alt/omnis` is unreadable from the sandbox and now history; read the
  experiment through this tree or `git show origin/gui-3d-experiment:<path>`.
- Unfinished work before a restart or compact is committed as `WIP`, never stashed.

## Carry-over
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` 902, `save_and_replay.rs` 810,
  `tactics_panel.rs` 795, `omnis-vector/src/arena.rs` ~775.
