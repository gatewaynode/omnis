# Continuity notes

Written 2026-10-11, after P5a–P5c and before the compact that precedes P5d. Rewrite this file every time it
is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, not pushed (the owner pushes).
- **Accepted by the owner 2026-10-11:** P2, P3, the party column, P4.
- **P5 (inventory, spells on the road, message line), plan `tasks/plans/p5.md`, approved.** Owner answers:
  the kit beside the stores with a member switch; **the party column stays beside** the inventory and cast
  screens and its selection is the target; the cast screen stays open after a cast; the message line in P5.
  - **P5a done, `315b823`** (gate 745): engine bug fixed (`CastView.cost` from the `spell_points.cost` slot,
    red test `api_views::a_cast_row_costs_what_the_cost_slot_charges`; `docs/api.md` §7.8); `Views.casts`;
    `Forms.selected` (moved from `Screens.selected`); `labels::{slot, count_label, reason}`; item and spell
    lines in `shell/text.rs`.
  - **P5b + P5c done, `3795b10`** (gate **757/0/13**, mutations 8/8, Sentrux 8984).
  - **P5d is next:** docs, acceptance, continuity. Nothing of it is written yet.

## P5d to do (after the catchup)
1. **presentation-ARCHITECTURE** (edits within the approved plan; status line "after P5"):
   - §4.2 `Views.casts`; §4.4 the cost defect fixed (row "Casting on the road" note);
   - §6.1 `ControlId::{Owner, Item{at, act}, Cast(n)}`, `Forms.{selected, owner}`;
   - §6.3 overlays Inventory and Cast (opened by the column, `Nav::show`, `paused`, allowed while base is
     none so read-only in a fight, not over a service); Esc closes;
   - §6.4 the column beside a screen (`Beside`, `make_room` pads the screen layer);
   - §6.6 the party column: Inventory live, Spells dim "No spells for the road", the column shown beside the
     two screens; the message line as built (`shell/message.rs`, `Session.message`: newest non-move line,
     refusal alert, cleared by a bare move, shown only exploring with no screen up; movement refusals have
     no line, the binder only counts them);
   - §6.7 Inventory and Cast rows "(built, P5)" with what was added: buttons per row by the view's facts, every
     button lit by a trial (the other client let dead/down/two-handed through), Give's column hints, counts
     of 1, "Nothing carried"/"Nothing in the stores", "{name} knows no spell for the road".
2. `tasks/acceptance/p5.md` (format of `p4.md`): `just run --windowed --seed 1`; select Brenna in the column →
   Inventory: take off and equip the longsword; select Pip, Give a crossbow bolt ("Brenna gives Crossbow bolt
   to Pip"); Stow, then Take from the stores; a potion needs MCP or the smith? (the smith sells no potion —
   check `service_get` at the smith; otherwise skip Use or give one via a dev command); Durin → Spells: Cure
   Wounds "on Brenna" after selecting Brenna, Light "lights the party's way", screen stays open; Esc closes;
   the message line after Interact ("There is nothing to do here"), a refusal in red, cleared by a step; in a
   fight the inventory only looks.
3. `tasks/plans/p5.md`: add "As built" (no cache: ~0.35 ms a build; no two-handed melee weapon in the packs, so
   "no hand free for a shield" is only unit-tested; the column's tool row follows the selection; kit buttons
   wrap at 1280×720 in the half-width panel).
4. code-map (vector: `screen/{inventory,cast}.rs`, `shell/message.rs`, `Beside`, `Forms.{selected,owner}`,
   `Views.casts`, `labels::reason`), TODO: tick P5d and P5 with the review, CONTINUITY.
5. Commit by named paths. Then the owner's acceptance.

## Facts
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
