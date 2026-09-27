# Continuity notes

Written 2026-09-27 mid M7 step 3b, for a client restart and a model change. Rewrite this file
every time it is used; keep it to state, next step and pointers. The durable knowledge lives in
`tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then read `tasks/knowledge/README.md`, `agreements.md`, and
  `tasks/plans/m7-portal-markers.md` (the approved plan for step 3b) before touching code.
- The commit trailer names whichever model is running (`Co-Authored-By: Claude <model> ...`);
  a `Claude-Session:` line only if the new session's attribution reminder gives one.

## State
- Branch `m7-tasks`. Pushed up to `5bce8b7` (CI green). Unpushed: every commit after it,
  through M7 step 3 (`81158b8`), its continuity (`2e28972`) and this docs commit.
- M7 steps 0–3 are done. **The owner played step 3**: the game starts in town, but no portal on
  any map can be seen (portals have never been drawn), and nothing in town can be used yet
  (services open in step 4, the panel is step 7). Lesson recorded in `tasks/LESSONS.md`.
- **Owner decisions, 2026-09-27**: (1) every portal gets a marker sprite, required by the loader
  on every map, with no way-out rule (one-way maps stay legal); (2) built now as step 3b, before
  step 4; (3) party gold is kept in copper from save schema 5 (step 4), shown as gp/sp/cp.
- Gate at `2e28972`: `tests passed 381 failed 0 ignored 6`; pins: tuple
  `(4, 4, 3, 24, 16, 11, 3, 29, 7)`, walk `17768808726456611042`, fight `14183048486799478772`.

## Work in progress (uncommitted, in the working tree; the gate fails until the packs catch up)
Plan section 1 (data) is written and builds (`cargo build -p omnis-data`):
- `map.rs`: `Portal.marker: Option<String>` (serde default); `MapDef::validate` refuses `None`
  ("portal at (x, y) has no marker; every portal must be visible"); `*` joins the reserved
  terrain glyphs (message now "... is reserved for edges and portals").
- `loader.rs`: `MapData::marker_at`; `check_surfaces` requires the marker to be an `Object`
  surface ("portal marker surface ...").
Not yet done from section 1: the `bad_packs.rs` expectations (the reserved-glyph wording at
line 58, the missing-marker rows for the broken pack's portals, one marker naming a `Floor`
surface) and a `marker_at` check in `load_test_pack.rs`.

## Next, in the plan's order
2. Baker (`omnis-cli/src/bake.rs`, read: `BakeSpec` :28, `SurfaceSpec` :56, the crop loop in
   `bake_to` :148, `slot_positions` :225, `render` :375, `front` :419): `BakeSpec.key` colour
   key (the sheets' background is palette pink `#ff678b`, no alpha), `SurfaceSpec.size` in
   tiles, `Object` slots where `Block` has them, a `standee` billboard at z = d + 0.5 fitted in
   a 1×1 box, transparent texels skipped; unit tests.
3. Art: pick crops by eye from enlarged crops of `assets/openrtp-tiles/{dungeon,exterior}.png`
   (the pink object area, lower right); surfaces for the way up (dungeon), the way down and a
   signpost (outdoor); re-bake with `omnis-cli tileset bake packs/test/bake/dungeon.ron
   packs/test/bake/outdoor.ron`; markers on meadow (16, 5), (16, 31), dungeon (0, 0), town
   (11, 2); rebaseline both replays.
4. Painter: move `plan::viewport`'s row body (:133-173) into `fn row` first (no behaviour
   change), then the marker pass after blocks; automap `PORTAL_MARK` fill before the party ops.
5. `query::map_text` prints `*` on portal tiles.
Then the gate, Sentrux, one commit, TODO 3b checked, and `--screenshot-composed` captures shown
to the owner with a "what you will see / what does not work yet" line.

## After 3b
Step 4 (sim: save schema 5 with gold in copper, `Mode::Town`, services), then 5–9 per the TODO.
Owed by the owner at acceptance a: `--frame-stats` with a panel open. Dated: Socket re-audit of
`rhai` 1.26.1 on 2026-10-10. Merged local branches `m5-tasks`, `m6c-tasks`,
`m6-closeout-tasks` can be deleted by the owner.
