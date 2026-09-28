# Continuity notes

Written 2026-09-27 after M7 step 6. Rewrite this file every time it is used;
keep it to state, next step and pointers. The durable knowledge lives in `tasks/knowledge/`
(start at its README).

## On resuming
- Run `/catchup`, then read `tasks/knowledge/README.md`, `agreements.md`, `verification.md` and
  `code-map.md` (the new Rest entry) before touching code.
- The commit trailer names whichever model is running (`Co-Authored-By: Claude <model> ...`);
  a `Claude-Session:` line only if the session's attribution reminder gives one.

## State
- Branch `m7-tasks`. Pushed up to `5bce8b7` (CI green); every later commit is unpushed
  (4a `1e73501`, 4b `14d4d84`, 3c `3261be1`, 5 `d27f89d`, 6, and the docs commits).
- M7 steps 0–6 and follow-up 3c are done; the owner's manual tests of 4a, 4b, 3c and 5 passed;
  step 6 is agent-facing only (MCP `service_get`, `screen_text`, script words), nothing new on screen.
- Gate: `tests passed 434 failed 0 ignored 8`; pins: tuple `(4, 4, 3, 24, 16, 11, 3, 31, 7)`,
  walk `9901411989274517557` under `WALK_SEED = 2`, fight `15728260309841309156` under the
  golden seed; `SAVE_SCHEMA 5`, fixtures v1–v4; MCP 20 tools, `oneOf` 11, proof 77 instances
  and 111 branches.
- Agent-launched game windows draw no frames on this machine: no captures; the owner does visual
  checks from a "what you will see / what does not work yet" line.

## Decisions this session (all recorded in `tasks/TODO.md`)
- 3c: the town gate lands beside the signpost at meadow (16, 30); arriving through a portal
  puts the portals beside the landing on the automap (`apply::know_portals_beside`).
- Step 5 (plan `tasks/plans/m7-step5.md`): both rests ambushable at the map's walking chance
  (per mille, `map_chance × 10`); a stable member at 0 may spend hit dice; ambush after 1d8 hours
  or 1d6 × 10 minutes; rest only outside services; **rest events are a per-map table by terrain**
  (`MapDef.rest_events`), placeholders at chance 0 in the town and meadow. The owner has ideas
  for them later; `Event::RestEvent` changes nothing yet.
- The four 4b departures (Party/Cast/Item inside a service; Buy/Sell by row; a room wakes the
  downed at 1 HP; town lines English in code) were confirmed by the owner on 2026-09-27.

## Next: M7 step 7 (the service panel), per `tasks/plans/m7-town.md`
- `PlayState::Service` following `Mode::Town`; `service_panel.rs` (Bevy-free ids, `apply` from a
  report to a `ServiceCommand`) and `service_scenes.rs` on the kit, built from
  `omnis_sim::service_view` (each `OfferView` is the command to send, its price or `pays`, and
  its refusal). Bank amounts by number input; Leave and Escape; the last refusal as a message.
  Tests as the creation panel's; `screen_text` (MCP tool 20) reads it without a picture. Plan it
  in plan mode first.
- Then 8 (camp panel: `PartyView.long_rest_wait` and `hit_dice_left` are ready for it), 9 (docs,
  acceptance a).

## Carry-over
- Owed by the owner at acceptance a: `--frame-stats` with a panel open.
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
- Merged local branches `m5-tasks`, `m6c-tasks`, `m6-closeout-tasks` can be deleted by the owner.
