# Continuity notes

Written 2026-10-03 after M7 step 8b. Rewrite this file every time it is used; keep it to state,
next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then read `tasks/knowledge/README.md`, `agreements.md`, `verification.md` and
  `code-map.md` (the tool bar, service and camp entries) before touching code.
- **Toolchain**: inside the sandbox every cargo and gate run needs
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools` (the sandbox cannot read Xcode's license
  plist; the license itself is accepted). See `verification.md`.
- Sentrux: scan `/Users/john/code/omnis/crates` (the rules file is `crates/.sentrux/rules.toml`).

## State
- Branch `m7a-b-tasks`, cut from `main` at `b002b09` (PR #10 merged M7 through step 8a; CI green).
  Unpushed: `de1c1ca` (8b sim), `f49a812` (8b app) and this docs commit.
- M7 steps 0–8b done (plan `tasks/plans/m7-step8.md`). Gate `tests passed 458 failed 0 ignored 8`;
  pins unmoved: tuple `(4, 4, 3, 24, 16, 11, 3, 31, 7)`, walk `9901411989274517557`
  (`WALK_SEED = 2`), fight `15728260309841309156`; `SAVE_SCHEMA 5`; MCP 20 tools, `oneOf` 11,
  proof 77/111.
- The owner's manual test of 8a and 8b passed (2026-10-03). Agent-launched windows draw no
  frames.

## Next: M7 step 9 (M7a docs and review, owner acceptance a), per `tasks/plans/m7-town.md`
- ARCHITECTURE.md §4.5 "as built" for M7a (ask before editing ARCH): the `UiPlugin` line still
  says it composes the "tool pad"; the tool bar, the camp and the rest view are new.
- Acceptance a: start in town, buy and sell, eat, rest at the inn and in the dungeon, raise a
  dead member, save under inn-only; once, `--frame-stats` with a panel open.

## Carry-over
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
- Merged local branches (`m5-tasks`, `m6c-tasks`, `m6-closeout-tasks`, `m7-tasks`) can be
  deleted by the owner.
