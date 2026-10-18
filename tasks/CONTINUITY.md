# Continuity notes

Written 2026-10-18, after M8.5 7d was reworked (the party saved as it stands, a journal of world changes). Rewrite this file every time it is used.
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
  | 7d, the character save and world folder | first `8f4d134`, `1906f45`; reworked `f0977b8`, `55c69b6`, `18f8a2d` | built, **awaiting the owner's walk of `tasks/acceptance/m8.5-7d.md`** |

  Then 7e (fold), 7f (full log across loads), 7g (docs and pins), 7h (pack changes).
- **The rework (owner, 2026-10-17):** "CRPGs often have 4 - 5 second load times"; "saves should only be capturing
  interactions, movement does not impact the world besides eating time." The owner chose "the party now plus the
  world's changes" over "replay with travel folded", and found objects and lore told as the party's knowledge.
  PRD D28 and §7.7 amended (approved in the plan).
- **Gate:** `tests passed 903 failed 0 ignored 14`, VERIFY-GREEN at `55c69b6`. Sentrux (scan `crates/`) rules
  pass, quality 8983.

## 7d as built (read TODO's 7d review for the whole)
- **The save** `.omnis/quick.ron`: `CharacterSave { schema 8, world id, bases, checkpoint (the party at save
  time), journal: [Entry { minute, at, command, changes: [Change] }], fingerprint }`, compact RON.
- **The journal** holds only commands that changed what the world holds. `apply` diffs `Held` (map states,
  world-scope flags, non-party clocks, contacts and streams) before and after each command. Entering another
  region IS an entry (its clock and contact are the world's).
- **Load:** join the bases with the party, `apply_changes` per entry, compare the fingerprint. No rule runs.
- **`World::known`** (`objects/knowledge.rs`): noticed, told and cell lore, the party's. `World::prune` after
  every command and on load.
- **Measured:** 10k wander, 34 interactions, 94 KB, 103 ms; 50k, 166, 158 KB, 297 ms. Most of the save is the
  automap.
- **The owner's old `.omnis/quick.ron`** (objects pack, pre-7c) is refused for other packs until 7h; the script
  uses `crates/omnis-sim/tests/saves/v7.ron` (base and test packs only).

## Next
- When the owner says the 7d walk passed, mark 7d accepted in TODO, then plan 7e in plan mode:
  - a fold applies the journal into new bases (`apply_changes` exists) and archives the entries;
  - `DevCommand::Fold`, `Event::Folded` (Event 73), the HUD line "The world has moved on";
  - a provisional threshold by the journal's bytes at safe points (a long rest, entering a settlement);
  - kept named bases and a manual tidy (`omnis-cli world tidy`).

  Each new command or event moves the API pins (see Process).
- **Open for 7f:** party-only events (a fight won against a random encounter, a purchase) leave no journal entry,
  so History across loads needs the party's own chronicle.

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
- **Replays:** walk `d9f7bb81ad3a7a25`, fight `69264fcb84cbbb06` (rebaselined in 7d; the rework moved neither). `SAVE_SCHEMA` 8,
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
  -F -`. Messages end with the Co-Authored-By line and, when the system gives one, the Claude-Session line. Never push. Owner world docs
  (`docs/background`, `docs/locations`) go in their own commit, unedited.
- **Context:** pause and recommend a compact past 350K.
