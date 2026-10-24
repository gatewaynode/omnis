# Continuity notes

Written 2026-10-24, after M5.6a (the attribute bar) was built; M5.5 accepted the same day. Rewrite this file
every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `W1-authoring-tools`**, not pushed (the owner pushes). M5.0 to M5.5 done and accepted (M5.5 on
  2026-10-24 after three fixes from the hand tests: `cc5bd9c` pointer convention + WASD, `4736b01` nearest-side
  edge aim + hover, `429de34` scroll kept; acceptance recorded in `34d3986`). Untracked and never staged:
  `assets/` and `docs/dnd_supplemental/` (the owner's).
- **M5.6 planned and approved 2026-10-24** (plan mode; the plan file is Claude's, the decisions are in
  `tasks/TODO.md` under M5.6): three commits 6a/6b/6c, `tasks/acceptance/m5-6.md` grows with them. Owner's
  choices: the attribute bar **above the plan**; list fields (stacks, contents, items, the random table's
  entries) as **dedicated row editors** with Add row / Remove, not RON text (over my recommendation).
- **M5.6a built** (this commit): `view::bar` over the plan (`Column::Bar`, `ScreenId::EditorBar`): the cursor's
  cell's Terrain (round through no ground), Height (−64..64 by one) and Rise (stair cells only), each step one
  undo step through `Editor::brushed`/`edited`; `refresh` rewrites words in place when a column's shape stands
  (`need` → `Need::{Nothing, Words, Nodes}`, `screen_draw::sync` filtered by `OfScreen(ScreenId)`), rebuilds
  with `keep` = the focused editor field else the column's first field; a plan click clears `InputFocus`;
  `Kind::Text.lines` (multi-line through Bevy's `EditableText.allow_newlines`/`visible_lines`); editor text
  fields carry `EditorPress`, so the game router never sees them; `Spawn { id, height, mark, keep }` threads
  through `rows`/`control_node`/`span_node` (roster passes `id: None`). Tests `tests/editor_forms.rs` (3), the
  M5.5 scroll test now asserts the words-only path keeps the nodes and a shape change rebuilds in place.
- **Design for 6b (next):** `editor/form.rs` generic `Form { kind, fields: Vec<Field { label, value: Value,
  slot: Slot::Top | Row(list, row) }>, lists: Vec<List { label, template, rows, max }>, existing }` with
  `rows() -> Vec<Row>` (one control per field; per list a heading + `[Add row]`, per entry its fields + `[Remove]`;
  then `[Apply] [Cancel] [Remove <kind>]`), `typed(i, text)`, `step(i, ±1)`, `add_row`, `remove_row`, parse helpers
  (`id`, `dice` `NdS[+M]`, `choice`); `editor/forms.rs` per kind `Form::open(kind, def, at, existing)` and
  `request(def) -> Result<Change { Place | Edit | Note }, String>`; `Editor.form`, acts `Field(u8)`, `ListAdd(u8)`,
  `ListRemove(u8, u8)`, `Apply`, `Cancel`, `Discard`, `SetStart`, `PlaceKind`, `Add`, `EditAt(u8)`, `RemoveAt(u8)`,
  `Random`, `RestAdd/RestEdit/RestRemove`; `here()` the placements at the cursor; the inspector shows the form
  first, then the cell lines, `[Set start here]`, the `here()` rows with Edit/Remove, `Add [< kind >] [Add]`, the
  map section (random table, rest events). Layer: `route` reads `Query<(Ref<EditableText>, &EditorPress)>` and
  calls `editing.typed(i, text)` on a change (no revision bump: words-in-place keeps the caret); `keep`'s
  first-field fallback should apply only to a form just opened (a `fresh` flag) so a rebuild after a plan click
  does not retake the keyboard. The random entry's inner stacks stay one text column (`goblin 1d4, wolf 2d6`).
  Payload field tables: see the plan in `tasks/TODO.md`'s M5.6 note and the data types in
  `crates/omnis-data/src/{map,encounter,service,rest_event,placement,lore}.rs`; `Place`/`Edit`/`Target` in
  `crates/omnis-data/src/edit/kinds.rs`; `Document::place/edit` one undo step each; `Session::set_note` written at
  once, not undone; note keys `<map>/<level>/<x>,<y>`, `<map>/level/<n>`, `<map>`.
- **Lessons 2026-10-24** (`tasks/LESSONS.md`): framework conventions tested through the framework's own
  systems; no mouse action on hidden state; a layer that rebuilds nodes carries transient state across.
- **Lesson 2026-10-22**: a windowed game launched from this shell never ticks; verify headlessly.

## Next
1. **Report 6a to the owner** (how to try: open the town, click a cell, step the bar's Height; the tools column
   does not move), then **6b** as designed above, red tests first (`tests/editor_forms.rs`), then **6c**
   (terrains, levels, notes; documents: acceptance steps, TODO, code-map, the plan's row 6 note on row editors).
2. Then M5.7 to M5.10 per the plan's steps table. Boundary tools after M5.6 at the earliest (owner).

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
  `PROTOCOL` 2, **ops 44, MCP tools 44, `Reply` 27 (`EVERY`), `OpError` 11** (since M5.3), `MAX_MIGRATIONS` 1024,
  `MAX_STEPS` 4096 (pinned to the docs by `omnis-mcp/tests/api_doc.rs`, run as `--test integration api_doc`;
  the tool counts also in `tools.rs`, `bridge.rs` twice, `omnis-cli/tests/headless.rs`, README, code-map).
- **Counts:** `Command` 16, `DevCommand` 14, `Event` 73, `Rejection` 71, `LoadError` 12.
- **Fixtures:** `crates/omnis-sim/tests/packs/{ahead,chain/v1,chain/v3,heights,levels,objects}`; a test that edits a
  pack works on a scratch copy (`omnis-data/tests/common::copy_pack`, `omnis-cli/tests/headless.rs::scratch_pack`)
  and reseals it; the bad-pack corpus `broken` carries `notes/bad.ron` (not hashed: no reseal was needed).
- **Limits the editor uses:** `MAX_STRING_BYTES` 4 KiB (a note, a key), `MAX_FILE_BYTES` 4 MiB (`bounded_file`, a
  check or write text), `MAX_COLLECTION` 65,536, `MAX_MAP_SIDE` 256, `MAX_LEVELS` 16, `UNDO_CAP` 100.

## Open, carried over
- `assets/README.md` font rows: the owner's file; report only, never edit.
- `docs/dnd_supplemental/`: the owner's, in progress; ask before committing anything there.
- P6 colours unconfirmed; step 5 revisits colour roles (joins, fills, crest, objects, beings).
- The automap still records the forward cone (§12.5); no objects or beings on it (owner, 2026-10-14).
- Dependencies: the rhai Socket re-audit (ask when); `smartstring` RUSTSEC-2026-0249; `ttf-parser`
  RUSTSEC-2026-0192 (built by the advisory Linux CI job; informational); Bevy 0.20 not before 2026-11-07.
- The first CI run with the advisory Linux job passed (owner, 2026-10-21).

## Process
- **Cargo and git:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools` for both (git refuses without it:
  the Xcode licence). `cargo fmt --all` first and again after any appended test. Workspace clippy with
  `--all-targets --features omnis-vector/test-fixtures -- -D warnings` before every commit. Sim tests:
  `cargo test -p omnis-sim --test integration <filter>`; data `-p omnis-data --test integration <filter>`; client
  `-p omnis-vector --features test-fixtures --test integration <filter>`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<item>.txt 2>&1`, VERIFY-GREEN, no source or pinned-document
  edits while it runs. In the background: one chain, `verify.sh > log; grep -q VERIFY-GREEN log && git add <paths>
  && git commit -F -` (lesson 2026-10-21); the runner's exit code is the wrapper's. Then Sentrux `rescan` (first
  `scan` on `/Users/john/code/omnis/crates` in a new session) and `check_rules`.
- **Tests red first; mutations** with a Python runner in the scratchpad (anchor checked unique, source restored
  in `finally` by rewriting its text with a *fresh* mtime, never `os.utime` back to the old one and never by
  moving a backup back: lesson 2026-10-18, broken 2026-10-22), then `git status` to confirm
  the sources are clean. No cargo command and no fixture edit while a mutation run or the gate runs.
- **Each new op** moves: `ops.rs` (`Op`, `is_host`, the `dispatch` HostOnly arm), `api_doc.rs` (`ops!`, `every_op`,
  the lists and literal counts), `replies.rs` (`tag`, `EVERY`, an ask), `tools.rs` (tool + example + count),
  `bridge.rs` (two counts), `omnis-cli/src/schema.rs` (op dump) and `tests/headless.rs` (count), `docs/api.md`
  (§5 table and "N ops.", §6, §10, changelog), README (count and names), `code-map.md` (count), ARCHITECTURE §9.3.
  Each new command, event or rejection also moves ARCHITECTURE §4.2's counts, `code-map.md`'s command list,
  `variants!` lists, the MCP `schema.rs` `oneOf` count test and `schema_proof.rs`, `common/commands.rs`, `word.rs`,
  client `labels::refused`.
- **Edits** by an anchored Python script in the scratchpad (every anchor asserted unique before writing); after
  `cargo fmt` the anchors move, so patch the file as it is (lesson 2026-09-12).
- **Commits:** stage the changed files by name (or `git add -u` plus each new file); never `git add -A` on `docs/`,
  `assets/` or the root; read `git status --short` before `git commit`. Messages end with the Co-Authored-By line
  and the Claude-Session line the system gives. Never push. Owner world docs (`docs/background`, `docs/locations`)
  go in their own commit, unedited. Ask before changing `PRD.md` or `ARCHITECTURE.md`; an owner's direction counts
  as the ask, flagged in the commit.
- **Context:** pause and recommend a compact past 350K.
