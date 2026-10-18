# Continuity notes

Written 2026-10-18, after M8.5 7h1 (the seal) was accepted, before a compact. Next: build 7h2. Rewrite this file every time it is used.
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
  | 7h2, schema 9 | | **next: build it** (owner, "on to 7h2") |

  7h is saves by name, in sub-steps 7h1–7h7 (TODO; the re-scope is at the top of
  `tasks/plans/m8.5-saves-by-name.md`; the full approved plan was in plan mode, summarised below).
- **Gate:** `tests passed 936 failed 0 ignored 15`, VERIFY-GREEN at `062a1fb` after a clean rebuild. Sentrux rules
  pass, quality 8982.
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
- **Build 7h2 now** (the owner said "on to 7h2"; no plan mode needed: the approved plan is above and in
  TODO's 7h items). Read the critical files cold first: `crates/omnis-sim/src/save/{mod,disk,join,fold,
  tidy,character}.rs`, `world.rs` (`MapState.cleared`, `checked` ~L547), `encounter.rs` (`EncounterSource::
  Fixed(u16)` ~L28, set ~L125/L287, read ~L407, `beings.rs:61`), `journal.rs` (`Change::Cleared` ~L47,
  made ~L258, laid ~L324), `omnis-data/src/encounter.rs` (`FixedEncounter` ~L68; `Stand.id` already
  exists ~L57), `migrate.rs`, the hosts' `pack.reload` (`omnis-vector/src/shell/host.rs:37`,
  `omnis-cli/src/headless.rs:83`).
- **7h2 items, red tests first:**
  1. `pack.reload` sets `world.packs` (red test: reload, save, load unforced must not be `PackMismatch`).
  2. `FixedEncounter.id` required (`[a-z0-9_]{1,32}`, unique per map; reuse the object-id check), bad-pack
     row; the test pack's maps gain ids (and the objects/levels/heights fixture packs if they have fixed
     encounters), resealed with `omnis-cli pack seal`.
  3. `MapState.cleared: BTreeSet<String>`, `EncounterSource::Fixed(String)`, `Change::Cleared { map, id }`;
     `SAVE_SCHEMA` 9; v8 → v9 converts rows to ids through the current maps (a row out of range refuses the
     load); the whole-world v1–v7 path converts too; the goldens rebaseline for the schema.
  4. `Manifest.names: NameTable` written by `save_files` (`NameTable::of(&Data)`, serde default, skipped
     when empty); a new `save/names.rs`.
  5. `CharacterSave.archive: [Segment { world: Option<String>, name }]`; `Archived::Held.from:
     Option<Manifest>`; tidy counts other folders' segments and manifests.
  6. `base_zero` → `Bases::default()` (plus a test that `split(World::new(..))` gives it).
  7. A frozen v9 fixture `crates/omnis-sim/tests/saves/v9/` (as v8: paths without `.omnis/`, written by an
     ignored test through `omnis_data::ron_io`, loaded by a pinned test); v8 still loads (forced) and
     converts.
  - Visible end: a save made before 7h2 loads (forced, via MCP `save_read force`) with its cleared
    encounters still cleared; the debug World page shows the manifest's name count. Then the acceptance
    script `tasks/acceptance/m8.5-7h2.md`, TODO review, docs touch (api.md changelog, ARCH §4.1.1), commit,
    continuity, report with numbered try-it steps.

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
- **Replays:** walk `14e46c0bea591749`, fight `aed8e1466adc88ea` (rebaselined in 7h1 for the pack hashes). `SAVE_SCHEMA` 8,
  `PROTOCOL` 2.
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
- **Commits:** stage new files by name; build the path list from `git status --short` without `assets/`,
  `.claude/` or `docs/dnd_supplemental/`, write it to a scratchpad file, `git commit --pathspec-from-file=<file>
  -F -`. Messages end with the Co-Authored-By line and, when the system gives one, the Claude-Session line. Never push. Owner world docs
  (`docs/background`, `docs/locations`) go in their own commit, unedited.
- **Context:** pause and recommend a compact past 350K.
