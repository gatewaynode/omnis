# Continuity notes

Written 2026-09-27 after M7 step 3b. Rewrite this file every time it is used; keep it to state,
next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then read `tasks/knowledge/README.md` and `agreements.md` before touching code.
- The commit trailer names whichever model is running (`Co-Authored-By: Claude <model> ...`);
  a `Claude-Session:` line only if the session's attribution reminder gives one.

## State
- Branch `m7-tasks`. Pushed up to `5bce8b7` (CI green); every commit after it is unpushed.
- M7 steps 0–3 and 3b are done. The working tree is clean after the 3b commit; nothing stashed.
- Gate: `tests passed 385 failed 0 ignored 6`; pins: tuple `(4, 4, 3, 24, 16, 11, 3, 29, 7)`,
  walk `2567641413976331380`, fight `16662894784943291610`.
- The owner's visual check of 3b is done (see Next). Earlier note: Agent-launched windows drew no frames on
  2026-09-27 (no capture, no `--frame-stats` lines), so no `--screenshot-composed` captures were
  taken. What to look at: the town gate's signpost (walk east along the street), the meadow's
  stairs down (north from the start) and signpost (south), the dungeon's stairs up at (0, 0),
  and cyan squares on the automap for portals seen. Services still do not open (step 4).

## Next
The owner tested 3b by hand (2026-09-27): it works, but the town entrance marker seems to be
missing (TODO 3c, worked after step 4). Step 4 (sim: save schema 5 with gold in copper shown as gp/sp/cp, `Mode::Town`, services), then
5–9 per the TODO. Plan it in plan mode first.

## Carry-over
- Also owed by the owner at acceptance a: `--frame-stats` with a panel open.
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
- Merged local branches `m5-tasks`, `m6c-tasks`, `m6-closeout-tasks` can be deleted by the owner.
