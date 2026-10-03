# Continuity notes

Written 2026-10-03 before a compact. M7a is closed, the owner has pushed `m7a-b-tasks`, and CI
is running. Rewrite this file every time it is used; keep it to state, next step and pointers.
The durable knowledge lives in `tasks/knowledge/` (start at its README). The tree is clean and
the last code commit passed the gate.

## On resuming
- Run `/catchup`, then read `tasks/knowledge/README.md`, `agreements.md`, `verification.md` and
  `code-map.md` before planning.
- **Toolchain**: inside the sandbox every cargo and gate run needs
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools` (the sandbox cannot read Xcode's license
  plist). A cached gate proves nothing about the linker (LESSONS 2026-10-02).
- Sentrux: scan `/Users/john/code/omnis/crates` (rules in `crates/.sentrux/rules.toml`).
- Ask the owner for the CI result on the push. On red, fix CI before anything else. M7b does not
  start on a red CI.

## State
- Branch `m7a-b-tasks` (from `main` at `b002b09`). Pushed through `ba79a29` (step 9 done).
  `2cd54d9` and this note's commit are local, docs only. The owner opens the PR and merges.
- **M7a closed (2026-10-03):**
  - steps 0–9 done;
  - owner acceptance a passed (`tasks/acceptance/m7a.md`, steps 1–11);
  - step 12 (frame time) waived;
  - the M7a review is in `tasks/TODO.md` under step 9;
  - ARCHITECTURE v0.5 matches M7a, with no gamepad (owner: keyboard and mouse first).
- Gate `VERIFY-GREEN`, `tests passed 458 failed 0 ignored 8`. Pins: tuple
  `(4, 4, 3, 24, 16, 11, 3, 31, 7)`, walk `9901411989274517557` (`WALK_SEED = 2`), fight
  `15728260309841309156`, `SAVE_SCHEMA 5`, MCP 20 tools / `oneOf` 11 / proof 77/111.
- **The track's focus (owner, 2026-10-03):** mechanics first. Feathers panels remain the UI
  choice while an alternate UI is explored on a separate branch. New screens are minimal
  Feathers panels on the existing kit, enough for manual testing. Effort goes to rules, sim,
  content balance and tests; no UI polish or UI-only measurement unless asked.

## Next: M7b planning (plan mode), after the catchup
- Read `tasks/plans/m7-town.md` (M7b, steps 10–13, and the done-when: create a party, clear the
  dungeon, return to town, level up) and the TODO's M7 block.
- Read PRD §8.2 (the leveling loop's D-table) and ARCH §4.5 and §4.7 cold.
- The scope as planned:
  - **10. Rules and sim:** `omnis-rules/level.rs` (`ready`, `level_up`: average HP,
    proficiency, the pool, features as labels), `spells_per_level`, the `max_spell_level` table,
    `ServiceCommand::Train` and `Learn`, and `Event::LevelUp` and `SpellLearned`.
  - **11. Content, measured first:** goblins, skeletons, a deeper dungeon wing, and four
    2nd-level SRD spells. The table: XP and gold per clear for parties of 2, 4 and 6, wipe
    rate, clears to levels 2 and 3, prices against income. Then the tuple and a rebaseline.
  - **12. Ops, MCP, app:** schema arms, trainer and guild rows in `service_view`, minimal panel
    bodies, XP and next level on the sheet, log lines, tests.
  - **13.** Docs, review, and owner acceptance b (the done-when).
- Size from the review: `crates/*/src` grew net +4,940 lines in M7a alone, which already
  reaches the plan's "about 5,000 before M7c". Re-estimate M7b from that.
- Trainer and guild already exist as `ServiceKind`, data files and price slots
  (`services.ron`: `trainer.cost` = `level * 1000` cp, a placeholder since the SRD prices no
  trainer; `guild.spell_cost` = `spell_level * 5000` cp). They have no commands yet.
- Watch these files, all over 800 lines: `plan.rs` 953, `bake.rs` 934, `screen.rs` 929,
  `ops.rs` 817 (a new view goes in its own file, like `service_view.rs`), `combat_text.rs`
  815, `combat_menu.rs` 810.

## Carry-over
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
- Merged local branches (`m5-tasks`, `m6c-tasks`, `m6-closeout-tasks`, `m7-tasks`) can be
  deleted by the owner.
