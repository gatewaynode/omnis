# Continuity notes

Written 2026-10-04 before a compact. M7c steps 0–8b are committed; the only thing left of M7 is the owner's
**acceptance c** and then commit 8c (closing M7). Rewrite this file every time it is used; keep it to state,
next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then wait for the owner's acceptance report (pass or fail by step number of
  `tasks/acceptance/m7c.md`). Don't start new work before it.
- **Toolchain**: every cargo and gate run needs `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.
- Sentrux: `git add` new files, scan `/Users/john/code/omnis/crates`, `check_rules`.
- Context: pipe test and gate runs through `grep`/`tail`; read files by range.
- Mutation passes: a scratchpad script of `(name, file, old, new)` restored in `finally`, run with
  `cargo test -p <crates> --no-fail-fast`. A break counts only when a named test fails (LESSONS 2026-10-02).
  Pass the breaks file by an **absolute path**, because the script `chdir`s. Recreate it, since the scratchpad
  is per session. Run it with `python3 -u` in the background and touch no file of the crate under test
  meanwhile.

## State
- Branch `m7a-b-tasks`, **not pushed since `4b1bd14`**; tree clean after this file's commit.
- This session's commits:
  - `f70a2c0` (7a), `89e8683` (7b), `d11c67f` (continuity);
  - `4a1fbd7` (8a: one-spell rule reverted);
  - `6b622d0` (8b: ARCH v0.7, PRD §14 notes, knowledge files, the M7c review, the acceptance script);
  - then this file's commit.
- Gate: `tests passed 531 failed 0 ignored 12`.
- Pins: tuple `(4, 4, 3, 24, 16, 18, 3, 34, 7)`, walk `7703481389045166225`, fight `4106035052781345400`,
  `SAVE_SCHEMA 6`, MCP 20 tools, proof 93/141.
- **Owner decisions, 2026-10-04 (step 8):**
  - **Revert 4b; PRD D24 stands.** A bonus-action spell does not limit the action to a cantrip, so two
    spells a turn are possible within the budget.
  - **The PRD §14 notes are added** (preparation; runbooks).
  - **ARCH §4.7's text was shown in full and approved** ("write it").
- LESSONS 2026-10-04: an SRD rule the PRD set aside is not "missing". Check the D-table "Rejected" column and
  §8.3 before adding one. The memory `srd-is-the-default-direction` notes the same.

## Next: acceptance c, then 8c
- **The owner plays `tasks/acceptance/m7c.md`:**
  - setup: fighter, rogue, cleric, wizard; XP 300 by the debug menu; train; the cleric picks Healing Word;
  - Part 1: steps 1–8;
  - Part 2: Bob, steps 9–11 (route: the depths' doors (2, 5), (5, 8), (11, 8), then Bob at (13, 8)).
- **Any failure:** a bug in `tasks/BUGS.md` with a test that reproduces it, then the fix (plan mode if
  non-trivial). Re-run the step with the owner.
- **All pass → commit 8c:**
  - TODO step 8 `[x]` with the acceptance result;
  - mark M7's heading closed in `tasks/TODO.md`;
  - mark the plan `tasks/plans/m7c-turn-budget.md` closed;
  - ARCH/PRD unchanged unless the owner asks.
- After M7: M8 (subjective time; the event bus with topics, external crates evaluated first, in
  `horizons.md`). Plan it in plan mode when the owner asks.

## Carry-over and watch-outs
- **Not built** (stated in ARCH §4.7, the review and the acceptance script):
  - `SpellCast` and `EnemyCasts` are raised but nothing answers them;
  - `EnemyFlees` and `OwnTurn` have no source;
  - items and features declare nothing as reactions.
- **Test gaps:**
  - no MCP or CLI test reaches a fight;
  - no bonus-action cast by pointer in the app tests;
  - the sheet's `in_fight` guard is unreachable through the UI and untested.
- The Use picker shows six rows; the canvas has no scroll.
- **Files:**
  - over 800 lines: `plan.rs` 953, `screen.rs` 935 (split before growing), `bake.rs` 934;
  - near 800: `tactics_panel.rs` 771, `command.rs` 761, `combat_text.rs` 708, `combat_menu.rs` 707.
- **Dated:** Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
