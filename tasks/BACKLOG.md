# Backlog: small findings and unused assets

Started 2026-10-11 (owner: "add all the small findings and extra assets to a BACKLOG.md file to review later for
addressing and removing"). Each entry says what the player or the reader sees, what is actually so, and where it was
found. An entry leaves this file when it is fixed (with a test, for a bug), moved into `tasks/TODO.md` as planned
work, or dropped by the owner.

Features and planned work stay in `tasks/TODO.md`: Look and Map, removing or editing a party member,
`DevCommand::Pass`, the `data.*` ops, `scripts/mcp-probe.py`, the rhai Socket re-audit.

## Interface
- **The HUD clock line has lost the date** (found at P9).
  - The player sees "Day 40 14:20", counted from the party's age.
  - The retired client showed the date and the age: "Year 1 day 40 14:20 night  age 2y 40d", with narrower forms for
    a narrow window. In a town the date snapped back while the age never went back (M8 Part 1, accepted).
  - `Here.date` already carries the date, so the fix needs no engine change. The old `panels::clock_text` and its
    tests: `git show 736558c:crates/omnis-app/src/panels.rs`.
- **At 200% the party column's entries wrap** (P8).
  - The column keeps a quarter of the window, so long entries break over two lines at the largest scale.
- **The picture window draws every monster as the rat** (P7).
  - Whatever the fight is against, the picture shows the rat.
- **Two of P6's calls are unconfirmed by the owner:**
  - down members show in red;
  - dead members stay dim, with "dead" in red.

## Tools and developer features
- **An offscreen `--screenshot` into a missing directory writes nothing and exits 0** (found at P9; older than P9).
  - `cargo run -p omnis-vector -- --screenshot .omnis/new/x.png` logs "Cannot save screenshot, IO error: No such
    file or directory" and the run succeeds.
  - Fix: create the parent directory, or exit non-zero.
- **A flag's Set on the debug panel cannot be tried** (P8).
  - The shipped packs declare no flags, so the World page shows "The packs declare no flags". The Set path is
    tested only through the model.
- **Pack text is not checked for non-ASCII.**
  - Screen words are kept ASCII by rule (presentation-ARCHITECTURE §7.4). Text that comes from a pack is not
    checked, so a pack could show glyphs the fonts lack.

## Docs and wording
- **The editor's toolkit is contradicted** (found at P9). **Resolved 2026-10-14: the owner chose `bevy_ui`; the docs agree.**
  - ARCHITECTURE A11 chose `bevy_ui` with Feathers for the editor; presentation-PRD §8 says the editor keeps
    `bevy_egui` until revisited. Feathers left the workspace with `omnis-app`.
  - ARCHITECTURE §8.2 marks it open. Decide it when the editor is planned (`tasks/plans/editor-v1.md`).
- **`assets/README.md` still says Inter and Alegreya Sans are compiled into the Feathers client only** (P8).
  - Since P8 every build of the game compiles them in (`omnis-vector/src/shell/fonts.rs`). This is the owner's file.
    `ATTRIBUTION.md` was corrected at P9.
- **`Op::Screenshot`'s doc still says "the canvas"** (`omnis-sim/src/ops.rs:123-129`, found at P9).
  - The protocol keeps both targets, `canvas` and `window`, and the game captures its window for both. A simulation
    crate, so it was left alone at P9.
- **Test names still say `other_client`** (found at P9).
  - For example `tests/inventory.rs::the_kit_s_events_read_as_the_other_client_s_lines`. The comments say "the
    retired client" since P9; the function names were left as they were.

Found 2026-10-20 in a read-only review of `ARCHITECTURE.md` (every item checked against the files it names). Fixed
the same day: the `ARCHITECTURE.md` items (§2, §3, §4.9, §9.3, §14, §15, A9), the lint's fail-open Bevy check, the
retired Feathers and canvas state in PRD D26, §11.1 and §14, the performance floor's wording in §11.1,
presentation-PRD §9's "3D features", and the headless-versus-offscreen sentence in presentation-ARCHITECTURE §10.3.
Left, waiting on a planned change:
- **The Bevy 0.20 plan changes tonemapping from `None` to `Linear`** (`tasks/plans/bevy-0.20-migration.md`);
  presentation-ARCHITECTURE §3 and §8.6 will need the change when it lands.

## Dependencies
- **`smartstring` RUSTSEC-2026-0249** (through rhai): to add to ARCHITECTURE §13's dependency notes.
- **`ttf-parser` 0.25.1 RUSTSEC-2026-0192**: reached only on Linux, through winit's Wayland window decorations
  (`winit` → `sctk-adwaita` → `ab_glyph` → `owned_ttf_parser`; `cargo tree --target all -i ttf-parser`). It is not in
  the macOS build. To review against the Bevy 0.20 move (not before 2026-11-07).
- **`webgl2` is enabled transitively** (found 2026-10-20). Bevy's `2d` and `ui` groups pull `default_platform`,
  which carries `webgl2` (`bevy_internal/webgl` → `bevy_render/webgl` → `wgpu/webgl`). Presumed inert on a native
  target; not verified. A narrower feature list would drop it; check with the Bevy 0.20 move.

## Rendering direction: GPU-first (recommendation, 2026-10-20)
The owner's concern (2026-10-20): an early miscommunication left the rendering built with old techniques. Found in
the read-only review: the look is the owner's (presentation-PRD X3, X13), the grid with quantized steps is the
rules, and neither is a rendering technique. The technique is Bevy's debug-drawing path: `shell/render.rs::draw`
walks every segment each frame, computes a fog fade on the CPU and submits a gizmo line, which Bevy re-uploads
every frame. The only retained geometry is the fills mesh (M8.5 step 3). There are no shaders, materials, compute
passes or render-graph nodes of the project's own anywhere in the repo. The whole GPU-facing surface is one file,
`shell/render.rs` (281 lines); the 1,866 lines of Bevy-free core (`geometry`, `fill`, `ground`, `scene`, `shapes`)
decide what exists and stay whatever is done below.

The measurement already made says where the time goes (presentation-ARCHITECTURE §8.6, offscreen, release, a
300-frame walk): 1.9 to 2.2 ms mean at 1920×1080 and the same at 5120×1440. No change over 3.5× the pixels means
the GPU is idle and the cost is CPU submission. At 2 ms a frame it is not a performance problem today; the case
for GPU-first is capability, not speed: pixel-constant anti-aliased lines, relief (X10), depth levels (X17),
density and glow (X13) are each planned, and on the current path each adds to the per-frame CPU loop.

Three sizes, each a candidate for `tasks/TODO.md` when the owner chooses:
1. **Retained geometry with an own shader** (the design already deferred in presentation-ARCHITECTURE §8.2 and
   VA4). Lines become `LineList` meshes built on change and drawn through a custom `Material` whose vertex and
   fragment shaders do fog, fade, pattern and weight; the per-frame CPU loop goes. New surface: one WGSL file
   (about 100 lines), a material module, a rewrite of `render.rs`. No new crate: `bevy_pbr`, already enabled, has
   the material trait. The core is untouched and the headless tests stand. This is also the measurement X20 asked
   for before the floor is set.
2. **The look computed on the GPU.** Lines expanded to screen-space quads in the vertex shader for pixel-constant
   width and anti-aliasing; relief as displacement over a per-map heightfield texture instead of the CPU noise
   planned for M8.5 step 5; depth levels and fog as material parameters; objects, piles and beings instanced. The
   CPU sends a scene once and a few uniforms a frame. Three to four shaders, an instancing path, a texture upload a
   map. This is where "GPU first" becomes true.
3. **A GPU-resident map.** Cells, heights and edges uploaded once as storage buffers; a compute pass or procedural
   vertex shader generates walls, floors and relief; a door opening is a one-cell update. The core shrinks to the
   collision mirror and the uploader. Needs render-graph and compute code, the most Bevy-version-sensitive kind;
   the agreement tests lose their CPU geometry reference. A horizon, not an exclusion.

What each size costs beyond code:
- **Docs.** Size 1 fits presentation-PRD X3 ("can grow into meshes"), X20 and VA4 as written. Sizes 2 and 3 are a
  presentation-PRD amendment (the owner's departure adds a direction) and change VA1, VA4 and §8.2.
- **Testing.** The gate has no GPU. Size 1 keeps the pattern (core tested headless, captures as acceptance on the
  owner's machine). Sizes 2 and 3 move the look into shaders that no headless test sees; whether GitHub's macOS
  runners expose a Metal device for offscreen capture is unverified and decides whether CI can ever check the
  picture.
- **Ordering.** Shader APIs churn between Bevy versions and the 0.20 move is planned. Shaders written on 0.19.1
  are written twice. Do 0.20 first.
- **Dependencies.** None at any size. Own WGSL in the repo matches the self-supporting rule; no line-rendering
  crate is needed.

**Recommendation:** size 1 straight after the Bevy 0.20 migration, with size 2 designed in the same amendment so
the shader is written once with width, fog and relief in mind; size 3 stays a horizon. When the owner chooses, the
chosen size moves to `tasks/TODO.md` with a plan under `tasks/plans/` and the amendment text for
`presentation-PRD.md`.

## Assets no code uses (review for removal; the owner's files)
Found at P9, when `omnis-app` left. Nothing here is deleted without the owner.
- `assets/fonts/inter/Inter-Bold.ttf` and `assets/fonts/alegreya-sans/AlegreyaSans-Bold.ttf`.
  - Only the retired client compiled them in (`feathers_fonts.rs`). The game uses the Regular faces.
- The baked tileset sprites in `packs/test/assets/tilesets/`.
  - Only the retired client drew them (`plan.rs`). The tileset data under `packs/test/data/tiles` is still loaded
    and validated by `omnis-data` and `omnis-cli validate`, and maps name a tileset, so removing the sprites means
    deciding what the tileset data is for.
- `assets/openrtp-tiles/*.png`.
  - No client loads them. They are the source sheets for `omnis-cli tileset bake` (`packs/test/bake/*.ron`), so they
    stand or fall with the baked tilesets above.
- Not code-referenced yet, but the owner's work in progress (listed so the review is complete, not proposed for
  removal): `assets/basic-glb-components/`, `assets/basic-svg-components/`, `assets/private/time-fantasy-icons/`.
