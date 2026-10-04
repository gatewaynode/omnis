# Continuity notes

Written 2026-10-04 mid-M7c, before a compact. Steps 0–5 of M7c are committed, plus the one-spell
rule (4b) and Bob the Rat King (5b, an owner-requested interlude); steps 6–8 remain. Rewrite this
file every time it is used; keep it to state, next step and pointers. The durable knowledge lives
in `tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then read `tasks/plans/m7c-turn-budget.md` (the approved M7c plan) and the M7c
  block of `tasks/TODO.md` (each done step records what was built, what moved, deviations and
  the measured tables). Bob's plan: `tasks/plans/m7c-bob-the-rat-king.md`.
- **Toolchain**: every cargo and gate run needs `DEVELOPER_DIR=/Library/Developer/CommandLineTools`
  (now in the README and ARCH §14).
- Sentrux: `git add` new files, scan `/Users/john/code/omnis/crates`, `check_rules`.
- Context: pipe test and gate runs through `grep`/`tail`; read files by range.
- Mutation passes: a scratchpad script of `(name, file, old, new)` restored in `finally`, run with
  `cargo test -p omnis-sim --no-fail-fast`, counting a break only by a named failing test
  (LESSONS 2026-10-02); the scratchpad is per session, so recreate it. Match `cargo fmt`'s text.

## State
- Branch `m7a-b-tasks`, **not pushed since `4b1bd14`**; the tree is clean except the owner's
  untracked `SIDE_NOTE_PLEASE_READ.md` (left alone: see open questions).
- This session's commits: `8fee81d` (docs: DEVELOPER_DIR, the trigger-field horizon), `25ce447`
  (the SRD one-spell rule), `974cdc9` (step 5, measured; `tests/measure/` split), `02d65e5`,
  `2226179`, `63c24e0` (Bob 1–3), then this file's commit.
- Gate `tests passed 505 failed 0 ignored 12`. Pins: base tuple `(4, 4, 3, 24, 16, 18, 3, 34, 7)`
  (the test pack has 7 monsters and 19 spells), walk `7703481389045166225`, fight
  `4106035052781345400`, `SAVE_SCHEMA 6`, MCP 20 tools, proof 93 instances / 141 branches, combat
  schema 6 branches.
- **Owner decisions for M7c** (2026-10-03; the TODO block and the plan): fight screen stays on the
  canvas; tactics = ARCH §4.7's whole shape, reactions only; monsters take opportunity attacks by
  a built-in rule; string ids a horizon; Cunning = bonus-action exchange without opportunity
  attacks + Hide; preparation = D24's fields only; a turn also ends when nothing is left to pay
  for (ARCH §4.7 wording in step 8, shown first).
- **Owner decisions for Bob** (2026-10-04): homebrew CR 1 on SRD rules; his actions a dice roll
  until monster runbooks; a new room in the depths (18 wide now, throne room at (13, 8));
  Shield stops Magic Missile both ways, his casts raise `EnemyCasts`, members save.
- Measured (TODO 5 and 5b): the budget lowers four members' clear wipes 26.3 → 23.0% without Bob
  and 47.0 → 37.2% with him; Bob alone wipes a level-1 four 49%, a level-2 four 4%. Nothing tuned;
  whether Bob is gentler is the owner's call after play.

## Next: steps 6–8 (plan file has the detail)
- **6. MCP and CLI**: views: `combat.get` the budget, reactions left, features with uses, the
  switch (and possibly each caster's points left, `Stack.spent`); `party.get` each member's
  declared reactions and reaction-capable actions with wire ids; tool descriptions; CLI schema
  words; `react-M-on|off` script word (not built). The event schema gains nothing to prove
  (events are output), but check the MCP's event rendering for `MonsterCast` / `ShieldStops`.
- **7. App**: React switch on the action row (`CombatIntent::Reactions` exists, unused); a budget
  line; features in the Use picker; bonus pay in the cast picker (today always `Pay::Action`;
  respect `SpellsCast::refuses`); the Feathers tactics panel (`UiScreen::Tactics`, TACTICS on
  the sheet); headless tests. The monster cast log lines exist.
- **8. Docs**: ARCH v0.7 §4.7 as built (shown first) including monster casting, `EnemyCasts`'s
  raise, the one-spell rule; PRD §14 line (ask); code-map, verification; the M7c review;
  `tasks/acceptance/m7c.md` (add: walk to Bob, see his casts, both sides' shields, the saves);
  owner acceptance c closes M7.

## Open questions for the owner
- The trigger bus is answered (2026-10-04): an event bus with topics in M8, external crates
  evaluated; recorded in `horizons.md` (M8) and the TODO's M8 block. Still open: commit or delete
  the untracked `SIDE_NOTE_PLEASE_READ.md` (its substance is now in the horizon).

## Carry-over and watch-outs
- `EnemyCasts` is raised at every monster cast and `SpellCast` at every member cast, but nothing
  answers either; `EnemyFlees`, `OwnTurn` have no source. Say so in step 8.
- Files near 800: `command.rs` 750, `ops.rs` 829 (not to grow; views in `view.rs`). New:
  `monster_cast.rs` 437, `tests/monster_cast.rs` 539.
- A rogue at level 2 or a fighter holding Action Surge always has a bonus or free option, so the
  turn waits for End (by design); the acceptance script should say so.
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
