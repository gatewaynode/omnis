# Continuity notes

Written 2026-10-05 before a compact. M7c steps 0–8b are committed, and so are the fixes from acceptance c so
far (B1, B2, the debug panel). M7 is still open until the owner finishes **acceptance c**; then commit 8c
closes it. Rewrite this file every time it is used, and keep it to state, next step and pointers. The durable
knowledge lives in `tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then wait for the owner's report. Don't start new work before it. The report should cover:
  - **B3:** a retest of gold, food and items through the new debug panel;
  - the rest of `tasks/acceptance/m7c.md`, by step number.
- **Toolchain:** every cargo and gate run needs `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.
- **Sentrux:** `git add` new files, scan `/Users/john/code/omnis/crates`, then `check_rules`.
- **Context:** pipe test and gate runs through `grep`/`tail`; read files by range.
- **Mutation passes:**
  - use a scratchpad `mutate.py` that takes a breaks JSON file of `(name, file, old, new)`, applies each
    break, runs `cargo test -p <crates> --no-fail-fast`, and restores the file in `finally`;
  - a break counts only when a named test fails (LESSONS 2026-10-02);
  - pass the breaks file by an **absolute path**, because the script `chdir`s;
  - recreate the script each session, since the scratchpad is per session;
  - run it in the background, and do not build crates that depend on the mutated crate meanwhile;
  - `| tail` holds back all output until the end.
- **Stashing:** when two commits' changes are in the tree, stash one with `git stash push -u -- <paths>` to
  gate the other. After `stash pop`, deletions come back **unstaged**: stage them with
  `git rm --cached` (C2 needed an amend for exactly this).

## State
- Branch `m7a-b-tasks`, **not pushed since `4b1bd14`**; the tree is clean after this file's commit.
- **This session's commits:**
  - `8491fd9` **B1**: a turn ends when its action is spent, unless a bonus-action spell can still be cast;
    features come before the action.
  - `34f7eec` **B2**: the weapon answers no trigger; the panel shows "nothing to declare yet". Then
    `a83e92e` (docs).
  - `6ae3aea` **C1** (sim): dev edits without artificial limits.
    - HP and SP may exceed their maximums; scores take any `u8`.
    - Constitution shifts `hp_max` and HP by the change in modifier times the level; a mental score
      recomputes the spell point pool.
    - `SetXp` grants levels free (`LevelUp`, cost 0) on a new `dev` stream; lowering XP never lowers the
      level.
  - `e300181` **C2** (app): the canvas debug menu is replaced by a Feathers panel.
    - Files: `debug_panel.rs` is the model; `feathers_debug.rs` holds `DebugPanelPlugin` (feature
      `devtools`, the backtick and Escape). `debug_menu.rs` keeps the view and `describe`.
    - `debug.rs` and `debug_screen.rs` are deleted.
    - `Payload::Commit(i64)` is reported after `Number` when a change is final (Enter, or leaving the field).
      The model ignores a repeated commit of the same value and one equal to the world's value.
    - The Feathers test app (`tests/common/mod.rs::feathers_app`) adds `DebugPanelPlugin` under `devtools`.
- Gate: `tests passed 533 failed 0 ignored 12`.
- **Pins:**
  - tuple `(4, 4, 3, 24, 16, 18, 3, 34, 7)`;
  - walk replay `7703481389045166225`;
  - fight replay **`13689675309031319555`** (rebaselined in B1: the fighter's `EndTurn` went);
  - `SAVE_SCHEMA 6`, MCP 20 tools, proof 93/141.
  - The measured tables are unchanged since 8a.
- **Owner decisions, 2026-10-04:**
  - B1: features are used before the action. ARCH §4.7 says so, as a departure from the SRD's free order.
  - B2: offer only what the game can fire. `horizons.md` holds two routes to reactions for the fighter,
    rogue and cleric: a flee source with opportunity attacks, and the Ready action.
  - The debug panel goes to Feathers with typed numbers. Lifted: the HP and SP caps, the 1–30 score limit,
    and "XP doesn't level"; derived numbers now follow a score. ARCH §8.1's plugin clause is updated.
- **BUGS.md:** B1 and B2 are fixed. **B3 is open:** "gold, food or items didn't stay", not reproduced
  headless. The persistence test is `tests/debug.rs::typed_numbers_reach_the_world_once_and_every_field_holds_after_close`.
- **LESSONS 2026-10-04:** a rule that keeps a turn open is counted in play, not only asserted. A test
  workaround for such a rule is a sign worth reporting to the owner.

## Next: finish acceptance c, then 8c
- **The owner retests:**
  - B3 through the panel;
  - steps 1, 5 and 8 (rewritten for B1 and B2);
  - every step not yet played, including Part 2 (Bob).
  - The setup now levels by typing XP 300 in the debug panel. The cleric's pick (Healing Word) is still made
    at the trainer at (7, 1).
- **Any failure:** a bug in `tasks/BUGS.md` with a test that reproduces it (red first), then the fix (plan
  mode if non-trivial).
- **All pass → commit 8c:**
  - TODO step 8 `[x]` with the acceptance result;
  - mark M7's heading closed in `tasks/TODO.md`;
  - mark the plan `tasks/plans/m7c-turn-budget.md` closed;
  - close B3 in BUGS.md if the retest passes.
- **After M7:** M8 (subjective time; the event bus with topics, external crates evaluated first, in
  `horizons.md`). Plan it in plan mode when the owner asks.

## Carry-over and watch-outs
- **Not built:**
  - `SpellCast` and `EnemyCasts` are raised but nothing answers them;
  - `EnemyFlees` and `OwnTurn` have no source;
  - items, features and the weapon declare nothing as reactions;
  - no pack declares flags, so the debug panel's flag menu is empty.
- **Test gaps:**
  - no MCP or CLI test reaches a fight;
  - no app test casts a bonus-action spell by pointer;
  - the sheet's `in_fight` guard is unreachable through the UI and untested.
- **UI:** the Use picker shows six rows and the canvas has no scroll. Feathers dropdowns do not scroll
  either (use a list pane for long lists).
- **Files:**
  - over 800 lines: `plan.rs` 953, `bake.rs` 934, `screen.rs` 880;
  - near 800: `tactics_panel.rs` 778, `command.rs` 761.
- **Dated:** Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
