# Continuity notes

Written 2026-09-20 at the end of the drift pass. Rewrite this file every time it is used; keep it
to state, next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## State
- Branch `m6-closeout-tasks`, cut from `main` at `4d963d3` (PR #8 merged M6c; CI green). On it:
  the owner's `09e478a` (justfile, a background document), `53cab2b` (two code comments), the
  docs commit `0c0a11a` "Docs: PRD and ARCH match M6 as built", and the MCP schema proof. Nothing pushed. The owner was play-testing
  M6c by hand; ask how it went.
- The drift pass is done, every item decided by the owner. It became a direction change, all in
  PRD v0.5 and ARCH v0.3: closer to the SRD and extend it (D21 turn budget, D22 declared
  reactions, D23 runbooks and the per-member auto flag, D24 three spell fields); tool
  proficiencies as pack data (§8.1); the editor held for the end of Phase 1 (D4); the time model
  is D25; **D26: a modern look and feel, pixel art is placeholder only**; A11 amended: Feathers
  in game as a bounded experiment; ARCH §4.7 is the tactics design (not built), A15.
- One deviation to confirm with the owner: ARCH §4.7 resolves auto members' turns inside the
  simulation (like monsters), not in the front end as I first suggested in chat.

## Next
1. The one open item in the TODO's "M6 closeout" block: the Feathers experiment (plan mode
   first; criteria in the TODO item and ARCH A11). The schema proof is done
   (`omnis-mcp/tests/schema_proof.rs`).
2. Plan M7 in plan mode (town, services, rest, progression). The turn budget and declared
   reactions belong in or beside it; see `tasks/knowledge/horizons.md` for everything queued
   (tactics, tools, modern presentation). Open PRD §14 questions need the owner: what
   preparation costs, the budget curves, runbook timing, what modern means for the viewport.
3. Owner documents that appear under `docs/background/` are committed at once, unedited.
