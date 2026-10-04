# Continuity notes

Written 2026-10-04 mid-M7c, before a compact. Steps 0–7 of M7c are committed, plus the one-spell
rule (4b) and Bob the Rat King (5b); step 8 (docs, review, acceptance c) remains and closes M7.
Rewrite this file every time it is used; keep it to state, next step and pointers. The durable
knowledge lives in `tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then read `tasks/plans/m7c-turn-budget.md` §"Step 8" and the M7c block of
  `tasks/TODO.md` (each done step records what was built, deviations, tables, sizes). Step 8 is
  non-trivial: enter plan mode first.
- **Toolchain**: every cargo and gate run needs `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.
- Sentrux: `git add` new files, scan `/Users/john/code/omnis/crates`, `check_rules`.
- Context: pipe test and gate runs through `grep`/`tail`; read files by range.
- Mutation passes: a scratchpad script of `(name, file, old, new)` restored in `finally`, run with
  `cargo test -p <crates> --no-fail-fast`, a break counts only by a named failing test (LESSONS
  2026-10-02); recreate the script (scratchpad is per session); match `cargo fmt`'s text; run it
  with `python3 -u` in the background and touch no file of the crate under test meanwhile (new
  files not yet in `lib.rs` are safe; a new `tests/*.rs` is not).

## State
- Branch `m7a-b-tasks`, **not pushed since `4b1bd14`**; tree clean.
- This session's commits: `ed434d8` (event bus horizon, M8), `0f7a268` (6a), `68ec10b` (6b),
  `3898ce2` (continuity), `f70a2c0` (7a: fight screen), `89e8683` (7b: tactics panel + step 7's
  TODO entry), then this file's.
- Gate `tests passed 531 failed 0 ignored 12`. Pins: tuple `(4, 4, 3, 24, 16, 18, 3, 34, 7)`, walk
  `7703481389045166225`, fight `4106035052781345400`, `SAVE_SCHEMA 6`, MCP 20 tools, proof 93/141.
- **Owner decisions for M7c** (2026-10-03, the TODO block and the plan): fight screen stays on the
  canvas; tactics = ARCH §4.7's whole shape, reactions only; monsters' opportunity attacks by a
  built-in rule; Cunning = bonus-action exchange without opportunity attacks + Hide; a turn also
  ends when nothing is left to pay for (ARCH §4.7 wording in step 8, shown first).
- **Owner, 2026-10-04**: the event bus with topics goes in M8, tried and true external crates
  evaluated first; constraints in `horizons.md` (M8).

## What step 7 built (for step 8's docs and acceptance script)
- Fight (canvas): React `o` = `Action(7)`; budget line `Action N   Bonus N   Reaction N (on|off)`;
  Use picker = features first (Cunning Action as "exchange" and "hide" rows), then items; cast
  picker pays `BonusAction` when `SpellView.bonus` is open; log lines for `FeatureUsed`,
  `OpportunityAttack` (`round_text.rs`). End already existed.
- Sheet: TACTICS button `Row(4)` (key `t`), grey in a fight (the tool bar keeps the sheet shut
  in a fight anyway) → `PlayState::Tactics`; Close/Escape back to the sheet.
- Tactics panel (`tactics_panel.rs`, `tactics_draft.rs`, `feathers_tactics.rs`): member menu,
  reactions checkbox, declared rows (Edit, Remove), action and trigger menus from
  `TacticsView.answers`, all/any of up to 15 conditions (eight kinds), Add, Save (new or over the
  edited row; generated name "Shield on attacked"), New; a deeper tree set over MCP is read-only.
- Tests: `omnis-app/tests/tactics.rs` (7), `tests/combat.rs` React and Second Wind by pointer.

## Next: step 8 (docs, review, acceptance c)
- ARCH v0.7 §4.7 as built (**show the text to the owner first**): budget, features, declared
  reactions, the turn-ending rule, monster casting, `EnemyCasts`'s raise, the one-spell rule,
  the views, the tactics panel; §9 tool rows.
- PRD §14 line that the preparation and runbook questions remain open (**ask first**).
- `tasks/knowledge/code-map.md`, `verification.md`; the M7c review in the TODO (commits, tests,
  rebaselines, pins, size against the estimate, files over 800 lines).
- `tasks/acceptance/m7c.md`: the plan's eight steps, each naming its test, plus walk to Bob, his
  casts, both sides' shields, the saves; the shield declared on the tactics panel; say that a
  level-2 rogue or a fighter holding Action Surge waits for End.
- Owner acceptance c closes M7 (mark M7 closed in the TODO and the plan after it).

## Carry-over and watch-outs
- `EnemyCasts` and `SpellCast` are raised but nothing answers them; `EnemyFlees`, `OwnTurn` have
  no source; items and features declare nothing. Say so in step 8.
- No MCP or CLI test reaches a fight (`combat_get` through the pipe untested; generic serde).
- No app test casts a bonus-action spell by pointer (recruits are fighters; unit-tested).
- Use picker shows six rows (canvas has no scroll); a long kit is cut after the features.
- Files over 800: `screen.rs` 935 (mostly tests; split before growing). Others near: `combat_text.rs`
  708, `combat_menu.rs` 707, `combat_screen.rs` 683, `tactics_panel.rs` 771, `command.rs` 770.
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
