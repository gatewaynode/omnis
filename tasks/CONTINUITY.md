# Continuity notes

Written 2026-10-18, after M8.5 7h2 (schema 9) was built and committed. Next: the owner walks 7h2; then build 7h3. Rewrite this file every time it is used.
Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes). Untracked and never staged: the four files under
  `assets/` and `docs/dnd_supplemental/` (the owner's).
- **Step 7, the save model (D28), plan `tasks/plans/m8.5-save-model.md` (amended 2026-10-17):**

  | Sub-step | Commit | State |
  | --- | --- | --- |
  | 7a, the journal and its measurement | `8d8773e` | accepted |
  | 7b, `split`/`join` in memory | `d5f14d6` | accepted |
  | 7c, flag scope (`data/flags`) | `ae99144` | accepted |
  | 7d, the character save and world folder | `f0977b8`, `55c69b6`, `18f8a2d` | accepted 2026-10-18 |
  | 7e, the fold, the archive and tidy | `b43443c`, `a19633a` | accepted 2026-10-18 (`b467758`) |
  | 7f, the party's log saved and retold | `e04eebf`, `1964e0e` | accepted 2026-10-18 (`5070624`) |
  | 7g, docs and pins | `c552aa7` (fix), `9e703b5`, `a4dd48e` | accepted 2026-10-18 (`f243e1d`) |
  | 7h1, the seal | `062a1fb`, `318603f`, script `97d62ab` | accepted 2026-10-18 |
  | 7h2, schema 9 | `9495867`, docs `ca9c556` | **built, awaiting the owner's walk** (`tasks/acceptance/m8.5-7h2.md`) |

  7h is saves by name, in sub-steps 7h1–7h7 (TODO; the re-scope is at the top of
  `tasks/plans/m8.5-saves-by-name.md`; the full approved plan was in plan mode, summarised below).
- **Gate:** `tests passed 948 failed 0 ignored 15`, VERIFY-GREEN at `9495867`. Sentrux rules pass, quality 8982.
  7h2 mutations 16 of 16.
- Owner world docs `docs/locations/the_floating_isles_of_the_west_reach.md` and `..._paratoron_empire_...` committed
  alone (`2368ea8`).
- Owner world doc `docs/background/introduction.md` changed twice and was committed alone each time (`63c61c5`,
  `2b7a62b`).

## 7h, the approved plan (owner 2026-10-18; read before each sub-step)
- **Owner's choices:** sub-steps each walked; all six migration kinds plus `migration draft`; saves made
  before 7h (no name table) load on changed packs only when forced (the break accepted); a log line on a
  removed map is kept, its objects unnamed; mid-fight saves stay allowed.
- **Name table in each world folder's manifest** (`Manifest.names`, serde default, skipped when empty so v8
  world ids hold): per kind the string ids by old number (maps, flags, regions, items, spells, monsters,
  conditions, races, classes, backgrounds, services), party-scoped flags, each map's terrain keys, each object
  kind's `once` interaction signatures (verb, label, state). Ids are interned in sorted string-id order per kind.
- **Load:** read in the save's own numbering (region bases keyed through the save's table), `join_raw`, lay
  the journal, **always** check the fingerprint; then tiers: same packs as today; same names (content-only
  edits) set `world.packs`, check places, no fold; different names remap the live World (`Remapper`, Missing
  with paths), `checked(new)`, then **fold** (journal archived in old numbering under its old folder; history
  never remapped). No table on other packs: `force` only.
- **Remap covers live state only:** position, maps (objects' contents and `used`, piles), automap (terrain by
  key), known, mode (encounter stacks, retreat, source; combat loot; town service), flags, clocks, contacts
  (+ `other`), rngs, party, characters (race, class, background, spells, kit, conditions, equipped, effects,
  tactics via `omnis-rules` `Rename`), `Logged.map`. Bus region topics rebuilt from `time::subscriptions`.
  `time:<h>:<h>` stream names parsed, remapped and **rebuilt through `StreamName::time`** (order can flip).
  Keyed maps rebuilt into new maps. Proof: an overlay pack shifting every number, all views equal (exploring,
  town, mid-fight), a mutation per field.
- **Also:** `base_zero` → `Bases::default()` (World::new puts nothing in a base); `CharacterSave.archive` →
  `[Segment { world, name }]`, `Archived::Held.from: Option<Manifest>`, tidy counts other folders; fixed
  encounters by required id, `SAVE_SCHEMA` 9; `pack.reload` must set `world.packs` (bug, red test in 7h2) and
  later go through `ops::reload`.
- **Sub-steps:** 7h2 schema 9 (encounter ids, names written, segments, base 0, reload packs, v9 fixture);
  7h3 remap; 7h4 report, `save.check`, MCP `save_check` (27 tools), load screen; 7h5 reload by name;
  7h6 migrations (the version chain: decide how patch bumps without a migration file chain); 7h7 docs (PRD
  §7.7 needs the owner's approval).

## Next
- **The owner walks 7h2.** Step 2 of the script is a forced load the owner asks for in session: call the omnis MCP
  `save_read` with `path: ".omnis/saves/folded-v8.ron"`, `force: true` (and `journal-v8.ron` for step 6), with the
  game running (`just run --windowed --pack packs/base --pack packs/test`, a new game started). Expected: "Loaded
  …: converted from a schema 8 save", the goblins' encounter at (3, 14); the World page "Manifest: 109 names, …".
- **Open for the owner (in the report):** the pack data `SCHEMA` stays 1 though `FixedEncounter.id` is now required
  and `Stand.id` is gone (only our packs exist); a bump would touch every data file.
- **7h2 as built (what 7h3 builds on):** `save/rows.rs` (rows read as `#n` markers via `row_or_id`/`rows_or_ids`
  deserializers on `MapState.cleared`, `EncounterSource::Fixed`, `Change::Cleared` (`alias = "index"`); `rows::world`
  in `World::from_ron`, `rows::parts` + `rows::log` in `load_save`); `save/names.rs` (`NameTable::of`, `len`,
  `Signature`; `Manifest::of(world, data)` used by `split` and `save_files`); `disk.rs` `CHARACTER_SCHEMA` 8,
  `converted` skips the fingerprint for schema 8; `Segment { world, name }`, in memory always `Some(folder)`, saved
  `None` for the save's own; `Archived::Held { journal, from: Option<Box<Manifest>> }` (7h3/7h5 set `from` on a
  names-change fold); `ops::reload` sets `world.packs`. A forced load keeps the save's packs on the world, so its
  next save is still "other packs" until 7h3's tiers.
- **Then 7h3, the remap** (plan above): load in the save's numbering (region bases keyed through the save's table),
  `join_raw`, journal, always the fingerprint, then the tiers; `Remapper` over live state; `time:` streams rebuilt
  through `StreamName::time`; bus from `time::subscriptions`; fold on a names change with `Held.from`; the overlay
  pack proof (every view equal: exploring, town, mid-fight) and a mutation per field. Visible: add a spell sorting
  first in `packs/test`, reseal, load: the wizard's spells unchanged, "The world has moved on".

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
- **Replays:** walk `8eab9a1dd95b472d`, fight `79ca2704c92bc312` (rebaselined in 7h2: test pack 0.1.1, schema 9).
  `SAVE_SCHEMA` 9, `PROTOCOL` 2. The base and test packs give 109 names.
- **MCP:** command `oneOf` 14, dev `oneOf` 14, item `oneOf` 10, schema proof 117/168, tools 26, ops 26.
- **Counts:** `Command` 16, `DevCommand` 14, `Event` 73, `Rejection` 71. The base pack has 19 spells.

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
  mutant in `target/`; `python3 -u` to a file), then `git status` to confirm the sources are clean.
- **Each new command, event or rejection** also moves: `docs/api.md` and its count lines, ARCHITECTURE §4.2's counts, `code-map.md`'s command list (all pinned by `api_doc.rs`), `api_doc.rs` pins
  and `variants!` lists, the MCP schema (`schema.rs` and its `oneOf` count test) and `schema_proof.rs`
  (instances, offered, the drift test's index), `common/commands.rs`, the word in `word.rs`, client
  `labels::refused`.
- **Commits:** stage new files by name with `git add` first (`--pathspec-from-file` does not add untracked files); build the path list from `git status --short` without `assets/`,
  `.claude/` or `docs/dnd_supplemental/`, write it to a scratchpad file, `git commit --pathspec-from-file=<file>
  -F -`. Messages end with the Co-Authored-By line and, when the system gives one, the Claude-Session line. Never push. Owner world docs
  (`docs/background`, `docs/locations`) go in their own commit, unedited.
- **Context:** pause and recommend a compact past 350K.
