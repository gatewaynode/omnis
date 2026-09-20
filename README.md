# Omnis

Omnis is a turn-based, first-person, grid-crawling open-world RPG inspired by the classic 1980s
Might and Magic I and II. The intent is to build on that open-world style with modern
conveniences, an ecosystem and environment engine, a story-building engine, and wider player and
non-player character building and advancement. From the ground up the game is designed to carry
the environment-editing and content-management systems that extremely complex world features and
stories need. The world will also support fully procedural generation, so vast areas can be
created and then left as they are or customized by hand.

`PRD.md` says what the game is; `ARCHITECTURE.md` says how it is shaped. Both are the source of
truth for every decision below. `tasks/TODO.md` is the build order and the record of what each
milestone delivered.

## Status

Phase 1 is in progress. Built so far, each milestone played by the owner before the next one starts (M6's remote sensing awaits that play):

| Milestone | Delivered |
|---|---|
| M0 | Workspace, lints, CI |
| M1 | Walkable dungeon, the automap, golden replays |
| M2 | Live instrumentation: the CLI, the dev socket, the MCP bridge |
| M3 | Party and characters on the SRD 5.1 structure, the bitmap-font canvas UI |
| M4 | Turn-based combat with a dice log |
| M5 | Editor v1, deferred by the owner; planned in `tasks/plans/editor-v1.md` |
| M6 | Spells, items and equipment, remote sensing through a spyglass, a debug menu |

Next is M7: town, services, rest and progression, which closes the Phase 1 loop from PRD §13.

## Layout

The workspace is a set of small crates. The simulation crates are `no_std`, use integers only,
and never touch Bevy, the clock, threads, or the network, so a replay produces the same
fingerprint on every platform. A CI job fails the build if any of them breaks that rule.

| Crate | Role |
|---|---|
| `omnis-core` | Ids, fixed-point numbers, dice, seeded random streams, map primitives |
| `omnis-expr` | The Rhai wrapper: `no_float`, `only_i64`, checked |
| `omnis-data` | The pack loader: RON data files, text, rules, tilesets; the only crate that reads files |
| `omnis-rules` | Characters, checks, saves, spells, effects, equipment slots |
| `omnis-sim` | The world, commands, events, movement, visibility, combat, casting, items, sensing, saves |
| `omnis-app` | The game: Bevy 0.19.1, a pixel canvas, menus, the dev socket |
| `omnis-cli` | Headless subcommands: validate, replay, play, map text, schema dump, tileset bake |
| `omnis-mcp` | The MCP bridge that lets an agent drive the game or a headless world |

Game content lives in `packs/`. `packs/base` is the shipped content (SRD 5.1 material under
CC-BY-4.0, see `ATTRIBUTION.md`); `packs/test` holds the fixtures the tests and golden replays
use. Every number a rule needs is a value or a Rhai slot in the pack's `rules/` files: Rust rolls
the dice, Rhai adds and compares.

## Building and running

The toolchain is pinned by `rust-toolchain.toml`. Dev builds carry the `devtools` feature (the
dev socket, `Dev` commands, the debug menu); a release build turns it off.

```sh
just run                                     # the same as the next line; flags pass through (`just run --window medium`)
cargo run -p omnis-app                       # fullscreen on the current monitor
cargo run -p omnis-app -- --window medium    # windowed: small, medium, large, huge
cargo run -p omnis-app -- --pack packs/base --seed 7 --save .omnis/quick.ron
cargo build -p omnis-app --release --no-default-features
```

Every action has a button or menu item; the keys are shortcuts. While exploring, the arrows or the
pad move, and the tool pad offers ITEMS, SPELLS, SHEET, LOOK, MAP and MENU (keys I, C, P, L, M,
Esc). In a fight, Up and Down pick the action, Left and Right the target, Enter confirms. The
pause overlay (Esc) holds Resume, Save, Load, Character sheet, Debug menu, Quit to title and
Quit. Saves are RON files; the quick save defaults to `.omnis/quick.ron`.

## Verification

`scripts/verify.sh` is the gate every commit passes before it lands. It runs formatting, clippy
on every target and on the release configuration of the app, every test, the simulation-crate
lint, the dependency-duplicate check, and pack validation, and it exits non-zero on any failure.
CI runs the same steps, without the release-configuration clippy, plus the golden replays
through the CLI.

```sh
scripts/verify.sh
cargo run -p omnis-cli -- validate packs/base packs/test
cargo run -p omnis-cli -- replay crates/omnis-sim/tests/replays/walk.ron
```

## Driving the game from an agent

`omnis-mcp` speaks the Model Context Protocol on stdio. `.mcp.json` launches it against a running
dev build through the address the game writes to `.omnis/dev.addr`; with `--headless` it hosts a
world of its own. The bridge exposes eighteen tools: `game_status`, `world_query`, `sim_command`,
`sim_script`, `events_tail`, `viewport_get`, `map_text`, `automap_get`, `save_write`, `save_read`,
`pack_reload`, `screenshot`, `party_get`, `party_create`, `combat_get`, `rules_list`, `rules_get`
and `rules_set`.

```sh
cargo build -p omnis-mcp
target/debug/omnis-mcp --headless --pack packs/base --seed 1
```

## Licensing

The code is licensed under MIT or Apache-2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`).
Original game content is CC-BY-4.0. Third-party material, its terms and where it is kept are
listed in `ATTRIBUTION.md`. Might and Magic is a trademark of its rights holder; Omnis reproduces
none of its text, art, maps or names.
