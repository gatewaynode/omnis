# Continuity notes

Written 2026-09-12 before a compact, at the end of M0. Rewrite this file every time it is used.

## Where we are
- Omnis: turn-based first-person grid-crawler RPG, Rust, Bevy 0.19.1 (owner exempts Bevy from the N-1 rule). World is Toel (`docs/background/introduction.md`).
- `PRD.md` v0.3 and `ARCHITECTURE.md` v0.2 are owner-reviewed and remain the source of truth; both carry a note that build order follows the milestones in `tasks/TODO.md`, not their phase tables.
- Branch `task-planning-1` (owner opened a PR from it; remote is `origin`). Commits since main: docs tidy, delivery plan + asset reorg, M0, CI fix, CI macOS-only. Tree clean.
- **M0 is done**: workspace with exact pins, six stub crates (`core`, `data`, `sim`, `app`, `cli`, `mcp`), `scripts/lint-sim.sh` (with `--self-test`), `scripts/check-duplicates.sh` + allow list, CI on macOS, licences, `ATTRIBUTION.md`, `packs/test/pack.ron`. All checks green locally after the final edit.
- **Next: M1 "Walkable"** in `tasks/TODO.md`. Start with `omnis-core` (typed IDs, `Fixed`, PCG32 named streams with FNV-1a + splitmix64, `RollTrace`, Direction/Rotation/Position/Clock), then `omnis-data` loader, `omnis-sim` movement/viewport/automap/save/replay, `omnis-cli tileset bake`, `omnis-app` pixel pipeline and viewport, `packs/test` maps. Re-read TODO M1 items and ARCH §4, §6, §8 before coding. Check in with the owner is not needed again for M1 (plan approved), but report at each checked item.

## Decisions and facts easy to get wrong after a compact
- Rules: SRD 5.1 is the spine; MM2 adaptations only where they serve the crawl loop. Party six slots, hirelings fill open slots (D10). Spell points: level × casting mod + other two mental mods, half casters half level, floor level (D12). Components list on spells, hard from spell level 5 by config (D11).
- Time: no global clock; per-holder `Clock`, `Contact` records, partial reconciliation on contact; single era in v1 (D20/A13). M1 has only the party clock.
- Viewport: detail depth 4 as sprites, visibility depth up to 20 as a procedural horizon band; internal resolution **320×180** (provisional, PRD §14).
- Scripting: Rhai in `omnis-expr` (M3), Formula profile flags in ARCH §5.2; never `unchecked`. Pinned 1.26.1 by owner exception, Socket re-audit due 2026-10-10.
- RNG (A14): one world seed; stream state = splitmix64(seed ^ fnv1a64(name)), increment = splitmix64(fnv1a64(name)) | 1; stateful streams persisted in `World.rngs`; `gen:*` streams stateless.
- MCP (A3): own stdio bridge, dual-era handshake, newline JSON loopback socket, `devtools` feature (default on, release uses `--no-default-features`), headless via `omnis_cli::Headless`. Tool JSON schemas from our own builder, no `schemars`.
- Simulation crate rules (CLAUDE.md section): eight crates; `core`, `rules`, `gen`, `eco`, `story`, `sim` are `#![no_std]`; every sim crate carries `#![deny(clippy::float_arithmetic)]`; only `omnis-data` does file I/O. A new crate goes in `Cargo.toml` members (explicit list, no glob) and `SIM_CRATES` in `scripts/lint-sim.sh` in the same change.
- Dependency policy: match Bevy's resolved versions; `check-duplicates.sh` allow list holds Bevy's own 30 duplicates; `--update` rewrites it only for an intentional change.
- Assets: `assets/openrtp-tiles/` is CC0 and committed (16×16 top-down chipsets, no monsters or characters). `assets/private/time-fantasy-icons/` is paid, gitignored, never shipped in a pack. `assets/simple-hallway*.png` are the owner's Krea-generated 3D renders, committed by the owner; file under a set folder when first used. No first-person crawler art exists: M1 bakes per-depth slots from 16×16 textures via `omnis-cli tileset bake`.
- CI: macOS only per owner (2026-09-12). Restore the Linux matrix, apt packages, fingerprint artifacts, and the compare job when asked; the workflow header lists them.
- Sentrux: no ignore setting; scan `crates/` (repo root counts the SRD markdown). Rules live in `crates/.sentrux/rules.toml`, gitignored per owner; free tier checks 5 of 18 rules. Baseline signal 10000 on `crates/` at M0 end.

## Verified facts worth not re-researching
- Bevy 0.19.1 features: `2d` and `ui` expand to `default_app` (asset, log, state, async_executor, reflect_auto_register) and `default_platform` (std, winit, x11, wayland, gilrs, multi_threaded, webgl2, default_font); `png` is separate; `3d` never enabled. crates.io checksum 4bfadbeb…df8f.
- Toolchain: rustc 1.98.0 locally, `rust-toolchain.toml` pins 1.98.0, MSRV 1.95. No lld/mold installed; no `.cargo/config.toml` yet.
- Bevy's tree duplicates `thiserror` 1 (via `calloop` in the Wayland stack), `bitflags`, `syn`, `windows-sys`, and 26 others; recorded in `scripts/duplicates.allow`.
- GitHub Actions pinned by SHA (all >30 days old): checkout v7.0.1 3d3c42e…, rust-cache v2.9.2 6323deb…; upload/download-artifact SHAs are in git history (`c436158`) if needed again.
- `gh` is not installed on this machine; use the REST API with `json.loads(strict=False)` (release notes contain control characters).
- Earlier facts (Bevy API names, MCP spec 2026-07-28, Rhai 1.26.1 details, policy pins with Socket audits) are in ARCH §5, §9, §13 and the previous continuity notes are superseded by those sections.

## Working agreements observed
- Owner wants numbers before opinions; both directions of any hybrid; v1 choices are horizons (see `LESSONS.md`, four entries).
- Verification runs only after the last tree change; a fresh `cargo metadata` first when workspace contents change (LESSONS 2026-09-12, from the CI failure).
- Commit per checked item or small group with the attribution trailer; never push (owner pushes and opens PRs). Never stage with `add -A` when unreviewed files may be present.
- Ask before updating vision documents on drift; owner chose to keep PRD §13 / ARCH §16 and annotate.
- Sentrux review after each task; `session_end` compares against the M0 baseline.

## Open items
- PRD §14: component economy; non-goal horizons (browser, 3D, LLM) still unanswered; resolution and depth are provisional.
- README.md rewrite from the PRD vision paragraph when asked (typos, predates decisions).
- Windows CI and Linux CI later; `.cargo/config.toml` when a fast linker is available.
- Monster and portrait art: owner to source a CC0 set before M4.
