# Continuity notes

Written 2026-09-27 before a compact, after M7 step 3b and the plan for step 4. Rewrite this file
every time it is used; keep it to state, next step and pointers. The durable knowledge lives in
`tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then read `tasks/knowledge/README.md`, `agreements.md`, `verification.md` and
  **`tasks/plans/m7-step4.md`** (approved 2026-09-27) before touching code.
- The commit trailer names whichever model is running (`Co-Authored-By: Claude <model> ...`);
  a `Claude-Session:` line only if the session's attribution reminder gives one.

## State
- Branch `m7-tasks`. Pushed up to `5bce8b7` (CI green); every commit after it is unpushed.
- M7 steps 0–3b are done; the working tree is clean after the docs commit; nothing stashed.
- Gate: `tests passed 385 failed 0 ignored 6`; pins: tuple `(4, 4, 3, 24, 16, 11, 3, 29, 7)`,
  walk `2567641413976331380`, fight `16662894784943291610`.
- The owner tested 3b by hand: exit markers in town and the dungeon's entry marker show; the
  meadow's town-entrance marker seems missing (TODO 3c, worked **after step 4**).
- Agent-launched game windows draw no frames on this machine (2026-09-27): no captures; the owner
  does the visual checks from a "what you will see / what does not work yet" line.

## Next: step 4a, then 4b (plan `tasks/plans/m7-step4.md`)
Money: stored as copper in `Party.gold` (name kept), shown as whole gold rounded down, broken out
by denomination in the inventory; pack files unchanged (gold in backgrounds and monster drops,
`cost_cp` on items); no floats. 4a starts by capturing `tests/saves/v4.ron` at HEAD (an ignored
`capture_schema_4_fixture`, as `capture_schema_3_fixture` in `save_and_replay.rs`) before any
schema change. 4b adds `Mode::Town`, eleven service commands, the MCP schema branch, log lines and
a confirmation panel before entering or leaving a service. One commit each, gate and Sentrux first.

## Carry-over
- Owed by the owner at acceptance a: `--frame-stats` with a panel open.
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
- Merged local branches `m5-tasks`, `m6c-tasks`, `m6-closeout-tasks` can be deleted by the owner.
