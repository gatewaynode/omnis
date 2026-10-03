# Alt continuity notes (the 3D experiment)

Written 2026-10-03, when the branch was first pushed. Rewrite this file every time it is used;
keep it to state, next step, pointers and gotchas. It is kept apart from `tasks/CONTINUITY.md`,
which belongs to the main build (stale: it describes `m6-closeout-tasks`, since merged as PR #9).

## State
- **Branch:** `gui-3d-experiment`, cut from `main` at `8e111d5`. Pushed to `origin` on 2026-10-03, if the push in this session succeeded; check with `git status -sb`.
- **Clone:** `/Users/john/code/omnis-alt/omnis`, separate from the main checkout at `/Users/john/code/omnis`.
- **Phase A is complete** (`tasks/alt-TODO.md`: A0–A8, each with a review). The A8 report answers alt-PRD §7. The gate is green at 407 passed and 6 ignored, about 11 s warm.
- **What the viewer is:** `cargo run -p omnis-vector [-- --windowed] [--no-vsync] [--monitor N]`.
  - Free movement in a glowing vector-line 3D view over the simulation's grid.
  - Extended WASD: W/S move, A/D sidestep, Q/E turn 90°, arrows also work.
  - Mouse look: a left click toggles it, and Space, a right click or Esc release it.
  - The action key, Space or a right click, opens the square's action menu; with Shift it runs the default action.
  - A fight notice: the encounter choices, a placeholder Attack, Dodge and Flee, and Start again.
  - A minimap from the automap.
  - Buttons for everything, with Actions, Save log and Quit bottom right.
  - The HUD shows a smoothed fps and frame time, and the window size.
- **The owner's verdicts:**
  - "Pretty neat, useable."
  - Buttons fixed in A7d.
  - Keys and the action menu as they directed (A7e).
  - The mouse trap fixed in A7f.
- **Measured:**
  - 280–310 fps (about 3.3 ms a frame) fullscreen at 5120×1440, vsync off, the same moving and still.
  - 7 crates added, none new to the lock.
  - Gate 10 s on `main` against 11 s on the branch.
- **The owner's displays:**
  - The ultrawide, a Samsung LS49AG95, is where they work: 5120×1440 at 144 Hz.
  - macOS's *main display* is an LS27A800U: a 4K panel scaled to 1920×1080, at 60 Hz.
  - A MacBook Air is a third display.
  - With vsync on, the frame rate falls to about 60 when still: pacing by the main display, not a cost.
  - The owner waived the 1920×1080 reading.
- **Owner scope rule (2026-10-03):** this branch is UI. Any change to underlying mechanics belongs to the mechanics branch.
- **The Xcode licence is unknown.** Builds here still use `DEVELOPER_DIR` (see Gotchas).
- **A message in the first session looked like a password.** The owner was told to rotate it if real. It was not used or stored anywhere; do not repeat it.

## Next
1. **Phase B, the 2D combat screen.** Plan it in plan mode with the owner first (alt-TODO B0).
   - Decide alt-PRD §10.3: reuse `omnis-app`'s Bevy-free `combat_menu.rs`, or write fresh.
   - It replaces A7b's placeholder Attack, Dodge and Flee in `shell/fight.rs`. The panel (`shell/panel.rs`) keeps the encounter choices or hands over to the new screen.
   - Per X7: on a fight, the `Camera3d` steps aside for a `Camera2d` screen with placeholder sprites, the action list and the roll log.
2. **Open items, none blocking:**
   - `--monitor N` is unverified: index 1 still opened on the ultrawide.
   - With vsync on, the frame rate exceeds the refresh while keys are held (seen as about 144 on the primary). Harmless.
   - The WASD-shaped on-screen pad and UI scaling with the window were offered and not picked.
   - For the mechanics branch: `Interact` finds a door only on the party's own cell's side of an edge.

## Gotchas
- **Builds need `export DEVELOPER_DIR=/Library/Developer/CommandLineTools`** until the Xcode licence is accepted ("Failed to locate 'clang'" otherwise).
- **Pushing from the agent's shell failed:** `git@github.com: Permission denied (publickey)`, because the agent cannot reach the SSH agent. The owner pushes with `! git push -u origin gui-3d-experiment`.
- **Windows opened from the agent's shell get no frames.** Use the offscreen capture:
  `./target/debug/omnis-vector --screenshot .omnis/shot.png --walk 260 [--size WxH]`
  then Read the PNG. `--walk 260` ends in the dungeon at (1, 5); `--walk 20` stays on the meadow.
- **To capture a panel** (the fight notice or the action menu), use a temporary placement in `Session::start`: (3, 6) facing South with `--walk 50` for the rats, and (3, 5) facing South with `Menu { open: true }` for the menu. Edit a backup copy and restore it; never commit the placement.
- **`~/Desktop` is blocked by macOS privacy** for the agent. The owner copies files into `.omnis/`.
- **Sentrux:** the rules file is local and gitignored, at `crates/.sentrux/rules.toml`. The scan reports `import_edges: 0`, so check cycles by hand (`grep '^use super' src/shell/*.rs`). Today `notice` is a leaf, and `panel` sits above `fight`, `actions` and `controls`.
- **Headless input:** keys and mouse buttons go in as `KeyboardInput` and `MouseButtonInput` messages, because `ButtonInput::press` loses `just_pressed`. Button tests set `Interaction` directly (`tests/common/app.rs::set`).
- **The binder logs only `Ok` commands.** The panel tries each choice on a clone of the world.
- **`Interact` with nothing there returns `Ok`** with `MessageKey::NothingHere`, not a refusal (`actions::does_something`).
- **Seed 1 fails the Run check** at the placed group; seed 2 succeeds. A test that places the party directly cannot replay its log.
- **One target directory shared with a `main` checkout** rebuilds Bevy for about 66 s on each switch (feature unification).
- **Commit trailer:** `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Pointers
- Vision and shape: `alt-PRD.md` (v0.2), `alt-ARCHITECTURE.md` (v0.2; §6, §8 and §9 as built through A7f).
- Plan, reviews and the A8 report: `tasks/alt-TODO.md`.
- Core (no Bevy): `crates/omnis-vector/src/{geom,grid,pose,collide,bind,geometry,minimap,party}.rs`.
- Shell: `src/shell/{mod,session,movement,controls,actions,notice,fight,panel,render,hud,minimap,text,capture}.rs`, and `src/main.rs`.
- Tests:
  - `tests/{agreement,binding,shell,fight,actions,minimap}.rs`
  - `tests/common/{mod,app}.rs`
- Main-build rules that still apply: `tasks/knowledge/agreements.md`, `tasks/LESSONS.md` (entries 2026-10-02 and 2026-10-03), `tasks/knowledge/verification.md`.
