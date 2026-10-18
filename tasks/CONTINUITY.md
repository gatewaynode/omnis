# Continuity notes

Written 2026-10-17, after M8.5 7d was built, before a compact. Rewrite this file every time it is used.
Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `M8.5-tasks`**, not pushed (the owner pushes). Untracked and never staged: the four files under
  `assets/` and `docs/dnd_supplemental/` (the owner's).
- **Step 7, the save model (D28), plan `tasks/plans/m8.5-save-model.md`:**

  | Sub-step | Commit | State |
  | --- | --- | --- |
  | 7a, the journal and its measurement | `8d8773e` | accepted |
  | 7b, `split`/`join` in memory | `d5f14d6` | accepted |
  | 7c, flag scope (`data/flags`) | `ae99144` | accepted |
  | 7d, the character save and world folder | `8f4d134`, `1906f45` | built, **awaiting the owner's walk of `tasks/acceptance/m8.5-7d.md`** |

  Then 7e (fold), 7f (full log across loads), 7g (docs and pins), 7h (the old saves-by-name plan, re-scoped).
- **Fixes made during the walks:**
  - a text field kept the keyboard through a rebuild (`9a208c9`, lesson written);
  - the 7c script now says the yard's lever is hidden (`8e7ac5c`, lesson written).
- **Gate:** `tests passed 900 failed 0 ignored 14`, VERIFY-GREEN at `1906f45`. Sentrux (scan `crates/`, not the
  repo root) rules pass, quality 8978.

## 7d as built (read TODO's 7d review for the whole)
- **The save** is `.omnis/quick.ron`, a character save written as compact RON: `CharacterSave { schema 8, world
  id, bases, checkpoint, journal: [Run {command, times}], fingerprint }`.
- **The world folder** is `.omnis/worlds/<16 hex>/`, holding `manifest.ron`, `world/<hash>.ron` and
  `regions/<pack>/<name>/<hash>.ron`.
- **`World.origin: Option<Split>`** (`None` = base 0). Until 7e, a new game's save replays from the start, and
  only a converted old save writes base files.
- **The code:** `save::{save_files, load_save}`; hosts use `ron_io::{write_files, read_under}`. `ops::save_text`
  and `load_text` are gone.
- **Measured:** 10k commands = 120 KB and a 0.2 s load; 50k = 605 KB and 1.8 s. Load time bounds folds, about
  25k commands for 1 s.
- **Open for 7e:**
  - `rules.set`/`pack.reload` change rules without a command, so a later save may not replay; a fold should
    rebase at that moment.
  - The provisional fold rule is to be set from the numbers above.
  - The owner's commands-an-hour figure from play is still to come.
- **The owner's old `.omnis/quick.ron`** (objects pack, pre-7c) is refused for other packs until 7h. The 7d
  script uses `crates/omnis-sim/tests/saves/v7.ron` instead.

## Next
- When the owner says the 7d walk passed, mark 7d accepted in TODO, then plan 7e in plan mode: `DevCommand::Fold`,
  `Event::Folded` (Event 73), the HUD line "The world has moved on", the provisional size rule at safe points
  (long rest, entering a settlement), archive segments (the entry events deferred from 7a), kept named bases,
  and a manual tidy (`omnis-cli world tidy`). Each new command or event moves the API pins (see Process).

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
- **Replays:** walk `d9f7bb81ad3a7a25`, fight `69264fcb84cbbb06` (rebaselined in 7d). `SAVE_SCHEMA` 8,
  `PROTOCOL` 2.
- **MCP:** command `oneOf` 14, item `oneOf` 10, schema proof 116/166, tools 26, ops 26.
- **Counts:** `Command` 16, `Event` 72, `Rejection` 71. The base pack has 19 spells.

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
