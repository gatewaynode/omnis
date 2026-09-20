# Continuity notes

Written 2026-09-20 before a compact. Rewrite this file every time it is used; keep it to state,
next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## State
- Branch `m6-closeout-tasks`, cut from `main` at `4d963d3`. On it: the owner's `09e478a`
  (justfile, a background document), `53cab2b` (two code comments), `0c0a11a` "Docs: PRD and
  ARCH match M6 as built", `6897144` the MCP schema proof, and this continuity commit. The owner
  said they would push around the time this commit was made: **check
  `git log origin/m6-closeout-tasks..m6-closeout-tasks` first**; anything listed is unpushed and
  must not be stranded by a merge (LESSONS 2026-09-19).
- The gate is green at 353 passed, 6 ignored; Sentrux rules pass; both replays hold.
- The drift pass is done and became a direction change, all in PRD v0.5 and ARCH v0.3: closer to
  the SRD and extend it (D21 turn budget, D22 declared reactions, D23 runbooks and the
  per-member auto flag, D24 three spell fields; design in ARCH §4.7, A15, nothing built); tool
  proficiencies as pack data (§8.1); the editor held for the end of Phase 1 (D4); the time model
  is D25; **D26: a modern look and feel, pixel art is placeholder only**; A11 amended: Feathers
  in game as a bounded experiment.
- Not yet answered by the owner: how the M6c play-test went; whether ARCH §4.7 resolving auto
  members' turns inside the simulation (not in the front end) stands.

## Next
1. The owner confirms CI on the push.
2. **Plan the Feathers experiment and the move away from pixel-art styling, in plan mode**
   (owner, 2026-09-20). Read first: the TODO's "M6 closeout" item, ARCH A11 and the §8.2 status
   note, PRD D26 and §11.1, and "Modern presentation" in `tasks/knowledge/horizons.md` (the
   Bevy source facts are already there). Report with numbers: crates added by the `ui` and
   `bevy_feathers` features, rendering and scaling on the 5120×1440 ultrawide, whether the MCP
   screenshot and the screen dumps still show the interface, whether headless tests still drive
   every widget, and whether a modern font can replace the bitmap font. PRD §14 still asks what
   modern means for the viewport (higher-resolution 2D, smooth scaling, or reopening 3D): ask.
3. Then plan M7 (town, services, rest, progression) with the turn budget and declared reactions
   in or beside it. PRD §14 questions needing the owner: what preparation costs, the budget
   curves, runbook timing.
4. Owner documents that appear under `docs/background/` are committed at once, unedited.
