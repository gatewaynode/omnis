# TODO

## PRD (2026-09-11)
- [x] Review skeleton files and reference docs
- [x] Resolve the eight foundational decisions with the owner (PRD §6)
- [x] Extract MM2 mechanical structure from the C64 manual (PRD §8.1)
- [x] Verify Bevy 0.19 facts (PRD §11.1)
- [x] Draft PRD v0.1
- [x] Owner review of PRD v0.1; rules inverted to SRD spine (D1), decisions D10–D19 recorded
- [x] Resolve §14 open questions (three remain, none blocking)
- [x] Derive ARCHITECTURE.md from the PRD (v0.2, owner-reviewed 2026-09-12)
- [ ] Fix README.md typos and align it with the PRD vision paragraph

## Architecture (2026-09-11)
- [x] Decide MCP approach: own minimal MCP, stdio bridge binary to a game localhost socket
- [x] Decide rules expression: tiny in-house language first, revised to Rhai in review (A4)
- [x] Research: MCP spec and transports; Bevy 0.19 headless, assets, UI, structure; dependency versions
- [x] Decide editor UI toolkit with owner: bevy_egui for the editor, bevy_ui for the game
- [x] Write ARCHITECTURE.md v0.1
- [x] Owner review round 1: A4 changed to Rhai, A11 confirmed; §5 rewritten as Rhai host; §4.4 subjective time; A14 RNG seed strategy proposed
- [x] rhai pinned at 1.26.1 by owner exception (re-audit 2026-10-10)
- [x] A14 seed strategy approved
- [x] ARCHITECTURE.md v0.2 reviewed by owner
- [ ] Plan Phase 0 tasks (after compact)
- [ ] Sentrux review once code exists

## Delivery plan (2026-09-12, awaiting owner approval)

Owner direction: reach a user-testable app as soon as feasible, then widen across the full scope while there is something to test. So: one vertical slice first (M0–M2), then each later milestone adds one system end to end (data schema → sim → app UI → MCP tool → CLI → tests) so every milestone leaves a build the owner can play. This reorders PRD §13 (Phase 0 there has no app); see the check-in questions at the end.

Standing rules for every milestone: tests on real `packs/test` data, no mocks; sim-crate lint green (CLAUDE.md); Sentrux review; `LESSONS.md` updated after any correction; one commit per checked item or small group; MCP tools and CLI subcommands ship with the system they expose, never later.

### M0 — Skeleton that builds and lints (done 2026-09-12)
- [x] Workspace `Cargo.toml`: `rust-version = "1.95"`, edition 2024, exact pins (bevy 0.19.1, serde 1.0.228, serde_json 1.0.150, ron 0.12.2, thiserror 2.0.19, rhai 1.26.1, bevy_egui 0.41.1), profiles from ARCH §14, `rust-toolchain.toml` at 1.98.0
- [x] Crates: `omnis-core`, `omnis-data`, `omnis-sim`, `omnis-app`, `omnis-cli`, `omnis-mcp` as compiling stubs; `core` and `sim` are `#![no_std]`
- [x] `scripts/lint-sim.sh` with `--self-test`; denies floats, hashed collections, time/thread/net, file I/O outside `omnis-data`, Bevy edges in `cargo tree`, unsafe Rhai features; requires `#![no_std]` and `deny(clippy::float_arithmetic)`
- [x] `scripts/check-duplicates.sh` with a committed allow list of Bevy's own duplicate crates; fails on any new duplicate
- [x] CI (GitHub Actions, macOS + Linux, actions pinned by commit SHA): fmt, lint-sim self-test and lint, duplicates, clippy `-D warnings`, tests, fingerprint artifact and cross-platform compare job (populated in M1)
- [x] `LICENSE-MIT`, `LICENSE-APACHE`, `ATTRIBUTION.md`, `.gitignore`
- [x] `assets/` folder convention agreed and applied (`assets/README.md`)
- [x] `packs/test/pack.ron` manifest
- [x] Sentrux rules in `crates/.sentrux/rules.toml` (layers and boundaries from ARCH §3); baseline signal 10000 on `crates/`
- **Done**: `cargo build`, clippy, tests, lint, and duplicate check green locally on macOS; lint self-test catches every planted violation. CI on GitHub runs on first push.

#### M0 review
- Six simulation crates are `#![no_std]` so the compiler, not only the lint, rules out `HashMap`, `std::time`, `std::thread`, `std::fs`, and `std::net`. `omnis-data` (file I/O) and `omnis-expr` (Rhai) stay `std` and rely on the lint. Every simulation crate also carries `#![deny(clippy::float_arithmetic)]` to catch unsuffixed float literals the grep cannot see.
- "Single-copy tree" is enforced relative to Bevy: Bevy 0.19.1's own tree already contains 30 duplicated crates (`thiserror` 1 via `calloop` in the Wayland stack, `bitflags`, `syn`, `windows-sys`, and so on). The allow list records them; any duplicate we introduce fails CI.
- No `.cargo/config.toml` yet: no `lld` or `mold` is installed here and Apple's linker is already fast. Add one when a Linux developer or CI needs it.
- CI could not be run from this machine (`gh` is not installed and pushing is the owner's call). The workflow uses `rustup toolchain install` from `rust-toolchain.toml` and two third-party actions (`checkout`, `rust-cache`) plus the artifact pair, all pinned by SHA and older than 30 days.
- Sentrux has no ignore setting, so it is scanned on `crates/` (the repo root scan counted the SRD markdown). Its free tier checks 5 of the 18 rules defined.
- `assets/simple-hallway*.png` (two photographic 3D renders) arrived during M0 with no licence file; they are untracked until the owner states their source and terms.

### M1 — Walkable: first user-testable build
- [ ] `omnis-core`: typed ID newtypes, `Fixed` (i64, 1/1000), `Pcg32` with named streams (`fnv1a64`, `splitmix64`, `RollTrace`), `Direction`, `Rotation`, `Position`, `Clock`, error type. Tests: PCG32 known-answer vectors, stream derivation golden values, `Fixed` arithmetic edge cases
- [ ] `omnis-data`: `pack.ron` manifest, `tiles/*.ron` (tileset with `detail_depth`, `width`, per-depth slot sprite paths), `maps/*.ron` (terrain, wall masks, doors, start tile, per-tile visibility depth), `text/<lang>`, `Registry` with ID interning, loader that collects every error, path normalization and size limits from ARCH §6.2, RON read and write. Tests: `packs/test` loads; `tests/packs-bad/` corpus rejected with expected error lists; write→read round trip
- [ ] `omnis-sim`: `World` (schema, packs, rngs, clocks with the party holder only, position, maps, automap, mode `Explore`, flags, settings), `Command::{Step, Turn, Interact}`, `Event::{Moved, Blocked, Visible, TimeAdvanced}`, `query::viewport` with detail-depth cut and visibility-depth cone, `query::automap`, `query::path`, save/load RON, `World::fingerprint`, `World::log`. Tests: movement and wall blocking on the test map, doors, automap fill, save round trip, one recorded replay under `tests/replays/` with a golden fingerprint
- [ ] `omnis-app`: pixel pipeline (`pixel_grid_snap` pattern, `default_nearest`, internal resolution placeholder), `PackAssetPlugin` (images by pack-relative path, magenta placeholder for missing), `SimPlugin` (`SimWorld` resource, ordered `collect → apply → publish` set, events as `Message`s), `InputPlugin` (arrows/WASD/QE, F5 save, F9 load), `ViewportPlugin` (detail rows from tileset slots, procedural horizon band), minimal `HudPlugin` (position, facing, clock, last message), automap overlay toggle, states `Boot → Playing::Explore` (main menu comes with M3). Smoke test: `MinimalPlugins` + `run_once`, boot, step, no panic
- [ ] `omnis-cli tileset bake`: generate per-depth viewport slot sprites from 16×16 textures (checked into `packs/test/assets/` as ordinary PNGs so the game never bakes at runtime)
- [ ] `packs/test`: one 24×24 dungeon level with doors and a 32×32 outdoor patch (visibility depth 4 vs 12), one tileset baked from the OpenRTP dungeon and exterior sheets
- **Done when**: the owner runs the app, walks both maps in first person, opens a door, watches the automap fill, saves and reloads, and the replay fingerprint matches on macOS and Linux CI

### M2 — Live instrumentation: MCP and CLI
- [ ] `omnis-cli`: `validate`, `schema dump`, `map text`, `play --script`, `replay` (asserts fingerprint), `omnis_cli::Headless` library; CI switches the replay job to it
- [ ] `omnis-app` feature `devtools`: `DevSocketPlugin`, non-blocking loopback listener polled per frame, `.omnis/dev.addr`, newline JSON protocol, input validation per ARCH §6.2, ops `game.status`, `world.query`, `sim.command`, `sim.script`, `events.tail`, `viewport.get`, `map.text`, `automap.get`, `save.write`, `save.read`, `pack.reload`, `screenshot`
- [ ] `omnis-mcp`: JSON-RPC 2.0 over stdio, dual-era handshake (`initialize` and `server/discover`), `tools/list` with `inputSchema` built from the Rust argument types by our own small schema builder (no `schemars`, self-supporting rule), `tools/call` translation, `--headless [pack...]` through `Headless`, logs to stderr only
- [ ] `.mcp.json` registration; verify from this Claude Code session which handshake era it speaks and record the answer in ARCH §9.2
- [ ] Tests: protocol round trip against a headless instance; malformed-line and oversize-line rejection; bridge handshake for both eras
- **Done when**: from this session I can list tools, step the running game, read the viewport as text, and receive a screenshot; the same tools work headless in CI

### M3 — Party and characters (SRD structure)
- [ ] `omnis-expr`: Rhai host with the Formula profile, `disable_symbol` list, limits, fixed hash seed, `d(n, sides)` bound to a named stream, slot compile and validate, hot swap. Tests: sandbox escapes rejected, limits enforced, determinism across two engines
- [ ] `omnis-data` schemas: races, classes, backgrounds, items, conditions, spells (point cost + component list), monsters; `rules/*.ron` slots; `packs/base` seeded with a small SRD subset and its attribution
- [ ] `omnis-rules`: abilities and modifiers, proficiency, HP, AC, checks and saves, spell point pool (D12 formula in `casting.ron`), leveling thresholds
- [ ] `omnis-sim`: `Party` with six slots, `PartyCommand::{Create, Reorder}`, difficulty settings, new game with world seed
- [ ] `omnis-app`: main menu (New Game with seed text, Load), character creation in `bevy_ui`, party panel in the HUD
- [ ] MCP: `party.get`, `party.create`, `rules.list`, `rules.get`, `rules.set`
- **Done when**: the owner creates a six-member party from base data and walks the test dungeon with it; changing the spell point formula through `rules.set` changes the pool without a rebuild

### M4 — Combat
- [ ] Monsters as stacks with rows, fixed and random encounters keyed to map tiles, pre-combat choice (attack, bribe, hide, run) with disposition, surprise, initiative, attack and damage with `RollTrace`, conditions, death, XP and loot, `Combat` sub-state UI with the roll math visible
- **Done when**: the owner clears a fixed encounter in the test dungeon and a replay of the fight reproduces its fingerprint

### M5 — Editor v1 (`bevy_egui`)
- [ ] `Editor` app state: tile paint (terrain, edges, doors, visibility depth), objects and triggers, encounter placement, data tables, text keys, playtest at cursor; writer is the loader's code path
- **Done when**: the M1 test maps are re-authored in the editor and every later map is built with it (D4)

### M6 — Spells, items, and remote sensing
- [ ] Casting in and out of combat, spell points and components (D11 threshold from config), cantrips, item use and equipment, `Sense` with a spyglass item feeding the automap as stale knowledge (D18)
- **Done when**: a caster empties a spell pool in combat and a spyglass reveals tiles that the automap marks as remotely seen

### M7 — Town, services, rest, and progression
- [ ] Town map with inn (rest, food, ambush), temple, trainer (level up), smith, tavern, bank, guild (spells for sale); overnight rest advances the party clock; MM2 leveling loop (D-table in PRD §8.2)
- **Done when**: the Phase 1 loop from PRD §13 is closed: create a party, clear the dungeon, return to town, level up

### M8 — Subjective time
- [ ] Clocks for every holder, `Contact` records, reconciliation rule in Rhai on region entry with bounded drift and the `time:<a>:<b>` stream, calendar display, rumor text that reports the teller's own elapsed time; MCP `time.clocks`, `time.reconcile`
- **Done when**: two regions visited in different orders produce different but replayable clock deltas, and PRD §7.8 examples are reproduced as tests

### M9–M12 — PRD Phases 2–5 (expand when reached)
- [ ] M9 procedural world (`omnis-gen`, lazy materialization, editor procgen panel, golden fingerprints)
- [ ] M10 ecosystem (`omnis-eco`, region tick, catch-up, derived outputs, `eco.*` tools)
- [ ] M11 story engine (`omnis-story`, quest graphs, templates, static check, `story.*` tools)
- [ ] M12 base campaign, audio, packaging, mod documentation, licensing audit, Windows CI

### Check-in resolved 2026-09-12
1. Ordering approved. PRD §13 and ARCH §16 keep the long-range phases with a note that build order follows this file; revisit them once the milestones settle.
2. `assets/` convention approved and applied: raw sets per folder with licence text, `assets/private/` gitignored for non-redistributable art, `assets/README.md` is the register. Pack asset search path: pack `assets/` first, repo `assets/` second.
3. Asset review: OpenRTP tiles are CC0, 16×16, top-down chipsets (world, exterior, interior, dungeon, ship), no characters or monsters. Time Fantasy icons are a paid pack whose licence forbids raw redistribution, so they stay local under `assets/private/` and never ship in a pack. Internal resolution set to **320×180, detail depth 4** (integer scales 4×, 6×, 8×, 12× land exactly on 720p, 1080p, 1440p, and 4K; 480×270 has no integer fit on 1440p). Recorded in PRD §14.

Consequences folded into the milestones:
- M1 gains a bake step: the first-person wall, floor, and ceiling slots per depth are generated from 16×16 textures by an `omnis-cli tileset bake` subcommand (front walls tiled and scaled per depth, side walls sheared into trapezoids), because no first-person crawler art exists in either set. The tileset RON still declares the slots, so hand-drawn art can replace baked slots later without a code change.
- Automap uses the OpenRTP tiles directly at 16 px.
- M3 UI uses the icons from `assets/private/` when present and the magenta placeholder otherwise, so a clean clone runs.
- Gap: no character portraits or monster sprites. Owner to source a CC0 set before M4; until then M4 uses generated silhouettes.

## Phase 0 — Foundation (superseded by M0–M2 above; PRD §13 criteria are met by M1 replay fingerprint and M2 headless `game.status`)
