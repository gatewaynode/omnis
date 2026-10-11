# Continuity notes

Written 2026-10-11, after P5's acceptance and before the compact that precedes P6 planning. Rewrite this file every time it is used. Durable knowledge lives in
`tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, not pushed (the owner pushes).
- **Accepted by the owner 2026-10-11:** P2, P3, the party column, P4, **P5** (manual test of
  `tasks/acceptance/p5.md`: "looks to be working as designed").
- P5 commits: P5a `315b823`, P5b + P5c `3795b10`, P5d `5c187c4` (docs), then the acceptance record.
- **Next: plan P6 in plan mode** (TODO: "Camp and rest; character sheet; tactics panel. Engine first: known
  spells with point and action cost (§4.4)"). Read cold first: presentation-ARCHITECTURE §4.4 (the known-spell
  row), §6.3 (Camp, Sheet, Tactics overlays), §6.6 (Sheet opens from the party column, read-only in a fight;
  Camp joins the bottom-right buttons), §6.7 (the Camp, Character sheet, Tactics rows: the parity checklist).
  Old client to port from (`crates/omnis-app/src/`): `camp_panel.rs`, `feathers_camp.rs`; `sheet_menu.rs` (reads
  `point_cost()` at :195, the §4.4 defect to move into the engine), `sheet.rs`, `sheet_screen.rs`;
  `tactics_panel.rs`, `tactics_draft.rs`, `feathers_tactics.rs`; `tool_bar.rs` (Camp's live rule). Engine views:
  `rest_view` (P6 adds `Views.rest`), `MemberView.tactics`. Ask the owner the layout questions before the plan,
  as for P5.

## Facts
- **The acceptance script** lowers HP and SP through MCP dev commands (`SetHp`, `SetSpellPoints`); every
  fixed member carries a Potion of healing (the acolyte background) and the smith sells one. A potion at full
  HP is accepted and spent.
- **Captures** of the inventory and spells screens at 1600×900 and 1280×720 were taken and checked
  (`.omnis/p5-*.png`): the column stands beside, no overlap. The message line was not captured.
- The fixed party: Brenna (fighter: chain mail, longsword, shield, light crossbow, 20 crossbow bolts), Durin
  (cleric: Guidance, Light, Bless, Cure Wounds on the road), Ilvara (wizard: Light…), Pip (rogue: arrows,
  daggers). Start: town (10,2) West, 60 gp, 40 food, the stores empty. North of the start: "There is nothing
  to do here".
- The town's sites: inn (1,1), temple (4,1), trainer (7,1), guild (10,1); smith (1,3), tavern (4,3), bank (7,3).

## Open, carried over
- rhai Socket re-audit after P9; `smartstring` RUSTSEC-2026-0249 to add to ARCH §13; `ttf-parser` 0.25.1
  RUSTSEC-2026-0192.
- Editor's toolkit: open until Editor v1 is planned. Bevy 0.20 not before 2026-11-07.
- Backlog: removing or editing a party member. Unscheduled: `DevCommand::Pass`; the `data.*` ops;
  `scripts/mcp-probe.py`. Small: nothing marks rows below a panel's fold; pack text not checked for non-ASCII.
- The owner's asset work (`.gitignore`, `assets/`, two untracked `cutout-basic-*.svg`) is theirs. Never stage
  it; commit only named paths (`git commit -- <paths>`, after `git add` for new files).

## Pins
- Gate `tests passed 757 failed 0 ignored 13` at P5b+c. Vector: 135 integration + 32 unit.
- Walk replay `78e57e238dcb93c0`, fight `48d160014c6a627a`; `SAVE_SCHEMA` 7, `PROTOCOL` 2, 25 ops, MCP 25
  tools (unchanged by P5).
- Sentrux: scan `crates/`: rules pass, quality 8984.

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, `cargo fmt --all` first. Vector clippy:
  `--lib`, `--no-default-features --lib`, `--features test-fixtures --tests` (each written out; zsh does not
  split a variable).
- **Tests:** `--test integration` with `--features test-fixtures`. Shared screen ctx: `common::ctx(data,
  world, views)`. App shells `common::app::{app_with, set, key}`; the test shell has no `UiPlugin` (no layout:
  check resources and markers, not rects).
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1` in the background, a new file each run,
  no edits while it runs.
- **Captures:** a temporary `PROBE-TEMP` hook (screens: in `shell/screen.rs::open_start` reading
  `PROBE_SCREEN`; places: in `main.rs` after `with_party`), built with `--features test-fixtures`, run as
  `./target/debug/omnis-vector --screenshot .omnis/x.png --size WxH`. Remove it and grep `PROBE`.
- **Mutations:** `python3 -u` to a file, never under `timeout`; check the sources are restored.
- **Unfinished work** before a compact is committed as `WIP`, never stashed. Stage by name, never `.claude/`.

## Carry-over
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` (omnis-app) 902, `omnis-vector/tests/screens.rs`
  about 900, `save_and_replay.rs` 810, `shell/screen.rs` (vector) about 760, `tests/service.rs` about 700.
