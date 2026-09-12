# Continuity notes

Written 2026-09-12 before a compact. Rewrite this file every time it is used.

## Where we are
- Project: Omnis, a turn-based first-person grid-crawler RPG in the Might and Magic I/II lineage, Rust, Bevy 0.19.1 (owner exempts Bevy from the N-1 rule).
- No code yet beyond a hello-world `src/main.rs`. Nothing committed; the repo has one untracked skeleton.
- `PRD.md` v0.3 and `ARCHITECTURE.md` v0.2 are written and owner-reviewed. They are the source of truth; re-read both before planning. Decision logs: PRD §6 (D1–D20), ARCHITECTURE §17 (A1–A14).
- Next step: plan Phase 0 tasks in `tasks/TODO.md` (workspace, core, expr, data, sim skeleton, cli, mcp headless), then check in with the owner before implementing.

## Decisions that are easy to get wrong after a compact
- Rules: SRD 5.1 is the structural spine; MM2 adaptations only where they serve the crawl loop (PRD §8). Not the other way round (first draft was inverted and the owner reversed it).
- Party: six slots, hirelings fill open slots (D10). Spell points: level × casting mod + other two mental mods, half casters half level, floor level (D12). Components: point cost is the primary curve; spells carry an item-quantity component list, hard requirement from spell level 5 by config (D11).
- Time: no global clock. Subjective clocks per holder, reconciled only partially on contact by a data rule; eras in the model, single era in v1 (PRD §7.8, ARCH §4.4, D20/A13).
- Viewport: fixed detail depth 4–6 tiles drawn as sprites; variable visibility depth up to 20 drawn as a procedural horizon band and fed to the automap (D16, A9).
- Remote sensing: layered perception tests; all remotely sensed knowledge is stale on record (D18).
- Non-goals with horizons: NPC free agency and aging are deferred, not rejected. Owner still has not said whether browser, 3D, and LLM content are rejected or deferred (PRD §14).
- Scripting: Rhai, not a hand-rolled language (A4 revised). `omnis-expr` is the Rhai host. Formula profile in v1: `only_i64`, `no_float`, `sync`, `no_function`, `no_closure`, `no_module`, `no_index`, `no_object`, `no_time`, `no_custom_syntax`; never `unchecked`. Pinned 1.26.1 by owner exception; Socket re-audit due 2026-10-10.
- MCP: own minimal bridge binary over stdio, dual-era handshake (legacy `initialize` and 2026-07-28 `server/discover`), newline-JSON socket to the game on loopback, devtools feature compiled out of release, headless mode through `omnis-cli` (A3, ARCH §9).
- RNG: one world seed; named PCG32 streams derived with own FNV-1a + splitmix64; stateful streams persisted; gen streams stateless (A14, ARCH §11).
- UI: `bevy_egui` 0.41.1 for the editor only; `bevy_ui` for the game (A11).
- Data: RON everywhere, matching Bevy's ron 0.12.2 (D15, A7). Packs bypass Bevy's asset system (A12). Game state lives in one serializable `World`, never in ECS (A10). No floats in simulation crates (A5).
- Dependency policy (memory file `dependency-policy-match-bevy`): match Bevy's resolved versions, single-copy tree, N-1 and 30 days otherwise, Socket audit when in doubt, owner exceptions logged in ARCH §13.

## Verified facts worth not re-researching
- Bevy 0.19.1 (2026-08-13), MSRV 1.95, edition 2024, `ron = "0.12"`; resources are components on singleton entities; buffered events are `Message`; Feathers exists but is experimental with no tabs, trees, or tables; `pixel_grid_snap` example is the integer-scaling pattern; no generic RON asset loader.
- MCP spec 2026-07-28 removed `initialize`/`ping`/sessions in favour of `server/discover` + `_meta`; Claude Code's support for it is unverified as of 2026-08-13 reports.
- Rhai 1.26.1 (2026-09-10), MSRV 1.66, actively maintained; `only_i64` exists; `Map` is a `BTreeMap`; `AST` is `!Send` without `sync`; `AST` is not serializable; hashing seed randomized per process unless set.
- Policy pins: serde 1.0.228, serde_json 1.0.150, thiserror 2.0.19, bevy_egui 0.41.1, ron 0.12.2, rhai 1.26.1. All Socket-audited 2026-09-12.
- MM2 mechanics reference was extracted from `docs/c64man_might-magic-2.pdf` into PRD §8.2 (adaptation table) during the first session; the manual is copyrighted, design reference only.

## Working agreements observed this session
- Owner wants objective analysis with numbers before decisions (spell point table, viewport depth math both changed decisions).
- Ask before updating vision documents when implementation drifts; the owner said yes to PRD updates for subjective time.
- Record every correction in `tasks/LESSONS.md` (three entries so far: non-goals vs information, v1 choice is a horizon, offer both hybrid directions).
- Use subagents for research; verify engine and protocol facts against sources, never from memory.
- Sentrux review after each task once code exists (none yet).

## Open items
- PRD §14: exact detail depth; component economy; non-goal horizons (browser, 3D, LLM).
- README.md still has typos and predates the PRD; rewrite from the PRD vision paragraph when asked.
- CLAUDE.md preamble date was filled in (2026-09-11).
