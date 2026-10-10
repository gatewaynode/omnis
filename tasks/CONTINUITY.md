# Continuity notes

Written 2026-10-10, before a compact. Rewrite this file every time it is used. Durable knowledge lives
in `tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, cut by the owner from `main` at `aa89fff` (the merge of PR #11,
  `m7a-b-tasks`). It holds all of protocol 2 (P2a–P2d, closing commit `8525ef2`). Tree clean before this
  note.
- **Next task (owner, 2026-10-10): migrate the user app to the successful experiment.** Nothing has been
  planned or touched yet. Plan mode first (CLAUDE.md); the owner has not yet said what "migrate" covers
  (replace `omnis-app` with `omnis-vector`, or carry its screens into `omnis-app`, or both side by side).
  Ask before designing.

## The experiment (read cold before planning)
- Branch `origin/gui-3d-experiment` (head `a1a1cf6`, pushed), cut from `main` at `8e111d5`: 28 commits
  of its own; this branch is 143 commits past that base. Separate clone: `/Users/john/code/omnis-alt/omnis`.
- Its state doc: `tasks/alt-CONTINUITY.md` on that branch (`git show origin/gui-3d-experiment:tasks/alt-CONTINUITY.md`);
  also `tasks/alt-TODO.md`, `alt-PRD.md` (v0.3, "the adopted presentation direction", draft under the
  owner's review), `alt-ARCHITECTURE.md`, `tasks/plans/bevy-0.20-migration.md`, and its Phase B plan
  `~/.claude/plans/snug-munching-gray.md`.
- What it is: crate `crates/omnis-vector` (about 8,000 lines with tests): a walkable 3D view over the
  grid, buttons for every action with keys as shortcuts, a minimap from the automap, a roll log, and a 2D
  fight screen (`arena.rs`, `combat_menu.rs`, `cinema.rs`, `shell/`). Its gate: 436 passed, 6 ignored.
  Bevy `=0.19.1` (same as here). The touched files outside the crate are only `Cargo.toml`, `Cargo.lock`,
  `tasks/LESSONS.md` and its own docs, so a merge should conflict little in text.
- **It was built on protocol 1.** It names members by marching-order slot (`Pick::Member(slot)`,
  `Action::{Cast(u8), Use(u8)}` as list rows) and reads fields renamed in protocol 2 (`MemberView.front`
  → `in_front`, `StackView.front` → `in_front`, spells and items as rows → string ids). Size not
  measured: compile `omnis-vector` against this tree to find out.
- **Collisions to expect:** its `B6` (Phase B step 6, in `alt-TODO.md`) and this tree's `tasks/BUGS.md` B6
  are different things; `tasks/LESSONS.md` has entries on both sides; its notes call this tree's
  `CONTINUITY.md` stale (it described `m6-closeout-tasks` then).
- Vision documents: adopting it changes PRD and ARCHITECTURE (`alt-PRD.md` v0.3 is under review). Ask the
  owner before editing `PRD.md`/`ARCHITECTURE.md` (CLAUDE.md). Memory `ui-replacement-branch` is now out
  of date (the experiment is being adopted).

## Open, carried over
- **Protocol 2 owner acceptance**: Parts 1 and 2 passed over the real `omnis-mcp --headless` binary
  (2026-10-10); Part 3 (play in the window) is the owner's. Then check P2d and the parent protocol-2 item
  in `tasks/TODO.md` and mark `tasks/plans/protocol-2.md` closed. The window may change with the migration:
  ask whether Part 3 is played on the old app first.
- **rhai 1.26.1 Socket re-audit (due 2026-10-10), not closed.** The Socket MCP is not connected in this
  session and socket.dev answers 403 to a fetch. Done without Socket: no newer release (1.26.1 is the
  latest, 30 days old today, so the 30-day rule now holds; N-1 stays waived); `Cargo.lock` checksum
  `0334…ebf5` matches crates.io, not yanked, published by `schungx`, owners `sophiajt`, `luciusmagn`,
  `schungx`; `cargo audit` (1296 advisories): no vulnerability anywhere; rhai's resolved deps unchanged
  (its declared `web-time` does not resolve here). New: `smartstring` 1.0.1 (via rhai) unmaintained,
  RUSTSEC-2026-0249 (2026-05-03), recorded nowhere; `ttf-parser` 0.25.1 (via Bevy) unmaintained,
  RUSTSEC-2026-0192. Owner to choose: connect the Socket MCP and rerun (recommended), paste scores from a
  browser, or close without Socket and redate. Either way add the smartstring advisory to ARCH §13's row;
  close the line in `tasks/knowledge/horizons.md` "Dated re-audits".
- Unscheduled: `DevCommand::Pass { minutes }`; the `data.*` ops; `scripts/mcp-probe.py`. Editor v1 held
  "after M8, before M9".

## Pins (this tree)
- Gate `tests passed 583 failed 0 ignored 13`, VERIFY-GREEN, 222 s (the first-launch wait is gone since
  the iTerm2 Developer Tools restart: 0.00 s on a never-seen binary).
- Walk replay `8711507745385976768`, fight `5247080599556612730`; `SAVE_SCHEMA` 7, `PROTOCOL` 2, MCP 24
  tools, schema proof 94/142. Sentrux quality 9003, rules pass.

## Process
- Every cargo and gate run: `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first.
- Gate: `scripts/verify.sh > <scratchpad>/gate.txt 2>&1` in the background; no edits while it runs.
- After any wire change: `cargo test -p omnis-mcp --test integration vocabulary::` (seconds).
- Tests: one `integration` binary per crate; a new test file goes into its crate's `tests/main.rs`
  (`scripts/check-test-modules.sh` enforces it). `omnis-vector`'s tests will need the same layout.
- Unfinished work before a restart or compact is committed as `WIP`, never stashed. The owner pushes.

## Carry-over
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` 902, `save_and_replay.rs` 810,
  `tactics_panel.rs` 795; on the experiment, `arena.rs` 773.
