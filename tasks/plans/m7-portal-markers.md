<!-- Approved in plan mode 2026-09-27 (M7 step 3b); copied from the session plan file so it outlives the session. -->

# M7 step 3b: every portal is visible (a marker sprite), and the loader requires it

## Context
The owner played step 3: the town starts the game, but its way out cannot be seen, and neither
can any other portal (the meadow's entrance to the dungeon, the dungeon's exit, the meadow's road
back to town). Portals have never been drawn: the viewport, the automap and `map_text` know walls,
doors and terrain only. Owner decisions (2026-09-27): **a marker sprite per portal**, **required
on every map** (no way-out rule; one-way maps stay legal), **built now as step 3b** before step 4.
Separately decided for step 4: party gold is kept in copper from save schema 5.

Survey facts this plan rests on:
- `SlotKind::Object` exists (`omnis-data/src/tileset.rs:32`) but is skipped by the baker
  (`omnis-cli/src/bake.rs:234`, `:409`) and never drawn.
- `plan::viewport` (`omnis-app/src/plan.rs:77-176`) is at the 100-line cap. Its per-row pass
  draws floor/ceiling, fronts, sides, then blocks (`:169-173`, resolved from the terrain in the
  app, the pattern to copy).
- The automap is `plan.rs:224-297`; party ops must stay last (tests read `ops[len-2..]`).
- `query::map_text` is `omnis-sim/src/query.rs:126-179`.
- The source sheets (`assets/openrtp-tiles/{dungeon,exterior}.png`, CC0) have no alpha; the
  background is palette pink `#ff678b`. Objects span more than one 16 px tile. The baked PNGs
  are committed and a test compares them with a fresh bake (`omnis-cli/tests/bake.rs:21`).

## Changes

### 1. Data (`omnis-data`)
- `Portal.marker: Option<String>` (serde default, so a file without one parses and gets a named
  error). In `MapDef::validate`, `None` is refused: "portal at (x, y) has no marker; every portal
  must be visible".
- In `loader::check_surfaces`, the marker must be an `Object` surface of the map's tileset
  (the same wording as the other surface checks).
- `MapData::marker_at(x, y) -> Option<&str>`, beside `portal_at` (`loader.rs:101`).
- Bad-pack rows: the `broken` pack's portals gain the missing-marker errors, and one names a
  `Floor` surface as its marker.

### 2. Baker (`omnis-cli/src/bake.rs`, ~+70; stays under 1000)
- `BakeSpec.key: Option<(u8, u8, u8)>`: the sheet's background colour, made transparent when an
  `Object` is cropped.
- `SurfaceSpec.size: (u32, u32)` in tiles, default `(1, 1)`, so an archway can be 2×2.
- `slot_positions` keeps `Object` where `Block` is kept (depth > 0 or beside the party).
- `render` gets an `Object` arm, `standee`: one upright billboard at the tile's centre
  (z = d + 0.5). The image is stretched once, fitted into a 1×1-unit box with its aspect kept,
  and stands on the floor. Transparent texels are skipped.
- Unit tests:
  - an Object has slots where a Block has them
  - a keyed crop renders with transparent corners and opaque pixels inside

### 3. Art and packs
- New Object surfaces:
  - dungeon spec: the way up
  - outdoor spec: the way down into the dungeon, and a signpost for the roads between maps
- Crops are chosen by eye from enlarged candidate crops of the two sheets (placeholder art,
  PRD D26). The spec's comment records the coordinates.
- Re-bake both tilesets (`omnis-cli tileset bake packs/test/bake/dungeon.ron
  packs/test/bake/outdoor.ron`); only new PNGs are added.
- Every portal names a marker:
  - meadow (16, 5): the way down
  - meadow (16, 31): the signpost
  - dungeon (0, 0): the way up
  - town (11, 2): the signpost
- Both replays are rebaselined (the pack hashes change).

### 4. App painter (`omnis-app/src/plan.rs`, ~+50)
- First, with no behaviour change: move the row body of `viewport` (`:133-173`) into
  `fn row(...)`, which makes room. The plan tests prove nothing moved.
- Then a fifth pass in `row`, after blocks: for each tile with `marker_at`, push its Object
  slot (`push_slot`).
- Automap: after a known tile's edges and before the party ops, a centred fill in
  `PORTAL_MARK` (a new colour constant) on each portal tile.

### 5. Text for agents (`omnis-sim/src/query.rs`)
- `map_text` prints `*` on a portal tile (the party's glyph still wins), and the doc comment
  says so.
- `*` joins the terrain glyphs reserved for edges in `MapDef::validate`.

## Verification
- **Tests:**
  - data:
    - a portal without a marker is refused
    - a marker that isn't an Object surface is refused
    - the test pack's four markers resolve through `marker_at`
  - bake: the unit tests above, and `committed_tilesets_match_a_fresh_bake` on the re-baked files
  - plan:
    - a marker is drawn after its row's floor, walls and blocks and before the nearer rows, at
      its slot's position (by analogy with `a_pillar_is_a_block_drawn_after_its_row`)
    - the automap marks a known portal tile, and the party ops stay last
  - query: `map_text` shows `*` at the town gate
- **Gate and commit:**
  - `scripts/verify.sh > log 2>&1`, read the status
  - both replays rebaselined in the same commit
  - Sentrux `scan` (new files `git add`ed first) and `check_rules`; `plan::viewport` shrinks
    below the cap
  - one commit, and TODO item 3b checked with the numbers
- **What the owner will see**, shown before the hand-over: `--screenshot-composed` captures of
  - the town facing the gate
  - the meadow facing the dungeon entrance
  - the dungeon's exit
  - the automap with portal marks

  The report says what does and does not work yet: services still open only in step 4.
- **Docs:** `code-map.md` and `verification.md` (pins, the bake line); a TODO item 3b in the M7
  block. The TODO line records the gold-in-copper decision for step 4.

## Size
About 350 lines plus the baked PNGs, in one commit.
