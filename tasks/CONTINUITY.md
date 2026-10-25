# Continuity notes

Written 2026-10-25, at the close of M5 Editor v1 (M5.10c, the documents commit). Rewrite this file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `W1-authoring-tools`**, not pushed (the owner pushes). **M5 Editor v1 is closed**: M5.0 to M5.10 built,
  M5.0 to M5.9 accepted by hand test, M5.10 awaiting the owner's hand test (`tasks/acceptance/m5-10.md`). Untracked
  and never staged: `assets/` and `docs/dnd_supplemental/` (the owner's). Owner world docs under `docs/background`
  that appear modified or new are committed at once, unedited, in their own commit (`01825db`, `2874be2`, `50796e0`).
- **M5.10 as built (2026-10-25):** `334243a8` M5.10a the pack: `packs/test`'s four maps in the writer's layout,
  their comments as `notes/<map>.ron` (the map's key; the dungeon's portal comment on its cell 23,23), sealed
  **0.2.0** (owner's choice), the town's bank cells (7,3) and (7,4) restored (the owner's checkpoint `6343c145`
  had painted them street by accident and committed a save at 0.1.3 with a note on the portal cell, which stays);
  `omnis-data/tests/edit_authoring.rs` (`requests_for(def)` derives the editor's requests from the map's own def;
  the test rebuilds each shipped map from blank on a scratch copy, saves, and compares the def, the reload and the
  file's bytes to the shipped pack; the ignored `rewrite_the_shipped_maps_through_the_editor` is the rewrite, run
  once); tests read the manifest's version (`seal::status`, the patch one higher after a save) and count notes from
  the shipped ones instead of pinning literals (`edit_session.rs`, `omnis-cli/tests/headless.rs`,
  `editor_forms.rs` expects the town's map note); replays rebaselined. `a7eb3a86` M5.10b: `editor/acts.rs`
  (`EditAct`, `Editor::apply`, re-exported) and `editor/forms/fields.rs` (the forms' limits, shared choice fields
  and readers) split from `mod.rs` (now 796 lines) and `forms.rs` (856); pure moves. M5.10c (this commit): the
  documents (ARCHITECTURE §8.2 as built, presentation-ARCHITECTURE §6.1–§6.4 and §8.6, PRD §10 and §13 flagged as
  the plan row's direction, README, code-map header and a M5.10 section, TODO M5 ticked with a review, plan row
  10, `tasks/acceptance/m5-10.md`, a lesson).
- **Observed, not fixed (told the owner 2026-10-25):** with the map's own file open as text and typed into, a
  paint and Save from the plan leave the text view older than the file; Save file would then write the old text
  over the paint. Narrow (needs a dirty text and a map save); a rule if the owner wants one.
- **Lessons 2026-10-24/25** (`tasks/LESSONS.md`): a field's text is not a word (fresh → rebuild); "changed" is not
  "typed"; a guard that dims a whole list reads as a broken list; a mode is set by the opening action, not
  inherited; a layout exception for one row is written as an exception; a literal about a fixture the owner edits
  through the product is a test that breaks on use (read the pack).

## Next
1. **The owner's hand test of M5.10** (`tasks/acceptance/m5-10.md`, on a scratch copy of `packs/test`); fix any
   report directly with a red test and a lesson.
2. **After M5 (owner's order, 2026-10-24):** boundary tools beyond the edge brush (TODO, under M5's block); the UI
   icon design pass and an image cache for UI nodes (`Icons` by name, size and role through `minimap::image`); the
   activities schema when a second kind has engine rules (decided in the M5.7 planning: an editor grouping until
   then, Service site first, an icon per `ServiceKind`). Then the plan in `tasks/TODO.md`: the look in data, M9.
3. Later: the map's own fields (`Edit::Map`), Resize and New map as forms; a random entry's creatures as rows; the
   slider's thumb re-synced after a refused value in the editor; plan edges one pixel at every zoom (may read thin
   at 128); the stale-text-over-paint rule above; the 4 MiB file cap stays soft (owner).

## The owner's way of working (keep)
- Each item ends with something visible or clickable, and the report says how to try it (numbered: where to
  stand, what to press, what appears). The owner replies "tested" or "accepted" and asks for the next.
- Hand-test launch: `just run --windowed --pack packs/base --pack packs/test --pack
  crates/omnis-sim/tests/packs/objects`, New game, devtools on; teleport with Esc, Debug, the World page, to
  `objects:map:yard`. The editor: Title → Editor, or `--editor test:map:town`. A Save reseals the pack on disk: hand
  tests of the editor run on a scratch copy (`cp -R packs/test /tmp/omnis-scratch`, named as the last pack).
- Offscreen captures: `scratchpad/yardshot.sh name x y facing walk [--screen title|new-game|create-party|pause|
  actions|verbs|inventory]` (`cargo build -p omnis-vector` first). A capture cannot run a command first.
- Bug reports: fix directly, with a red test, a lesson in `tasks/LESSONS.md`, then report. Choices with real
  alternatives: AskUserQuestion with a recommended option first; larger additions get plan mode. A study's
  recommended answers become the owner's choices when the owner says "settle" (2026-10-21).
- Owner UI choices (memory `editor-pickers-are-modals`): pickers as modals over the editor; a decision asked at
  the action, not spread as dim reasons; wheel zoom about the pointer to room scale.
- The owner edits and saves through the editor and commits checkpoints of their own: tests never pin what those
  saves move (the pack's version, its notes); a content change the owner did not intend is asked about, not fixed.

## Pins
- **Replays:** walk `c5ce78d4a5d9655a`, fight `f90c92e47e6b2777` (rebaselined in M5.10a, `packs/test` 0.2.0, hash
  2508070832228001662). `SAVE_SCHEMA` 9, `PROTOCOL` 2, **ops 44, MCP tools 44, `Reply` 27 (`EVERY`), `OpError` 11**
  (since M5.3), `MAX_MIGRATIONS` 1024, `MAX_STEPS` 4096 (pinned to the docs by `omnis-mcp/tests/api_doc.rs`, run as
  `--test integration api_doc`; the tool counts also in `tools.rs`, `bridge.rs` twice, `omnis-cli/tests/headless.rs`,
  README, code-map).
- **Counts:** `Command` 16, `DevCommand` 14, `Event` 73, `Rejection` 71, `LoadError` 12.
- **Fixtures:** `crates/omnis-sim/tests/packs/{ahead,chain/v1,chain/v3,heights,levels,objects}`; a test that edits a
  pack works on a scratch copy (`omnis-data/tests/common::copy_pack`, `omnis-cli/tests/headless.rs::scratch_pack`,
  `omnis-vector/tests/common/editor.rs::in_editor_scratch`) and reseals it; the bad-pack corpus `broken` carries
  `notes/bad.ron` (not hashed: no reseal was needed). `packs/test` ships notes on all four maps and on the town's
  cell (11,2); `crates/omnis-sim/tests/saves/v9/.../manifest.ron` still names 0.1.1 and loads with `force`.
- **Limits the editor uses:** `MAX_STRING_BYTES` 4 KiB (a note, a key), `MAX_FILE_BYTES` 4 MiB (`bounded_file`, a
  check or write text; soft cap by the owner's decision), `MAX_SAVE_BYTES` 8 MiB, `MAX_COLLECTION` 65,536,
  `MAX_MAP_SIDE` 256, `MAX_LEVELS` 16, `UNDO_CAP` 100. `Session::files()` lists `pack.ron` and `.ron` under
  `data`, `text`, `notes`, `migrations` only (normal components, no symlinks, depth 8).
- **Client test helpers:** `tests/common/app.rs` (`keyboard_on`, `keyboard`, `tap`, `set`, `press_ui`, `top`,
  `churn_fields`, `wheel`, `ui_editor_app` with `UI_PHYSICAL` 2400×1400 at scale 2), `tests/common/editor.rs`
  (`in_editor`, `in_editor_scratch`, `press_edit`, `arrow`, `point_at`, `point_frac`, `leave_plan`, `type_into`,
  `field_entity`, `field_of`, `panel`). Under Bevy picking a faked `Interaction::Pressed` is reset before the route
  reads it: drive the model through `shell::editor::act` in `ui_editor_app` tests. Dim kit controls carry no
  press marker. `tap` sends no character: dirty a field with `keyboard`. `omnis-cli/tests/headless.rs` is its own
  test binary (`--test headless`), not part of the CLI's `integration`.

## Open, carried over
- `assets/README.md` font rows: the owner's file; report only, never edit.
- `docs/dnd_supplemental/`: the owner's, in progress; ask before committing anything there.
- P6 colours unconfirmed; step 5 revisits colour roles (joins, fills, crest, objects, beings).
- The automap still records the forward cone (§12.5); no objects or beings on it (owner, 2026-10-14).
- Dependencies: the rhai Socket re-audit (ask when); `smartstring` RUSTSEC-2026-0249; `ttf-parser`
  RUSTSEC-2026-0192 (built by the advisory Linux CI job; informational); Bevy 0.20 not before 2026-11-07.

## Process
- **Cargo and git:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools` for both (git refuses without it:
  the Xcode licence). `cargo fmt --all` first and again after any appended test. Workspace clippy with
  `--all-targets --features omnis-vector/test-fixtures -- -D warnings` before every commit. Sim tests:
  `cargo test -p omnis-sim --test integration <filter>`; data `-p omnis-data --test integration <filter>`; client
  `-p omnis-vector --test integration <filter>` (one filter per invocation).
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<item>.txt 2>&1`, VERIFY-GREEN, no source or pinned-document
  edits while it runs (about seven minutes). Then `git add <paths by name>`, `git commit -F <msg>`, Sentrux `rescan`
  (first `scan` on `/Users/john/code/omnis/crates` in a new session) and `check_rules`. The scratchpad is
  `/private/tmp/claude-501/-Users-john-code-omnis/787890da-120b-4905-befb-f4247b7d8f51/scratchpad` (absolute).
- **Tests red first; mutations** by sed or a Python runner in the scratchpad (anchor checked unique, source
  restored by rewriting its text or copying the backup back, then `git status`/`diff -q` to confirm). No cargo
  command and no fixture edit while a mutation run or the gate runs.
- **Moves and edits** by an anchored Python script in the scratchpad (every anchor asserted unique before writing;
  a file split cuts exact line ranges with the boundary lines asserted). After `cargo fmt` the anchors move, so
  patch the file as it is (lesson 2026-09-12).
- **Each new op** moves: `ops.rs` (`Op`, `is_host`, the `dispatch` HostOnly arm), `api_doc.rs` (`ops!`, `every_op`,
  the lists and literal counts), `replies.rs` (`tag`, `EVERY`, an ask), `tools.rs` (tool + example + count),
  `bridge.rs` (two counts), `omnis-cli/src/schema.rs` (op dump) and `tests/headless.rs` (count), `docs/api.md`
  (§5 table and "N ops.", §6, §10, changelog), README (count and names), `code-map.md` (count), ARCHITECTURE §9.3.
  Each new command, event or rejection also moves ARCHITECTURE §4.2's counts, `code-map.md`'s command list,
  `variants!` lists, the MCP `schema.rs` `oneOf` count test and `schema_proof.rs`, `common/commands.rs`, `word.rs`,
  client `labels::refused`.
- **A change to `packs/` or a serialized `World`** rebaselines the replays in the same commit
  (`cargo test -p omnis-sim --test integration rebaseline -- --ignored`) and records the fingerprints in
  `docs/api.md`'s changelog, `tasks/knowledge/verification.md` and the pins above.
- **Commits:** stage the changed files by name; never `git add -A` on `docs/`, `assets/` or the root; read
  `git status --short` before `git commit`. Messages end with the Co-Authored-By line and the Claude-Session line
  the system gives. Never push. Owner world docs (`docs/background`, `docs/locations`) go in their own commit,
  unedited. Ask before changing `PRD.md` or `ARCHITECTURE.md`; an owner's direction counts as the ask, flagged in
  the commit. Interface strings are ASCII only; every action has a mouse entry point.
- **Context:** pause and recommend a compact past 350K.
