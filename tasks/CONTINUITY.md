# Continuity notes

Written 2026-10-02 after M7 step 8a. Rewrite this file every time it is used; keep it to state,
next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then read `tasks/knowledge/README.md`, `agreements.md`, `verification.md` and
  `code-map.md` (the tool bar and service panel entries) before touching code.
- **Toolchain**: inside the sandbox every cargo and gate run needs
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools` (the sandbox cannot read Xcode's license
  plist, so `xcodebuild -find clang` fails; the license itself is accepted). See `verification.md`.
- Sentrux: scan `/Users/john/code/omnis/crates` (the rules file is `crates/.sentrux/rules.toml`).

## State
- Branch `m7-tasks`. Pushed up to `5bce8b7`; every later commit is unpushed.
- M7 steps 0–7 and 8a done. Step 8 was split by the owner (plan `tasks/plans/m7-step8.md`):
  8a moved the canvas tool pad to a Feathers bar (seven buttons, CAMP dim); 8b is the camp panel.
- Gate `tests passed 447 failed 0 ignored 8`; pins unmoved: tuple `(4, 4, 3, 24, 16, 11, 3, 31, 7)`,
  walk `9901411989274517557` (`WALK_SEED = 2`), fight `15728260309841309156`; `SAVE_SCHEMA 5`;
  MCP 20 tools, `oneOf` 11, proof 77/111.
- Owed by the owner: a manual look at the 8a bar (on the ultrawide too); agent-launched windows
  draw no frames.
- ARCHITECTURE.md §8 line on `UiPlugin` still says it composes the "tool pad"; it is drift for
  step 9's "as built" pass (ask the owner before editing ARCH).

## Next: M7 step 8b (the camp panel), per `tasks/plans/m7-step8.md` part "8b"
- Commit 2: `omnis-sim` `rest_view.rs` (read-only; `food_need` split out of `rest.rs`).
- Commit 3: the app: `PlayState::Camp`, `camp_panel.rs` + `feathers_camp.rs`, CAMP live on the map
  (`tool_bar::tool_states`, `tool_for`), R in `input::shell_for`, `follow_mode` gains `Camp`,
  hit dice on the sheet's stats page row 4. Then step 9 (docs, acceptance a).

## Carry-over
- Owed by the owner at acceptance a: `--frame-stats` with a panel open.
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
- Merged local branches `m5-tasks`, `m6c-tasks`, `m6-closeout-tasks` can be deleted by the owner.
