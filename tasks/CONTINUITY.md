# Continuity notes

Written 2026-09-27 before a compact, after M7 step 4b. Rewrite this file every time it is used;
keep it to state, next step and pointers. The durable knowledge lives in `tasks/knowledge/`
(start at its README).

## On resuming
- Run `/catchup`, then read `tasks/knowledge/README.md`, `agreements.md`, `verification.md` and
  `code-map.md` (the new Town and confirmation entries) before touching code.
- The commit trailer names whichever model is running (`Co-Authored-By: Claude <model> ...`);
  a `Claude-Session:` line only if the session's attribution reminder gives one.

## State
- Branch `m7-tasks`. Pushed up to `5bce8b7` (CI green); every commit after it is unpushed
  (4a `1e73501`, 4b `14d4d84`, and the docs commits).
- M7 steps 0–4b are done; the owner's manual tests of 4a and 4b passed. The tree is clean after
  the continuity commit.
- Gate: `tests passed 413 failed 0 ignored 7`; pins: tuple `(4, 4, 3, 24, 16, 11, 3, 29, 7)`,
  walk `4150529802456538600`, fight `13558397520870512998`; `SAVE_SCHEMA 5`, fixtures v1–v4;
  MCP `oneOf` 10, proof 75 instances and 107 branches.
- Agent-launched game windows draw no frames on this machine: no captures; the owner does visual
  checks from a "what you will see / what does not work yet" line.

## 4b departures from the plan (reported to the owner, not yet explicitly confirmed)
- `Party`, `Cast` and `Item` commands work inside a service (the plan said `WrongMode`).
- `Buy` takes a row of the service's stock, `Sell` a row of the stores.
- A room wakes a downed (not dead) member at 1 hit point without the rest's benefits (SRD).
- Town log lines are English in `service_text.rs`; names and rumors come from the text pack.
- Refusals and town lines name prices with every coin (the 4a open question, settled on the
  recommendation); fights keep whole gold.

## Next: TODO 3c, then steps 5–9
- 3c: the meadow's town-entrance marker at (16, 31) (`signpost`) seems missing, in the viewport
  and on the automap, against the dungeon entrance at (16, 5), which shows. Root cause first,
  then a fix with a test that bites.
- Then step 5 (rest in the field; reuse `rest::long_rest_restore`), per `tasks/plans/m7-town.md`.
- Not working yet in the game (by design until later steps): service buttons (step 7 panel),
  Use inside a building (refused), trainer and guild (step 6), bank and hit dice on screen.

## Carry-over
- Owed by the owner at acceptance a: `--frame-stats` with a panel open.
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
- Merged local branches `m5-tasks`, `m6c-tasks`, `m6-closeout-tasks` can be deleted by the owner.
