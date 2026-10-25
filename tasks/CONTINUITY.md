# Continuity notes

Written 2026-10-25, before a compact, after M5.9 (a to d) was accepted by hand test. Rewrite this file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `W1-authoring-tools`**, not pushed (the owner pushes). M5.0 to M5.9 built and accepted. Untracked and
  never staged: `assets/` and `docs/dnd_supplemental/` (the owner's).
- **The owner's working tree holds their own edits, never to be committed by me:** `.gitignore`,
  `assets/svg-icons/README.md`, `packs/test/data/maps/town.ron`, `packs/test/pack.ron` (an accidental save of the
  town through the editor, resealed 0.1.2: it shows the writer's layout, comments dropped) and untracked
  `packs/test/notes/`. The two pack files break two gate tests, so the gate runs with
  `git stash push -u -q -m "owner's pack edits and notes" -- packs/test` before `scripts/verify.sh` and
  `git stash pop -q` after. Owner world docs under `docs/background` that appear modified are committed at once,
  unedited, in their own commit (done for `industry_in_the_3_kingdoms.md` `01825db` and `introduction.md` `2874be2`).
- **M5.9 as accepted (2026-10-25):** `60ba22a` the RON editor; `f826536` a fresh form or file rebuilds its column;
  `6552f92` M5.9b the file picker is a modal over the editor (`screen/editor_files.rs`, `ScreenId::EditorFiles`);
  `9e38b8c` a stale field never types into a fresh file (the window marks every field changed every frame:
  `route` ignores a field while the model is `fresh`; `churn_fields` in tests); `0f83c9d` M5.9c Open file... asks
  about unsaved text (Save and continue, Discard changes, Cancel) and clears a clean file; `c21fa85` the document
  control (`Kind::Document`, `shell/document.rs`: line numbers, no wrap, a scrollbar each axis, the view follows
  the caret, hand-checked only for the caret system); `b94a13b` a map opened from the tools shows its plan
  (`Editor::open` sets `Mode::Plan`, `reopen` keeps the mode); `a881c20` a crowded column's rows keep their
  height (only the document's row has `min_height: 0`; layout test under `ui_editor_app`); `b350020` M5.9d the
  wheel over the plan zooms about the pointer (`View::zoomed_at`, `Editor::zoom_at`, the `Zooming` accumulator in
  `shell/editor.rs`, a trackpad's pixels add up to the kit's `WHEEL_LINE`); `c00fc6f` the zooms run to 128 px a
  cell (`ZOOMS` has ten entries; a small map fitted whole starts partway up).
- **Observed, not fixed (told the owner 2026-10-25):** with the map's own file open as text and typed into, a
  paint and Save from the plan leave the text view older than the file; Save file would then write the old text
  over the paint. Narrow (needs a dirty text and a map save); a rule if the owner wants one.
- **Lessons 2026-10-24/25** (`tasks/LESSONS.md`): a field's text is not a word (fresh → rebuild); "changed" is not
  "typed"; a guard that dims a whole list reads as a broken list; a mode is set by the opening action, not
  inherited; a layout exception for one row is written as an exception. Earlier: framework conventions tested
  through the framework's own systems; no mouse action on hidden state; a layer that rebuilds nodes carries
  transient state across; a windowed game launched from this shell never ticks, verify headlessly.

## Next
1. **M5.10 re-authoring and close** (plan row 10 of `tasks/plans/m5-editor-v1.md`; **plan mode first**):
   `tests/edit_authoring.rs` builds each `packs/test` map from blank with ops only and matches the loaded def; the
   four maps' comments moved to notes and the files rewritten through the editor; `packs/test` resealed; both
   replays rebaselined; documents as built (ARCHITECTURE §8.2, presentation-ARCHITECTURE §6/§8, PRD §10 status,
   code-map, README; ask before PRD/ARCHITECTURE edits, the plan row counts as the owner's direction, flagged in
   the commit); TODO M5 ticked. Start by splitting the files over the 1000-line guideline: `editor/mod.rs`
   (1170) and `editor/forms.rs` (1042); `shell/editor.rs` is 935. The owner's `packs/test` edits in the tree
   must be reconciled with the rewrite (ask: their town save is already the writer's layout).
2. Boundary tools after (owner, 2026-10-24); the UI icon design pass and an image cache for UI nodes (`Icons`
   by name, size and role through `minimap::image`, waits for the first control that takes an icon); the
   activities schema when a second kind has engine rules (owner's decision in the M5.7 planning: activities are
   an editor grouping, Add → Activity → a list, Service site first, an icon per `ServiceKind`).
3. Later: map fields (`Edit::Map`), Resize and New map forms; a random entry's creatures as rows; the slider's
   thumb re-synced after a refused value in the editor; optional recorded decision rows on text-over-binary map
   layout (assessed 2026-10-24: ~1.5–2× a binary packing, fine) and a derived map-file cap (owner declined: the
   4 MiB cap stays soft).

## Done on this branch before M5.9 (accepted; details in the acceptance files)
- M5.6 the bar, forms, terrains, levels, notes (`tasks/acceptance/m5-6.md`); marks and defaults (`0b4a534`
  and the plan marks commit; a stored automap bit was backed out: it would fold older saves); icons
  (`omnis-icon`, the Lucide set `7973dac`, the crate `0504374`, `tasks/acceptance/icons.md`); M5.7 the preview
  (`45fad59`, `7b19865`, `6438668`); M5.8 playtest from the editor (Back to editor on pause and defeat).

## The owner's way of working (keep)
- Each item ends with something visible or clickable, and the report says how to try it (numbered: where to
  stand, what to press, what appears). The owner replies "tested" or "accepted" and asks for the next.
- Hand-test launch: `just run --windowed --pack packs/base --pack packs/test --pack
  crates/omnis-sim/tests/packs/objects`, New game, devtools on; teleport with Esc, Debug, the World page, to
  `objects:map:yard`. The editor: Title → Editor, or `--editor test:map:town`.
- Offscreen captures: `scratchpad/yardshot.sh name x y facing walk [--screen title|new-game|create-party|pause|
  actions|verbs|inventory]` (`cargo build -p omnis-vector` first). A capture cannot run a command first.
- Bug reports: fix directly, with a red test, a lesson in `tasks/LESSONS.md`, then report. Choices with real
  alternatives: AskUserQuestion with a recommended option first; larger additions get plan mode. A study's
  recommended answers become the owner's choices when the owner says "settle" (2026-10-21).
- Owner UI choices (memory `editor-pickers-are-modals`): pickers as modals over the editor; a decision asked at
  the action, not spread as dim reasons; wheel zoom about the pointer to room scale.

## Pins
- **Replays:** walk `c5ce78d4a5d9655a`, fight `f90c92e47e6b2777` (rebaselined in M5.10a, `packs/test` 0.2.0). `SAVE_SCHEMA` 9,
  `PROTOCOL` 2, **ops 44, MCP tools 44, `Reply` 27 (`EVERY`), `OpError` 11** (since M5.3), `MAX_MIGRATIONS` 1024,
  `MAX_STEPS` 4096 (pinned to the docs by `omnis-mcp/tests/api_doc.rs`, run as `--test integration api_doc`;
  the tool counts also in `tools.rs`, `bridge.rs` twice, `omnis-cli/tests/headless.rs`, README, code-map).
- **Counts:** `Command` 16, `DevCommand` 14, `Event` 73, `Rejection` 71, `LoadError` 12.
- **Fixtures:** `crates/omnis-sim/tests/packs/{ahead,chain/v1,chain/v3,heights,levels,objects}`; a test that edits a
  pack works on a scratch copy (`omnis-data/tests/common::copy_pack`, `omnis-cli/tests/headless.rs::scratch_pack`,
  `omnis-vector/tests/common/editor.rs::in_editor_scratch`) and reseals it; the bad-pack corpus `broken` carries
  `notes/bad.ron` (not hashed: no reseal was needed).
- **Limits the editor uses:** `MAX_STRING_BYTES` 4 KiB (a note, a key), `MAX_FILE_BYTES` 4 MiB (`bounded_file`, a
  check or write text; soft cap by the owner's decision), `MAX_SAVE_BYTES` 8 MiB, `MAX_COLLECTION` 65,536,
  `MAX_MAP_SIDE` 256, `MAX_LEVELS` 16, `UNDO_CAP` 100. `Session::files()` lists `pack.ron` and `.ron` under
  `data`, `text`, `notes`, `migrations` only (normal components, no symlinks, depth 8).
- **Client test helpers:** `tests/common/app.rs` (`keyboard_on`, `keyboard`, `tap`, `set`, `press_ui`, `top`,
  `churn_fields`, `wheel`, `ui_editor_app` with `UI_PHYSICAL` 2400×1400 at scale 2), `tests/common/editor.rs`
  (`in_editor`, `in_editor_scratch`, `press_edit`, `arrow`, `point_at`, `point_frac`, `leave_plan`, `type_into`,
  `field_entity`, `field_of`, `panel`). Under Bevy picking a faked `Interaction::Pressed` is reset before the route
  reads it: drive the model through `shell::editor::act` in `ui_editor_app` tests. Dim kit controls carry no
  press marker. `tap` sends no character: dirty a field with `keyboard`.

## Open, carried over
- `assets/README.md` font rows: the owner's file; report only, never edit.
- `docs/dnd_supplemental/`: the owner's, in progress; ask before committing anything there.
- P6 colours unconfirmed; step 5 revisits colour roles (joins, fills, crest, objects, beings).
- The automap still records the forward cone (§12.5); no objects or beings on it (owner, 2026-10-14).
- Plan edges are one pixel wide at every zoom; at 128 px a cell they may read thin (offered to the owner).
- Dependencies: the rhai Socket re-audit (ask when); `smartstring` RUSTSEC-2026-0249; `ttf-parser`
  RUSTSEC-2026-0192 (built by the advisory Linux CI job; informational); Bevy 0.20 not before 2026-11-07.

## Process
- **Cargo and git:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools` for both (git refuses without it:
  the Xcode licence). `cargo fmt --all` first and again after any appended test. Workspace clippy with
  `--all-targets --features omnis-vector/test-fixtures -- -D warnings` before every commit. Sim tests:
  `cargo test -p omnis-sim --test integration <filter>`; data `-p omnis-data --test integration <filter>`; client
  `-p omnis-vector --test integration <filter>` (one filter per invocation).
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<item>.txt 2>&1` with the owner's `packs/test` edits stashed
  (above), VERIFY-GREEN, no source or pinned-document edits while it runs. Then `git add <paths by name>`,
  `git commit -F <msg>`, Sentrux `rescan` (first `scan` on `/Users/john/code/omnis/crates` in a new session) and
  `check_rules`. The scratchpad is
  `/private/tmp/claude-501/-Users-john-code-omnis/787890da-120b-4905-befb-f4247b7d8f51/scratchpad` (absolute).
- **Tests red first; mutations** by sed or a Python runner in the scratchpad (anchor checked unique, source
  restored by rewriting its text or copying the backup back, then `git status`/`diff -q` to confirm). No cargo
  command and no fixture edit while a mutation run or the gate runs.
- **Each new op** moves: `ops.rs` (`Op`, `is_host`, the `dispatch` HostOnly arm), `api_doc.rs` (`ops!`, `every_op`,
  the lists and literal counts), `replies.rs` (`tag`, `EVERY`, an ask), `tools.rs` (tool + example + count),
  `bridge.rs` (two counts), `omnis-cli/src/schema.rs` (op dump) and `tests/headless.rs` (count), `docs/api.md`
  (§5 table and "N ops.", §6, §10, changelog), README (count and names), `code-map.md` (count), ARCHITECTURE §9.3.
  Each new command, event or rejection also moves ARCHITECTURE §4.2's counts, `code-map.md`'s command list,
  `variants!` lists, the MCP `schema.rs` `oneOf` count test and `schema_proof.rs`, `common/commands.rs`, `word.rs`,
  client `labels::refused`.
- **Edits** by an anchored Python script in the scratchpad (every anchor asserted unique before writing); after
  `cargo fmt` the anchors move, so patch the file as it is (lesson 2026-09-12, bitten again 2026-10-25).
- **Commits:** stage the changed files by name; never `git add -A` on `docs/`, `assets/` or the root; read
  `git status --short` before `git commit`. Messages end with the Co-Authored-By line and the Claude-Session line
  the system gives. Never push. Owner world docs (`docs/background`, `docs/locations`) go in their own commit,
  unedited. Ask before changing `PRD.md` or `ARCHITECTURE.md`; an owner's direction counts as the ask, flagged in
  the commit. Interface strings are ASCII only; every action has a mouse entry point.
- **Context:** pause and recommend a compact past 350K.
