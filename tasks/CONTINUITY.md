# Continuity notes

Written 2026-10-03 before a compact. M7b is closed; M7c (the turn budget) is next and has not
been planned. Rewrite this file every time it is used; keep it to state, next step and pointers.
The durable knowledge lives in `tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then read `tasks/knowledge/README.md`, `agreements.md`, `verification.md` and
  `code-map.md` before planning.
- **Toolchain**: inside the sandbox every cargo and gate run needs
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools` (the sandbox cannot read Xcode's license
  plist). A cached gate proves nothing about the linker (LESSONS 2026-10-02).
- Sentrux: `git add` new files, scan `/Users/john/code/omnis/crates`, `check_rules`.
- Ask the owner for the CI result on the latest push before starting M7c; on red, fix CI first.

## State
- Branch `m7a-b-tasks`, pushed through `5e6fd1a` (ARCH v0.6). The TODO's step 13 check-off and
  this note are local docs commits. The owner opens the PR and merges.
- **M7b closed (2026-10-03)**: steps 10–13 done; the owner's manual test of acceptance b
  (`tasks/acceptance/m7b.md`): "Manually tested.", no failures reported. Review in
  `tasks/TODO.md` under step 13. ARCHITECTURE v0.6 matches M7b.
- Gate `VERIFY-GREEN`, `tests passed 473 failed 0 ignored 9`. Pins: tuple
  `(4, 4, 3, 24, 16, 18, 3, 31, 7)`, walk `251448457721971531` (`WALK_SEED = 2`), fight
  `15358139695133922299`, `SAVE_SCHEMA 5`, MCP 20 tools / `oneOf` 11 / `service_schema` 12 /
  proof 80/114.
- Built in M7b: `omnis-rules/src/level.rs`; `ServiceCommand::Train`, `Choose` (free pick a level
  owes, `Character.spell_picks`), `Learn` (guild or temple, `spell.learn_cost`) in
  `omnis-sim/src/service_level.rs`; `Event::LevelUp`, `SpellLearned`; `service_view` offers;
  `game.status.groups_cleared`; the depths map; seven spells; `measure.rs::clear_over_seeds`.
- **Owner guidance this session** (saved as memories): tune content for a party of **four**;
  the game is **open-world, not hack-and-slash** — a map is passed by skill, guile, creativity
  or combat, so balance tables are sanity checks; level 2 per clear is enough; the trainer's
  price stays `level × 1000` cp. Mechanics first; minimal Feathers panels for manual testing.

## Next: M7c planning (plan mode), after the catchup
- Read the M7c outline in `tasks/plans/m7-town.md` ("M7c (outline …)"), ARCH §4.7 (the turn
  budget, tactics, schema 6), PRD §8.2/§8.3 rows D21–D24, and the TODO's M7c line.
- Scope as outlined: slots `turn.actions`, `turn.bonus_actions`, `turn.reactions`; several
  commands per turn and `CombatCommand::EndTurn`; costs on spells, items and features; D24's
  three spell fields; declared reactions with the closed trigger list; `Character.tactics`
  replacing `auto_cast` (save schema 6, with a migration and fixture); Second Wind, Action
  Surge, Cunning Action with effect; the fight screen to `bevy_ui` (re-check against the
  mechanics-first focus: ask the owner whether the fight screen move stays in M7c). Measured
  over seeds before content (R12, `tests/measure.rs`).
- Size: M7b came in at src +1,022 / tests +1,245 against an estimate of 1,300 / 1,050.

## Carry-over and watch-outs
- Files over 800 lines: `plan.rs` 953, `bake.rs` 934, `screen.rs` 929, `ops.rs` 829,
  `combat_text.rs` 815, `combat_menu.rs` 810. M7c touches combat: `combat_menu.rs` and
  `combat_text.rs` will need splitting before they grow.
- Content ids are interned in file order; new content renumbers them and old saves are refused
  (horizon: string ids). Schema 6 in M7c is a natural moment to raise it with the owner.
- The wizard's level-2 picks have nothing eligible (its list's four first-level spells are all
  known at level 1); more SRD wizard spells would fix it (content, not rules).
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
- Merged local branches (`m5-tasks`, `m6c-tasks`, `m6-closeout-tasks`, `m7-tasks`) can be
  deleted by the owner.
