# Continuity notes

Written 2026-10-04 mid-M7c, before a compact. Steps 0–6 of M7c are committed, plus the one-spell
rule (4b) and Bob the Rat King (5b); steps 7–8 remain. Rewrite this file every time it is used;
keep it to state, next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start
at its README).

## On resuming
- Run `/catchup`, then read `tasks/plans/m7c-turn-budget.md` §"Step 7" (the approved M7c plan) and
  the M7c block of `tasks/TODO.md` (each done step records what was built, deviations, tables).
  Step 7 is non-trivial: enter plan mode first.
- **Toolchain**: every cargo and gate run needs `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.
- Sentrux: `git add` new files, scan `/Users/john/code/omnis/crates`, `check_rules`.
- Context: pipe test and gate runs through `grep`/`tail`; read files by range.
- Mutation passes: a scratchpad script of `(name, file, old, new)` restored in `finally`, run with
  `cargo test -p <crates> --no-fail-fast`, a break counts only by a named failing test (LESSONS
  2026-10-02); recreate the script (scratchpad is per session); match `cargo fmt`'s text (a break
  missed its pattern in step 6 after fmt rewrapped a line).

## State
- Branch `m7a-b-tasks`, **not pushed since `4b1bd14`**; tree clean (the side note was deleted on
  the owner's word).
- This session's commits since the last compact: `ed434d8` (event bus horizon, M8), `0f7a268`
  (6a: party view out of `ops.rs`), `68ec10b` (6b: the views, `react` word), then this file's.
- Gate `tests passed 511 failed 0 ignored 12`. Pins: tuple `(4, 4, 3, 24, 16, 18, 3, 34, 7)`, walk
  `7703481389045166225`, fight `4106035052781345400`, `SAVE_SCHEMA 6`, MCP 20 tools, proof 93/141.
- **Owner decisions for M7c** (2026-10-03, the TODO block and the plan): fight screen stays on the
  canvas; tactics = ARCH §4.7's whole shape, reactions only; monsters' opportunity attacks by a
  built-in rule; Cunning = bonus-action exchange without opportunity attacks + Hide; a turn also
  ends when nothing is left to pay for (ARCH §4.7 wording in step 8, shown first).
- **Owner, 2026-10-04**: the event bus with topics goes in M8 (clock ticks first, combat triggers
  after), tried and true external crates evaluated first; constraints in `horizons.md` (M8).

## What step 7 can read (built in step 6, `omnis-sim/src/view.rs`, `party_view.rs`)
- `CombatView`: `budget`, `reactions` (left, by `ActorRef`), `hidden`, `spells_cast`, `members:
  Vec<FighterView { index, reactions_on, features: Vec<FeatureView { index, name, cost,
  uses_left, blocked }> }>`; `SpellView.blocked` (action, now with budget and one-spell rule) and
  `SpellView.bonus` (bonus action), both from `combat::payable`; `StackView.points_left`,
  `shielded`.
- `MemberView.tactics: TacticsView { reactions_on, auto, reactions: Vec<ReactionView>, answers:
  Vec<AnswerView> }`.
- `combat::feature::refusal` exists for a choice-aware feature check (Cunning's exchange target).

## Next: step 7 (app), then step 8 (docs)
- **7. App**: React switch on the fight action row (`CombatIntent::Reactions` exists in
  `combat_menu.rs:205`, handled at `combat.rs:167`, unused in the UI); a budget line; features in
  the Use picker (or their own row); bonus pay in the cast picker (today always `Pay::Action`;
  read `SpellView.bonus`); the Feathers tactics panel (`UiScreen::Tactics`, TACTICS on the sheet:
  declare/remove reactions via `TacticsCommand`); headless tests. Monster cast log lines exist
  (`spell_text.rs`). App file sizes now: `screen.rs` 930 (near the 1,000 cap: split
  before growing), `combat_text.rs` 690, `combat_menu.rs` 648, `combat.rs` 194.
- **8. Docs**: ARCH v0.7 §4.7 as built (shown first) including monster casting, `EnemyCasts`'s
  raise, the one-spell rule, the views; PRD §14 line (ask); code-map, verification; the M7c
  review; `tasks/acceptance/m7c.md` (add: walk to Bob, his casts, both sides' shields, the saves);
  owner acceptance c closes M7.

## Carry-over and watch-outs
- `EnemyCasts` is raised at every monster cast and `SpellCast` at every member cast, but nothing
  answers either; `EnemyFlees`, `OwnTurn` have no source. Say so in step 8.
- No MCP or CLI test reaches a fight (`combat_get` through the pipe untested; generic serde).
- Files: `command.rs` 770, `ops.rs` 606, `view.rs` 322, `party_view.rs` 339, `monster_cast.rs` 451.
- A level-2 rogue or a fighter holding Action Surge always has a bonus or free option, so the
  turn waits for End (by design); the acceptance script should say so.
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
