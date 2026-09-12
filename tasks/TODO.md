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

## Delivery plan (2026-09-12, approved)

Owner direction: reach a user-testable app as soon as feasible, then widen across the full scope while there is something to test. So: one vertical slice first (M0–M2), then each later milestone adds one system end to end (data schema → sim → app UI → MCP tool → CLI → tests) so every milestone leaves a build the owner can play. This reorders PRD §13 (Phase 0 there has no app); see the check-in questions at the end.

Standing rules for every milestone: tests on real `packs/test` data, no mocks; sim-crate lint green (CLAUDE.md); Sentrux review; `LESSONS.md` updated after any correction; one commit per checked item or small group; MCP tools and CLI subcommands ship with the system they expose, never later.

### M0 — Skeleton that builds and lints (done 2026-09-12)
- [x] Workspace `Cargo.toml`: `rust-version = "1.95"`, edition 2024, exact pins (bevy 0.19.1, serde 1.0.228, serde_json 1.0.150, ron 0.12.2, thiserror 2.0.19, rhai 1.26.1, bevy_egui 0.41.1), profiles from ARCH §14, `rust-toolchain.toml` at 1.98.0
- [x] Crates: `omnis-core`, `omnis-data`, `omnis-sim`, `omnis-app`, `omnis-cli`, `omnis-mcp` as compiling stubs; `core` and `sim` are `#![no_std]`
- [x] `scripts/lint-sim.sh` with `--self-test`; denies floats, hashed collections, time/thread/net, file I/O outside `omnis-data`, Bevy edges in `cargo tree`, unsafe Rhai features; requires `#![no_std]` and `deny(clippy::float_arithmetic)`
- [x] `scripts/check-duplicates.sh` with a committed allow list of Bevy's own duplicate crates; fails on any new duplicate
- [x] CI (GitHub Actions, macOS only per owner 2026-09-12, actions pinned by commit SHA): fmt, lint-sim self-test and lint, duplicates, clippy `-D warnings`, tests. Linux runner and the cross-platform fingerprint compare return later.
- [x] `LICENSE-MIT`, `LICENSE-APACHE`, `ATTRIBUTION.md`, `.gitignore`
- [x] `assets/` folder convention agreed and applied (`assets/README.md`)
- [x] `packs/test/pack.ron` manifest
- [x] Sentrux rules in `crates/.sentrux/rules.toml` (layers and boundaries from ARCH §3), kept local and gitignored per owner; baseline signal 10000 on `crates/`
- **Done**: `cargo build`, clippy, tests, lint, and duplicate check green locally on macOS; lint self-test catches every planted violation. CI on GitHub runs on first push.

#### M0 review
- Six simulation crates are `#![no_std]` so the compiler, not only the lint, rules out `HashMap`, `std::time`, `std::thread`, `std::fs`, and `std::net`. `omnis-data` (file I/O) and `omnis-expr` (Rhai) stay `std` and rely on the lint. Every simulation crate also carries `#![deny(clippy::float_arithmetic)]` to catch unsuffixed float literals the grep cannot see.
- "Single-copy tree" is enforced relative to Bevy: Bevy 0.19.1's own tree already contains 30 duplicated crates (`thiserror` 1 via `calloop` in the Wayland stack, `bitflags`, `syn`, `windows-sys`, and so on). The allow list records them; any duplicate we introduce fails CI.
- No `.cargo/config.toml` yet: no `lld` or `mold` is installed here and Apple's linker is already fast. Add one when a Linux developer or CI needs it.
- CI could not be run from this machine (`gh` is not installed and pushing is the owner's call). The workflow uses `rustup toolchain install` from `rust-toolchain.toml` and two third-party actions (`checkout`, `rust-cache`) plus the artifact pair, all pinned by SHA and older than 30 days.
- Sentrux has no ignore setting, so it is scanned on `crates/` (the repo root scan counted the SRD markdown). Its free tier checks 5 of the 18 rules defined.
- CI failure on the first PR: the `crates/*` member glob matched `crates/.sentrux`. Fixed with an explicit member list (a new crate is now added deliberately, alongside the lint list) and the sentrux directory is gitignored. Root cause of the missed local check is in `LESSONS.md`.
- `assets/simple-hallway*.png` (two photographic 3D renders) arrived during M0 with no licence file; they are untracked until the owner states their source and terms.

### M1 — Walkable: first user-testable build
- [x] `omnis-core` (2026-09-12): typed ID newtypes, `Fixed` (i64, 1/1000), `Pcg32` with named streams (`fnv1a64`, `splitmix64`, `RollTrace`), `Direction`, `Rotation`, `Position`, `Clock`, error type. Tests: PCG32 known-answer vectors, stream derivation golden values, `Fixed` arithmetic edge cases
- [x] `omnis-data` (2026-09-12): `pack.ron` manifest, `tiles/*.ron` (tileset with `detail_depth`, `width`, per-depth slot sprite paths), `maps/*.ron` (terrain, wall masks, doors, start tile, per-tile visibility depth), `text/<lang>`, `Registry` with ID interning, loader that collects every error, path normalization and size limits from ARCH §6.2, RON read and write. Tests: `packs/test` loads; `tests/packs-bad/` corpus rejected with expected error lists; write→read round trip
- [x] `omnis-sim` (2026-09-12): `World` (schema, packs, rngs, clocks with the party holder only, position, maps, automap, mode `Explore`, flags, settings), `Command::{Step, Turn, Interact}`, `Event::{Moved, Blocked, Visible, TimeAdvanced}`, `query::viewport` with detail-depth cut and visibility-depth cone, `query::automap`, `query::path`, save/load RON, `World::fingerprint`, `World::log`. Tests: movement and wall blocking on the test map, doors, automap fill, save round trip, one recorded replay under `tests/replays/` with a golden fingerprint
- [x] `omnis-app` (2026-09-12): pixel pipeline (`pixel_grid_snap` pattern, `default_nearest`, internal resolution placeholder), `PackAssetPlugin` (images by pack-relative path, magenta placeholder for missing), `SimPlugin` (`SimWorld` resource, ordered `collect → apply → publish` set, events as `Message`s), `InputPlugin` (arrows/WASD/QE, F5 save, F9 load), `ViewportPlugin` (detail rows from tileset slots, procedural horizon band), minimal `HudPlugin` (position, facing, clock, last message), automap overlay toggle, states `Boot → Playing::Explore` (main menu comes with M3). Smoke test: `MinimalPlugins` + `run_once`, boot, step, no panic
- [x] `omnis-cli tileset bake` (2026-09-12): generate per-depth viewport slot sprites from 16×16 textures (checked into `packs/test/assets/` as ordinary PNGs so the game never bakes at runtime)
- [x] `packs/test` (2026-09-12): one 24×24 dungeon level with doors and a 32×32 outdoor patch (visibility depth 4 vs 12), one tileset baked from the OpenRTP dungeon and exterior sheets
- **Done when**: the owner runs the app, walks both maps in first person, opens a door, watches the automap fill, saves and reloads, and the replay fingerprint is recorded as a golden value in CI (cross-platform compare when Linux CI returns)

#### M1 review (2026-09-12, awaiting the owner's walk-through)
- Built in eight commits on `m1-tasks` (`8d4bbe3` core … `895c47c` app). 66 tests, all on `packs/test`; clippy, fmt, `lint-sim`, `check-duplicates` green; CI unchanged (the golden replay `crates/omnis-sim/tests/replays/walk.ron` runs under `cargo test`).
- Run: `cargo run -p omnis-app -- [--seed N] [--pack DIR]`. Devtools: `--script forward,turn-left,use,map --screenshot-canvas out.png` renders unattended. F5/F9 use `.omnis/quick.ron`.
- Verified by eye from canvas captures: meadow with road and shaded horizon band, a dungeon room with side walls and a pillar shadow, a front wall, the automap with walls, door, and party mark. Not verified here: the window itself and the HUD text (Bevy's window screenshot and macOS `screencapture` both return black on this machine; the canvas capture is the trusted path). The owner's run is the acceptance.
- Decisions made during the build: map layout is a text grid with shared edges (`crates/omnis-data/src/map.rs` docs); line of sight is an integer supercover ray to a tile's centre or any corner, either way round exact corners; door state is a canonical edge key in `MapState`; `Slot` carries `x`/`y` placement and `Tileset` a `viewport`; `Terrain` has `opaque` and `color`; manifest `entry` names the start map; `omnis-sim` re-exports `omnis_core` and `omnis_data`; `omnis-cli` depends on `omnis-data` and `png` 0.18.1 (Bevy's version).
- Known placeholders: opaque impassable tiles (pillars, trees) draw as floor swatches, not blocks; open doors draw nothing; the party's own tile is a thin strip at the bottom (focal 0.9·height); tree/rock silhouettes in the band are flat colour.
- Sentrux at M1 end: signal 6508 (M0 baseline 10000 was an empty skeleton); bottleneck modularity 0.20, depth 7, equality 0.46 (`query/path.rs` is the largest file); no rule violations; five functions Sentrux calls complex, none over `max_cc 25`.
- Owner's walk-through (2026-09-12): "pretty decent for a first pass". Fixed on the spot: backdrop confined to the viewport with a panel colour around it; last message clears on a move; window launches at 3840×2160 (`--window WxH`); sidebar minimap always on at 2 px/tile, M toggles the large overlay.
- Follow-ups from the walk-through:
  - [x] (2026-09-12) Opaque impassable terrain draws as a block, open doors as a frame. `SlotKind::Block` (near face plus the side face toward the party, never at depth 0) and `SlotKind::DoorFrame` (front face with the opening cut out, jambs and lintel an eighth of a tile); `Terrain.block` and `MapDef.door_open` name the surfaces, both optional; the loader checks their kinds and the broken-pack corpus exercises the checks; the planner draws frames with the row's doors and blocks last in the row. Commits `7e28d02` (schema, loader, bake), `bf66df8` (pack), `9981f63` (planner).
  - [x] (2026-09-12) Dungeon feel: owner chose detail depth 6 at width 5 with dungeon visibility 6, so the far wall of a 6-tile room is a 20 px sprite (243 px on the 4K window). The horizon band now mirrors its ground strip as a ceiling strip where the terrain has a ceiling, opaque silhouettes stand at their near edge like the baked blocks, and the tileset width no longer clips band fills. Sprites: dungeon 100 → 257, outdoor 116 → 131, about 1 KB each. Verified by canvas capture: far wall with its door, pillar as a block hiding the door behind it, open-door frame, tree block with silhouettes beside it. Sentrux signal 6495 after both follow-ups (6508 at M1 end), rules pass.
- Next: M2 grows from `omnis-app/src/dev.rs` (script and screenshot) and `omnis-cli` (validate, replay, headless).

### M2 — Live instrumentation: MCP and CLI
- [x] (2026-09-12) `omnis-cli`: `validate`, `schema dump` (the data files, a save, a replay, and the protocol ops by example, each written by the real types), `map text`, `play --script`, `replay` (asserts fingerprint), `omnis_cli::Headless`; CI runs the golden replay through the binary. The protocol itself lives in `omnis_sim::ops` (`Op`, `Reply`, `OpError`, `dispatch`) so the socket, the headless driver, and the bridge share one implementation; host ops (`save.*`, `pack.reload`, `screenshot`) are refused by `dispatch` and handled by the host. Script words are `Command::word`/`from_word`; the app's `--script` uses them too
- [x] (2026-09-12) `omnis-app` feature `devtools`: `DevSocketPlugin` (`crates/omnis-app/src/socket.rs`), on by default at a free loopback port (`--dev-socket <ip:port>` to choose, `--no-dev-socket` to disable), address in `.omnis/dev.addr`; non-blocking listener polled in `SimSet::Collect`, one client at a time, extra callers told to wait; newline JSON `{id, op, args}` → `{id, ok, result | error}`; lines over 1 MiB drop the client with a reason, save and screenshot paths are relative `.ron`/`.png` without `.`/`..`; every op above, with `screenshot` answered when the canvas readback lands; socket commands re-publish their events so the viewport and HUD follow. Test: a real loopback client under `MinimalPlugins`
- [x] (2026-09-12) `omnis-mcp`: JSON-RPC 2.0 over stdio, dual era per the 2026-07-28 spec pages read today: a request carrying `_meta` `io.modelcontextprotocol/protocolVersion` is served statelessly (`server/discover` returns `supportedVersions`, `capabilities`, `instructions`, serverInfo under `_meta`; every modern result carries `resultType: "complete"`; unsupported versions get `-32022` with the supported list), an `initialize` request selects the legacy handshake (`notifications/initialized`, `ping`). Twelve tools, one per op, names `game_status` … `screenshot`; `inputSchema` from a twenty-line `Schema` trait over the argument types, with a test that every schema's example deserializes as the op. `--headless` runs `Headless` in-process; otherwise a lazy connection to the address in `<root>/.omnis/dev.addr`; `screenshot` in game mode returns the PNG as image content (own base64). `--log <file>` records every line for the era check. No new dependencies
- [x] (2026-09-12) `.mcp.json` registration verified from this session after two restarts: the first failed because Claude Code passed a literal `${CLAUDE_PROJECT_DIR}` to `posix_spawn`, so the entry is a `sh -c` launcher that expands the variable or falls back to `$PWD`. The log shows cwd = project root, the variable unset, and the legacy handshake: `initialize` at `2025-11-25` from `claude-code 2.1.269`, then `notifications/initialized`, `tools/list`. Recorded in ARCH §9.2
- [x] (2026-09-12) Tests: the bridge binary over its stdio against a headless game (both eras, tool list, calls, `-32700`/`-32600`/`-32601`/`-32602`/`-32022`), game mode against a fake dev socket (op framing, image content), the socket over loopback (malformed line, unknown op, oversize line, busy second client), and the shared ops on real pack data
- **Done when**: from this session I can list tools, step the running game, read the viewport as text, and receive a screenshot; the same tools work headless in CI

#### M2 review (2026-09-12, item 4 awaiting the owner)
- Commits on `m2-tasks` after the M1 follow-ups: `8ce7f4e` CLI and shared protocol, `f92c67a` dev socket, `5a5a5f8` bridge and `.mcp.json`, then the lock file and these notes. 97 tests; clippy in both app feature configurations, fmt, `lint-sim`, `check-duplicates` green; Sentrux rules pass, signal 6966 (6495 after the M1 follow-ups). No new dependencies: the app's `serde_json` is optional under `devtools`, already in the tree for the bridge.
- End to end, with the game window running and the bridge binary driven by hand: `initialize`, `tools/list`, `sim_script` (three commands applied), `game_status` (turn and clock advanced), `map_text`, and `screenshot` returned as PNG image content (25 KB, the canvas). Done when, met 2026-09-12 from this Claude Code session after the owner's restart: the twelve tools listed, `sim_script` walked the party twenty commands from the meadow into the dungeon, `map_text` showed it at (3,4) facing south, and `screenshot` returned the canvas as an image. Found on the way: a script's `Visible` events returned thousands of tiles (52 kB for twenty steps), so the bridge now compacts every `Visible` event to `{"count": n}` in tool results; `viewport_get` and `automap_get` are the tools for tiles.
- Decisions: the protocol (`Op`, `Reply`, `OpError`, `dispatch`) lives in `omnis_sim::ops` so the socket, the headless driver, and the bridge share it, with host ops named there and refused by `dispatch`; tool names use underscores (`sim_command`) and map to dotted op names; the socket reads a missing `args` as `{}` and tolerates an empty one on argument-free ops (serde's adjacently tagged enums want the opposite in each case); the bridge decides the era per request, as the 2026-07-28 spec asks of dual-era servers; `--root` tells the bridge where the game runs because a stdio server's working directory is undocumented; the socket is on by default, loopback only, one client at a time, one megabyte per line; a screenshot's reply waits for the renderer's readback.
- Known limits: `pack.reload` re-reads data files but images already loaded stay cached; the bridge waits up to 60 s per op; `world.query` is the inspector's dotted syntax, not JSON pointers; `schema dump` is by example, not reflection.
- Lessons noted in `tasks/LESSONS.md`: read the formatted text before patching it; tests of the socket must keep frames running while they wait, and never write a megabyte into a socket the game reads only between frames.

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
