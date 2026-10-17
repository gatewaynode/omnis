# Continuity notes

Written 2026-10-17, after M8.5 step 2c was built, ahead of the owner's walk of its acceptance script. Rewrite
this file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes). Untracked and never staged: the four files under
  `assets/` and `docs/dnd_supplemental/` (the owner's, in progress).
- **M8.5 build order:** 1a, 4, 4b, 1b, 3, 1c, **2 (2a, 2b, 2c accepted by the owner 2026-10-17)**, 7 (saves by name, now
  waiting for the save-model study), editor (M5), 5, 6.
- **Today's commits (2026-10-17):**
  - 2b refinements, each hand-tested by the owner: notice fade and close `f7a5f93`; the full log `d599c60`
    (L, the "Log  L" button, a click on the running log; filter; in-memory history).
  - PRD D28 and the study `8d489c4`.
  - The owner's world doc `ae20844`.
  - 2c: data `6b44f52`, engine `b1689ad`, client `2deb324`, docs and acceptance `c68bf6a`.
- **Acceptance:** the owner walked 2a, 2b and 2c ("Manually tested, good progress so far"); step 2 is closed in TODO.
- **Gate:** `tests passed 861 failed 0 ignored 12`, VERIFY-GREEN at `2deb324`. Sentrux rules pass, quality 8988.

## The save model (PRD D28, owner 2026-10-17)
- The save centres on the characters and their interactions, replayed from the last base state; a world file
  apart, which only the simulation writes: immutable authored bases, layers of change, folds at thresholds
  and story points.
- Study `tasks/plans/world-state.md`: the six questions were answered on 2026-10-17 and written into the study,
  PRD D28, §7.7 and §14. In short: migrate, then report a changed pack (the player loads without it or cancels);
  flags scoped per flag (party or world); a world directory with one file per region under a manifest; characters
  carry between worlds (a horizon); folds visible to the player; the bases a save names are kept.
- Still open: the fold thresholds (PRD §14). Owner: they come from the world simulation, a simplified 4X engine over
  biospheres, settlements, regions and the world, so they wait for its mechanics. The save model builds the fold
  as a mechanism with no threshold of its own.
- Until it is built, a save is the whole `World`, and the full log's history lasts the session.
- Lesson of the day: when a feature has no home in a core structure, question the structure before
  wedging it in.

## 2c as built (for the owner's walk and questions)
- **Data:** `FixedEncounter.stand: Option<Stand { id, pos, facing }>`; `Monster.look` (a `Figure`) or
  `figure_for(size)`; `Shape::Figure`. A being's id is shared with objects' ids; its figure keeps the spill
  rules. Yard: two pack rats `rat` at (5, 1), pos (1, 0), facing West, `once`.
- **Engine:** `beings.rs` (`BeingView`, `in_sight`, `on_level`, `reached`, `approach`), `encounter::meet`;
  `ViewportModel.beings`, `ObjectsView.beings`, `Reply::Objects.beings`, `api::beings_on_level`;
  `Command::Approach { being }` (word `approach-B`), `Choice::Being` (after the door, before the objects),
  `Rejection::NoSuchBeing`. Not shown while met or fought, nor once cleared. Blocks nothing.
- **Client:** `shapes::figures` on `SegKind::Being` (orange (3.0, 0.9, 0.15), provisional); reshape on
  `EncounterStarted`, `Bribed`, Hide or Run checks; "Approach: Pack rat x2" on Use; Approach in the verbs
  (closes them); a search names the group; `NoSuchBeing` reads "no one there to approach".
- Expect the owner's notes on the figure's shape and colour, and perhaps on hostile groups that should
  come to the party (no wandering in 2c, by the plan).

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

## Next
- Step 7 is now the save model, plan `tasks/plans/m8.5-save-model.md` (approved 2026-10-17), sub-steps 7a–7h.
  Next: 7a, the journal and the replay timing measurement.

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
