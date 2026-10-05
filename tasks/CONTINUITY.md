# Continuity notes

Written 2026-10-05 before a compact. M7c steps 0–8b are committed, and so are the fixes from acceptance c
(B1, B2, the debug panel). The owner's retest passed B3. M7 is still open until **step 8 (commit 8c)** is
done. Rewrite this file every time it is used, and keep it to state, next step and pointers. The durable
knowledge lives in `tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`. The owner said "let's prepare for another compact before we push through step 8": the
  next work is **commit 8c** (below). Confirm with the owner before starting if anything is unclear.
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
- **Stashing:** after `stash pop`, deletions come back **unstaged**: stage them with `git rm --cached`.
- **Seeing the canvas headless:** a throwaway test can write `common::frame(&app).frame.raster` as a PPM
  (`P6 w h 255`, RGB of each RGBA pixel), then `sips -s format png x.ppm --out x.png` and Read it. Delete the
  test afterwards.

## State
- Branch `m7a-b-tasks`, **not pushed since `4b1bd14`**; the tree is clean after this file's commit.
- **Commits since the last push** (newest last): `6b622d0` (8b), `2199b74` (docs), `8491fd9` **B1**,
  `34f7eec` **B2**, `a83e92e` (docs), `6ae3aea` **C1** (dev edits without limits), **`f9940d6` C2** (the
  Feathers debug panel; it was amended, so `e300181` in older notes is wrong), `085ad32` (docs), then this
  file.
- Gate: `tests passed 533 failed 0 ignored 12`.
- **Pins:**
  - tuple `(4, 4, 3, 24, 16, 18, 3, 34, 7)`;
  - walk replay `7703481389045166225`;
  - fight replay **`13689675309031319555`** (rebaselined in B1);
  - `SAVE_SCHEMA 6`, MCP 20 tools, proof 93/141.
  - The measured tables are unchanged since 8a.
- **Owner's report, 2026-10-05:**
  - "Gold food and character edits stick and are easier to do now." **B3 passes.**
  - **Log anomaly, not a bug yet:** "everything worked, just not everything was visible in the logs";
    first described as no cast line for party casters, only the effects. **Not reproduced:** headless, a
    wizard's Fire Bolt logs "Ilvara casts Fire Bolt (free)" above its hit or miss in the EVENTS column at
    1280×720 and ultrawide. Every party path emits `Event::SpellCast` before its effect: `casting.rs`
    (explore), `combat/turn.rs` and `combat/reaction.rs`, all through `combat/cast.rs::pay`. The text is
    `spell_text.rs::spell_line`. The message line at the top of the band shows only the batch's **last** line
    (`ui.rs::message_line`), which for a cast is the effect: the likeliest explanation, unconfirmed. The owner
    will send a screen capture next time; then it becomes B4 (red test first) if real.
- **BUGS.md:** B1 and B2 are fixed. B3 is still marked open in the file and is closed in 8c.

## Next: commit 8c, which closes M7
- `tasks/TODO.md`: step 8 `[x]` with the acceptance result (B1, B2, the debug panel, B3 passed; the log
  anomaly awaiting a capture); mark the **M7 heading** (line 227) closed.
- `tasks/plans/m7c-turn-budget.md`: mark closed.
- `tasks/BUGS.md` B3: **closed**, "the owner's retest with the Feathers panel passed (2026-10-05); never
  reproduced with the canvas menu; the panel's test guards it", fixed with `f9940d6`.
- Run the gate and Sentrux even for a docs commit if anything else changed; stage by name; do not push.
- Then tell the owner M7 is closed and that pushing is theirs. **After M7:** M8 (subjective time; the event
  bus with topics, external crates evaluated first, in `horizons.md`), planned in plan mode when asked.

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
