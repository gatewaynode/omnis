# Line patterns and weight in the line shader (rendering size 2, part 2)

## Context

Rendering size 1 moved the 3D view's lines onto retained meshes under `LineMaterial` and `lines.wesl`; part 1
of size 2 (edge feathering) is built and reviewed by the owner (2026-11-12, "looks good so far"). Every vertex
already carries a `style` `[weight, pattern, role, level]`, written `[1, 0, 0, 0]`; the shader reads only the
weight. The presentation PRD (X19, section 5.3) requires meaning to be carried without colour, by line pattern
and line weight among other channels; none exists yet.

The owner asked for part 2, and chose (2026-11-12):
- **Dashes are measured in the world** (cells along the line), not in screen pixels. This departs from the
  size 2 design's text ("in pixels") by the owner's choice; the design file is corrected.
- **A provisional table from presentation-PRD section 5.3** is applied now, to be replaced by M8.5 step 5's
  vocabulary, as today's colours are provisional.

Outcome: the shader draws four patterns (solid, dashed, dotted, double) and three weights; one table in
`strokes.rs` says which kind of line gets which; the view and the editor's preview both show it.

## Design

### A. `style.rs`: the two channels (Bevy-free; presentation-ARCHITECTURE section 7.1 names them there)
- `pub enum Pattern { Solid, Dashed, Dotted, Double }` with `code() -> f32` (0, 1, 2, 3: what the shader reads).
- `pub enum Weight { Hairline, Normal, Heavy }` with `factor() -> f32` (0.5, 1.0, 1.5: a factor on the
  material's width).
- Sawtooth (named in the PRD for hostile) is not in this step: it needs a quad wider than the line. Noted as
  the next pattern; `Double` stands in for hostile until then.

### B. `strokes.rs`: the table
- `pub fn channels(kind: SegKind) -> (Weight, Pattern)`, an exhaustive match (a new kind cannot be forgotten):

  | Kinds | Weight | Pattern |
  |---|---|---|
  | Wall, Door, Block, Barrier, Cliff, Ramp, Stair, Ladder, Hole, Activity | Normal | Solid |
  | Floor, Ceiling (the grids) | Hairline | Solid |
  | Object, Pile (interactive) | Heavy | Dashed |
  | Being (hostile) | Heavy | Double |
  | Portal | Normal | Dotted |

- `Strokes::push` takes the style; `Strokes::of` writes `[weight.factor(), pattern.code(), 0, 0]`. Crests
  (`shell/lines.rs::crests`) stay plain. `PLAIN` stays as the crests' style.

### C. `shell/lines.wesl`
- Vertex: two more varyings. `run`: the distance in cells from the segment's own first end (`in.a`, before the
  near-plane cut) to this vertex's world point, interpolated perspective-correct, so a dash is a fixed piece of
  the world. `pattern`: `style.y`, `@interpolate(flat)`.
- Fragment, multiplied into the existing `cover`:
  - **Dashed**: period 0.25 cell, drawn for half of it. **Dotted**: period 0.125 cell, drawn for 0.3 of it.
    Edges softened over one pixel with `fwidth(run)`.
  - **Far filter**: where a period covers fewer than about 3 pixels, the pattern blends to its average (the
    line dims evenly) instead of shimmering.
  - **Double**: the middle third of the width is left open (two rails), feathered; when the line is under
    3 pixels wide (Heavy at Low quality is 1.5) it is drawn solid, since two rails would not resolve.
- Hairline at Low quality (0.5 pixel) uses the existing rule: one pixel wide, half as bright.
- No new uniform, no new Bevy shader import, no mesh layout change, no dependency.

### D. What does not change
The click test (`aim.rs`, on `Body`), the fills, fog, depth, feathering, the fight screen's 2D lines, the sim
crates. The editor's 3D preview shares the path and so shows the same patterns.

## Steps (one commit each; red tests first, mutation checks, fmt, clippy, the gate against 1293, Sentrux)

0. This plan to `tasks/plans/rendering-size-2-patterns.md` and into `tasks/TODO.md` as checkable items;
   baseline captures (yard with objects and piles, a map with a portal, a being in view, the hills; High and
   Low; `--settings .omnis/test-saves/still.ron`) into the scratchpad.
1. **The channels and the table (CPU).** `style.rs` enums, `strokes::channels`, `push`/`of`. Tests in
   `tests/strokes.rs`: each row of the table by kind; every vertex of a segment carries the same style; the
   pattern codes are distinct and Solid is 0; weights ordered Hairline < Normal < Heavy; crests plain.
   `tests/lines.rs`: the mesh's style attribute carries what the strokes hold. Mutations: two kinds swapped in
   the table, the weight dropped from `of`, two codes made equal. (The weight shows at once, since the shader
   already reads it: captures confirm grids thinner, objects, piles and beings heavier.)
2. **The patterns (shader).** `lines.wesl` as in C. Verified by capture, since the gate has no GPU: zoomed
   before/after pairs of each pattern near and far, at Low and High; a walk pair to confirm dashes stay put;
   a far view to confirm the filter (no shimmer, even dim); crossings and corners for artifacts. Pixel
   differences listed by region with `pixdiff.py`; anything not on a patterned or re-weighted line is a defect.
   Timing three runs against 2.0 ms.
3. **Documents.** presentation-ARCHITECTURE section 8.2 (the patterns, the world measure) and section 7.1
   (pattern and weight built, provisional table; roles still step 5); presentation-PRD section 11 "Built"
   (one line; changed under this approved plan and flagged in the commit); `tasks/plans/rendering-size-2.md`
   (part 2 built, "pixels" corrected to the world by the owner's choice); code-map; an acceptance sheet
   `tasks/acceptance/rendering-size-2-patterns.md`; TODO review; `tasks/CONTINUITY.md` rewritten.

Reported as "built, please try".

## Verification
- Per commit: `cargo fmt --all`; workspace clippy (`--all-targets --features omnis-vector/test-fixtures
  --locked -- -D warnings`); `scripts/verify.sh` to a scratchpad log, committed only on `VERIFY-GREEN`, the
  count read as a number; Sentrux `rescan` + `check_rules`. `Cargo.lock` unmoved.
- Look: offscreen capture pairs as in `tasks/knowledge/verification.md`, on scratch copies of the fixture
  packs (never the owner's pack).
- By hand (owner): `just run --windowed --pack packs/base --pack packs/the_lower_kingdoms`: grids fainter and
  thinner; objects and piles dashed and heavier; a group in view heavy with two rails; a portal's mark
  dotted; walking toward a dashed line, the dashes stay on the thing; the same in the editor's preview.

## Risks, said plainly
- **The gate cannot see a shader.** Step 2 is verified only by captures on this machine (Metal).
- **Short segments.** Objects, piles and portal marks are made of short segments (some under a dash period);
  each segment starts its own pattern, so small shapes may read as broken outlines rather than dashed. If the
  captures show that, I stop and ask (a shorter period for small shapes, or the pattern run continuously along
  a shape's outline, which needs a per-vertex offset in the mesh) before going on.
- **Double at low width.** Below 3 pixels it falls back to solid, so at Low quality hostile is told apart by
  weight and colour only until sawtooth exists.
- **The look changes in play**, provisionally; one table to change.

## Not in this plan
Sawtooth; glyph markers and motion (the other two channels); colour roles and palettes (part 3, needs the PRD
amendment); depth levels (part 4); the fight screen and HUD taking the same channels (M8.5 step 5); the
owner's uncommitted edits to `packs/the_lower_kingdoms` stay out of every commit.
