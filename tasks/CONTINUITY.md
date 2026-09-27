# Continuity notes

Written 2026-09-27 before a compact, after M7 step 4a. Rewrite this file every time it is used;
keep it to state, next step and pointers. The durable knowledge lives in `tasks/knowledge/`
(start at its README).

## On resuming
- Run `/catchup`, then read `tasks/knowledge/README.md`, `agreements.md`, `verification.md` and
  **`tasks/plans/m7-step4.md`** (section 4b) before touching code.
- The commit trailer names whichever model is running (`Co-Authored-By: Claude <model> ...`);
  a `Claude-Session:` line only if the session's attribution reminder gives one.

## State
- Branch `m7-tasks`. Pushed up to `5bce8b7` (CI green); every commit after it is unpushed.
- M7 steps 0–4a are done; 4a is `1e73501`; the tree is clean after the continuity commit.
- Gate: `tests passed 389 failed 0 ignored 7`; pins: tuple `(4, 4, 3, 24, 16, 11, 3, 29, 7)`,
  walk `4150529802456538600`, fight `13558397520870512998`; `SAVE_SCHEMA 5`, fixtures v1–v4.
- The owner is manually testing 4a (inventory stores line "15 gp 0 sp 0 cp  food 10", debug gold
  row with every coin and whole-gold steps, bribe prices and victory gold unchanged, old quick
  saves load with gold ×100). Record the result in TODO 4a before starting 4b; a failure is fixed
  first with a test.
- Agent-launched game windows draw no frames on this machine: no captures; the owner does visual
  checks from a "what you will see / what does not work yet" line.

## Next: step 4b (plan `tasks/plans/m7-step4.md`)
`Mode::Town(ServiceState { service, kind })`, entering by step and by `Interact`, a step out
leaves, eleven `ServiceCommand`s with named refusals and prices through `services.ron` slots (all
copper), events, `may_save` true in an inn, the MCP schema branch and proof counts, log lines and
the enter/leave confirmation panel (`bevy_ui`, Go/Stay, Enter/Escape). One commit; gate and
Sentrux first; a "what you will see" line in the report.
**Open for 4b (carried from 4a):** service prices can end in silver or copper, and
`Rejection::CannotAfford` shows both sides as whole gold rounded down, so a refusal can read
"costs 16 gold; the party has 16". 4b must choose how a refusal (and a price row) names a
non-whole price; ask the owner if it is not obvious (the coin breakdown is the likely answer).

## Carry-over
- TODO 3c (the meadow's town-entrance marker at (16, 31) seems missing) is worked after step 4.
- Owed by the owner at acceptance a: `--frame-stats` with a panel open.
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
- Merged local branches `m5-tasks`, `m6c-tasks`, `m6-closeout-tasks` can be deleted by the owner.
