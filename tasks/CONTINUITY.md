# Continuity notes

Written 2026-10-17, after the save model (D28) was planned and approved, before a compact. Rewrite this file every
time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes). Untracked and never staged: the four files under
  `assets/` and `docs/dnd_supplemental/` (the owner's, in progress).
- **M8.5 build order:** 1a, 4, 4b, 1b, 3, 1c, 2 (2a, 2b, 2c), all accepted (2a–2c by the owner 2026-10-17,
  "Manually tested"). Then **7, now the save model (D28), sub-steps 7a–7h**, then the editor (M5), 5, 6.
- **Today's later commits:** `aa1475e` (study answered, step 2 accepted), `1f195ca` (the step 7 plan).
- **Gate:** last run `tests passed 861 failed 0 ignored 12`, VERIFY-GREEN at `2deb324` (docs-only commits since).
  Sentrux rules pass, quality 8988.

## Next: 7a, the journal and measurement
- Read `tasks/plans/m8.5-save-model.md` in full first (the approved plan: shape, sub-steps, verification).
- 7a builds `World.journal` (`#[serde(skip)]`, not fingerprinted), appended in `apply`
  (`crates/omnis-sim/src/apply.rs:19-79`) for each accepted command: `Entry { minute, command }`, minute = the
  party's age AFTER the command (as the client's History stamps it, `time_view.rs:107`); events kept in memory
  only, without `Visible` and per-tile events. A replay must rebuild the journal identically, so `World:
  PartialEq` stays meaningful (watch tests comparing a world against one loaded from the old whole-World save:
  the loaded one has an empty journal).
- Measurement: a release-mode timing harness (`omnis replay --time` and an ignored test) driving a seeded
  wandering party for 10k and 50k commands on the test packs: µs per command for apply and for replay, bytes
  per journal entry. No timings exist anywhere today. Commands per hour need the owner's play (the window's
  `Session::save_log`, `.omnis/vector-session.ron`, which does not exist yet): ask the owner for a session.
- Visible end: the debug World page shows the journal's length and "replay would take ~T ms".
- Plan mode for 7a's detail is likely warranted (owner's practice for larger items); write an acceptance script
  `tasks/acceptance/m8.5-7a.md`.

## The save model as decided (owner, 2026-10-17; PRD D28, §7.7, §14; study `tasks/plans/world-state.md`)
- Character save = checkpoint (party, party time and clock, party-side contacts, RNG streams, automap,
  position, mode, bus, party flags, settings, turn) at the last fold + the journal since + the bases it names.
- World folder `.omnis/worlds/<w>/`: manifest + `world/<hash>.ron` + `regions/<region>/<hash>.ron`; bases only.
  **Layers are derived from the journal** (replayed on load). Base 0 = the authored packs, no file.
- Changed pack: migrate (7h), then report what did not fit; the player loads without it or cancels.
- Flags scoped per flag (party or world; undeclared = world). Folds are visible ("The world has moved on").
  Named bases kept; dropping unnamed ones is a manual tidy (departure on timing only, in the plan).
- **Provisional size fold** at safe points (long rest, entering a settlement) when the journal passes a rule value
  set from 7a's measurement; M10's world sim (owner: a simplified 4X engine over biospheres, settlements, regions
  and the world) sets real thresholds later. Story points (M11) fold too.
- Pre-fold history archived as events beside the save. Characters may carry between worlds (a horizon).
- Facts found: saving mid-fight is allowed and tested (kept); the 65,536 collection limit applies to packs, not
  saves (4 MiB file limit is the save's); bases carry registry numbers until 7h, so the `PackMismatch` gate stays
  until then; `ops.rs` 782 and `shell/session.rs` 822 lines, so new code goes in new modules.

## The owner's way of working (keep)
- Each item ends with something visible or clickable, and the report says how to try it (numbered: where to
  stand, what to press, what appears). The owner replies "Manually tested" and asks for the next.
- Hand-test launch: `just run --windowed --pack packs/base --pack packs/test --pack
  crates/omnis-sim/tests/packs/objects`, New game, devtools on; teleport with Esc, Debug, the World page, to
  `objects:map:yard`.
- Offscreen captures: `scratchpad/yardshot.sh name x y facing walk [--screen title|new-game|create-party|pause|
  actions|verbs|inventory]` (`cargo build -p omnis-vector` first). A capture cannot run a command first.
- Bug reports: fix directly, with a red test, a lesson in `tasks/LESSONS.md`, then report. Choices with real
  alternatives: AskUserQuestion with a recommended option first; larger additions get plan mode.


## Pins
- Replays: walk `8d880647a25097b2`, fight `d06fa80460a947ef` (unchanged by 2c). `SAVE_SCHEMA` 7,
  `PROTOCOL` 2.
- MCP: command `oneOf` 14, item `oneOf` 10, schema proof 116/166, tools 26, ops 26. `Command` 16, `Event` 72,
  `Rejection` 71. Base pack: 19 spells.

## Open, carried over
- `assets/README.md` font rows: the owner's file; report only, never edit.
- `docs/dnd_supplemental/`: the owner's, in progress; ask before committing anything there.
- P6 colours unconfirmed; step 5 revisits colour roles (joins, fills, crest, objects, beings).
- The automap still records the forward cone (§12.5); no objects or beings on it (owner, 2026-10-14).
- The full log: whether L types an "l" into the filter on the frame it opens was left to the owner's eye
  (the owner hand-tested the log on 2026-10-17 and reported nothing).
- Dependencies: the rhai Socket re-audit (ask when); `smartstring` RUSTSEC-2026-0249; `ttf-parser`
  RUSTSEC-2026-0192; Bevy 0.20 not before 2026-11-07.

## Process
- **Cargo and git:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools` for both (git refuses without it:
  the Xcode licence). `cargo fmt --all` first and again after any appended test. Workspace clippy with
  `--all-targets --features omnis-vector/test-fixtures -- -D warnings` before every commit. Sim tests:
  `cargo test -p omnis-sim --test integration <filter>`; client `-p omnis-vector --features test-fixtures
  --test integration <filter>`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<item>.txt 2>&1`, VERIFY-GREEN, no edits while it runs. Then
  Sentrux `rescan` and `check_rules`.
- **Tests red first; mutations** with a Python runner in the scratchpad (anchor checked unique, source restored
  in `finally`, `python3 -u` to a file), then `git status` to confirm the sources are clean.
- **Each new command, event or rejection** also moves: `docs/api.md` and its count lines, `api_doc.rs` pins
  and `variants!` lists, the MCP schema (`schema.rs` and its `oneOf` count test) and `schema_proof.rs`
  (instances, offered, the drift test's index), `common/commands.rs`, the word in `word.rs`, client
  `labels::refused`.
- **Commits:** stage new files by name; build the path list from `git status --short` without `assets/`,
  `.claude/` or `docs/dnd_supplemental/`, write it to a scratchpad file, `git commit --pathspec-from-file=<file>
  -F -`. Messages end with the Co-Authored-By and Claude-Session lines. Never push. Owner world docs
  (`docs/background`, `docs/locations`) go in their own commit, unedited.
- **Context:** pause and recommend a compact past 350K.
