# Continuity notes

Written 2026-10-03 before a compact, after M7 step 8b and the owner's manual test of 8a and 8b.
Rewrite this file every time it is used; keep it to state, next step and pointers. The durable
knowledge lives in `tasks/knowledge/` (start at its README). The tree is clean and HEAD passed
the gate.

## On resuming
- Run `/catchup`, then read `tasks/knowledge/README.md`, `agreements.md`, `verification.md` and
  `code-map.md` before planning.
- **Toolchain**: inside the sandbox every cargo and gate run needs
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools` (the sandbox cannot read Xcode's license
  plist; the license itself is accepted). A cached gate proves nothing about the linker
  (LESSONS 2026-10-02).
- Sentrux: scan `/Users/john/code/omnis/crates` (the rules file is `crates/.sentrux/rules.toml`).

## State
- Branch `m7a-b-tasks`, cut from `main` at `b002b09` (PR #10 merged M7 through step 8a; CI
  green). Unpushed: `de1c1ca` (8b sim: rest view), `f49a812` (8b app: camp panel), `718a75f`
  (docs: manual test) and this continuity commit.
- M7 steps 0–8b done; the owner's manual test of 8a and 8b passed (2026-10-03). Step 8's plan is
  `tasks/plans/m7-step8.md` (split into 8a, the tool bar on Feathers, and 8b, the camp).
- Gate `tests passed 458 failed 0 ignored 8`; pins unmoved: tuple `(4, 4, 3, 24, 16, 11, 3, 31, 7)`,
  walk `9901411989274517557` (`WALK_SEED = 2`), fight `15728260309841309156`; `SAVE_SCHEMA 5`;
  MCP 20 tools, `oneOf` 11, proof 77/111.
- Agent-launched windows draw no frames: no captures; the text trees stand in.

## Next: M7 step 9 (M7a docs and review, owner acceptance a) — plan mode first
Per `tasks/plans/m7-town.md` step 9 and `tasks/TODO.md` (M7 block):
- **ARCHITECTURE.md §4.5 "as built" for M7a** — ask the owner before editing ARCH; draft first.
  Known drift: §8's `UiPlugin` line says it composes the "tool pad" (now a Feathers bar,
  `FeathersUiPlugin`); the `FeathersUiPlugin` line says "M7's service screens next" (now built:
  confirm, service, camp, tool bar). New since M7 began: town and services (`service.rs`,
  `service_view.rs`), rest (`rest.rs`, `rest_view.rs`), save schema 5 (copper), the camp, the
  tool bar on Feathers, `screen_text`.
- **Knowledge sweep**: `code-map.md` header still says "as of M6"; `horizons.md` for M7a items.
- **M7a review in TODO** with the measurements gathered from steps 3–8b (ambush rates of step 5,
  the tool bar's fit numbers of 8a, test counts 376 → 458).
- **Acceptance a script for the owner**: start in town, buy and sell, eat, rest at the inn and in
  the dungeon (the camp), raise a dead member, save under inn-only; once, `--frame-stats` with a
  panel open against the canvas alone (owner's machine).

## Carry-over
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
- Merged local branches (`m5-tasks`, `m6c-tasks`, `m6-closeout-tasks`, `m7-tasks`) can be
  deleted by the owner.
