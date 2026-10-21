# Continuity notes

Written 2026-10-21, at the start of the re-plan toward world building. Rewrite this file every time it is
used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes). Untracked and never staged: the files under `assets/`
  and `docs/dnd_supplemental/` (the owner's).
- **The MVP is proven and archived.** Everything through M8.5 step 7 is in `tasks/archive/TODO-2026-10-20.md`.
  The owner's commits after the archive: `573cb1d` (BACKLOG: GPU-first), `17bc672` (world building assets),
  `755499e` and `0b3f8bf` (ARCHITECTURE and PRD consistency, the lint's Bevy edge fails closed).
- **`tasks/TODO.md` refactored (W0.1, 2026-10-21):** the goal "ready for world building" (the Three Kingdoms,
  `docs/locations/`), phases W0 to W5 with the milestone numbers kept inside them, each phase naming the PRD §13
  phase it closes. Owner's choices 2026-10-21: the ready bar is the end of W1 (the editor); the accounts study
  covers all four purposes (identity in multiplayer, signed saves, signed packs, a hosted service).
- **Gate:** unchanged since 7h7 (`tests passed 1020 failed 0 ignored 15`, VERIFY-GREEN) plus the owner's
  lint fix; this re-plan touched no code.

## Next
- **W0 continues:** W0.2 archive `tasks/alt-TODO.md`; W0.3 the README's Status table; W0.4 the vision-document
  changes, each approved by the owner first (PRD §13's note, ARCHITECTURE §16 and its stale TODO pointers,
  presentation-PRD §11 steps 1 and 4, presentation-ARCHITECTURE's status line, §4.3 and §7); W0.5 the three
  studies (accounts, multiplayer, multi-platform) as `tasks/plans/study-<name>.md`, read-only.
- **Then W1:** plan M5 Editor v1 in plan mode against the format as built.

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
  in `finally` by rewriting its text, never by moving a backup back; `python3 -u` to a file), then `git status`
  to confirm the sources are clean. No cargo command and no fixture edit while a mutation run or the gate runs.
- **Each new command, event or rejection** also moves: `docs/api.md` and its count lines, ARCHITECTURE §4.2's
  counts, `code-map.md`'s command list (all pinned by `api_doc.rs`), `api_doc.rs` pins and `variants!` lists,
  the MCP schema (`schema.rs` and its `oneOf` count test) and `schema_proof.rs`, `common/commands.rs`, the word
  in `word.rs`, client `labels::refused`.
- **Commits:** stage the changed files by name; never `git add -A` on `docs/`, `assets/` or the root; read
  `git status --short` before `git commit`. Messages end with the Co-Authored-By line and, when the system
  gives one, the Claude-Session line. Never push. Owner world docs (`docs/background`, `docs/locations`) go in
  their own commit, unedited. Ask before changing `PRD.md` or `ARCHITECTURE.md`.
- **Context:** pause and recommend a compact past 350K.
