# Continuity notes

Written 2026-09-20 before a compact. Rewrite this file every time it is used; keep it to state,
next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## State
- Branch `m6c-tasks`, seven commits past `main` (`96abe13`, the PR #6 merge), tree clean,
  nothing pushed. It holds M6c sensing, the cherry-picked debug-menu pause item (`46b58a7`), the
  README rewrite, and this knowledge set. Local `m5-tasks` is fully carried over and can go.
- M6 is complete on the branch. M6a, the entry points and M6b are owner-accepted; **M6c is not
  yet played** (build an Explorer, open a door, press LOOK or L, see the ring past the party's
  sight on the map).

## Next
1. Ask what CI said on the push.
2. **The drift list** (owner instruction 2026-09-19): one item at a time, state what the vision
   document says, what is built, both ways to close the gap (document or code) with a
   recommendation; edit `PRD.md`/`ARCHITECTURE.md` only on the owner's say-so per item; bump
   ARCH to v0.3; one commit "Docs: PRD and ARCH match M6 as built". Items: PRD §8.3 action
   economy and reactions; PRD §8.1 crawler abilities vs the SRD stand-ins in the Explorer; ARCH
   §8.1 plugin list (`DebugPlugin`, `SheetPlugin`, `InventoryPlugin`); ARCH §9.3 generated
   schemas (hand-written); ARCH §4.1 `rng` vs `rngs`; `world.rs:149` Relief items in M6 (M7);
   the editor in Phase 1 (PRD D4/§13, ARCH §16); PRD §11.1 Feathers; two D20 rows; §14 depth 4.
3. Plan M7 in plan mode (town, services, rest, progression; PRD §13 closes the Phase 1 loop).
