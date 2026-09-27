# Continuity notes

Written 2026-09-27 after M7 step 3. Rewrite this file every time it is used; keep it to state,
next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## State
- Branch `m7-tasks`. The owner pushed up to `5bce8b7` (CI green). **Every commit after it is
  unpushed**: the docs commits `c426d8f`, `53d5320`, `184bde2`, `8a664eb`, `d3b163a`, `5043154`,
  M7 step 1 `f8e2879` (changes `ci.yml`), step 2 `65c9102`, step 3 `81158b8`. If the owner
  reports a merge: `git branch --show-current`, `git log --oneline -1`, `git log HEAD..main`,
  `git log main..m7-tasks` first.
- M7 steps 0–3 are done (the TODO's M7 block; design in `tasks/plans/m7-town.md`).
- Step 3 as built: `omnis-data/src/service.rs`, seven base services, `services.ron` and
  `rest.ron`; the test pack depends on base (owner) and starts in `test:map:town` (10, 2, west);
  the gate at (11, 2) leads to the meadow's start. Tests from before the town use
  `common::new_world` (sim) or the plan tests' `new_world` (meadow start).
- Gate green: `tests passed 381 failed 0 ignored 6`; Sentrux passes. Pins: tuple
  `(4, 4, 3, 24, 16, 11, 3, 29, 7)`, walk `17768808726456611042`, fight `14183048486799478772`,
  save schema 4, 18 MCP tools.
- Commit trailer: `Co-Authored-By: Claude Opus 5.5`; no `Claude-Session:` line (this session's
  reminder gives none; `agreements.md` updated).

## Next: M7 step 4, sim
Save schema 5 (`Character.hit_dice_spent`, `Party.bank`, `Party.last_long_rest`, `v4_to_v5`,
fixture `tests/saves/v4.ron`), `Mode::Town(ServiceState)`, entering a site by step and by
`Interact`, the `ServiceCommand`s with named refusals and slot prices, the events, `may_save`
in an inn; rebaseline. **Ask the owner first**: `Party.gold` is whole gold pieces, the price
slots return copper (an arrow is 5 cp). Options: gold kept in copper in schema 5 (the display
divides), or prices rounded to gold in Rust (buy up, sell down), or bundles.

## Later
Rest (step 5, measured first), ops/MCP (6), the service panel (7), camp (8), docs and acceptance
a (9). Owed by the owner at acceptance a: `--frame-stats` with a panel open. A call when
convenient: a longer name limit for wide scripts (`horizons.md`). Dated: Socket re-audit of
`rhai` 1.26.1 on 2026-10-10. Merged local branches `m5-tasks`, `m6c-tasks`,
`m6-closeout-tasks` can be deleted by the owner.
