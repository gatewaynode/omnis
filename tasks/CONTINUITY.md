# Continuity notes

Written 2026-09-12 at the end of M1, before the owner's walk-through. Rewrite this file every time it is used.

## Where we are
- Omnis: turn-based first-person grid-crawler RPG, Rust, Bevy 0.19.1 (owner exempts Bevy from the N-1 rule). World is Toel (`docs/background/introduction.md`). `PRD.md` v0.3 and `ARCHITECTURE.md` v0.2 are the source of truth; build order follows `tasks/TODO.md` milestones.
- Branch `m1-tasks` (from main after PR #1). M1 commits: `8d4bbe3` core, `27b0a8b`+`3b481cf` data, `95da8bf` sim, `56190fe` bake, `d691d0b` planner, `895c47c` app, then the docs commit. Never push; the owner opens PRs.
- **M1 "Walkable" is code-complete**; every TODO item is checked and the review section in `tasks/TODO.md` lists what was verified and what the owner must confirm by running `cargo run -p omnis-app`. Acceptance is the owner's walk-through.
- **Next: M2 "Live instrumentation"** in `tasks/TODO.md`: `omnis-cli` subcommands (`validate`, `schema dump`, `map text`, `play --script`, `replay`) and `omnis_cli::Headless`; `DevSocketPlugin` in the app (grow it from `crates/omnis-app/src/dev.rs`); `omnis-mcp` bridge with the dual-era handshake; `.mcp.json`. Re-read ARCH §9, §10 and TODO M2 before starting. Plan is approved; report per checked item.

## Decisions and facts easy to get wrong after a compact
- Crate deps are exactly: app → sim + bevy (data and core reach the app via `omnis_sim::omnis_data` / `omnis_sim::omnis_core` re-exports); cli → sim, data, core, png, serde; mcp → cli, serde_json. New crates go in `Cargo.toml` members (explicit list) and `scripts/lint-sim.sh` `SIM_CRATES` in the same change.
- Maps are a text grid: `2h+1` rows of `2w+1` chars, odd positions tiles, even positions edges (`-`/`|` wall, `=`/`:` door, space open); boundary must be walls. Loader collects every error; `tests/packs-bad/` is the corpus and its test asserts the full message list.
- Any edit to `packs/test` changes the pack fingerprint and therefore the golden replay: re-baseline with `cargo test -p omnis-sim rebaseline -- --ignored` in the same commit. Tileset files under `packs/test/data/tiles/` are generated: edit `packs/test/bake/*.ron` and run `cargo run -p omnis-cli -- tileset bake packs/test/bake/dungeon.ron packs/test/bake/outdoor.ron`; a test fails if the committed file drifts from a fresh bake.
- Visibility (`crates/omnis-sim/src/visibility.rs`): integer supercover rays in half-tile coordinates to the target's centre and four corners, both orders at exact corner ties; opaque terrain seen but not seen through; edges checked at every crossing. Cone offsets are `-d..=d`; the tileset `width` clamps only what the renderer draws.
- Viewport geometry (bake and renderer agree): eye at the near edge of the party's tile, half a tile up, focal `0.9 × viewport height`; tile at depth `d` has its far edge at distance `d+1`. Canvas 320×180, viewport 240×135 at the top-left, automap overlay at (8,8) with 4 px tiles.
- Screenshot verification: `omnis --script ... --screenshot-canvas file.png` works; `--screenshot` (window) and macOS `screencapture` return black on this machine, so never trust a black window capture as evidence of a bug.
- Sentrux: scan `crates/`; rules in `crates/.sentrux/rules.toml` (local, gitignored); `max_fn_lines 100` bites integration tests too (split long tests). M1 end signal 6508.
- Editing Rust with `sed` breaks on `|` closures and on rustfmt's rewrapping: use a Python replace with an assert, after `cargo fmt`, matching the formatted text.

## Verified facts worth not re-researching
- Bevy 0.19.1 API (from sources, 2026-09-12): `RenderTarget` is a component beside `Camera`; `Anchor::TOP_LEFT` is a separate component; `TextFont::from_font_size(f32)`; `MessageWriter::write`/`MessageReader::read`; `StatesPlugin` and `InputPlugin` are not in `MinimalPlugins`; `register_asset_source` must precede `DefaultPlugins`; `Image::new_target_texture(w, h, format, None)` makes a render target; `Screenshot::image(handle)` + `save_to_disk`; `LoadState::Failed(Arc<_>)`.
- `png` 0.18.1 is in Bevy's tree (no new duplicate); `Transformations::ALPHA` expands palettes to RGBA.
- Earlier facts (feature groups, toolchain 1.98.0, action SHAs, no `gh`, `json.loads(strict=False)`) are in ARCH §5, §9, §13, §14 and git history.

## Working agreements observed
- Owner wants numbers before opinions; both directions of any hybrid; v1 choices are horizons (`LESSONS.md`).
- Verification runs only after the last tree change; fresh `cargo metadata` when workspace contents change. Commit per checked item with the attribution trailer; stage files by name, never `add -A`.
- Ask before changing vision documents on drift; small factual notes were added to ARCH §3, §4.2, §8.3, §13 at M1 end and are listed in the docs commit for the owner to review.

## Open items
- Owner acceptance of M1 (walk both maps, door, automap, F5/F9). HUD text and window scaling unverified by capture.
- Placeholders to revisit: opaque tiles as blocks, open-door art, horizon silhouettes, first-person sprites for objects and monsters (M4), CC0 monster/portrait art (owner, before M4).
- PRD §14 component economy and non-goal horizons still open; resolution 320×180 provisional.
- README.md rewrite when asked; Linux/Windows CI later; `.cargo/config.toml` when a fast linker exists; Socket re-audit of rhai 1.26.1 due 2026-10-10; file the Krea hallway PNGs under a set folder when first used.
