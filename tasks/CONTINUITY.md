# Continuity notes

Written 2026-10-03 mid-M7c, before a compact. Steps 0–4 of M7c are committed; 5–8 remain. Rewrite
this file every time it is used; keep it to state, next step and pointers. The durable knowledge
lives in `tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then read `tasks/plans/m7c-turn-budget.md` (the approved plan) and the M7c
  block of `tasks/TODO.md` (each done step records what was built, what moved, and deviations).
- **Toolchain**: every cargo and gate run needs `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.
- Sentrux: `git add` new files, scan `/Users/john/code/omnis/crates`, `check_rules`.
- Context: this session reached 513k of 1M before the compact, a third of it Bash output; pipe
  test and gate runs through `grep`/`tail` and read files by range.
- Mutation passes: scratchpad scripts `mutate.py` / `mutate4.py` (list of `(name, file, old, new)`,
  restore in `finally`; the scratchpad is per session, so recreate them; run in the background
  without `tail` so results arrive as they finish); a break counts only with a named failing test (LESSONS 2026-10-02).

## State
- Branch `m7a-b-tasks`, **not pushed since `4b1bd14`**; local commits: `0672521` (M7c plan docs),
  `112c200` (step 0, v5 fixture), `93bf40d` (step 1, splits), `1467ee5` (step 2, data),
  `98b71dd` (step 3, budget), `3681bd8` (step 4, reactions, schema 6). Tree clean, gate green.
- Gate `tests passed 498 failed 0 ignored 10`. Pins: tuple `(4, 4, 3, 24, 16, 18, 3, 34, 7)`,
  walk `11552158104858336387`, fight `5165996742789241042`, `SAVE_SCHEMA 6`, MCP 20 tools, proof
  93 instances / 141 branches, combat schema 6 branches.
- **Owner decisions for M7c** (2026-10-03, in the TODO block and the plan): fight screen stays on
  the canvas; tactics = ARCH §4.7's whole shape, reactions only (auto stored, inert; no runbook
  editor); monsters take opportunity attacks by a built-in rule; string ids stay a horizon;
  Cunning Action = bonus-action exchange without opportunity attacks + Hide; preparation =
  D24's fields only. Approved with the plan: a turn also ends by itself when nothing is left
  to pay for (ARCH §4.7 wording to change in step 8, shown first).
- Pulled forward from later steps (recorded in the TODO): the MCP combat and party schema
  (step 6's schema half is done), the fight screen's End action (column 42, hotkey `n`), the
  reaction picker row and the three new log lines (part of step 7).

## Next: steps 5–8 (plan file has the detail)
- **5. Measured**: `measure.rs::budget_over_seeds` comparing one-command turns with the M7c
  policy (Second Wind under half, Action Surge round 1, Healing Word with the bonus, shield
  declared, opportunity attacks on); table into the TODO before play. `common::act` plays one
  command as one whole turn (EndTurn when the action is spent).
- **6. MCP and CLI**: views: `combat.get` the budget, reactions left, features with uses, the
  switch; `party.get` each member's declared reactions and reaction-capable actions with their
  wire ids; tool descriptions; CLI schema print words; `react-M-on|off` script word (not built yet).
- **7. App**: React switch on the fight's action row (`CombatIntent::Reactions` exists, unused);
  a budget line; features in the Use picker (Second Wind, Action Surge, Cunning: Exchange, Hide),
  bonus pay chosen by the cast picker (bonus when allowed and left, else action; today it always
  sends `Pay::Action`); the Feathers tactics panel (`UiScreen::Tactics`, a TACTICS button on the
  sheet) editing the default runbook's reactions; headless tests by event and by pointer.
- **8. Docs**: ARCH v0.7 §4.7 as built (show the owner first), PRD §14 line (ask), code-map and
  verification, the M7c review, `tasks/acceptance/m7c.md`; owner acceptance c closes M7.

## Carry-over and watch-outs
- `SpellCast` trigger is raised but nothing answers it (`tactics::answers` accepts no action for
  it); `EnemyFlees`, `EnemyCasts`, `OwnTurn` have no source. Say so in step 8's docs.
- Files near 800: `command.rs` 727+, `ops.rs` 829 (not to grow; views in `view.rs`).
- A rogue at level 2 always has a bonus option (Cunning), so its turn waits for End; a fighter
  holding Action Surge too. By design under the approved rule; the acceptance script should say so.
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
