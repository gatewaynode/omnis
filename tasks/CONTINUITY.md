# Continuity notes

Written 2026-10-20, with M8.5 7h7 (docs and close for saves by name) built and awaiting the owner's read. Rewrite
this file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes). Untracked and never staged: the files under `assets/`
  and `docs/dnd_supplemental/` (the owner's).
- **Step 7, the save model (D28):** 7a–7g accepted 2026-10-17/18. **7h, saves by name** (plan
  `tasks/plans/m8.5-saves-by-name.md`, its "Built" note at the top lists every sub-step's commit and the
  departures): 7h1 seal, 7h2 schema 9, 7h3 load by name, 7h4 save check and Load screen, 7h5 reload by name,
  7h6 pack migrations (`c39d019`, accepted 2026-10-20). **7h7 built 2026-10-20**: ARCHITECTURE §4.1.1
  (migrations), §4.5, §4.9, §6.1, §6.2, §12, A19; PRD §7.7 (owner's wording; the pre-D28 line removed);
  `horizons.md` "Saves by name"; api.md §14; a docs pin test. Script `tasks/acceptance/m8.5-7h7.md`.
- **Gate:** `tests passed 1020 failed 0 ignored 15`, VERIFY-GREEN; clippy clean; Sentrux rules pass, 9001.

## Next
1. The owner reads 7h7. On acceptance: check 7h7, 7h and M8.5 step 7 in TODO (line "M8.5 step 7 checked" in the
   7h7 Built note).
2. Then the owner sets the order: open M8.5 steps are 1 (the 3D map format, partly done), 5 (colour scheme and
   biosphere palettes), 6 (mana in map packs, needs the owner's intent), then the editor (M5). Ask.

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
- **Replays:** walk `8eab9a1dd95b472d`, fight `79ca2704c92bc312` (rebaselined in 7h2). `SAVE_SCHEMA` 9,
  `PROTOCOL` 2, ops 27, MCP tools 27, `MAX_MIGRATIONS` 1024, `MAX_STEPS` 4096 (all pinned to the docs by
  `omnis-mcp/tests/api_doc.rs`, run as `--test integration api_doc`).
- **Counts:** `Command` 16, `DevCommand` 14, `Event` 73, `Rejection` 71, `LoadError` 12.
- **Fixtures:** `crates/omnis-sim/tests/packs/{ahead,chain/v1,chain/v3,objects}`; a test that edits a pack reseals
  it (`omnis-cli pack seal`).

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
  in `finally` **by rewriting its text, never by moving a backup back**, which leaves an old mtime and a stale
  mutant in `target/`; `python3 -u` to a file), then `git status` to confirm the sources are clean. No cargo command and no fixture edit while a mutation run or the gate runs.
- **Each new command, event or rejection** also moves: `docs/api.md` and its count lines, ARCHITECTURE §4.2's counts, `code-map.md`'s command list (all pinned by `api_doc.rs`), `api_doc.rs` pins
  and `variants!` lists, the MCP schema (`schema.rs` and its `oneOf` count test) and `schema_proof.rs`
  (instances, offered, the drift test's index), `common/commands.rs`, the word in `word.rs`, client
  `labels::refused`.
- **Commits:** stage new files by name with `git add` first (`--pathspec-from-file` does not add untracked files); build the path list from `git status --short` without `assets/`,
  `.claude/` or `docs/dnd_supplemental/`, write it to a scratchpad file, `git commit --pathspec-from-file=<file>
  -F -`. Messages end with the Co-Authored-By line and, when the system gives one, the Claude-Session line. Never push. Owner world docs
  (`docs/background`, `docs/locations`) go in their own commit, unedited.
- **Context:** pause and recommend a compact past 350K.
