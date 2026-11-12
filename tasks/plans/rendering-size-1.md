# The 3D view onto Bevy meshes and Material (rendering size 1, size 2 designed in)

Status: **approved 2026-11-12; steps 0 to 3 done (see `tasks/TODO.md` for two departures in detail), step 4 (the swap) next** (on `W-2_look_and_engine`).


## Context

The 3D view's lines are drawn through Bevy's *debug* facility: `shell/render.rs::draw` walks every segment
every frame, computes the fog fade on the CPU and submits a throwaway gizmo line. The editor's 3D preview
(`shell/preview.rs`) copies that loop. Nothing of the project's own is a mesh, material or shader except the
black occluding fills.

The owner chose (2026-11-12) **size 1 with size 2 designed in**, as leaning on Bevy *more*: the lines become
retained Bevy meshes drawn through Bevy's `Material` trait with one shader of ours. Asked the same day, the
owner chose to **keep the line width now**: each segment is a thin quad the shader widens to a constant pixel
width (the technique of Bevy's own gizmo shader), because a plain `LineList` is always 1 pixel wide and
today's look is 2 pixels (1 / 1.5 / 2 by quality).

Outcome: no per-frame CPU loop over segments; the look unchanged (same colours, fade, width, depth bias,
bloom), shown by before/after captures; the mesh format and the material already carry what size 2 needs.

## What was found (surveys of the client and of the pinned Bevy 0.20.0 source)

- `render.rs` (324 lines): `Lines { map, scene }`, `rebuild` (on map/level change or `Session.reshape`),
  `follow` (camera at the pose, eased eye height), `draw` (gizmos). Fade: 1.0 to half the fog end
  (`sight + 2`), linear to 0 at the end, by the segment midpoint's XZ distance from the pose; colour sent as
  `rgb * fade * dim` with alpha `fade`, alpha-blended. Gizmo `depth_bias` -0.002 lifts lines off coplanar
  fills. `colour(segment)` maps the 16 `SegKind`s (terrain `rgb` for Block, Barrier, Floor, Ceiling).
- `Scene::strokes(grid)` yields `(&Segment, dim)`: own level 1.0, neighbours 0.2; `grid` off hides Floor and
  Ceiling. `Ground::crests(eye, reach)` depends on the eye and is recomputed every frame (none on flat ground).
- The camera's XZ is exactly the pose's XZ, so the shader can fade from `view.world_position` with no
  per-frame uniform.
- Other gizmo users: the preview (3D, same loop, sight 8, render layer 2) and the fight screen (2D,
  `line_2d`). `shell/settings.rs::apply_quality` sets the width on every gizmo group.
- Bevy 0.20.0 facts (read from source, not memory): `Material` is in `bevy_pbr` (already enabled); shaders are
  **WESL** (`.wesl`, `import bevy_pbr::render::mesh_view_bindings::view;`, no `#import`/`#ifdef`); the
  material bind group is `@group(constants::MATERIAL_BIND_GROUP)` (3); a crate ships a shader with
  `embedded_asset!` and `ShaderRef::Path(embedded_path!(..) with source "embedded")`; custom vertex
  attributes need `specialize` to set the layout, and `specialize` is also called for the prepass and shadow
  pipelines, so the material returns `false` from `enable_prepass` and `enable_shadows`; `Material::depth_bias`
  only sorts, so the real bias is done on clip z in the shader (the gizmo shader's formula); blended materials
  go to `Transparent3d` without depth writes; `MaterialPlugin::build` touches no render device without a
  `RenderApp`, so headless tests can assert meshes, materials and components. No new feature or crate.
- The gate has no GPU; this machine does, and `--screenshot PATH [--walk N] [--size WxH] [--timing]`
  (`shell/capture.rs`) renders offscreen.

## Design

### A. `strokes.rs` (new, Bevy-free): the vertex buffers
`pub struct Strokes { a: Vec<[f32;3]>, b: Vec<[f32;3]>, corner: Vec<[f32;2]>, colour: Vec<[f32;4]>,
style: Vec<[f32;4]>, indices: Vec<u32> }`: four vertices and six indices a segment. Each vertex carries both
endpoints (`a`, `b`), its corner (`along` 0/1, `side` -1/+1), the linear HDR colour with the level's dim in
alpha, and `style` = `[weight, pattern, role, level]`, written `[1, 0, 0, 0]` in size 1 and **reserved for
size 2** so the mesh format does not change again.
- `Strokes::of(segments: impl Iterator<Item = (&Segment, f32)>)`; `Scene::structure()` and `Scene::grid()`
  split today's `strokes(grid)` into the always-drawn kinds and Floor + Ceiling (reusing `SegKind::drawn`).
- `colour(&Segment)` moves here from `render.rs` unchanged; `fade` and `fog_end` stay as the CPU reference
  the shader's formula is written from (and the preview's sight uses).

### B. `shell/lines.rs` (new) and `shell/lines.wesl` (new, about 150 lines)
- `LineMaterial { #[uniform(0)] params: LineParams }` with `fog_end`, `width` (pixels), `depth_bias`;
  `alpha_mode` Blend; `enable_prepass`/`enable_shadows` false; `specialize` sets the layout
  (`POSITION` = `a`, plus custom attributes `b`, `corner`, `colour`, `style`). Registered with
  `MaterialPlugin::<LineMaterial>`; the shader embedded with `embedded_asset!`.
- Vertex shader: both ends to clip space, near-plane clip, expand by `width` pixels along the screen-space
  normal using `view.viewport.zw`, the gizmo shader's negative-bias depth formula; passes the world position.
- Fragment shader: `fade` from the XZ distance of the fragment to `view.world_position` (1 to half
  `fog_end`, linear to 0), output `vec4(rgb * fade * dim, fade)`: today's numbers, per fragment instead of
  per segment midpoint (a long line now fades along its length; the only intended difference).
- `fn mesh(strokes: &Strokes) -> Mesh` (TriangleList), beside `render::mesh` for the fills.

### C. `shell/render.rs` and `shell/preview.rs`: entities instead of a loop
- Three line entities a view: **structure**, **grid** (its `Visibility` follows `Prefs.grid`: no rebuild),
  **crests** (a small mesh replaced only when the eye has moved and the ground has relief). `rebuild`
  replaces the first two with the fills, on the same triggers as today.
- One `LineMaterial` a view: the game's `fog_end` follows `views.sight`; the preview's is its fixed sight 8;
  `apply_quality` sets `width` on every `LineMaterial` (and still on the fight screen's gizmo group).
- The preview uses the same spawn and rebuild functions on render layer 2; its copied `draw` loop goes.
- The fight screen's 2D gizmos stay (2D, small, a different picture); noted in the docs, not touched.

### D. Size 2, designed in (documents, no code)
`tasks/plans/rendering-size-2.md` and a **proposed** presentation-PRD amendment (text for the owner to
approve; not applied by this plan): what size 1 already gives (screen-space width, fog in the shader, the
`style` attribute and where pattern, weight, colour role and depth level X17 plug into the shader), and what
remains with its order and cost: edge feathering, patterns, the colour roles of M8.5 step 5 as a material
palette, relief (X10) as displacement over a per-map heightfield texture, crests found in the shader,
instancing for objects, piles and beings, and how each is tested without a GPU in the gate.

## Steps (one commit each; red tests first, mutation checks, fmt, clippy, the gate against 1285, Sentrux)

Before step 1: this plan to `tasks/plans/rendering-size-1.md` and into `tasks/TODO.md` as checkable items;
baseline captures and `--timing` of the gizmo renderer (town, hills fixture, yard; 1920x1080 and 1280x720;
standing and a walk) into the scratchpad.

0. **Spike, thrown away.** A hard-coded quad through a minimal `LineMaterial` and `.wesl` in an offscreen
   capture, to prove on this machine: the embedded WESL loads, the imports and bind group resolve, the
   width is in pixels, the bias formula holds. If WESL embedding fails, stop and report before building.
1. **`strokes.rs`.** Buffers, `Scene::structure`/`grid`, `colour` moved. Tests: four vertices and six
   indices a segment with both ends and the right corners; colour and dim per kind and level; structure and
   grid partition `strokes(true)` and structure equals `strokes(false)`; the flat-map oracle untouched.
   Mutations: a corner sign, the dim dropped, a kind in the wrong half.
2. **The material and the game view, beside the gizmos.** `lines.rs`, `lines.wesl`, the entities, and a dev
   switch (`--lines gizmo|mesh`, default gizmo) so both renderers exist for comparison (the swap rule of
   presentation-ARCHITECTURE section 8.2). Headless tests: after a rebuild the entities hold meshes of the expected
   vertex counts; a reshape replaces them; the grid pref hides only the grid entity; sight and quality reach
   the material. Then captures in both modes and a pixel comparison; differences listed and explained.
3. **Crests and the preview** through the same path. Tests: crests appear on the hills fixture and only
   change when the eye moves; the preview's entities sit on layer 2 with sight 8 (`tests/editor_preview.rs`
   extended).
4. **The swap.** Mesh becomes the only renderer for the view and the preview; the gizmo loop, the switch and
   the 3D gizmo styling go. Timing measured again (three runs each) and recorded. Docs:
   presentation-ARCHITECTURE section 8.2, section 8.6, VA4, Q3; presentation-PRD section 11 "Built"; BACKLOG; code-map;
   `tasks/knowledge/verification.md` (how to capture and compare).
5. **Size 2's design** (D above): the plan file and the proposed amendment, for the owner's decision.

Then `tasks/acceptance/rendering-size-1.md` (numbered, with the capture pairs and the timing table), TODO
review, `tasks/CONTINUITY.md` rewritten. Reported as "built, please try".

## Verification
- Per step: `cargo fmt --all`; workspace clippy (`--all-targets --features omnis-vector/test-fixtures
  --locked -- -D warnings`); `scripts/verify.sh` to a scratchpad log, committed only on `VERIFY-GREEN`, the
  count read as a number; Sentrux `rescan` + `check_rules`. No sim crate is touched (floats stay in the
  client); no dependency or feature changes (`Cargo.lock` unmoved).
- Look: offscreen capture pairs (gizmo vs mesh) at two sizes on three maps, compared by a scratchpad pixel
  script and by eye; the eye eases by real time, so standing captures are the exact ones and walks are
  compared by eye.
- Speed: `--timing`, release, a 300-frame walk, against section 8.6's 1.9 to 2.2 ms mean. Expected: not slower.
- By hand (owner): `just run --windowed --pack packs/base --pack packs/the_lower_kingdoms`: the view looks
  as before at each quality; the grid toggle, doors, piles taken, a fight and back, the editor's preview.

## Risks, said plainly
- **Shader authoring in a new dialect (WESL, new in 0.20).** Mitigated by step 0 and by writing from Bevy's
  own shipped shaders (`bevy_gizmos_render/src/lines.wesl`, `bevy_pbr/src/decal/forward.wesl`).
- **The gate cannot see a shader.** A broken shader passes the gate; it is caught only by a capture. Every
  step that changes the shader ends with captures, and the acceptance sheet says so.
- **Blend order.** Lines are one blended mesh in the transparent phase, as gizmos were; if overlapping
  lines differ visibly from today, it is shown in the capture comparison and asked before the swap.

## Not in this plan
- Any of size 2's code; the fight screen's 2D lines; the colour roles (M8.5 step 5); CI captures on a GPU
  runner (BACKLOG, with the multi-platform study).
- The owner's uncommitted edits to `packs/the_lower_kingdoms` stay out of every commit.
