# Continuity notes

Written 2026-10-02 after M7 step 7, before a client restart. The tree is clean, and HEAD passed
the gate (444/8). Rewrite this file every time it is used; keep it to state,
next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then read `tasks/knowledge/README.md`, `agreements.md`, `verification.md` and
  `code-map.md` (the service panel entry) before touching code.
- **First, the toolchain** (written before a client restart for new sandbox rules):
  - On 2026-10-02 Xcode 26.5's license was still unaccepted. Unsandboxed, `xcodebuild -find clang`
    said so, `/Library/Preferences/com.apple.dt.Xcode.plist` did not exist, and
    `xcodebuild -checkFirstLaunchStatus` exited 69.
  - The owner was asked to run `sudo xcodebuild -license accept` and
    `sudo xcodebuild -runFirstLaunch` in their own Terminal (`sudo` is blocked in the session).
  - Check with `xcrun --find clang` and `xcodebuild -checkFirstLaunchStatus` (0 is ready).
  - Until both pass, prefix cargo and `scripts/verify.sh` with
    `DEVELOPER_DIR=/Library/Developer/CommandLineTools` (process-local; it links fine).
  - The owner also changed the sandbox to fix access in some directories: re-check that the
    plist is readable from inside the sandbox.
- Sentrux: scan `/Users/john/code/omnis/crates` (the rules file is `crates/.sentrux/rules.toml`).

## State
- Branch `m7-tasks`. Pushed up to `5bce8b7`; every later commit is unpushed (4a, 4b, 3c, 5, 6,
  7 and the docs commits).
- M7 steps 0–7 done; the owner's manual test of step 7 passed (2026-10-02). What they
  saw: walking into any of the seven services (after the Enter question) shows a panel over the
  map; buy, sell, eat, hear a rumor, be treated or raised, bank, take a room; Leave or Escape
  asks "Leave the …?"; the trainer and guild say M7b. Not yet: training and spells (M7b), counts
  above one per press, camp (step 8).
- Gate `tests passed 444 failed 0 ignored 8`; pins unmoved: tuple `(4, 4, 3, 24, 16, 11, 3, 31, 7)`,
  walk `9901411989274517557` (`WALK_SEED = 2`), fight `15728260309841309156`; `SAVE_SCHEMA 5`;
  MCP 20 tools, `oneOf` 11, proof 77/111.
- Agent-launched windows draw no frames: no captures; the text trees (`screen_text`) stand in.

## Next: M7 step 8 (camp panel), per `tasks/plans/m7-town.md`
- Plan mode first. Its door is measured first from three captured candidates the owner picks
  from (captures need the owner's machine); `PartyView.long_rest_wait` and `hit_dice_left` are
  ready; R as the key; hit dice on the sheet. Then 9 (docs, acceptance a).

## Carry-over
- Owed by the owner at acceptance a: `--frame-stats` with a panel open.
- Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10.
- Merged local branches `m5-tasks`, `m6c-tasks`, `m6-closeout-tasks` can be deleted by the owner.
