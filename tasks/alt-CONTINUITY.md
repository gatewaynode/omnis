# Alt continuity notes (the 3D experiment)

Written 2026-10-03, before a compact in the middle of Phase B (B2 done, B3 next). Rewrite this
file every time it is used; keep it to state, next step, pointers and gotchas. It is kept apart
from `tasks/CONTINUITY.md`, which belongs to the main build (stale; it describes
`m6-closeout-tasks`).

## State
- **Branch:** `gui-3d-experiment`, cut from `main` at `8e111d5`.
  - **The owner pushes:** the agent's sandbox has no GitHub access (memory `owner-pushes`).
  - The owner pushed through `5083a2c`. These are local until they push again: `bcee3b9` (Phase B plan), `6e856e5` (B1), `17c5552` (B1a), `6a68900` (B2), and this file's commit. Check with `git status -sb`.
- **Clone:** `/Users/john/code/omnis-alt/omnis`. The main checkout is `/Users/john/code/omnis`.
- **Gate:** green at 425 passed, 6 ignored. Sentrux rules pass at signal 8911.
- **Phase A is complete** (A0–A8; the report is in `tasks/alt-TODO.md`).
- **Phase B, the 2D combat screen.** The plan, `~/.claude/plans/snug-munching-gray.md`, was approved 2026-10-03. **Read it first.**
  - **B0, B1, B1a, B2 done** (details in `tasks/alt-TODO.md`).
  - **B1, the roll log:** `src/rolllog.rs`.
    - `describe(&Event, &Names, &Data)`.
    - `Names` numbers each monster as it was met (`follow` on `EncounterStarted` and `Death`), and keeps members a fight buries (`observe`).
    - `Session.fight_log` holds the last 40 lines, cleared when monsters are met. `Session.names` is updated in `note()`.
  - **B1a, the picture window (owner request after playing the modal):**
    - `src/cinema.rs`: `Scene::Enemy(MonsterId)`; `opening(world)`; `drawing` (the placeholder rat for every monster); `paint` fits the drawing's bounds inside a 12 px margin, as a glow under a core on an opaque ground; `SIZE` is 492×200.
    - `Notice.scene`.
    - The panel spawns a `shell::panel::Screen(scene)` node as its first child in a fight.
    - `shell/cinema.rs::CinemaPlugin` (window or capture only) uploads the image.
    - `Raster` moved to `src/raster.rs` (`filled`, `put`, `lighten`, `stroke`).
    - Horizons: scenes queued from fight events, animated; a drawing per monster.
  - **B2, the action model:** `src/combat_menu.rs`.
    - `CombatMenu{step: Step::{Top, Spells, Items, Target(Action)}}`, with `Action::{Attack, Cast(u8), Use(u8), Swap}`.
    - `prompt`, `entries -> Vec<Entry{label, act: Act::{Command, Open(Step), Back}, blocked}>`, `clickable -> Vec<Pick>`, `choose(&Act) -> Option<Command>`, `back`, `pick(world, data, Pick) -> Option<Command>`.
    - `Pick::{Stack(u8), Member(slot)}`.
    - Free functions: `command(action, pick)` and `targets(world, data, action)`, found by trial.
    - `src/trial.rs`: `refusal` and `accepted` on a world clone. `shell/notice.rs` uses it.
- **Owner decisions:**
  - Phase B (alt-PRD §10.3): **write fresh**. Glowing vector-line figures. Every fight action, with targets picked by clicking. A short roll log.
  - **2026-10-03:** they like the modal, but **the full 2D screen stays the plan** (B3/B4 as written). The picture window carries into B4, **above the action column**.
- **Standing owner rules:**
  - UI only; mechanics changes go to the mechanics branch.
  - **Rebase onto the mechanics work before merging** (B7).
  - Buttons before keys.
  - One commit per item, the gate and Sentrux before each.
- **Keys:** extended WASD (W/S, A/D, Q/E). Space or a right click is the action key, Shift with it runs the default action, and a left click toggles mouse look.
- **Owner displays:**
  - The ultrawide is a Samsung LS49AG95, 5120×1440 at 144 Hz.
  - macOS's main display is an LS27A800U, 60 Hz, 4K scaled to 1920×1080.
- **Don't repeat it:** a message in the first session looked like a password. It was not used or stored.

## Next: B3, the arena (`src/arena.rs`, Bevy-free), then B4–B6
- **Layout** in logical pixels for a given window size, as plain data a test can check:
  - **Monster stacks** across the top half; front stacks larger and lower.
    - One figure per living individual, up to 8, then "×N".
    - Each figure is scaled by the monster's `Size` (`data.monsters[&id].size`) and has an hp bar.
    - Read from `combat_view` (`StackView`: index, name, initial, hp per living, front, alive, reachable).
  - **The party** in two rows across the bottom, the front row nearer. Each member has a figure, a name slot, an hp bar, and a mark for down or dead.
    - Read from `omnis_sim::ops::party_view(world, data).members` (`MemberView`: index, name, hp, hp_max, front, conditions, plus down and dead).
  - **Highlights:** the acting member (`combat_view().current`), the valid targets (`CombatMenu::clickable`), and the hovered pick.
  - **Space** for the picture window (`cinema::SIZE`) above the action column, and for the roll log on the right.
- `segments(&Arena) -> Vec<Seg2>` for drawing, and `pick(&Arena, point) -> Option<Pick>` (reuse `combat_menu::Pick`).
- **Tests:**
  - figures and counts per stack;
  - two party rows;
  - a pick hitting the right stack or member, and missing between figures;
  - scaling at 1600×900 and 5120×1440;
  - nothing off-screen or overlapping the picture window or log.
- **B4:**
  - `ViewState` (needs `StatesPlugin` in headless tests).
  - The `Camera3d` is deactivated; a `Camera2d` gets `Hdr`, `Bloom`, `Tonemapping::None`, `RenderLayers::layer(1)` and `IsDefaultUiCamera` (moved from the 3D camera).
  - Copy the offscreen `RenderTarget` for captures.
  - `CombatGizmos` on layer 1.
  - Hide the movement pad and minimap in Fight.
  - `follow()` and `reshape` on exit.
  - Wire `CombatMenu` buttons, digits, Esc = back, and clicks via `arena::pick`.
- **B5:** retire the A7b notice's fight content (`shell/fight.rs`, `panel::current`), adapt `tests/fight.rs`, docs as built.
- **B6:** the owner plays. **B7:** rebase before merging.

## Gotchas
- **Builds need `export DEVELOPER_DIR=/Library/Developer/CommandLineTools`.** The Xcode licence is unaccepted.
- **The gate:** run `scripts/verify.sh > <scratchpad>/verify.log 2>&1` unpiped, then grep `tests passed` and `VERIFY-GREEN`. Clippy is `-D warnings`: use `as_chunks::<4>()` rather than `chunks_exact(4)`, and `next_back()` rather than `last()` on double-ended iterators.
- **Sentrux: scan `/Users/john/code/omnis-alt/omnis/crates`, not the repo root.** The rules file is `crates/.sentrux/rules.toml` (ignored by git). From the root, `check_rules` finds no rules. `import_edges` is 0, so check `use super` cycles in `src/shell` by hand.
- **Windows get no frames from the agent's shell.** Use the offscreen capture: `./target/debug/omnis-vector --screenshot .omnis/shot.png --walk N [--size WxH]`, then Read the PNG.
- **To capture an encounter:**
  - Back up `src/shell/session.rs` to the scratchpad.
  - In `Session::start`, before `let pose = Pose::at(world.position);`, set `world.position` to the dungeon's (3, 6) facing South.
  - Run with `--walk 90`, then restore the backup and check that `git diff` is empty. Never commit the placement.
- **`~/Desktop` is blocked for the agent.** The owner copies files into `.omnis/`.
- **Headless input:**
  - Keys and mouse buttons go in as `KeyboardInput` and `MouseButtonInput` messages.
  - Buttons get their `Interaction` set directly (`tests/common/app.rs::set`).
  - The builders are `tests/common/app.rs::{app, app_logging_to, app_with(log, seed), met_with(seed)}`. `met_with` walks into the placed group: seed 1 fights, seed 2's run succeeds.
  - For a Bevy-free test, take the `Session` out with `app.world_mut().remove_resource::<Session>()`.
- **The seed-1 fight:** two giant rats. Initiative: Pip 19, rats 18, Durin 9, Brenna 8, Ilvara 4.
  - The party: Brenna (fighter, slot 0), Durin (cleric, slot 1), Ilvara (wizard, slot 2), Pip (rogue, slot 3).
  - Everyone carries a "Potion of healing" (lower-case h; labels are as the pack writes them).
- **The binder logs only `Ok` commands.** Choices are tried on a clone first.
- **`Interact` with nothing there returns `Ok`** with `NothingHere`.
- **A test that places the party directly cannot replay its log.**
- **Python edit scripts:** assert that each anchor exists. `cargo fmt` rewraps lines, so read the current text before anchoring on it.
- **Mutation checks:** back up the file to the scratchpad, break the fix with perl, run the test, restore, and confirm `git diff` is clean.
- **Commit trailer:** `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Pointers
- **Vision:** `alt-PRD.md` (Phase B in §6, X7, §10.3) and `alt-ARCHITECTURE.md` (§6 plugin table, with `CinemaPlugin` added; §8; §9, with the picture window and roll log as built).
- **Plan and reviews:** `tasks/alt-TODO.md` (B items). The Phase B plan file is listed above.
- **Core:** `crates/omnis-vector/src/{geom,grid,pose,collide,bind,geometry,minimap,party,raster,rolllog,cinema,trial,combat_menu}.rs`.
- **Shell:**
  - `src/shell/{mod,session,movement,controls,actions,notice,fight,panel,render,hud,minimap,cinema,text,capture}.rs`
  - `fight.rs` holds the A7b notice that Phase B replaces.
  - `panel.rs::current` shows it.
- **Tests:** `tests/{agreement,binding,shell,fight,actions,minimap,rolllog,combat_menu}.rs` and `tests/common/{mod,app}.rs`.
- **Simulation reads:**
  - `omnis_sim::{combat_view, CombatView, StackView, SpellView, bribe_cost, CombatCommand, Target, EncounterChoice, apply}`
  - `omnis_sim::ops::party_view`
  - `omnis_sim::combat::state::can_fight`
  - Event fields: `crates/omnis-sim/src/event.rs`.
- **Rules:** `tasks/LESSONS.md` (entries 2026-10-02 and 2026-10-03) and `tasks/knowledge/agreements.md`.
