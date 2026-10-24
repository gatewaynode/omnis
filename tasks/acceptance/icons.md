# Acceptance: the icon library (after M5.6)

The owner added the Lucide icon set (ISC) under `assets/svg-icons/` on 2026-10-24 and asked for the rendering as
"a self contained library" of our own, its own step after M5.6 and before M5.7. Two commits: the crate, then the
client drawing icons.

## What was built

### 1. `omnis-icon` (2026-10-24)
- **A new crate**, no dependencies, Bevy-free. Its build script compiles every `assets/svg-icons/icons/*.svg`
  into the binary by name, sorted, with the set's licence text (ISC asks that the notice travel with copies).
  Nothing is read at run time; a missing folder fails the build with the path it expects.
- **The reader** takes exactly the set's subset of SVG: one root with its fixed attributes and flat self-closing
  children (`path`, `circle`, `rect`, `line`, `ellipse`, `polyline`, `polygon`). Anything else is refused by
  name, so a future Lucide release that adds a feature fails the tests rather than drawing wrong. A child's
  `fill` is the set's dot (a circle of radius a half), covered by the stroke's own width.
- **Paths** read every command in both cases, implicit repeats and the compact syntax the set writes (`00-2`,
  `.01`, `1.124.992z`); curves are flattened to pieces a quarter unit long (at most sixteen), arcs by the
  specification's endpoint-to-centre conversion in 15° steps, landing on their endpoint exactly.
- **An icon** is its strokes in the 24-unit box (`Icon { name, strokes }`, each stroke open or closed);
  `segments(size)` gives its lines at any size and `width(size)` the stroke width; `get(name)` reads one from
  the catalogue, `names()` lists the set.

## Steps
1. `cargo test -p omnis-icon`.
   *You will see* every icon of the set read within its box (1,903, a pinned count), the circle icon as one
   closed 32-point ring at radius 10, `door-open` as five strokes, the licence beginning "ISC License".
2. `cargo run -p omnis-icon --example show -- door-open` (or any name in `assets/svg-icons/icons/`).
   *You will see* the icon as `#` characters, 48 columns by 24 rows.

Covered by `crates/omnis-icon/tests/{catalogue,path}.rs` and the reader's unit test.

## Known and expected
- Curves are flattened once at the icon's scale; at very large sizes (over about 200 pixels a side) the pieces
  show as facets. The game draws icons at cell and button sizes.
- Only `icons/` is compiled in; `lab/` (the experimental icons) is not, until one is wanted.
