# Rendering size 2, part 3: colour roles as a palette, with biospheres in pack data

## Context

The owner's order (2026-11-13): rendering size 2 parts 3 to 6, in the order 3, 4, 5, 6, each its own plan.
This is part 3, and by the owner's choice it takes in **all of M8.5 step 5's colour half**: the fixed
colour roles, a biosphere's palette in pack data, and where a biosphere is assigned.

Today a line's colour is decided on the CPU and baked into every vertex (`strokes::colour` by `SegKind`);
floor, ceiling, block and barrier lines take it from the terrain's own colour, which presentation-PRD X18
lists as the rejected alternative. After this plan a line carries a **role**, the colour comes from a
**palette held on Bevy's line material**, and a pack's **biosphere** supplies that palette, so changing map
or biosphere swaps colours with no mesh rebuild.

The owner's decisions (2026-11-13, asked in plan mode):
- **X23 approved as written** (the proposed presentation-PRD amendment in `tasks/plans/rendering-size-2.md`).
- **Role set**: PRD 5.2's set with "interactive" split in two. Nine roles: structure, ground, passage,
  thing, party, hostile, magic, danger, interface.
- **Assignment**: a region names its biosphere; a map may name its own.
- **Terrain colour**: a terrain names its role; the palette alone decides colour in the 3D view. The automap
  and the editor's plan keep terrain colours.
- **Surfaces**: the 3D view and the editor preview now. The fight screen, minimap and HUD follow in their
  own plan before part 4.

**The look changes on every map**: floors stop taking the terrain's colour. The owner judges by capture.

## Design

### A. Pack data (`omnis-data`; integers only)
- `ColourRole` enum (new `biosphere.rs`): `Structure, Ground, Passage, Thing, Party, Hostile, Magic, Danger,
  Interface`. Open for more: a palette is a list keyed by role, not a struct of nine fields.
- New kind **`biosphere`**, `data/biospheres/<name>.ron`, id `pack:biosphere:name`:
  `BiosphereDef { schema, id, name (text key), palette: BTreeMap<ColourRole, Ink> }`,
  `Ink { rgb: (u8, u8, u8), glow: u16 }` (sRGB as `Terrain.color` is; glow per mille, 1000 = no bloom,
  capped at 4000). A role a biosphere leaves out takes the built-in default's ink, so a new role never
  breaks an old pack. `deny_unknown_fields`.
- `Terrain.role: Option<ColourRole>` (`#[serde(default)]`): left out, its grids are ground and its solid
  forms structure. `RegionDef.biosphere` and `MapDef.biosphere`: `Option<String>`, default none, checked
  at resolution (an unknown id is a load error). Defaults mean **no existing pack's seal changes**.
- One lookup, `Data::biosphere_of(map) -> Option<&Biosphere>`: the map's, else its region's (the loader's
  existing `owner(map)`), else none. The client already reads `MapData` from `Data` for terrain colour;
  this is the "one function" of presentation-ARCHITECTURE 7.2.
- The kind follows the `flag` checklist: `lib.rs`, `content.rs`, `loader.rs` (`gather`, `has_schema!`),
  `catalog.rs` (`Definition::Biosphere`, `KINDS` 15 to 16), `edit/files.rs` (typed RON editing),
  `edit/notes_store.rs`, `omnis-cli/src/schema.rs` (a section and example), bad-pack fixtures.
- **Pins that move**: `KINDS` 16 and `docs/api.md`'s kind wording. `PROTOCOL` stays 2 (an added kind in
  the `data.*` ops is additive, as before). No command, event or rejection changes.
- **Fixtures**: two biospheres in the `objects` fixture pack; its region names one, one map overrides
  with the other (if the pack has a single map, the override goes in the `levels` pack). Both resealed.
  `test` and `base` are untouched, so no replay is rebaselined. **The owner's pack is not touched**: it
  shows the built-in default until the owner writes biospheres.

### B. The client's palette (`omnis-vector/src/style.rs`, Bevy-free)
- `Ink { rgb: [f32; 3] }` linear with the glow multiplied in; `Palette { inks: [Ink; SLOTS] }` with
  `SLOTS = 16` (nine used; room to grow without a new shader layout); `Palette::default()` built in;
  `Palette::of(&Biosphere)` (sRGB to linear, times glow, gaps from the default).
- The built-in default keeps today's hues where a role has one: structure the walls' green, passage the
  doors' amber, thing the objects' violet, hostile the beings' orange, danger the holes' red; ground a dim
  green, magic a pale cyan, party and interface the HUD's green. Values are tuned in step 4 by the checks.
- `strokes::role(&Segment) -> ColourRole`: Wall, Cliff to structure; Block, Barrier to the terrain's role
  or structure; Floor, Ceiling, Ramp to the terrain's role or ground; Door, Portal, Stair, Ladder to
  passage; Object, Pile, Activity to thing; Being to hostile; Hole to danger. Crests and the aimed
  floor's outline: ground and thing.
- `geometry::Segment.rgb` becomes `role: Option<ColourRole>` (the terrain's named role, resolved where the
  terrain is read today). The minimap, the editor's plan and swatches keep reading `terrain.color`.
- The vertex keeps its layout. `style.z` is the role's index. The `colour` attribute becomes a **strength**
  on the role's ink (1 for most; the ceiling grid half the floor's; a crest brighter), its alpha still the
  level's brightness (part 4's).

### C. The shader and the material (`shell/lines.rs`, `lines.wesl`, `shell/render.rs`, `shell/preview.rs`)
- `LineMaterial` gains `#[uniform(1)] palette: [Vec4; 16]`; the vertex stage reads
  `palette[u32(style.z)].rgb * colour.rgb`. No new import in `lines.wesl`.
- A system sets the view's material palette from `biosphere_of(current map)` when the map or the loaded
  data changes, and the preview's from the edited map. **No mesh is rebuilt for a palette change.**

### D. Checks (presentation-ARCHITECTURE 7.3; `style.rs`)
- `style::clashes(&Palette) -> Vec<Clash>`: each used role against black and against every other, by
  luminance ratio and colour distance, as seen and under simulated protanopia, deuteranopia and
  tritanopia (fixed matrices). Thresholds are set in step 4 from measurements and shown to the owner.
- Tests run it on the default and on every fixture biosphere. If the default cannot pass with today's
  hues, I show which pairs fail and the adjusted values in captures; nothing is waved through.

### E. Editor
- Biosphere files edit as typed RON text (the existing files editor). The terrain form gains
  `Role < default | structure | ... >` beside its colour fields. No biosphere form in this plan.

## Steps (one commit each; tests red first; mutation checks with a touch after restore; fmt, clippy, the gate against 1312, Sentrux)

0. **Documents first.** This plan to `tasks/plans/rendering-size-2-palette.md` and `tasks/TODO.md`; X23
   and the X10, X18/X19 additions applied to `presentation-PRD.md`, with 12.2 and 12.3 marked answered
   (flagged in the commit as the owner-approved amendment).
1. **Data.** `ColourRole`, the `biosphere` kind, `Terrain.role`, the two `biosphere` fields,
   `biosphere_of`, catalogue, CLI schema, bad-pack fixtures, fixture biospheres and reseals, `docs/api.md`.
   Tests: loads and resolves; map override beats region; none gives none; unknown id, glow over the cap
   and an unknown field refused; `KINDS` covers the loader. Mutations: region beats map; unknown id passes.
2. **Roles on the CPU.** `style::{Ink, Palette}`, `strokes::role`, `Segment.role`, `style.z` and the
   strength written. `tests/strokes.rs`, `tests/pile_reach.rs`, `tests/geometry.rs` move to roles; new: a
   terrain's named role reaches its floor and block lines; every `SegKind` has a role. Mutations: the
   terrain's role ignored; two kinds' roles swapped.
3. **The palette on the material.** The uniform, the shader read, the swap system for view and preview.
   Headless tests: the material's palette is the map's biosphere's; crossing to a map of another biosphere
   changes the palette and leaves the line mesh asset untouched; a map with none shows the default.
   Captures (Low first, then High) of the yard under default and under each fixture biosphere, for the owner.
4. **Checks.** `style::clashes`, thresholds, tests over the default and the fixtures, the default tuned.
5. **Editor and documents.** The terrain form's role; presentation-ARCHITECTURE 7 and 8.2,
   `rendering-size-2.md` status, code-map, verification notes, `tasks/acceptance/rendering-size-2-palette.md`,
   TODO review, `tasks/CONTINUITY.md`.

Reported as "built, please try".

## Verification
- Per commit: `cargo fmt --all`; workspace clippy (`--all-targets --features omnis-vector/test-fixtures
  --locked -- -D warnings`); `scripts/verify.sh` to a scratchpad log, committed only on `VERIFY-GREEN`, the
  count read as a number; Sentrux `rescan` and `check_rules`.
- The shader, which the gate cannot see: offscreen capture pairs with the still settings
  (`tasks/knowledge/verification.md`), judged at Low first.
- By hand (owner), `just qa`: the yard in its biosphere; walk to the map that overrides it and see the
  colours change at the crossing; in the editor, set a terrain's role to danger and see its floor change.

## Risks, said plainly
- **Every map looks different after step 3**: floors lose their terrain colours, and grass and road look
  alike underfoot unless given different roles. That is the choice made; the captures show it before the
  owner's pack is ever involved.
- **A uniform array on Bevy's `AsBindGroup`** is new to this crate. I read Bevy 0.20's source for the form
  before step 3; if an array will not bind, the alternative (a small palette texture) is asked first.
- **The default palette may fail the colour-deficiency check** with today's hues (green against amber is
  the likely pair). Then the default changes, by capture, with the owner's eye on it.
- **`style::Role`** (the interface's text roles) and `ColourRole` stand side by side until the following
  plan joins the interface to the palette.
- Not seen in a window by me; shader work is checked by capture.

## Not in this plan
The fight screen, minimap, picture window and HUD on the palette (the following plan, before part 4); a
colour-deficiency preset in Settings; glyph markers and motion; relief noise per biosphere; a biosphere
form in the editor; biospheres for `packs/the_lower_kingdoms` (the owner's to write; their uncommitted
edits stay out of every commit); parts 4, 5 and 6.
