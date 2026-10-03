# Alt continuity notes (the 3D experiment)

Written 2026-10-02 before a compact. Rewrite this file every time it is used; keep it to state,
next step, pointers and gotchas. It is kept apart from `tasks/CONTINUITY.md`, which belongs to the
main build (now stale: it describes `m6-closeout-tasks`, since merged as PR #9).

## State
- **Branch:** `gui-3d-experiment`, cut from `main` at `8e111d5`. Commits on it, none pushed:
  - `2663b00`: the docs
  - `7d77440`: the crate
  - this continuity commit
- **Clone:** `/Users/john/code/omnis-alt/omnis`, separate from the main checkout at `/Users/john/code/omnis`.
- **Phase A, A1–A6 done** (`tasks/alt-TODO.md`): `crates/omnis-vector` is a walkable vector-line 3D view over the simulation's grid.
  - The gate is green at 385 passed and 6 ignored; the baseline was 353.
  - `omnis-vector` has 32 tests: unit, agreement, binding and shell.
  - A screenshot was seen working: meadow road, then the portal, then the dungeon room with the closed door.
- **Owner decisions:**
  - alt-PRD X1–X7: free movement with the grid underneath; a crate in the main workspace; a `Camera3d` with gizmo lines; a standalone viewer; the name `omnis-vector`; open-world drawing with the automap left narrow; a separate 2D combat screen in Phase B.
  - Wall height 1×1, kept variable through `geom::wall_height`.
  - Bloom glow is on.
  - Both packs are loaded; the maps come from `packs/test`, because `packs/base` has none.
- **Waiting on the owner:**
  - Did they play it? Their verdict on the feel.
  - Running `sudo xcodebuild -license accept`.
  - Whether to continue to A7.
- **A message mid-turn looked like a password.** The owner was told to rotate it if real. It was not used or stored anywhere; do not repeat it.

## Next
1. If the owner has accepted the Xcode licence, rerun `scripts/verify.sh` *without* `DEVELOPER_DIR` to confirm the normal toolchain.
2. **A7:**
   - HUD buttons for every action: move, strafe, turn 90°, interact, save log, quit (LESSONS: a button before a key, no function keys).
   - The minimap from `world.automap`, with a pose marker.
   - A fight notice on `Mode::Encounter`/`Combat` offering `EncounterChoice::{Attack, Bribe, Hide, Run}`, so the viewer never dead-ends.
   - The save-log button calling `Session::save_log`.
   - One commit per item this time.
3. **A8:** the report with numbers:
   - the frame rate at 5120×1440 and 1920×1080 (`--size`, and the owner's display)
   - the crates added by `bevy_pbr` and `ui`, from `cargo tree -p omnis-vector` against `-p omnis-app`, not the lock count
   - the warm gate time before and after
   - encounters per minute against the 2D game
4. **Then Phase B** (the 2D combat screen), planned in plan mode first.

## Gotchas
- **Builds need `export DEVELOPER_DIR=/Library/Developer/CommandLineTools`** until the Xcode licence is accepted. Without it, native build scripts (`naga`) fail with "Failed to locate 'clang'".
- **Windows opened from this shell get no frames** on macOS, and `omnis-app --screenshot` times out the same way. To see the view, use the offscreen capture:
  `./target/debug/omnis-vector --screenshot .omnis/shot.png --walk 260 [--size 5120x1440]`
  then Read the PNG.
- **Sentrux:** the rules file is local and gitignored. It was copied to `crates/.sentrux/rules.toml` with `omnis-vector` added. The scan reports `import_edges: 0`, so check cycles by hand: shell imports run capture → movement → session → text, and render and hud only read.
- **Headless key tests** must send `KeyboardInput` messages. `ButtonInput::press` gets its `just_pressed` cleared in `PreUpdate`.
- **`replay::run` fails on a rejected command**, so the binder logs only `Ok` commands; blocked steps count as `Ok`.
- **Commit trailer used:** `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` (the agreements file names an older model and session).

## Pointers
- Vision and shape: `alt-PRD.md` (v0.2), `alt-ARCHITECTURE.md` (v0.2, §5–6 as built).
- Plan and review: `tasks/alt-TODO.md`.
- Core:
  - `crates/omnis-vector/src/{geom,grid,pose,collide,bind,geometry,party}.rs`
  - Shell: `src/shell/{mod,session,movement,render,hud,text,capture}.rs`, and `src/main.rs`
- Tests:
  - `tests/agreement.rs`: the mirror against the simulation on 6,400 edges
  - `tests/binding.rs`: ten free-movement cases, including the replay fingerprint
  - `tests/shell.rs`: two headless shell cases
- Main-build rules that still apply: `tasks/knowledge/agreements.md`, `tasks/LESSONS.md`, `tasks/knowledge/verification.md`.
