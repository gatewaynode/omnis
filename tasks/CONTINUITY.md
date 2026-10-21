# Continuity notes

Written 2026-10-21, after M5.1 (W1, the editor). Rewrite this file every time it is
used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes). Untracked and never staged: the files under `assets/`
  and `docs/dnd_supplemental/` (the owner's).
- **The MVP is proven and archived** (`tasks/archive/TODO-2026-10-20.md`). `tasks/TODO.md` is the execution plan
  toward world building, phases W0 to W5; the ready bar is the end of W1 (the editor).
- **W0 is done, decisions included (2026-10-21):** W0.1 to W0.5 in `1902277`, `efd86b8`, `83d752e`, `80f460f`;
  the owner settled the three studies as they recommended: PRD D29 (accounts: a local Ed25519 keypair, signed
  packs at W5, no own hosted service), D30 (multiplayer: several parties by the time model, async over a shared
  world folder first at W4, the `PartyState` refactor with M9), D31 (platforms: Linux CI advisory now, Windows at
  W1's close, web deferred, mobile deferred without a milestone). W0.6: the `linux` job in
  `.github/workflows/ci.yml`, `continue-on-error`, same steps as the macOS check; the owner asked for it to be
  optional so no merge waits on a Linux build no one uses.
- **Gate:** unchanged since 7h7 (`tests passed 1020 failed 0 ignored 15`, VERIFY-GREEN) plus the owner's lint
  fix; W0 touched no Rust. The non-Bevy crates type-check for `x86_64-unknown-linux-gnu` from the Mac; the
  Bevy build on Linux is proven only by the first GitHub run (the owner reports CI; `gh` is not installed).

## Next
- **W1 is under way on branch `W1-authoring-tools`** (the M8.5 branch merged as PR #13; CI green with the advisory
  Linux job). The approved M5 plan: `tasks/plans/m5-editor-v1.md`, ten steps ticked in `tasks/TODO.md`.
- **M5.1 landed** (notes store `notes/*.ron`, `MapData::resolve`; `tasks/acceptance/m5-1.md`). Owner's direction
  during it: notes are a keyed store, never sealed, dangling keys allowed (lesson 2026-10-21).
- **Next: M5.2**, `omnis_data::edit` (`grid.rs`, `kinds.rs`, `document.rs`, `session.rs`, `files.rs`): byte-identical
  layouts on the eight fixture maps first (red), then the document and the session on a scratch copy of
  `packs/test` (`common::copy_pack` to write). The session also reads and writes the notes store (`notes/*.ron`,
  `NotesFile`, `is_note_key`) and lists orphaned keys. `loader.rs` now exposes `pub(crate) parse_checked`,
  `check_manifest`, `check_text_file`, `check_id`, `HasSchema` for the RON check by directory.
- **Then M5.3** the ops (14 + notes), M5.4 onward the client (see the plan's steps table).

## The owner's way of working (keep)
- Each item ends with something visible or clickable, and the report says how to try it (numbered: where to
  stand, what to press, what appears). The owner replies "Manually tested" and asks for the next.
- Hand-test launch: `just run --windowed --pack packs/base --pack packs/test --pack
  crates/omnis-sim/tests/packs/objects`, New game, devtools on; teleport with Esc, Debug, the World page, to
  `objects:map:yard`.
- Offscreen captures: `scratchpad/yardshot.sh name x y facing walk [--screen title|new-game|create-party|pause|
  actions|verbs|inventory]` (`cargo build -p omnis-vector` first). A capture cannot run a command first.
- Bug reports: fix directly, with a red test, a lesson in `tasks/LESSONS.md`, then report. Choices with real
  alternatives: AskUserQuestion with a recommended option first; larger additions get plan mode. A study's
  recommended answers become the owner's choices when the owner says "settle" (2026-10-21).

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
  RUSTSEC-2026-0192 (now built by the Linux job; informational); Bevy 0.20 not before 2026-11-07.

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
