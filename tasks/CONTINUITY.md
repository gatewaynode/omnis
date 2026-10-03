# Continuity notes

Written 2026-10-03 after the docs half of M7 step 9. Rewrite this file every time it is used;
keep it to state, next step and pointers. The durable knowledge lives in `tasks/knowledge/`
(start at its README). The tree is clean and HEAD passed the gate.

## On resuming
- Run `/catchup`, then read `tasks/knowledge/README.md`, `agreements.md`, `verification.md` and
  `code-map.md` before planning.
- **Toolchain**: inside the sandbox every cargo and gate run needs
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools` (the sandbox cannot read Xcode's license
  plist). A cached gate proves nothing about the linker (LESSONS 2026-10-02).
- Sentrux: scan `/Users/john/code/omnis/crates` (rules in `crates/.sentrux/rules.toml`).

## State
- Branch `m7a-b-tasks`, cut from `main` at `b002b09`. Unpushed: `de1c1ca`, `f49a812` (8b),
  `718a75f`, `ed4a8d4` (docs), `95501a2` (step 9 docs half: the M7a review, the knowledge
  sweep, `tasks/acceptance/m7a.md`), `c6e51a9` (ARCHITECTURE v0.5, owner-approved), and this
  note's commit.
- Gate `VERIFY-GREEN`, `tests passed 458 failed 0 ignored 8`; pins unmoved (tuple
  `(4, 4, 3, 24, 16, 11, 3, 31, 7)`, walk `9901411989274517557`, fight `15728260309841309156`,
  `SAVE_SCHEMA 5`, MCP 20 / `oneOf` 11 / proof 77/111).
- ARCH v0.5 matches M7a; the gamepad is gone (owner: keyboard and mouse first, other controls
  stretch goals; LESSONS 2026-10-03).

## Next: the push, then M7b
- **M7a is closed (2026-10-03):** acceptance a passed; the frame-time numbers were waived by the
  owner, because a different UI approach on a separate branch will replace this one. The owner
  pushes, opens the PR and merges. M7b does not start on a red CI.
- **Before planning M7b's screens (step 12), ask the owner** how they relate to the new UI branch
  (build on Feathers now, wait, or build on the new approach).
- Then M7b (steps 10–13) in plan mode. Size it from the review's numbers: `crates/*/src` grew
  net +4,940 lines in M7a alone, already the plan's "about 5,000 before M7c".

## Carry-over
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
- Merged local branches (`m5-tasks`, `m6c-tasks`, `m6-closeout-tasks`, `m7-tasks`) can be
  deleted by the owner.
- Files over 800 lines (debt, none over 1,000): `plan.rs` 953, `bake.rs` 934, `screen.rs` 929.
