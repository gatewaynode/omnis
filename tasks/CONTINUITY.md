# Continuity notes

Written 2026-10-11, after P6 and the owner's two rounds of manual-test fixes, before a compact. Rewrite this
file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, not pushed (the owner pushes).
- **Accepted by the owner:** P2, P3, the party column, P4, P5 (all 2026-10-11).
- **P6 built** (`46844c0`, `7b17c9d`, `4976c33`, `aa9c860`), then two rounds from the owner's manual test:
  - `96c016d` P6 fix: the screens' layer carries `GlobalZIndex(1)` (the HUD's lines drew over Camp); `Grab`
    runs every frame (R opening Camp while the mouse looked left the cursor locked); `grab_cursor` moved from
    `shell/render.rs` to `shell/movement.rs` (headless, so the test shell runs it). `tests/overlay.rs`.
  - `9c6b358` P6 additions (owner asked): a `bevy_ui_widgets` `Scrollbar` beside each screen panel, shown
    only while its rows overflow (`screen_draw::bars`); party column entries yellow (`Role::Caution`) under
    half HP, red (`Role::Alert`) under a quarter, down members red, dead dim; `Row::tint` carries the role to
    a row's live controls.
  - The owner's last word on these: "Better." **P6 acceptance is not yet stated.** After the compact, ask
    whether P6 is accepted (or whether more of `tasks/acceptance/p6.md` remains to run); on acceptance record
    it on P6's TODO line (as P5's), with the owner's words.
- **Then:** plan P7 in plan mode: fight-screen parity (features, React, the Use picker, the budget line).
  Engine first: `SpellView`'s action cost (`pay`, as `KnownSpellView.pay` and `FeatureView.pay`), the fight's
  usable items with refusals (§4.4); the fight's controls onto the screen kit (§6.5). Read cold first:
  presentation-ARCHITECTURE §4.4, §6.5, §6.7 (the Fight row), §9; old client `combat_menu.rs` (`:174` the cost
  compare), `use_menu.rs` (`:103` "not here"), the vector `combat_menu.rs` and `shell/combat.rs`. Ask the owner
  the layout questions before the plan; state any defect as shown versus actual (LESSONS 2026-10-11).
- Calls made without asking, reported, not yet confirmed: down members red; dead members dim with "dead" in red.

## Facts
- **Camp:** start (town street) is Explore, so Camp is live there; inside a service and in a fight it is dim.
  The night eats 4 food (1 a living member); a long rest needs `long_rest_every_minutes` (1440) since the last.
  Brenna d10, Durin d8, Ilvara d6, Pip d8; level 1, one die each. Brenna 12 HP max, Pip 9.
- **Sheet (Durin):** HP 12/12 SP 2/2 AC 15; race label "Dwarf (hill dwarf)".
- **Tactics:** no fixed member knows a reaction spell; Shield (wizard) is the guild's (10, 1). Nested criteria
  JSON for the lock is in the acceptance script, Part 3 step 7.
- **Layering:** only the screen layer has a `GlobalZIndex` (1); every other root is 0. A screen panel is a row
  node holding the panel (`Scrolls`) and its scrollbar.
- Only R opens a pausing screen by key; clicks already turn mouse look off.
- Town sites: inn (1,1), temple (4,1), trainer (7,1), guild (10,1); smith (1,3), tavern (4,3), bank (7,3).
  Start (10,2) West, 60 gp, 40 food.

## Open, carried over
- rhai Socket re-audit after P9; `smartstring` RUSTSEC-2026-0249 to add to ARCH §13; `ttf-parser` 0.25.1
  RUSTSEC-2026-0192.
- Editor's toolkit: open until Editor v1 is planned. Bevy 0.20 not before 2026-11-07.
- Backlog: removing or editing a party member. Unscheduled: `DevCommand::Pass`; the `data.*` ops;
  `scripts/mcp-probe.py`. Small: pack text not checked for non-ASCII; the green/yellow/red health colours not
  checked for colour deficiency (the style step's check, presentation-ARCHITECTURE §7).
- The owner's asset work (`.gitignore`, `assets/`, two untracked `cutout-basic-*.svg`) is theirs. Never stage
  it; commit only named paths (`git commit -- <paths>`, after `git add` for new files).

## Pins
- Gate `tests passed 781 failed 0 ignored 13`, VERIFY-GREEN at `9c6b358`. Vector: 157 integration + 32 unit.
- Walk replay `78e57e238dcb93c0`, fight `48d160014c6a627a`; `SAVE_SCHEMA` 7, `PROTOCOL` 2, 25 ops, MCP 25
  tools (unchanged since P5).
- Sentrux: scan `crates/`: rules pass, quality 8995.

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, `cargo fmt --all` first. Vector clippy, each
  written out (a shell loop over quoted flag strings splits them wrongly): `--lib`; `--no-default-features
  --lib`; `--features test-fixtures --tests`.
- **Tests:** `--test integration` with `--features test-fixtures` (the file is `tests/main.rs`; a new test file
  is added there). App shells `common::app::{app_with, set, key, click, press_ui, tap, top}`. The test shell
  has no `UiPlugin`: check components and markers, or set `ComputedNode` fields by hand (as `overlay.rs`).
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1` in the background, a new file each run,
  no edits while it runs.
- **Captures:** a temporary `PROBE-TEMP` block in `shell/screen.rs::open_start` (env `PROBE_SCREEN`; back the
  file up to the scratchpad first and restore it), built with `--features test-fixtures` (it reaches
  `world.party` mutably), run as `./target/debug/omnis-vector --seed 1 --screenshot .omnis/x.png --size WxH`.
  Fill the HUD's recent lines for a new screen (LESSONS 2026-10-11). Grep `PROBE` after.
- **Mutations:** `python3 -u` to a file, never under `timeout`; check the sources are restored.
- **Unfinished work** before a compact is committed as `WIP`, never stashed. Stage by name, never `.claude/`.

## Carry-over
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` (omnis-app) 902, `omnis-vector/tests/screens.rs`
  about 840, `save_and_replay.rs` 810, `shell/screen.rs` (vector) 756, `tests/service.rs` about 700,
  `screen/mod.rs` (vector) 678, `screen/tactics.rs` 588, `shell/screen_draw.rs` 512.
