# Continuity notes

Written 2026-10-10, mid protocol 2 (paused for a compact before P2d). M8 is closed (`8921c07`, owner acceptance
2026-10-08). Branch `m7a-b-tasks`, not pushed since `4b1bd14`. Rewrite this file every time it is used. Durable
knowledge lives in `tasks/knowledge/`.

## Protocol 2: one meaning per field name (owner, 2026-10-08)
- Owner: "Close it, and fix the field names now before protocol 2"; chose **identities everywhere**. Plan
  `tasks/plans/protocol-2.md` (approved; its "The rule" section is the ARCH §4.9 text to add in P2d — approving
  the plan approved that text). TODO item at the top of "M9–M12" with sub-steps; P2a–P2c are checked off there
  with full detail.
- **Done:**
  - `09ea035` **P2a**: members by `CharacterId` in commands, refusals, views; `PROTOCOL = 2`; script words in
    `omnis-sim/src/word.rs`.
  - `4fec68e` **P2b**: spells, items, features by string id in commands, refusals, views; generic criteria
    (`CriteriaSet<N: Names = Ids>`, `Named`, `Naming(&Data)`); the save keeps `Ids`.
  - `fe34da6` **P2c**: events and views in identities.
    - `omnis-sim/src/names.rs`: `Place { map, x, y, facing }` (`Place::of`), `id_of` (`#n` for an unnamed
      number), `action_named`. Exported from the crate root and `api`.
    - Every event definition field is a string id; `Reaction.action: ActionRef<Named>`; `Rumor.rumor`,
      `RestEvent.entry`; `Moved.{from,to}`, `Here.position`, `Status.position`, `CombatView.retreat` are `Place`;
      `Here.map` and `Status.map` were DROPPED (duplicates of `position.map`); `ViewportModel.{map,tileset}`
      strings; `StackView.stack`.
    - Beyond the plan (reported to the owner): `Exchanged { member, with }` by id (was slots `a`/`b`);
      `Reconciled.{a,b}` and `TimeAdvanced.holder` are strings (`party:0` or the region's id).
    - Effects helpers and a few others take `data` now. App: `defs::{map, tileset, position}`; text lookups by
      string id; combat text "A and B exchange".
    - Still numeric by design (say so in the docs): `EraId` (runtime count), `ActorRef::Stack`/`Monster`
      (positions, saved), `ViewTile.terrain` (row of the map's terrains), `query::automap(MapId)` in the Rust
      tier, `world_get` paths (the saved world), `EncounterSource::Fixed(u16)` (placement row, saved).
- **Next: P2d, the contract, the drift guard, the docs** (plan §P2d). Ask nothing more about ARCH: the plan's
  rule text is approved.
  1. ARCH §4.9: add the rule (plan "The rule" items 1–5); A17 gains "protocol 2: one meaning per name".
  2. `docs/api.md`:
     - §4 states the rule (line ~136 currently states it wrongly);
     - §7–§10 field tables; known stale bits: examples at lines ~89/96 (`"map":3`, `"holder":{"Party":0}`,
       `"protocol":1`), the `Position` row, §7.1 `Status` still lists `map`, §7.4 `retreat: Position` and
       `StackView.index`, the `EncounterStarted` row `[[MonsterId, count]]`, the note near line 759 about numeric
       registry ids, the `NoSuchMember` text;
     - §14 gaps shrink (events no longer carry registry numbers; labels still need the `data.*` ops);
     - changelog "Protocol 2 (2026-10-xx)" with an old → new table covering P2a–P2c and the migration.
     - `tests/api_doc.rs` checks only event names; it will not catch field drift.
  3. `crates/omnis-mcp/tests/vocabulary.rs`: collect every JSON key from the schema proof's command instances,
     every event of the golden walk and fight, and replies from a live headless world. Fail when one key has
     different JSON types in different places; a member-named key (`member`, `caster`, `with`, `target`,
     `{"Member":…}`) holds a number that is not a live `CharacterId`; a banned key appears (`index` outside
     `Monster`, bare `row`, `caster_id`, `kit_row`?). Prove it both ways (a planted numeric `spell` in an event
     fails it), modelled on `the_proof_catches_a_schema_that_drifted`. Also covers the MCP schema's free-form
     `when` tree through instances.
  4. `tasks/acceptance/protocol-2.md`: short owner check over MCP — `game_status` says protocol 2 and
     `position.map` is a string; cast Bless by id; the `SpellCast` event, `party_get` and `cast_get` agree on
     `base:spell:bless` and the caster id. "What you will see": nothing changes in the window.
  5. Knowledge files (`tasks/knowledge/verification.md` pins, `code-map.md` gains `names.rs`/`word.rs`), TODO P2d
     and the parent item checked, plan marked closed.
- **Way of working that went well:** I design the sim boundary, then a general-purpose subagent with an exact spec
  does the mechanical spread (tests, app, MCP) and reports judgement calls and leftovers; I review the diff at the
  judgement sites, run a mutation spot check, Sentrux, and commit.

## Pins (after P2c)
- Gate `tests passed 580 failed 0 ignored 13`.
- Walk replay `8711507745385976768`, fight replay `5247080599556612730`, unchanged through P2a–P2c.
- `SAVE_SCHEMA` 7, `PROTOCOL` 2, MCP 24 tools, schema proof 94/142, dev branches 13, 62 rejections.
- Base pack tuple `(4, 4, 3, 24, 16, 18, 3, 36, 7)`. Sentrux quality 9038, rules pass.

## Process
- Every cargo and gate run: `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first.
- Gate: `scripts/verify.sh > <scratchpad>/gate.txt 2>&1` in the background (5–10 min); no edits while it runs.
- Sentrux: `git add` new files, scan `/Users/john/code/omnis/crates`, then `check_rules`.
- Mutation: `python3 -u <scratchpad>/mutate.py breaks.json crate1,crate2 > log` in the background (breaks are
  `[name, file, old, new]`; check each `old` occurs once with Python, not grep; each break takes ~15 min on
  omnis-sim). Never under `timeout` or `| tail`. Afterwards check no break's `new` text is left in the sources.
- zsh: split a file list with `${=files}` when staging by name.

## Other open TODO items (unscheduled, owner's call)
- `DevCommand::Pass { minutes }`; the `data.*` ops (labels and definitions); `scripts/mcp-probe.py`.
- Editor v1 is held "after M8, before M9" (owner, 2026-09-20).

## Carry-over
- Not built: nothing answers `SpellCast`/`EnemyCasts`; `EnemyFlees`/`OwnTurn` have no source; region catch-up
  waits for M10.
- Large files: `plan.rs` 980 (P2c grew it), `bake.rs` 934, `screen.rs` 902, `save_and_replay.rs` 810, `tactics_panel.rs` 795.
- **Due today (2026-10-10):** the Socket re-audit of `rhai` 1.26.1. Raise it with the owner.
