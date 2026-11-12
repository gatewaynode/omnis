# Acceptance: the icon library (after M5.6)

The owner added the Lucide icon set (ISC) under `assets/svg-icons/` on 2026-10-24 and asked for the rendering as
"a self contained library" of our own, its own step after M5.6 and before M5.7. Two commits: the crate, then the
client drawing icons.

**Accepted by the owner 2026-10-24** ("Manually tested. Having these icons handy should make the map and UI
work easier as we can start to abstract it away from text where it makes sense.").

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

### 2. The client draws icons (2026-10-24)
- `omnis-vector` depends on `omnis-icon`. `Raster::icon` draws an icon with the round brush at any size (the
  brush's radius the icon's stroke width), for the plan and any image node; `geometry::icon_on_floor` lays an
  icon's lines on a cell's floor at the ground's heights, inside a margin.
- `icons::activity()` is the activities' placeholder, Lucide `face-grinning` (the set has no `smile`), read once
  and kept. It replaces the hand-drawn smiley on the plan's site mark and on the 3D floor of a service's cell.
- Nothing else of the game draws an icon yet: which controls get icons is a design pass with the owner, and the
  activities' per-kind icons wait on the category's data shape (M5.7 planning).

## Steps
1. `cargo test -p omnis-icon`.
   *You will see* every icon of the set read within its box (1,903, a pinned count), the circle icon as one
   closed 32-point ring at radius 10, `door-open` as five strokes, the licence beginning "ISC License".
2. `cargo run -p omnis-icon --example show -- door-open` (or any name in `assets/svg-icons/icons/`).
   *You will see* the icon as `#` characters, 48 columns by 24 rows.
3. `just run --windowed --pack packs/base --pack packs/test`, Editor, open the town, zoom in.
   *You will see* the Lucide grinning face in white on each service's cell, inset in the cell, scaling with the
   zoom; the other marks as before.
4. New game, in the town, face a service's door.
   *You will see* the same face on the cell's floor in gold, lying on the ground.

Covered by `crates/omnis-icon/tests/{catalogue,path}.rs`, the reader's unit test, `raster.rs`'s unit test (an
icon drawn inside its box, hollow where the icon is), `tests/geometry.rs` (each site carries the icon line for
line, nowhere else) and `tests/editor_forms.rs` (the site's mark on the painted plan).

## Known and expected
- Curves are flattened once at the icon's scale; at very large sizes (over about 200 pixels a side) the pieces
  show as facets. The game draws icons at cell and button sizes.
- Only `icons/` is compiled in; `lab/` (the experimental icons) is not, until one is wanted.
