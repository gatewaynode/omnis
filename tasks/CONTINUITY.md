# Continuity notes

Written 2026-10-24, after M5.9b (the file picker modal) was built. Rewrite this file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `W1-authoring-tools`**, not pushed (the owner pushes). M5.0 to M5.5 done and accepted (M5.5 on
  2026-10-24, `34d3986`). Untracked and never staged: `assets/` and `docs/dnd_supplemental/` (the owner's).
- **M5.6 built, awaiting the owner's hand test** (`tasks/acceptance/m5-6.md`, 14 steps on a scratch copy):
  `7906f8e` 6a the attribute bar over the plan (`view::bar`, `Column::Bar`; the cursor's cell's Terrain, Height,
  Rise, one undo step each; `refresh` rewrites words in place by `Screen::shape` with `OfScreen` markers and
  `screen_draw::sync`; `keep` for the keyboard; a plan click clears `InputFocus`; `Kind::Text.lines`; editor
  fields carry `EditorPress`); `dedbc6a` 6b the generic `Form` (`editor/form.rs`) and the placement kinds
  (`editor/forms.rs`: portal, encounter with stacks rows, random table with entry rows, site, rest event,
  object with contents rows and lore, ground pile, lore), `Editor { form, place_kind }`, `here()`, the
  inspector's start/placement/Add/map rows, `route` reading editor fields and `screen::Slid`; **6c (this
  commit)** terrains, levels and notes (`FormKind::{Terrain, LevelAdd, Level, Note}`, `Change::{Place, Edit,
  Note}`, `Editor::change`/`note_key`/`note`, tools' Add/Edit terrain and Add level/Level… buttons, the
  inspector's note rows; notes through `Session::set_note` at once, outside the history). Owner's choices in
  the plan: the bar above the plan; list fields as row editors. Tests `tests/editor_forms.rs` (9), `form.rs`
  unit (2); mutations killed per commit (4, 5, and 6c's).
- **Not in M5.6** (said in the acceptance's Known): map fields (`Edit::Map`), Resize, New map have no form;
  a random entry's creatures are one typed line; editing a rest event's key makes a new entry; the slider's
  thumb is not re-synced after a refused value in the editor (the kit's `slide` is the screen router's).
- **Lessons 2026-10-24** (`tasks/LESSONS.md`): framework conventions tested through the framework's own
  systems; no mouse action on hidden state; a layer that rebuilds nodes carries transient state across.
- **Lesson 2026-10-22**: a windowed game launched from this shell never ticks; verify headlessly.

## In hand (2026-10-24, after the M5.6 report; plan approved)
- **Marks and defaults**, two commits (owner's direction 2026-10-24, plan approved): `0b4a534` the minimap and
  the 3D floor mark portals (rectangle + circle, door colour) and a site's cell the activities' placeholder
  smiley (`SegKind::Portal`/`Activity`, `raster::{rect,circle,portal,triangle,smiley}_pixels`), derived in the
  client from the map on a known tile (a stored automap bit was built and backed out: it would fold every older
  save on load and breaks the frozen schema-9 fixture's journal comparison); *this commit* `plan::Mark` and
  `plan::marks(def, level)` on the editor's plan, `forms::encounter` prefilled (free id, one stack of the first
  loaded monster, Stacks at `MAX_STACKS`), `Form::keys` refusing a taken encounter/object/lore id and a second
  encounter on a cell, `apply_form` saying the loader's first error after a change; a LESSONS entry.
- **Owner's new direction (2026-10-24):** an "activities" category (recorded in `tasks/TODO.md` under M5.6 as a
  proposal; decide the data shape at the next planning).

## Icons, built (owner 2026-10-24, after M5.6, before M5.7; plan approved)
- Owner's choices: a new crate `omnis-icon`, Bevy-free, no dependencies; the whole Lucide set compiled in. The
  set itself is `7973dac` (the owner's addition, force-added at their ask, the tarball dropped); the crate
  `0504374`; the client's use *this commit*: `Raster::icon`, `geometry::icon_on_floor`, `icons::activity()` =
  `face-grinning` on the plan's site mark and the 3D activity mark. `tasks/acceptance/icons.md` has the four
  steps. An image cache for UI nodes (`Icons` by name, size and role through `minimap::image`) waits for the
  first control that takes an icon. The owner's `.gitignore` edit (ignoring `assets/svg-icons/`) was theirs to
  revert; `assets/README.md`'s row is theirs.
- History note: the M5.6 acceptance commit had swallowed the staged asset files; the last commits were redone
  (unpushed) so the set has its own commit, the tree unchanged.

## M5.7 accepted (2026-10-24)
- `45fad59` the scene with no world, `7b19865` the picture, `6438668` the fix (the preview's systems had sat
  in `VectorSet::Draw`, gated on a `Session`; now after `VectorSet::Screens` on the editor's own condition,
  the plugin headless-safe and tested in the shell; two glyphs the font lacks replaced). The owner: "Arrows,
  WASD, and 3D view work" (the WASD report did not reproduce; a focus-plugins test is kept).
- The owner's hand test saved the town onto `packs/test` itself (canonical layout, comments dropped, resealed
  0.1.2): told, left to the owner to revert (`git checkout -- packs/test/...`); the gate ran with those two
  files stashed and restored. Their `.gitignore` and `assets/svg-icons/README.md` edits are theirs.
- **Owner's decision in the M5.7 planning:** activities are an editor grouping (Add → Activity → a list,
  Service site first, an icon per `ServiceKind`: inn, temple, trainer, smith, tavern, bank, guild); the schema
  later when a second kind has engine rules.

## Next
1. **M5.9 built** (`60ba22a`); the owner's first round found the second file showing the first's text, fixed
   in `f826536` (a fresh form or file rebuilds its column; lesson in LESSONS). The owner then asked for the
   file list as a modal picker: **M5.9b (this commit)**, `screen/editor_files.rs`, Plan | Text | Open file...
   in the tools column, the unsaved guard. Awaiting the owner's hand test (`tasks/acceptance/m5-9.md`, six
   steps on a scratch copy). After acceptance: a memory note that the owner wants pickers as modals over the
   editor, not lists in a column.
2. **M5.10 re-authoring and close** (plan row 10: `tests/edit_authoring.rs` builds each `packs/test` map from
   blank with ops only and matches the loaded def; the four maps' comments moved to notes and the files
   rewritten through the editor; `packs/test` resealed; both replays rebaselined; documents as built
   (ARCHITECTURE §8.2, presentation-ARCHITECTURE §6/§8, PRD §10 status, code-map, README); TODO M5 ticked).
   Plan mode first. The owner's accidental save of the town (still in their tree) shows the writer's layout.
3. Boundary tools after (owner, 2026-10-24); the UI icon design pass; the activities schema when a second
   kind has rules.

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
