# Continuity notes

Written 2026-09-12 after the two M1 follow-ups, before M2. Rewrite this file every time it is used.

## Where we are
- Omnis: turn-based first-person grid-crawler RPG, Rust, Bevy 0.19.1 (owner exempts Bevy from the N-1 rule). World is Toel (`docs/background/introduction.md`). `PRD.md` v0.3 and `ARCHITECTURE.md` v0.2 are the source of truth; build order follows `tasks/TODO.md` milestones.
- M1 merged to main as PR #2 (`ad975fd`). Branch `m2-tasks` carries the M1 follow-ups: `7e28d02` Block/DoorFrame schema, loader, bake; `bf66df8` pack data, dungeon detail 6; `9981f63` planner; then the docs commit. Never push; the owner opens PRs.
- **Both M1 follow-ups are done** (TODO, M1 review): pillars and tree clumps draw as blocks, open doors as frames, the dungeon draws six sprite rows at width 5 with visibility 6 (owner's call), and the horizon band has a ceiling strip. Verified by canvas capture, not yet walked through by the owner.
- **Next: M2 "Live instrumentation"** in `tasks/TODO.md`: `omnis-cli` subcommands (`validate`, `schema dump`, `map text`, `play --script`, `replay`) and `omnis_cli::Headless`; `DevSocketPlugin` in the app (grow it from `crates/omnis-app/src/dev.rs`); `omnis-mcp` bridge with the dual-era handshake; `.mcp.json`. Re-read ARCH §9, §10 and TODO M2 before starting. Plan is approved; report per checked item.

## Decisions and facts easy to get wrong after a compact
- Crate deps are exactly: app → sim + bevy (data and core reach the app via `omnis_sim::omnis_data` / `omnis_sim::omnis_core` re-exports); cli → sim, data, core, png, serde; mcp → cli, serde_json. New crates go in `Cargo.toml` members (explicit list) and `scripts/lint-sim.sh` `SIM_CRATES` in the same change.
- Maps are a text grid: `2h+1` rows of `2w+1` chars, odd positions tiles, even positions edges (`-`/`|` wall, `=`/`:` door, space open); boundary must be walls. Loader collects every error; `tests/packs-bad/` is the corpus and its test asserts the full message list.
- Slot kinds: `Floor`, `Ceiling`, `WallFront`, `WallLeft`, `WallRight`, `Door`, `DoorFrame`, `Block`, `Object`, `Monster`. `Terrain.block: Option<String>` and `MapDef.door_open: Option<String>` are optional. Block slots exist for depth ≥ 1 only; a block's near face is at distance `d` (drawn last within its row), a door frame is the front face with an opening cut out (jambs and lintel ⅛ tile). In `packs/test` the rock and tree terrains stand on a plain floor surface with the block over it.
- Any edit to `packs/test` changes the pack fingerprint and therefore the golden replay: re-baseline with `cargo test -p omnis-sim rebaseline -- --ignored` in the same commit. Tileset files under `packs/test/data/tiles/` are generated: edit `packs/test/bake/*.ron` and run `cargo run -p omnis-cli -- tileset bake packs/test/bake/dungeon.ron packs/test/bake/outdoor.ron`; a test fails if the committed file drifts from a fresh bake. Dungeon: detail 6, width 5, 257 sprites; outdoor: detail 4, width 3, 131 sprites.
- Visibility (`crates/omnis-sim/src/visibility.rs`): integer supercover rays in half-tile coordinates to the target's centre and four corners, both orders at exact corner ties; opaque terrain seen but not seen through; edges checked at every crossing. Cone offsets are `-d..=d`. Dungeon floor visibility 6, meadow grass 12.
- Viewport geometry (bake and renderer agree): eye at the near edge of the party's tile, half a tile up, focal `0.9 × viewport height` (121.5 px); a tile at depth `d` has its far edge at distance `d+1`; a face at distance `z` is `121.5/z` px tall. Canvas 320×180 (`crates/omnis-app/src/layout.rs`): viewport 240×135 top-left with the backdrop as a fill inside it, panel colour (24,24,34) elsewhere; sidebar minimap rect (248,8,64,64) at 2 px/tile (`plan::automap_window`); large overlay at (8,8) with 4 px tiles on M. HUD text is `bevy_ui` at right 1.5%, top 42%.
- Window: `WindowResolution::new` is a logical size; the owner's display is 1× so 3840×2160 is a 4K window on the 8K screen. Bevy's window screenshot and macOS `screencapture` return black on this machine; `--screenshot-canvas` is the trusted capture.
- Unattended checks: `cargo run -q -p omnis-app -- --seed 1 --window 1280x720 --script forward,turn-left,use,map --screenshot-canvas file.png --settle 40`; steps are `forward back left right turn-left turn-right around use map save load`. From the meadow start, eleven `forward`s reach the dungeon at (1,0) facing South; the first room's pillar is (3,3), its south door (3,5), the second room's south door (9,5).
- Sentrux: scan `crates/`; rules in `crates/.sentrux/rules.toml` (local, gitignored); `max_fn_lines 100` bites integration tests too (split long tests).
- Editing Rust with `sed` breaks on `|` closures and on rustfmt's rewrapping: use a Python replace with an assert, after `cargo fmt`, matching the formatted text.

## Verified facts worth not re-researching
- Bevy 0.19.1 API (from sources, 2026-09-12): `RenderTarget` is a component beside `Camera`; `Anchor::TOP_LEFT` is a separate component; `TextFont::from_font_size(f32)`; `MessageWriter::write`/`MessageReader::read`; `StatesPlugin` and `InputPlugin` are not in `MinimalPlugins`; `register_asset_source` must precede `DefaultPlugins`; `Image::new_target_texture(w, h, format, None)` makes a render target; `Screenshot::image(handle)` + `save_to_disk`; `LoadState::Failed(Arc<_>)`.
- `png` 0.18.1 is in Bevy's tree (no new duplicate); `Transformations::ALPHA` expands palettes to RGBA.
- Earlier facts (feature groups, toolchain 1.98.0, action SHAs, no `gh`, `json.loads(strict=False)`) are in ARCH §5, §9, §13, §14 and git history.

## Working agreements observed
- Owner wants numbers before opinions; both directions of any hybrid; v1 choices are horizons (`LESSONS.md`).
- Verification runs only after the last tree change; fresh `cargo metadata` when workspace contents change. Commit per checked item with the attribution trailer; stage files by name, never `add -A`. A commit must build on its own: when a schema change breaks a match elsewhere, the two land together.
- Ask before changing vision documents on drift; factual notes added to ARCH §3, §4.2, §8.3, §13 are listed in the docs commits for the owner to review.

## Open items
- Later art: horizon silhouettes as sprites, object and monster slots (M4), CC0 monster/portrait art (owner, before M4). The block's hidden side and the door's swung leaf are not drawn; hand-drawn slots can replace any baked sprite by path.
- Sidebar minimap shows whole maps up to 32×32; bigger maps scroll. Nothing yet uses the bottom band of the canvas except the `bevy_ui` message lines.
- PRD §14 component economy and non-goal horizons still open; resolution 320×180 provisional.
- README.md rewrite when asked; Linux/Windows CI later; `.cargo/config.toml` when a fast linker exists; Socket re-audit of rhai 1.26.1 due 2026-10-10; file the Krea hallway PNGs under a set folder when first used.
