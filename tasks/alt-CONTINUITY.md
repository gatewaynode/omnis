# Alt continuity notes (the 3D experiment)

Written 2026-10-02 at the end of the day; the owner resumes tomorrow. Rewrite this file every
time it is used; keep it to state, next step, pointers and gotchas. It is kept apart from
`tasks/CONTINUITY.md`, which belongs to the main build (now stale: it describes
`m6-closeout-tasks`, since merged as PR #9).

## State
- **Branch:** `gui-3d-experiment`, cut from `main` at `8e111d5`. None of it is pushed:
  - `2663b00`: the docs
  - `7d77440`: the crate (A1–A6)
  - `281e214`: continuity
  - `cf95654` (A7a): the button pad
  - `3ef075a` (A7b): the fight notice
  - `5afd71f` (A7c): the minimap
  - `1d12d4c` (A7d): opaque buttons; Use, Save log and Quit moved bottom right
  - this commit: the WASD direction and continuity
- **Clone:** `/Users/john/code/omnis-alt/omnis`, separate from the main checkout at `/Users/john/code/omnis`.
- **Phase A, A1–A7 done** (`tasks/alt-TODO.md`, with A1–A6 and A7 reviews).
  - The gate is green at 398 passed and 6 ignored.
  - `omnis-vector` has 45 tests: unit, agreement, binding, shell, fight and minimap.
- **The owner played it** (windowed, 2026-10-02): "Pretty neat, useable, buttons need some work". Their screenshot is in `.omnis/owner-shot.png`, which is untracked.
  - They picked two fixes, both now in A7d: opaque backgrounds, and Quit kept away from the movement pad.
  - Offered and not picked: a WASD-shaped pad, and scaling with the window.
- **Owner decisions:**
  - alt-PRD X1–X7: free movement with the grid underneath; a crate in the main workspace; a `Camera3d` with gizmo lines; a standalone viewer; the name `omnis-vector`; open-world drawing with the automap left narrow; a separate 2D combat screen in Phase B.
  - Wall height 1×1, kept variable.
  - Bloom glow is on.
  - Both packs are loaded.
  - **New, end of day:** the standard extended WASD layout for 3D (A7e below).
- **The Xcode licence is unknown.** Builds still use `DEVELOPER_DIR` (see Gotchas). Ask whether `sudo xcodebuild -license accept` was run.
- **A message mid-turn in the first session looked like a password.** The owner was told to rotate it if real. It was not used or stored anywhere; do not repeat it.

## Next
1. **A7e, the key layout.** The owner said: "if we are moving to 3D then we should use the standard extended WASD layout. Where Q = turn left, E = turn right, space = use/action menu."
   - The change:
     - Q turns left (as now).
     - E turns right (now R).
     - Space uses (now E).
     - W, S, A, D and the arrow keys stay.
     - The bindings live in `shell/movement.rs::read_input`.
     - The labels live in `shell/controls.rs::Action::label`.
     - The headless test `q_turns_the_party_left_by_a_quarter` stays. Add tests for E turning right and Space using, by `KeyboardInput` messages.
   - **Ask first:** "use/action menu" may mean Space opens an action menu (use now; items, spells and sense later) rather than only using the faced edge. Don't build a menu without the owner's answer. Plain use with Space is safe either way.
   - The fight notice's number keys (1–9) are unaffected.
2. **A8: the report with numbers** (alt-PRD §7):
   - The frame rate at 5120×1440 and 1920×1080. Ask the owner for the HUD's fps fullscreen on their primary display (`cargo run -p omnis-vector`). Offscreen numbers come from `--size`.
   - The crates added by `bevy_pbr` and `ui`, from `cargo tree -p omnis-vector` against `-p omnis-app`, not the lock count.
   - The warm gate time, before and after.
   - Encounters per minute against the 2D game.
3. **Then Phase B** (the 2D combat screen), planned in plan mode first. It replaces A7b's placeholder Attack, Dodge and Flee.

## Gotchas
- **Builds need `export DEVELOPER_DIR=/Library/Developer/CommandLineTools`** until the Xcode licence is accepted. Without it, native build scripts (`naga`) fail with "Failed to locate 'clang'".
- **Windows opened from this shell get no frames** on macOS. To see the view, use the offscreen capture:
  `./target/debug/omnis-vector --screenshot .omnis/shot.png --walk 260 [--size 5120x1440]`
  then Read the PNG. `--walk 260` ends in the dungeon at (1, 5); `--walk 20` stays on the meadow.
- **To capture the fight notice,** you need a temporary placement: set `world.position` in `Session::start` to dungeon (3, 6) facing South, then use `--walk 50`. Do it from a backup copy and restore it; never commit the placement.
- **Reading `~/Desktop` is blocked** by macOS privacy ("Operation not permitted"). Ask the owner to copy files into `.omnis/` with `! cp ...`.
- **Sentrux:** the rules file is local and gitignored, at `crates/.sentrux/rules.toml`. The scan reports `import_edges: 0`, so check cycles by hand. Imports today:
  - controls → movement, session
  - fight → controls, session
  - movement → fight, session
  - minimap (shell) → core minimap, session
  - render → capture, fight, movement, session
  - hud → controls, session
  - nothing imports render or hud
- **Headless key tests** must send `KeyboardInput` messages, because `ButtonInput::press` gets its `just_pressed` cleared. Button tests set `Interaction` directly (`tests/common/app.rs::set`).
- **The binder logs only `Ok` commands**, because a replay fails on a refused one. The fight notice tries each choice on a clone of the world, so refusals stay at 0.
- **A test that places the party directly cannot replay its log.** The binding replay test walks there instead.
- **Seed 1 fails the Run check** at the placed group; seed 2 succeeds.
- **Commit trailer:** `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Pointers
- Vision and shape: `alt-PRD.md` (v0.2), `alt-ARCHITECTURE.md` (v0.2; §6, §8 and §9 as built through A7d).
- Plan and reviews: `tasks/alt-TODO.md`.
- Core (no Bevy): `crates/omnis-vector/src/{geom,grid,pose,collide,bind,geometry,minimap,party}.rs`.
- Shell: `src/shell/{mod,session,movement,controls,fight,render,hud,minimap,text,capture}.rs`, and `src/main.rs`.
- Tests:
  - `tests/agreement.rs`
  - `tests/binding.rs`
  - `tests/shell.rs`: keys, buttons, save, layout
  - `tests/fight.rs`
  - `tests/minimap.rs`
  - `tests/common/{mod,app}.rs`
- Main-build rules that still apply: `tasks/knowledge/agreements.md`, `tasks/LESSONS.md` (new entry 2026-10-02 on overlays and destructive buttons), `tasks/knowledge/verification.md`.
