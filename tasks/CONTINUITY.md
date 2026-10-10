# Continuity notes

Written 2026-10-10, after P2d's closing commit `8525ef2`. M8 is closed (`8921c07`). Branch `m7a-b-tasks`.
Rewrite this file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## State of the tree
- Protocol 2 is built (P2a–P2d; `4ecb1d6` WIP, `3a13481`, `8525ef2`). Gate `tests passed 583 failed 0
  ignored 13`, VERIFY-GREEN, 222 s. CLI replays unchanged. Sentrux rules pass, quality 9003 (9038 before
  P2d; the drop came with the P2d code in the WIP commit, not looked into).
- **Open: owner acceptance** (`tasks/acceptance/protocol-2.md`). Parts 1 and 2 were run over the real
  `omnis-mcp --headless` binary on 2026-10-10 and passed (protocol 2, `Place` position, Bless by id agrees
  across `cast_get`, `SpellCast`, `EffectApplied`, `party_get`; after a reorder `caster: 0` still reaches
  Wren, `caster: 1` is Ash). Part 3 is the owner's play in the window. When it passes: check P2d and the
  parent protocol-2 item in `tasks/TODO.md`, mark `tasks/plans/protocol-2.md` closed.

## The first-launch wait is gone
- After the owner restarted iTerm2 (listed under Developer Tools), a never-seen binary launches in 0.00 s
  the first time, from this session too. Recorded in `verification.md` "Test binaries".

## Next (owner's call)
- The Socket re-audit of `rhai` 1.26.1 (due 2026-10-10).
- Unscheduled TODO items: `DevCommand::Pass { minutes }`; the `data.*` ops; `scripts/mcp-probe.py`.
- Editor v1 is held "after M8, before M9" (owner, 2026-09-20); M8 is closed.

## Pins
- Gate 583/0/13. Walk replay `8711507745385976768`, fight `5247080599556612730`.
- `SAVE_SCHEMA` 7, `PROTOCOL` 2, MCP 24 tools, schema proof 94/142, dev branches 13, 62 rejections.
- Base pack tuple `(4, 4, 3, 24, 16, 18, 3, 36, 7)`.

## Process
- Every cargo and gate run: `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first.
- Gate: `scripts/verify.sh > <scratchpad>/gate.txt 2>&1` in the background; no edits while it runs.
- After any wire change: `cargo test -p omnis-mcp --test integration vocabulary::` (seconds).
- A new test file goes into its crate's `tests/main.rs`.

## Carry-over
- Not built: nothing answers `SpellCast`/`EnemyCasts`; `EnemyFlees`/`OwnTurn` have no source; region
  catch-up waits for M10.
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` 902, `save_and_replay.rs` 810,
  `tactics_panel.rs` 795.
- The command instances (`omnis-mcp/tests/common/commands.rs`) have no `Predicate::Row`, so the vocabulary
  never sees it on real data (the self-check covers it).
