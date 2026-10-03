# Omnis Alt — 3D Presentation Experiment PRD

| | |
|---|---|
| Status | Draft v0.2, under discussion (v0.2, 2026-10-02: grid triggers during free movement, open-world rendering, a separate 2D combat screen, the crate named `omnis-vector`) |
| Branch | `gui-3d-experiment` |
| Parent documents | `PRD.md` (v0.5), `ARCHITECTURE.md` (v0.3) |
| Owner | gatewaynode |

## 1. Purpose

Prove that the existing terrain data model can drive a real-time 3D presentation, with free
movement, while the simulation's grid stays authoritative underneath it. As the player moves
freely, the grid cell they occupy is computed continuously. Every cell entered is processed by the
simulation exactly as a step in the 2D game is: movement rules, time, the automap, portals,
encounters, and later traps and perception checks. All of those rules stay in `omnis-sim`, and the
3D client implements none of them.

The look for the proof is deliberately minimal: *Battlezone*-style vector graphics, glowing lines
on black. The art style is not being decided here. Vector lines are the cheapest way to see the
geometry and the grid binding without an art pipeline.

## 2. Relationship to the main PRD

This is an experiment that deliberately runs against the main vision documents. Nothing here
changes `PRD.md` or `ARCHITECTURE.md` unless the owner adopts a result. Where the experiment
contradicts them:

| Main document | Says | Experiment does |
|---|---|---|
| PRD §5 non-goals | "3D rendering" | Renders in 3D |
| PRD D2 | First-person grid crawler, 2D rendering; a real 3D crawler was the alternative not taken | Takes that alternative, for a proof of concept |
| PRD §7.2, §1 | Discrete steps and 90° turns | Free movement and free look, quantized to steps and turns for the simulation |
| PRD D16, ARCH §8.3 | The viewport draws a forward cone to the detail depth, with a horizon band beyond | Draws the scene like an open-world 3D game, with the visibility depth used as fog distance (X6) |
| ARCH §8.1 | "The `3d` group (pbr, gltf) is never enabled" | Enables Bevy 3D features in its own crate only |
| PRD §14 (D26) | Open question: what "modern" means for the viewport, possibly reopening 3D | Gives evidence toward that question |

The main build, which is concerned with RPG rules and mechanics, continues independently. The
experiment adds no requirements to it. Rules the main build adds to cell entry (traps, perception)
reach the 3D client for free, because they arrive through the same `Step` command.

## 3. Decisions log

| # | Topic | Decision | Alternatives considered | Rationale |
|---|---|---|---|---|
| X1 | Movement model | Free movement and mouse look in 3D. The grid cell is computed continuously from the pose. Each cell boundary crossed emits `Command::Step`, and the simulation processes everything that cell entry triggers. A yaw that snaps to a new cardinal direction emits `Command::Turn` | Discrete steps with tweening (a smaller proof); discrete now and free later | Owner decision (2026-10-02). It tests the stronger claim: the grid rules survive a presentation that does not look like a grid. |
| X2 | File space | A new crate in the main workspace, `crates/omnis-vector` | A separate Cargo workspace under `alt/`; a separate repository | Owner decision (2026-10-02). It shares the lock, `verify.sh` and the dependency policy. The cost is in §8. |
| X3 | Rendering | Bevy `Camera3d` with perspective, and geometry drawn as line gizmos | Lines hand-projected onto the existing `2d` features | Owner decision (2026-10-02). It is the real 3D path and can grow into meshes, lighting and models; hand projection proves less and is a dead end. |
| X4 | Scope of the binary | A standalone viewer: the 3D view, movement, and a minimal HUD and minimap, driving `omnis-sim` directly | Reuse `omnis-app`'s canvas UI, menus and combat | Owner decision (2026-10-02). It keeps the experiment out of `omnis-app`'s internals. |
| X5 | Crate name | `omnis-vector` | `omnis-alt`, `omnis-3d` | Owner decision (2026-10-02). |
| X6 | What is drawn | The 3D view behaves like a normal open-world 3D game: the map's geometry is drawn in every direction the camera looks, out to a fog or fade distance. The simulation's forward cone does not limit the 3D view. The automap stays as built: narrow, recorded from the simulation's cardinal cone on each command | Draw only the tiles in the simulation's cone; draw only automap-known tiles | Owner decision (2026-10-02). The automap may be rethought if 3D offers better options (§10). |
| X7 | Combat | A fight replaces the 3D view with a separate 2D combat screen: animated 2D graphics eventually, static placeholders at first. The fight itself is `omnis-sim`'s combat state machine, unchanged | Combat in the 3D scene; a frozen "fight in progress" notice | Owner decision (2026-10-02). Exploration is 3D and combat is a 2D tactical screen. |

## 4. The grid under free movement

The simulation never sees a continuous coordinate. It keeps receiving the same commands it
receives today, and stays `no_std`, integer-only and deterministic.

### 4.1 Positions

- **Two positions.**
  - The simulation's `Position { map, x, y, facing }` is authoritative.
  - The client keeps a continuous *pose*: a world-space x and z and a yaw, as floats. Floats exist only in `omnis-vector`, which is not a simulation crate.
- **Cell mapping.** Cell `(x, y)` occupies the unit square `[x, x+1) × [y, y+1)` on the ground plane. North is −y, as in `omnis-core::geom::Facing`.
- **Facing from yaw.** The simulation's facing is the cardinal direction nearest the yaw. When the nearest cardinal changes, the client emits one `Turn` per 90° of change before any `Step`, so that relative step directions are computed against the right facing.

### 4.2 Cell entry is a step

When the pose crosses into a neighbouring cell, the client converts the absolute direction of the
crossing into `Step(Forward | Back | Left | Right)` relative to the simulation's current facing,
and applies it. Moving sideways or backwards is legal, because `Direction` already carries all
four.

What a step already does as built (`crates/omnis-sim/src/apply.rs:67`, `step`, then `move`, then
`encounter::trigger`):

1. It refuses on a wall, a closed door, impassable terrain or the map edge, with an `Event::Blocked` reason.
2. It moves the party and emits `Event::Moved`.
3. It advances the party clock by the terrain's `step_minutes`.
4. It records the cell as visited on the automap.
5. It follows a portal on the target cell (a teleport).
6. It starts an encounter: a placed group on the cell that is not cleared, or else a roll on the map's random table.

Every successful command then ends in `look` (`apply.rs:239`), which records the cardinal cone to
the automap and emits `Event::Visible`.

`Turn` only rotates the position (`apply.rs:170`). It advances no clock and emits no events of its
own, so free look costs no game time, though each `Turn` still records the new cone to the
automap.

Traps and passive perception checks do not exist in the simulation yet. When the main build adds
them, their natural home is this same cell-entry path, and the 3D client receives them without
change.

### 4.3 Feasibility issues and the rule for each

| Case | Rule |
|---|---|
| Hovering on a boundary emits steps back and forth, and each one costs minutes and rolls for an encounter | **Hysteresis.** The client counts a cell as entered only when the pose is a margin (for example 0.15 of a cell) past the boundary. |
| A diagonal crossing passes through a corner | The movement segment's crossings are ordered by where the segment meets each boundary, and emitted as two steps in that order. If the first step is refused, collision slides the pose along the wall and the second step is re-evaluated. |
| The simulation refuses a step the client thought was legal | The simulation wins. The pose is pushed back into the authoritative cell. The client collides against cell edges read from `MapData` and the door state, so this is rare. |
| A portal teleports the party | The pose snaps to the destination cell and facing on the second `Event::Moved`. |
| An encounter starts mid-motion | Input freezes, the pose settles inside the entered cell, and the combat screen replaces the 3D view (X7). |
| Run or flight retreats the party (`apply.rs` `retreat`) | The pose snaps to the retreat cell, facing away, when exploration resumes. |
| Time | Game time advances per cell crossed, by `step_minutes`, never per frame, so standing still costs nothing. The terrain's `step_minutes` can also scale walking speed in 3D, making slow terrain slow to cross. |
| Doors | They are opened by `Command::Interact` on the faced edge, where the faced edge follows the cardinal facing. |

### 4.4 Proof of binding

The commands the client emits are logged in the replay format. Replaying that log through
`omnis-cli replay` must reproduce the session's final world fingerprint. That proves the 3D session
was a legal grid session, and that every roll came from the simulation's seeded streams, not from
frame timing.

## 5. Rendering

- **Geometry from terrain data:**
  - wall edges as rectangles from floor to ceiling height
  - doors as distinct line shapes, open or closed
  - `block` terrain as boxes
  - a faint floor grid
  - one line colour per terrain or edge kind, from the terrain's `color`
- **Draw distance:**
  - The whole loaded map can be drawn, with lines fading by distance.
  - The visibility depth of the party's cell (D16: a property of the world, not the camera) sets the fog distance, so a dark dungeon reads close and an open plain reads far.
- **Camera:** first-person at eye height, with a horizontal field of view chosen for both 32:9 and 16:9.

## 6. Proof-of-concept scope

Phase A, exploration (the proof):
- The test pack's maps drawn as in §5.
- Free movement (WASD), mouse look for yaw, and an interact key. Every action also gets an on-screen button, per the LESSONS rule that a button comes before a key, applied to the viewer's HUD.
- The grid binding of §4, with hysteresis, corner ordering, collision and snapping.
- A minimal HUD in a modern font: the simulation's cell and facing, the simulation mode, the last event or refusal, the party clock, and the frame rate.
- A 2D minimap that shows the automap and the party marker.
- The emitted command log, written to a file the CLI can replay.
- Window sizing that works on the owner's 5120×1440 ultrawide and on 16:9 panels.

Phase B, the combat screen (after A reports):
- A 2D screen that replaces the 3D view while the simulation is in Encounter or Combat mode. It shows static placeholder graphics for the monster stacks and the party, the encounter choices, and the combat actions, all driven by `omnis-sim`'s combat commands. When the fight ends, exploration resumes in 3D.

Out of scope for the proof (later, or never):
- Animated combat graphics, textures, meshes, lighting, models, art direction.
- Menus, party creation, character sheets. The viewer loads a fixed party or uses a devtools world.
- Changes to `omnis-sim`, `omnis-data`, `omnis-rules` or `omnis-app`.

## 7. Success criteria

The experiment reports these as numbers or as yes/no:

1. Every map in the test pack renders, and a person can walk it with free movement without passing through a wall.
2. At rest, the cell under the camera always equals the simulation's position. A test that drives the pose headless checks this, including diagonal crossings, boundary jitter, a refusal, and a portal.
3. A placed encounter starts when its cell is entered by free movement, and the random table rolls once per cell entered, never per frame.
4. Replaying the emitted command log reproduces the session's world fingerprint.
5. Frame rate at 5120×1440 and at 1920×1080 on the owner's machine.
6. Crates added to `Cargo.lock` by the Bevy 3D features (the main tree is 323 crates as of M3b), and any duplicates the duplicate check reports.
7. `scripts/verify.sh` stays green, with its run time before and after (§8).

## 8. Constraints and consequences of X2 (shared workspace)

- `omnis-vector` is added to the root `Cargo.toml` member list. That line and the `Cargo.lock` changes are the only edits outside `crates/omnis-vector`.
- `omnis-vector` is not a simulation crate. It may use floats and Bevy, and it is not added to `scripts/lint-sim.sh`. It must never pass a float into the simulation: its only channels in are `Command`s, and its only reads are queries and events.
- **Feature unification.**
  - `verify.sh` runs clippy and tests across the whole workspace, so in those runs Bevy compiles with the 3D features for every crate, `omnis-app` included.
  - `cargo build -p omnis-app` alone is unaffected (resolver 3).
  - The cost is measured: gate time before and after, and whether `omnis-app`'s release clippy configuration changes.
- **Dependency policy applies unchanged:** pinned, N−1, older than 30 days, checksums in the lock. Bevy's 3D features are part of Bevy at the pinned `=0.19.1`, so no new third-party version choices are expected; any that appear are listed.
- **Merging from `main`.** `Cargo.lock` is the likely conflict point; it is regenerated, not hand-merged.

## 9. Risks

| # | Risk | Mitigation |
|---|---|---|
| XR1 | The client and simulation positions diverge | The simulation is authoritative and the pose snaps to it. §4.3 gives a rule per case, and a headless test covers each one. |
| XR2 | The 3D view shows tiles the automap has not recorded, because the 3D view is open but the automap is narrow (X6) | Accepted for the proof. Player knowledge stays a system of its own (LESSONS 2026-09-11), and the automap is revisited in §10. |
| XR3 | 3D features slow the shared gate or add duplicate crates | Measured in §7. If the cost is bad, the owner can revisit X2 (move to a separate workspace). |
| XR4 | Gizmo lines look thin or aliased on a high-DPI ultrawide | Gizmo line width and MSAA settings, measured on the owner's display. |
| XR5 | Free movement makes random encounters feel more frequent, because cells are crossed faster than they were stepped | Measured: cells entered per minute of play, and encounters per minute. Tuning the encounter rate stays a pack value. |

## 10. Open questions

1. **Camera height and field of view:** the eye height, and a field of view that suits 32:9 as well as 16:9. The wall height is settled at 1.0 × 1.0 to start, with room to vary per terrain later (owner, 2026-10-02). Bloom glow is in Phase A. Phase A runs on `packs/base` and `packs/test` together.
2. **Captures:** should the viewer support the dev socket and MCP `screenshot`, or is a command-line screenshot enough for the proof?
3. **The combat screen's code:** in Phase B, should it reuse `omnis-app`'s Bevy-free combat menu model (`combat_menu.rs`), which depends on `omnis-app` as a library, or be written fresh in `omnis-vector`?
4. **The automap in 3D:**
   - It could record what the 3D camera actually sees, from a camera frustum test, rather than the cardinal cone. That needs a simulation command, so it is a change to the main build and out of scope here.
   - It could also be shown as a 3D overlay.
5. **After the proof:** if it succeeds, what is next: meshes and textures on the same binding, folding the 3D view into `omnis-app`, or amending PRD D2 and §5?
