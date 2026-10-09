# Continuity notes

Written 2026-10-09, mid protocol 2 (paused for a compact). M8 is closed (`8921c07`, owner acceptance 2026-10-08).
Branch `m7a-b-tasks`, not pushed since `4b1bd14`. Rewrite this file every time it is used. Durable knowledge
lives in `tasks/knowledge/`.

## Protocol 2: one meaning per field name (owner, 2026-10-08)
- Owner: "Close it, and fix the field names now before protocol 2"; chose **identities everywhere**. Plan
  `tasks/plans/protocol-2.md` (approved; its rule text is the ARCH §4.9 text to add in P2d). TODO item at the top
  of "M9–M12" with sub-steps.
- Found at M8 acceptance through the real bridge: `{"Cast":{"caster":2,"spell":3}}` gave
  `{"SpellCast":{"caster":2,"spell":1}}` (slot and list row in the command, CharacterId and registry number in
  the event).
- **Done:**
  - `09ea035` **P2a, members by identity:**
    - every member field in commands, refusals and views is a `CharacterId` (`Party::slot_of`, `Party::ids`);
    - short-rest dice are `[{member, count}]` (`HitDiceSpend`), rolled in marching order; `MemberTwice` new, so
      62 rejections;
    - `combat::cast::Aim` is the checked target inside a cast plan;
    - script words moved to `omnis-sim/src/word.rs`;
    - `PROTOCOL = 2`, and the bridge's instructions say so.
  - `4fec68e` **P2b, definitions by string id:**
    - spells, items (by kind; kit and stores merge by kind, so no `kit_row`) and features by string id in
      commands, refusals and views;
    - `omnis-rules` criteria are generic: `CriteriaSet<N: Names = Ids>`, `Named`, `Rename`,
      `Naming(&Data)`; the save keeps `Ids`;
    - `Word::command(world, data)` resolves rows when applied;
    - `FeatureView.name` was dropped as a duplicate of `.feature`.
- **Next: P2c, events and views in identities** (plan §P2c):
  - every `Event` field naming a definition carries a string id: `SpellCast`, `EffectApplied`, `EffectEnded`,
    `Concentration`, `SpellLearned`, `MonsterCast`, `Equipped`, `Unequipped`, `ItemMoved`, `ItemUsed`, `Sensed`,
    `Bought`, `Sold`, `Condition`, `ServiceEntered`, `ServiceLeft`, `Rumor`, `Door`, `RestEvent`,
    `EncounterStarted.stacks`, and `Reaction.action` as `ActionRef<Named>`;
  - about 35 construction sites in the sim, each with `Data` at hand. The event log is not saved, so neither the
    save nor the fingerprints move. But `ActorRef` and `Surprise` ARE saved in `CombatState`: do not rename them;
  - `Place { map: String, x, y, facing }` for `Moved.{from,to}`, `Here.position`, `Status.position` and
    `ViewportModel.map`; the World keeps `Position`;
  - `Rumor.index` becomes `rumor`, `RestEvent.index` becomes `entry`;
  - `StackView.index` becomes `stack`;
  - `Event::Dev { command }` echoes `DevCommand`, which already has ids (P2a);
  - the MCP bridge's `compact_tiles` hard-codes the `Visible`/`Sensed` `tiles` field (`omnis-mcp/src/bridge.rs`
    around line 239).
- **Then P2d:**
  - ARCH §4.9 rule text and A17 "protocol 2";
  - `docs/api.md` (§4 rule; §7–§10 tables; §14 gaps shrink; changelog "Protocol 2" with an old → new table;
    the `NoSuchMember` text and line 136 are wrong);
  - `omnis-mcp/tests/vocabulary.rs`: every key's JSON type agrees everywhere; no banned `index`/`row`/
    `caster_id`; member keys hold live ids. Prove it both ways;
  - the MCP schema's `when` tree is free-form (a gap the vocabulary test should cover through instances);
  - `tasks/acceptance/protocol-2.md`, a short MCP check: cast Bless by id, event and view agree;
  - verification and code-map knowledge files; TODO; close the item.
- **Way of working that went well:** I design the sim boundary, then a general-purpose subagent with an exact spec
  does the mechanical spread (tests, app, MCP); I review its report and decisions, run the gate and a mutation
  spot check, and commit.

## M8 acceptance (2026-10-08)
- Passed. Parts 2 and 5 were run through the real `omnis-mcp` binary over stdio
  (`<scratchpad>/mcp.py`, `Bridge(log).call(tool, args)`), because `/mcp` reconnect never launched the bridge in
  this session. The live game the owner ran is a protocol 1 build.
- The M7 log anomaly is dropped (no B6).

## Pins (after P2b)
- Gate `tests passed 577 failed 0 ignored 13`.
- Walk replay `8711507745385976768`, fight replay `5247080599556612730`, both unchanged through P2a and P2b.
- `SAVE_SCHEMA` 7, `PROTOCOL` 2, MCP 24 tools, schema proof 94/142, dev branches 13, 62 rejections.
- Base pack tuple `(4, 4, 3, 24, 16, 18, 3, 36, 7)`.

## Process
- Every cargo and gate run: `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first.
- Gate: `scripts/verify.sh > <scratchpad>/gate.txt 2>&1` in the background (5–10 min); no edits while it runs.
- Sentrux: `git add` new files, scan `/Users/john/code/omnis/crates`, then `check_rules`.
- Mutation: `python3 -u <scratchpad>/mutate.py breaks.json crate1,crate2 > log` (breaks are
  `[name, file, old, new]`; check each `old` occurs once with Python, not grep). Never under `timeout` or
  `| tail`. Afterwards check that no break's `new` text is left in the sources.
- zsh: split a file list with `${=files}` when staging by name.

## Other open TODO items (unscheduled, owner's call)
- `DevCommand::Pass { minutes }`; the `data.*` ops (labels and definitions); `scripts/mcp-probe.py`.
- Editor v1 is held "after M8, before M9" (owner, 2026-09-20).

## Carry-over
- Not built: nothing answers `SpellCast`/`EnemyCasts`; `EnemyFlees`/`OwnTurn` have no source; region catch-up
  waits for M10.
- Large files: `screen.rs` 902, `plan.rs` 978, `bake.rs` 934, `save_and_replay.rs` 810, `tactics_panel.rs` 795.
- Dated: the Socket re-audit of `rhai` 1.26.1 is due 2026-10-10.
