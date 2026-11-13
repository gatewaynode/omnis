# Rendering size 2: the look computed on the GPU (design, for the owner's decision)

Status: **design, 2026-11-12; part 1 (edge feathering) built the same day at the owner's go; part 2 (patterns and weight) built 2026-11-13; part 3 (colour roles as a palette, with biospheres in pack data) built 2026-11-13**. **The owner's order 2026-11-13: parts 3 to 6, in the order 3, 4, 5, 6, each its own plan; the amendment at the end approved as written and applied to `presentation-PRD.md` as X23. Part 3's plan: `tasks/plans/rendering-size-2-palette.md`.** It is step 5 of
`tasks/plans/rendering-size-1.md`: the owner chose size 1 "with size 2 designed in". Building any part of it
needs the owner's go and, for the parts marked *amendment*, the presentation-PRD text at the end.

## Where size 1 left things

- The view's and the preview's lines are one retained mesh a view, drawn through Bevy's `Material` trait by
  `shell/lines.wesl` (`LineMaterial`). The fills are a second mesh under Bevy's `StandardMaterial`.
- Already in the shader, pulled forward from size 2 at the owner's choice: **screen-space line width** and
  **fog by distance**.
- Already in the mesh, unused: a per-vertex **`style`** `[weight, pattern, role, level]`, written
  `[1, 0, 0, 0]`. The shader reads `weight` (a factor on the width); the other three are ignored.
- Still on the CPU: the colour of each line (`strokes::colour`, baked into the vertex), the crests (found on
  the CPU when the eye moves), the shapes of objects, piles and beings (segments like any other), and the whole
  mesh is rebuilt when anything in the scene changes (a door, a pile taken).
- Measured: about 2.0 ms a frame offscreen with either renderer. Size 2 is for capability, not speed.

## The parts, in the order I would build them

Each is one plan-mode step of its own, independent unless said. "Shader only" means no mesh or data change.

| # | Part | What changes | Needs |
|---|------|--------------|-------|
| 1 | **Edge feathering** | The fragment stage fades the quad's last pixel across its width, so lines are smooth without relying on MSAA, and a line thinner than a pixel dims instead of flickering. Shader only: the quad already carries `corner.y`. | Nothing. **Built 2026-11-12**: slanted lines are smooth at Low and High in zoomed captures, no artifact where lines cross; about 1% of pixels change (line edges). |
| 2 | **Patterns and weight** (presentation-ARCHITECTURE §7.1) | `style.x` weight and `style.y` pattern (solid, dashed, dotted, double) drawn in the fragment stage from the distance along the line **in the world** (the owner's choice, 2026-11-12, over this design's first "in pixels"). `strokes.rs` writes them per `SegKind` from a table. | The pattern vocabulary is M8.5 step 5's open question. **Built 2026-11-13** (`tasks/plans/rendering-size-2-patterns.md`) on a provisional table from presentation-PRD 5.3; between its dashes a line is lighter, not open (the owner, after seeing voids break the shapes). |
| 3 | **Colour roles as a palette** (X18, X19; M8.5 step 5) | The vertex carries a role index (`style.z`), not a colour; the material holds the palette (a small uniform array, or a 1D texture a biosphere supplies). Changing biosphere or a colour-deficiency preset swaps the palette with no mesh rebuild. | *Amendment.* **Built 2026-11-13** (`tasks/plans/rendering-size-2-palette.md`) with all of M8.5 step 5's colour half at the owner's choice: nine roles, a region names its biosphere and a map may override, a terrain names its role; the palette a uniform array on the material. |
| 4 | **Depth levels** (X17) | `style.w` level (immediate, intermediate, distant) selects width, brightness and fog per level in the shader; the neighbour dim (0.2 today, baked into the colour's alpha) becomes one of them. | The three levels' looks are a design choice (X17 names them; no numbers yet). |
| 5 | **Instanced objects, piles and beings** | Each kind's shape is one small mesh; a placement is an instance (transform, role, state). A pile taken or an object's state changing updates one instance, not the map's mesh. Bevy batches instances of one mesh and material automatically. | Shapes move from `shapes.rs` segments to per-kind meshes; the click test (`aim.rs`) keeps using `Body`. The largest code move of the six. |
| 6 | **Relief as displacement** (X10) | A per-map heightfield texture (the authored eighths plus seeded noise per terrain kind); the vertex stage of both the lines and the fills samples it, so the ground's fine relief is the GPU's. Crests are then found in the shader (a line is a crest where the two faces beside it face opposite ways from the eye), ending the last per-move CPU walk. | *Amendment.* It changes what X10 says is computed where, and the collision mirror must sample the same height (the same noise on the CPU for `Ground::height`, pinned by a test against the texture's bytes). The riskiest part: do last. |

Not in size 2: a GPU-resident map with compute passes (size 3, a horizon); the fight screen's 2D lines (they
can take `LineMaterial` through a 2D material later; no need today).

## What each part costs beyond code

- **Testing.** The gate has no GPU. Size 1's pattern holds for parts 1 to 5: what the CPU decides (which role,
  weight, pattern and level a line has; which instances exist) is tested headless in `strokes.rs` and the
  entity tests; what the shader draws is compared by capture on this machine (`tasks/knowledge/verification.md`).
  Part 6 moves geometry itself into the shader, which no headless test sees: it needs either a CPU twin of the
  displacement kept in step by a test, or captures in CI on a GPU runner (BACKLOG, the multi-platform study).
- **Shader dialect.** WESL is new in Bevy 0.20 and Bevy's own shaders are still moving to it. Each Bevy
  upgrade re-checks `lines.wesl` against the imports it uses (today one: `bevy_pbr::render::mesh_view_bindings::view`).
  The fewer Bevy shader modules the shader imports, the cheaper an upgrade; size 2 as designed adds none.
- **Dependencies.** None. No new crate or Bevy feature in any part.
- **Documents.** Parts 3 and 6 need the amendment below; parts 1, 2, 4 and 5 fit presentation-PRD X3, X13 and
  X17 as written and need only presentation-ARCHITECTURE §8.2 kept current.

## Recommendation

Build 1 (feathering) next, alone: small, shader only, and it settles whether the lines should rely on MSAA.
Then 2 to 4 together with M8.5 step 5 (the colour scheme), since step 5 decides the vocabulary they draw.
Then 5. Hold 6 until relief is actually wanted on screen; it is the only part with a real risk to the
collision mirror.

## The amendment to `presentation-PRD.md` (approved as written and applied 2026-11-13)

> **X23 (the next free number). The look is computed on the GPU (owner, 2026-11-12, "GPU-first", size 2).** The client decides
> *what* exists on the CPU (cells, edges, objects, their roles and states) and sends it once as meshes,
> instances and small tables; *how it looks* (width, fade, pattern, weight, colour from the role's palette,
> depth level, and in time the ground's fine relief) is computed in shaders of the client's own, drawn through
> Bevy's mesh and material path. Rules never depend on anything a shader computes: where relief is displaced
> on the GPU, the collision mirror computes the same height on the CPU, and a test holds the two together.
> A GPU-resident map generated by compute passes (size 3) stays a horizon.
>
> X10 gains: "Relief may be applied as displacement in the vertex stage from a per-map heightfield; the CPU
> keeps the same function for the camera and collision."
> X18 and X19 gain: "A line carries its colour role; the palette is material data, so a biosphere or an
> accessibility preset changes the palette without rebuilding geometry."
