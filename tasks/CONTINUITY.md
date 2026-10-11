# Continuity notes

Written 2026-10-10, after parity P2, before a compact (owner: "prepare for a compact and then move on to P3 after the catchup"). Rewrite this file every time it is used. Durable knowledge lives in
`tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, not pushed (the owner pushes). After P0–P1 (accepted, "Manually tested")
  and the presentation architecture v0.4 (`d02ad2c`, accepted "Looks good"): P2a `75b7da9` (labels, copper
  lines fixed), P2b `4c1f166` (screen kit), P2c `6c797dd` (title, new game, pause, save/load, defeat), and
  the P2 docs commit after it.
- **P2 waits for the owner's acceptance:** `tasks/acceptance/p2.md` (window: title → new game → pause →
  save/load → fight Esc → fall; MCP at the title).
- **The screen kit as built** (presentation-ARCHITECTURE §6, updated to the code): `src/screen/`
  descriptions + `apply -> Effect`; `shell/screen.rs` renderer/router; `Nav` overlays over a world base;
  `Packs` always, `Session` only in a game; `VectorSet::Screens` always runs, `Grab/Input/Move` only while
  `playing`. Text fields are `bevy_ui_widgets`' `EditableText` (in `ui`, no new crate).

## Next
- **P3 (character creation) in plan mode, right after the catchup.** P2's acceptance is still the owner's to
  report (not yet said). Read cold for P3: presentation-ARCHITECTURE §4.4, §6 (all), §6.7's creation row;
  `omnis-app/src/creation_menu.rs` (`Catalog`, point buy at :74-82), `creation_panel.rs`, `feathers_creation.rs`;
  `docs/api.md` §7.2 and §8 (`PartyCommand::Create`, `Draft`); `crates/omnis-vector/src/screen/new_game.rs`
  as the pattern for a form.
- P3 itself: P3 starts with the engine addition of
  presentation-ARCHITECTURE §4.4: a point-buy view (budget, score range, cost table); the catalogue of
  races, classes, backgrounds comes from `Data` (labels/catalogue, VA9). P3 replaces `party::fixed()` in
  `Session::new_game` with the CreateParty base screen (§6.3). Needs `Kind::Number`/slider and probably
  `TabNavigationPlugin` for the name field. Details to carry: §6.7's creation row.
- Not yet built from §6.6: the roster band and the message line (with P4/P5).
- The owner is making placeholder SVGs for in-world features (`assets/*.svg.kra`, prefixes like
  `cutout-`, `extrude-`); I said an `omnis-cli` converter (SVG paths → RON line model; cutout / spin /
  extrude) belongs to the shaped-structures step, or a small step after P2 if the owner wants it sooner.
  Do not stage the owner's `assets/` files.

## Open, carried over
- rhai Socket re-audit after P9; `smartstring` RUSTSEC-2026-0249 to add to ARCH §13; `ttf-parser` 0.25.1
  RUSTSEC-2026-0192.
- Editor's toolkit: open until Editor v1 is planned.
- Bevy 0.20: `tasks/plans/bevy-0.20-migration.md`, not before 2026-11-07.
- Unscheduled: `DevCommand::Pass { minutes }`; the `data.*` ops; `scripts/mcp-probe.py`.

## Pins
- Gate `tests passed 701 failed 0 ignored 13`, VERIFY-GREEN. Vector: 84 integration + 32 unit.
- Walk replay `78e57e238dcb93c0`, fight `48d160014c6a627a`; `SAVE_SCHEMA` 7, `PROTOCOL` 2, MCP 24 tools.
- Sentrux: **scan `crates/`** (rules in `crates/.sentrux/rules.toml`): rules pass, quality 8960.

## Process
- Every cargo and gate run: `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first.
- Gate: `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1`, a **new file name each run** (a run that
  stops early leaves the old file; P2 read a stale 673 that way). No edits while it runs.
- Vector capture: `cargo run -p omnis-vector -- --screenshot .omnis/x.png --size 1600x900 [--screen title|new-game|pause|actions] [--walk N]`.
- A window started from the agent's session runs no frames; live window checks go to the owner.
- Vector tests: `common::app::{app, app_in_game(save), app_at_title(save)}`; a test that saves names its
  file under `.omnis/test-saves/`; after writing a fixture on a `Session`, call `session.refresh()`; a
  control opened by a corner button (Input set) is drawn the next frame.
- `pkill`/`pgrep` do not work in the sandbox; stop a background process with TaskStop.
- Unfinished work before a restart or compact is committed as `WIP`, never stashed.

## Carry-over
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` (omnis-app) 902, `save_and_replay.rs` 810,
  `tactics_panel.rs` 795, `omnis-vector/src/arena.rs` 772, `omnis-vector/src/shell/screen.rs` 755.
