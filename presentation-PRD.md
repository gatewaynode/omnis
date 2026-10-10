# Omnis — 3D Vector Presentation PRD

| | |
|---|---|
| Status | v0.3, accepted by the owner 2026-10-10 ("The PRD looks sufficient") and merged; `PRD.md` v0.8 points here (D27) (v0.3, 2026-10-10: the experiment is adopted as the presentation direction; visual language, world geometry, interface, client and the amendments to the main PRD, X8–X22. v0.2, 2026-10-02: grid triggers during free movement, open-world rendering, a separate 2D combat screen, the crate named `omnis-vector`) |
| Branch | Merged from `gui-3d-experiment` on 2026-10-10 (branch `vector-adoption`; `tasks/plans/vector-adoption.md`) |
| Parent documents | `PRD.md` (v0.8), `ARCHITECTURE.md` (v0.8, A18); companion `presentation-ARCHITECTURE.md` (v0.3) |
| Owner | gatewaynode |

## 1. Purpose

This document is the presentation PRD for Omnis. It began as an experiment (Phases A and B,
§11) to prove that the terrain data model could drive a real-time 3D view with free movement
while the simulation's grid stayed authoritative underneath. The proof succeeded. The owner
adopted the result on 2026-10-10 (X8): the glowing-line 3D presentation and free movement replace
the 2D canvas UI as the direction for the game.

What it keeps from the experiment:
- **The grid stays the rules.** Every cell the party enters is processed by `omnis-sim` exactly as
  a step in the 2D game: movement rules, time, the automap, portals, encounters, and later traps
  and perception. The client implements none of those rules (§4).
- **Free movement and free look** in a first-person 3D view, which proved more engaging than
  discrete steps.
- **Glowing vector lines on dark**, which the experiment chose as the cheapest way to see the
  geometry. It has become the look (§5).

What it adds:
- **A world that is not flat or square.** Terrain has height. Structures have shapes beyond the
  cell edges. The style leaves room for a great deal of line-art detail (§6).
- **One visual language** for the whole game. Colour carries meaning by role, each biosphere has
  its own palette, and a second channel that does not rely on colour carries the same meanings (§5).
- **`omnis-vector` becomes the game client** (§8).

## 2. Relationship to the main PRD

`PRD.md` stays the vision for the rules, the world, the simulation and the content. This document
governs how the game is presented: the view, movement, the visual language, the world's geometry
as drawn, and the interface. Where the two disagree on presentation, this document wins once the
owner has merged it. At the merge, the main PRD is amended to point here (X9):

| Main document | Says | Amended to |
|---|---|---|
| PRD §5 non-goals | "3D rendering" | Removed: the game renders in 3D (this document) |
| PRD D2 | First-person grid crawler, 2D rendering | First-person 3D with free movement over an authoritative grid (X1, X8) |
| PRD §1, §7.2 | Discrete steps and 90° turns | Free movement and free look, quantized to steps and turns for the simulation (§4) |
| PRD D16, §7.2 | Detail depth drawn with full sprites; visibility depth as a horizon band | Three depth levels: immediate (high detail), intermediate (moderate to low detail) and distant (huge objects in low detail, slowest parallax). Visibility depth still sets the fog distance (X17) |
| PRD D20, §11.1 | Integer-scaled 720-row canvas, bitmap font | Resolution-independent 3D view and `bevy_ui` in the neon style (X13); D20's display targets (5120×1440 first, 16:9 supported) stand |
| PRD D26, §14 | "Modern" undecided; possibly reopening 3D | Decided: the 3D vector presentation of this document |
| PRD R10, R13 | Sprite-tileset art pipeline and its risks | Replaced by procedural and authored line art (X12) and §10's risks |
| ARCH §8.1 | "The `3d` group (pbr, gltf) is never enabled" | Bevy 3D features enabled for the client |
| ARCH §8.2, A11 | Canvas sprites for the HUD and menus | `bevy_ui` in the neon style (X13); `bevy_egui` stays the editor's choice until revisited |

Height (X10) and cliffs (X11) need pack-data and rules support that belongs to the mechanics work
(`omnis-data`, `omnis-sim`). This document states what the presentation needs. The mechanics
branch decides how the rules use height beyond cliffs.

## 3. Decisions log

| # | Topic | Decision | Alternatives considered | Rationale |
|---|---|---|---|---|
| X1 | Movement model | Free movement and mouse look in 3D. The grid cell is computed continuously from the pose. Each cell boundary crossed emits `Command::Step`, and the simulation processes everything that cell entry triggers. A yaw that snaps to a new cardinal direction emits `Command::Turn` | Discrete steps with tweening (a smaller proof); discrete now and free later | Owner decision (2026-10-02). It tests the stronger claim: the grid rules survive a presentation that does not look like a grid. Proven in Phase A. |
| X2 | File space | A crate in the main workspace, `crates/omnis-vector` | A separate Cargo workspace under `alt/`; a separate repository | Owner decision (2026-10-02). It shares the lock, `verify.sh` and the dependency policy. |
| X3 | Rendering | Bevy `Camera3d` with perspective. Geometry drawn as lines (gizmos in the proof) | Lines hand-projected onto the existing `2d` features | Owner decision (2026-10-02). It is the real 3D path and can grow into meshes. |
| X4 | Scope of the binary | During the proof, a standalone viewer driving `omnis-sim` directly. Superseded by X14 | Reuse `omnis-app`'s canvas UI, menus and combat | Owner decision (2026-10-02). |
| X5 | Crate name | `omnis-vector` | `omnis-alt`, `omnis-3d` | Owner decision (2026-10-02). |
| X6 | What is drawn | The 3D view behaves like an open-world 3D game: geometry in every direction the camera looks, out to a fog distance. The simulation's forward cone does not limit the 3D view. The automap is recorded from the simulation's cardinal cone on each command | Draw only the tiles in the cone; only automap-known tiles | Owner decision (2026-10-02). |
| X7 | Combat | A fight replaces the 3D view with a separate 2D combat screen in the same vector style. It uses static figures for now, and animated line figures and art later. The fight is `omnis-sim`'s combat state machine, unchanged | Combat in the 3D scene; a frozen notice | Owner decision (2026-10-02). Built in Phase B and kept by the owner on 2026-10-10 after playing it. |
| X8 | Adoption | The 3D vector presentation with free movement replaces the 2D canvas UI as the game's presentation direction | Keep the experiment separate; fold only the 3D view into the canvas UI | Owner decision (2026-10-10), after playing Phases A and B: the line style is "leading somewhere interesting" and free movement is "definitely more engaging". |
| X9 | This document's role | The presentation PRD. At the merge, PRD.md's presentation sections are amended to point here (§2) | Fold this content into PRD.md and retire it; keep the experiment framing | Owner decision (2026-10-10). |
| X10 | Terrain height | Maps carry a coarse authored height in pack data, available to the rules. The client adds fine relief from a seeded noise per terrain kind, which never changes the rules | Client-only relief (height could never matter to the rules); authored height only, with no relief | Owner decision (2026-10-10). The ground should never read as flat, and the rules can use height later (line of sight, climbing, falling). |
| X11 | What height does in the rules, first | Cliffs block. An edge between two cells whose heights differ by more than a step is impassable, like a wall. The cliff is derived when the pack loads, so the simulation and the client agree and stay integer. Everything else about height is visual at first: the camera follows the ground | Visual only; cliffs plus a slope cost in step minutes | Owner decision (2026-10-10). Slopes, line of sight and falling are left to the mechanics branch. |
| X12 | Structures | Walls and edges keep their cell-edge logic and collision, but are drawn as shaped line forms per terrain kind: jagged rock, broken ruin, rounded towers, organic trees. Packs can also place authored line-art set pieces (statues, arches, machines) on cells | Procedural shaping only; authored line models only | Owner decision (2026-10-10). Nothing should read as a grid of boxes. |
| X13 | Look ceiling | Glowing lines are the whole look. Dark, solid surfaces occlude the lines behind them (hidden-line removal), so detail can get dense without becoming see-through noise. No textures | Pure see-through wireframe; lines first with shaded materials or textures later | Owner decision (2026-10-10). |
| X14 | Client | `omnis-vector` grows into the game client: menus, party creation, sheets, town, saves. `omnis-app` stays until the new client reaches feature parity, then is retired. The canvas UI is replaced, not ported | Fold the 3D view into `omnis-app`; keep both clients | Owner decision (2026-10-10). |
| X15 | Interface | Every screen beyond the 3D view and the fight screen (menus, character sheets, inventory, the tactics panel, town) is `bevy_ui` in the neon style of the HUD and fight screen, under one style guide (§7) | Feathers with a neon theme; panels drawn as vector line art | Owner decision (2026-10-10). |
| X16 | References | Tron (1982 and *Legacy*): glowing edges on dark solids, strong colour coding, architectural scale. Technical drawing and blueprint: drafting-style line detail, hatching, contour lines, an annotated, engraved feel | Battlezone and Tempest; Rez and Thumper | Owner decision (2026-10-10). The arcade vector games were the experiment's starting point, not the goal. |
| X17 | Depth levels (replaces PRD D16's two depths) | Three levels. **1, immediate:** high detail. **2, intermediate:** moderate to low detail. **3, distant:** huge objects in low detail, such as a mountain range or clouds, in the slowest parallax. Visibility depth stays a property of the world and sets the fog distance, as built (§6.3) | Two depths (detail and visibility, as in D16); visibility only, with full detail to the fog | Owner decision (2026-10-10): "3 depth levels is usually a better approach." The levels bound the line count while the performance floor is measured (X20). |
| X18 | Colour | Colour is semantic and relational. Meanings are fixed roles, and each biosphere maps the roles to its own palette, so a biosphere can change the whole scheme | Per-terrain colours (as built in the proof); mostly monochrome | Owner decision (2026-10-10). |
| X19 | Meaning without colour | Every role also has a channel that does not rely on colour: line pattern, line weight, glyph markers and motion, with room for more. It serves colour-blind players and keeps meanings readable across biospheres | Colour alone | Owner decision (2026-10-10): "We'll likely need all four visual devices proposed and need to leave room for more." |
| X20 | Performance floor | Measure first. The main PRD's floor (60 fps at 1080p on 2020 integrated graphics) is neither kept nor dropped until the occluding fills, relief and detail are built and measured | Keep the main PRD's floor; raise it to a discrete GPU | Owner decision (2026-10-10). |
| X21 | Grid | A faint floor grid stays visible and follows the relief like contour lines, fitting both references. A setting can hide it | Hidden by default; a strong glowing grid as a motif | Owner decision (2026-10-10). It is honest about the rules underneath. |
| X22 | Automap | The minimap becomes a blueprint-style map in the neon style, showing height as contour lines. What it records stays an open question for the mechanics side (§12) | Record the 3D camera's view now; keep it as built | Owner decision (2026-10-10). |

## 4. The grid under free movement

The simulation never sees a continuous coordinate. It keeps receiving the same commands as the 2D
game, and stays `no_std`, integer-only and deterministic. This section is built and tested
(Phase A); height (X10, X11) adds to it without changing the rule that the grid decides.

### 4.1 Positions

- **Two positions.**
  - The simulation's `Position { map, x, y, facing }` is authoritative.
  - The client keeps a continuous *pose*: a world-space x and z and a yaw, as floats. With height, the camera's y follows the drawn ground; it is never sent to the simulation. Floats exist only in `omnis-vector`, which is not a simulation crate.
- **Cell mapping.** Cell `(x, y)` occupies the unit square `[x, x+1) × [y, y+1)` on the ground plane. North is −y, as in `omnis-core::geom::Facing`.
- **Facing from yaw.** The simulation's facing is the cardinal direction nearest the yaw. When the nearest cardinal changes, the client emits one `Turn` per 90° of change before any `Step`, so that relative step directions are computed against the right facing.

### 4.2 Cell entry is a step

When the pose crosses into a neighbouring cell, the client converts the absolute direction of the
crossing into `Step(Forward | Back | Left | Right)` relative to the simulation's current facing,
and applies it.

What a step does (`crates/omnis-sim/src/apply.rs`, `step`, then `move`, then `encounter::trigger`):

1. It refuses on a wall, a closed door, impassable terrain or the map edge, with an `Event::Blocked` reason. A cliff edge (X11) refuses the same way.
2. It moves the party and emits `Event::Moved`.
3. It advances the party clock by the terrain's `step_minutes`.
4. It records the cell as visited on the automap.
5. It follows a portal on the target cell (a teleport).
6. It starts an encounter: a placed group on the cell that is not cleared, or else a roll on the map's random table.

Every successful command then ends in `look`, which records the cardinal cone to the automap and
emits `Event::Visible`. `Turn` only rotates the position, so free look costs no game time.

Traps and passive perception checks arrive from the mechanics work on this same cell-entry path,
and the client receives them without change.

### 4.3 Feasibility issues and the rule for each

| Case | Rule |
|---|---|
| Hovering on a boundary emits steps back and forth | **Hysteresis.** A cell counts as entered only when the pose is a margin past the boundary. |
| A diagonal crossing passes through a corner | The crossings are ordered along the movement segment and emitted as two steps. If the first is refused, collision slides the pose and the second is re-evaluated. |
| The simulation refuses a step the client thought was legal | The simulation wins and the pose is pushed back. The client's collision mirror reads walls, doors and (with X11) cliffs from the same loaded data, and an agreement test holds the two together. |
| A portal teleports the party | The pose snaps to the destination cell and facing. |
| An encounter starts mid-motion | Input freezes, the pose settles in the entered cell, and the fight screen replaces the 3D view (X7). |
| Run or flight retreats the party | The pose snaps to the retreat cell when exploration resumes. |
| Time | Game time advances per cell crossed, by `step_minutes`, never per frame. |
| Doors | Opened by `Command::Interact` on the faced edge, from the action menu or its key. |
| Shaped structures (X12) | The drawn shape is decoration. Collision follows the cell edges, so a jagged rock face is walked against as a straight edge. Shapes are kept close enough to the edge that this reads as natural; how close is measured (§10, XR7). |

### 4.4 Proof of binding

The commands the client emits are logged in the replay format. Replaying that log through
`omnis-cli replay` reproduces the session's final world fingerprint. That proves the 3D session
was a legal grid session, and that every roll came from the simulation's seeded streams.

## 5. Visual language

### 5.1 The look

- **Lines are the look** (X13). Every object is drawn as glowing lines on a dark world, with bloom.
- **Dark solids occlude.** Surfaces are drawn as dark solid fills that hide what is behind them, so
  a wall hides the room beyond it and a statue hides the wall behind it. Lines are drawn on the
  edges and the detail of those surfaces. The proof's see-through wireframe is a stage, not the goal.
- **Detail is lines.** Variety comes from line work, not texture: hatching on rock, contour lines
  on the ground, engraved patterns on structures, annotation-like marks. This is the blueprint half
  of X16.
- **Scale and colour coding** come from the Tron half of X16: architecture that reads as large,
  edges that glow, and colour that means something (§5.2).

### 5.2 Colour roles and biospheres

- **Roles are fixed** (X18). Each role is a meaning that holds everywhere in the game. A first set,
  to be refined: structure, ground, interactive (doors, levers, items), the party, hostile, magic,
  danger (traps, hazards), and the interface.
- **A biosphere is a palette and style set** defined in pack data. It maps every role to a colour
  and can change the whole scheme: a cold cavern and a burning desert may share no hue. How a
  biosphere is assigned (per region, per map, or per terrain) is open (§12).
- **Contrast is a requirement.** Each biosphere's palette is checked so that roles stay distinct
  from one another and from the dark ground.

### 5.3 Meaning without colour

Every role also carries at least one channel that does not rely on colour (X19). The channels are
the same in every biosphere, so a meaning learned in one region reads the same in the next.

| Channel | What it carries | Example (illustrative, to be designed) |
|---|---|---|
| Line pattern | The kind of thing | Solid for structure; dashed outline for interactive; chained or sawtooth for hostile; dotted for magic |
| Line weight | How much it matters now | Heavy for hostile and interactive; hairline for scenery |
| Glyph markers | A fixed vocabulary of line-art symbols at points of interest | Door, trap, portal, item, hostile, exit |
| Motion | A signature per role | Interactive pulses slowly; hostile flickers; magic drifts. A reduced-motion setting turns it off |

The set is open: further channels can be added (X19). A colour-blind check (simulated deficiency
views) is part of reviewing any new screen or biosphere.

### 5.4 The fight screen and the interface share the language

The 2D fight screen (X7) and every `bevy_ui` screen (X15) use the same roles, palettes and
non-colour channels as the 3D view. A hostile stack on the fight screen reads as hostile in the
same way that a monster marker on the map does.

## 6. World geometry

### 6.1 Terrain

- **Height** (X10). Maps carry a coarse height in pack data. Whether it is stored per cell or per
  corner is open (§12). The client draws the ground at that height, and the camera follows it.
- **Relief.** On top of the authored height, the client adds fine relief from a seeded noise
  chosen by terrain kind: rolling grass, broken rock, rippled sand, uneven cave floor. Relief is
  presentation only. It never changes a rule, and the same seed always draws the same ground.
- **Cliffs** (X11). An edge whose two cells differ in height by more than a step is a cliff. It is
  drawn as a rock face and blocks like a wall. The step threshold is pack data.
- **The grid** (X21) is drawn faintly on the ground and follows the relief, like contour lines.

### 6.2 Structures

- **Shaped from the edges** (X12). Walls, doors and blocking terrain keep their cell-edge positions
  and collision, but each terrain kind has a shape generator: jagged rock for caves, broken masonry
  for ruins, rounded towers, organic trunks and canopies for forests. Generators are seeded per
  cell, so the world is stable between visits and sessions.
- **Authored set pieces.** Packs can place line-art models on cells or edges: statues, arches,
  machines, landmarks. The format is open (§12).
- **Doors stay readable.** Whatever the shape, a door carries the interactive role's channels
  (§5.3), and an open door looks different from a closed one.

### 6.3 Depth and detail

Three depth levels (X17):

| Level | Range | Detail | Holds |
|---|---|---|---|
| 1, immediate | Near the party | High: full line detail, hatching, engraving, glyphs | The cells around the party, structures, set pieces, doors |
| 2, intermediate | Out to the fog | Moderate to low: silhouettes and main edges, little surface detail | The rest of the map within visibility depth |
| 3, distant | Beyond the map's drawn geometry | Low: huge forms only | Mountain ranges, clouds, distant towers and other landmarks, drawn in the slowest parallax |

- **Visibility depth** (PRD D16) stays a property of the world and sets the fog distance, as built
  (X6). A dark dungeon reads close, an open plain reads far. Levels 1 and 2 are cut by it.
- **Level 3 is a backdrop.** It is drawn beyond the fog where the environment allows: outdoors,
  under a sky. Underground there is none. Whether weather and darkness dim or hide it is open (§12).
  It takes over the role of D16's horizon band ("terrain colour and landmark silhouettes").
- **The boundaries between levels 1 and 2** are tuned by measurement (X20) and may become a quality
  setting. A shape changes level without a visible pop.

## 7. Interface

- **One style guide** for every screen (X15): dark opaque panels, the interface role's colours,
  glowing borders and accents, a modern font, and the non-colour channels of §5.3.
- **Buttons before keys.** Every player action has a button on screen before it has a key
  (LESSONS 2026-09-19). Keys are shortcuts. Function keys are not bindings on macOS.
- **Opaque over moving scenes.** Panels drawn over the 3D view are opaque unless translucency is
  asked for (LESSONS 2026-10-02). A destructive action (quit, restart, delete) stands apart from
  frequent ones.
- **A mode that takes the pointer shows its way out** (LESSONS 2026-10-03).
- **Scale with the window.** Text and figures scale with the window height, so 5120×1440 and 16:9
  panels read alike. This is the fight screen's open scale item from B6, deferred to the art
  update.
- **Screens to build** (X14), in an order set by the plan: party creation, character sheets,
  inventory and items, spells, the tactics panel, town services, save and load, settings
  (including reduced motion, grid visibility and quality), and the developer tools.

## 8. The client

- **`omnis-vector` becomes the game** (X14). It reaches feature parity with `omnis-app` screen by
  screen. `omnis-app` is retired once nothing depends on it.
- **The architecture stays as built:** a Bevy-free core, tested headless on the real packs, and a
  thin Bevy shell (presentation-ARCHITECTURE §1). The simulation is authoritative and floats stop at its
  boundary.
- **Developer tools** (the dev socket, MCP screenshot, the debug menu) move to the new client as
  part of parity, so agents and the owner keep their capture and inspection paths.
- **The editor** keeps `bevy_egui` (ARCH A11) until its toolkit is revisited.

## 9. Constraints

- **The simulation crates' lints stand** (CLAUDE.md): no floats, no hash maps, no Bevy in the
  simulation crates. Height and cliffs in `omnis-data`/`omnis-sim` are integers.
- **`omnis-vector` is not a simulation crate.** It may use floats and Bevy. It must never pass a
  float into the simulation: its only channels in are `Command`s, and its only reads are queries
  and events.
- **Feature unification.** `verify.sh` builds the workspace with Bevy's 3D features. Its run time
  is tracked.
- **Dependency policy applies:** pinned, older than 30 days, checksums in the lock. Bevy itself is
  exempt from N−1 (PRD D9). The 0.20 migration is planned in `tasks/plans/bevy-0.20-migration.md`.
- **Merging with the mechanics work.** `Cargo.lock` is regenerated, not hand-merged.

## 10. Risks

| # | Risk | Mitigation |
|---|---|---|
| XR1 | The client and simulation positions diverge | The simulation is authoritative and the pose snaps to it. §4.3 gives a rule per case, and a headless test covers each one. |
| XR2 | The 3D view shows tiles the automap has not recorded | Accepted. Player knowledge stays a system of its own. The automap's sensing is an open question (§12). |
| XR3 | 3D features slow the shared gate or add duplicate crates | Measured in Phase A. Re-measured as the client grows. |
| XR4 | Lines look thin or aliased on a high-DPI ultrawide | Line width, MSAA and bloom settings, measured on the owner's display. |
| XR5 | Free movement makes random encounters feel more frequent | Measured: cells entered and encounters per minute of play. The encounter rate stays a pack value. |
| XR6 | Dense line detail, occluding fills and bloom are too slow | Measure first (X20). The depth levels (X17) bound the line count, and a quality setting can lower density, distance and bloom. |
| XR7 | Shaped structures and relief disagree with grid collision, so the party walks into a visible rock or stops short of a drawn wall | Shapes stay within a margin of the cell edge, and relief never changes the walkable surface's rules. Measured in play. |
| XR8 | Meaning depends on colour, or a biosphere's palette collapses two roles | Every role has a non-colour channel (X19). Palettes are checked for contrast and simulated colour deficiency. |
| XR9 | Rebuilding every screen in `omnis-vector` takes longer than expected, and the two clients drift | `omnis-app` stays until parity (X14). Screens share Bevy-free models where possible, as the fight screen did. |
| XR10 | Height in pack data conflicts with the mechanics branch's own plans for the map format | The need is stated here (X10, X11). The data shape is designed with the mechanics work, not in this branch. |

## 11. Phases

### Built

- **Phase A, exploration (2026-10-02/03):** the test pack's maps drawn as lines, free movement and
  mouse look with buttons for every action, the grid binding of §4, a HUD, a 2D minimap, a
  replayable command log, and window sizing for 5120×1440 and 16:9. Report in `tasks/alt-TODO.md`.
- **Phase B, the fight screen (2026-10-03):** the 2D combat screen with the roll log, the action
  model with targets, the arena layout, the picture window, and the switch between the 3D view and
  the fight. Played by the owner on the ultrawide (B6).

### Next (proposed; each planned in plan mode when the owner asks)

1. **Merge** with the mechanics work (B7), then amend the main PRD per §2.
2. **Occluding fills** (X13): move from gizmos to line meshes over dark solid surfaces, and measure (X20).
3. **The style system** (X18, X19): colour roles, biosphere palettes in pack data, and the first
   non-colour channels, applied to the 3D view, the fight screen and the HUD.
4. **Terrain height and cliffs** (X10, X11): the data shape with the mechanics work, then relief,
   the contour grid and the camera following the ground.
5. **Shaped structures** (X12): shape generators per terrain kind, then the first authored set piece.
6. **Interface parity** (X14, X15): the screens of §7, then retiring `omnis-app`.

The art update (animated figures, the fight screen's scale) fits alongside 3 and 6.

## 12. Open questions

1. **Height's data shape:** per cell or per corner; its range and units; the cliff threshold; how
   the editor authors it. Designed with the mechanics work (X10, XR10).
2. **Biosphere assignment:** per region, per map or per terrain; whether a biosphere also selects
   relief noise and shape generators, or only the palette.
3. **The role set:** the final list of colour roles and their non-colour channels (§5.2, §5.3), and
   the glyph vocabulary.
4. **Authored set pieces:** the format. Candidates are a RON line-model format in packs (vertices,
   edges and occluding faces, validated by `omnis-data`), SVG import, glTF import, or an in-editor
   tool. Decided when the first set piece is built.
5. **The automap's sensing** (X22): whether it should record what the 3D camera sees, using a
   frustum test, rather than the cardinal cone. That needs a simulation command, so it belongs to
   the mechanics work.
6. **Performance floor** (X20): set after measuring occluding fills, relief and detail.
7. **The distant level** (X17): where its content comes from (pack data per region or biosphere,
   generated from the region's terrain, or both); whether weather and darkness dim or hide it; and
   how a distant landmark relates to the real place when the party travels toward it.
8. **Camera:** eye height, and a field of view suited to 32:9 and 16:9, revisited once the ground
   has height.
