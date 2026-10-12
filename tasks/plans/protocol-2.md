# Protocol 2: one meaning per field name (identities everywhere)

**Closed 2026-10-10** (P2a–P2d; the record is in `tasks/TODO.md`): accepted by the owner the same day
(`tasks/acceptance/protocol-2.md`, all three parts). As built, beyond this plan: 11 more renames the vocabulary test found
(owner: "Rename all 11"), `on` kept as the bool it is (not a member key), and one integration test binary per
crate (`8eed1cc`).

## Context
M8's acceptance (2026-10-08) ran the bridge against the live game and found field names that mean different
numbers depending on where they appear. Casting Bless with `{"Cast":{"caster":2,"spell":3,…}}` produced
`{"SpellCast":{"caster":2,"spell":1}}`:
- in the command, `caster` is a position in marching order and `spell` a row in that caster's known list;
- in the event, `caster` is a `CharacterId` and `spell` is the spell's number in the pack registry;
- in the view, `spell` is a string id.

An inventory (two explorations) found twelve colliding names: `member`, `caster`, `spell`, `item`, `target`,
`index`, `map`, `service`, `condition`, `feature`, `id` and `row`. `docs/api.md` line 136 states the rule wrongly.

**Owner, 2026-10-08:**
- "fix the field names now before protocol 2";
- chose **identities everywhere**: a bare noun names an identity in commands, events, views and rejections alike.

## The rule (added to ARCH §4.9 and `docs/api.md` §4; approving this plan approves the text)
1. **A member is named by its `CharacterId`, wherever it appears:**
   - fields: `member`, `caster`, `with`, `from`, `to`, `target`;
   - `{"Member": id}` in `Target`, `ActorRef`, `EffectTarget` and `ItemPlace`;
   - `Reorder.order`.
   Commands no longer address marching-order positions, so a command survives a reorder.
2. **A definition is named by its string id** (`pack:kind:name`; a feature by its name key), wherever it appears:
   - fields: `spell`, `item` (an item *kind*), `monster`, `map`, `service`, `condition`, `class`, `feature`;
   - `ActionRef::{Spell, Item}`, and the `Predicate`'s monster and condition.
   No registry number crosses the protocol.
3. **A thing with no identity is named by its position, under a name that says which list:**
   - `stack` (a stack in the fight);
   - `{"Monster": {"stack", "index"}}` (an individual among a stack's living);
   - `kit_row` (a carried item: two daggers are two rows);
   - `stores_row` only where stores can repeat a kind;
   - `entry` (a runbook entry, was `at`).
   No other `index` or `row` field remains. Lists in views are in marching or list order.
4. **A place on a map is `Place { map: <string id>, x, y, facing }`** in events, views and `Status`.
5. **The `World`'s own fields keep their registry numbers.** They are not API (§4.9). The wire types convert at the
   boundary (resolve on `apply`, name on view and event). So the save and the fingerprint do not move.

The fix bumps `ops::PROTOCOL` from 1 to 2. `docs/api.md` gets a changelog entry with an old-to-new table, and
§14 shrinks: events stop carrying registry numbers, though labels still need the `data.*` ops.

## What stays fixed (checked at every commit)
- **`SAVE_SCHEMA` 7**, no migration: the saved runbook keeps `Spell(15)` internally, and the command and view
  carry its string form.
- **Walk replay `8711507745385976768` and fight replay `5247080599556612730`:**
  - the worlds are identical, so the fingerprints must not move;
  - `fight.ron` is re-recorded only because its command *text* changes;
  - `walk.ron` holds no fields and should not change.
- **Nothing in play changes.** The app sends the same actions, addressed by id.

## Sub-steps (one commit each, gate green, Sentrux, staged by name, not pushed)

**P2a. Members by identity**
- **Commands:**
  - `Command::Cast.caster`, `Target::Member`;
  - `CombatCommand::{Use.target, Exchange.with}`, `FeatureChoice::Exchange.with`;
  - `PartyCommand::Reorder.order`, the `TacticsCommand` `member`s;
  - `ItemCommand` `member`, `from`, `to`, `target`; `ServiceCommand` `member`s;
  - `RestCommand::Short.dice`, which becomes `[{member, count}]`;
  - `DevCommand` `member`s.
  All become `CharacterId`, resolved to a slot in one helper (`party.slot_of(id) -> Option<u8>`).
- **Rejections:** a missing member is `NoSuchMember { member: CharacterId }`; the twelve `index` rejections
  become `member`.
- **Views:**
  - `MemberView.index`, `FighterView.index` and `OfferView.member` are dropped or turned into ids;
  - `CastView.caster` becomes the id, and `caster_id` goes.
- **App:** every sender takes `MemberView.id`.
- **MCP:** `schema.rs` changes the `Member` schema; the bridge test literals follow.
- `PROTOCOL = 2` from this commit on (the bridge's instructions line too), and the changelog is started.

**P2b. Definitions by string id in commands, rejections and the runbook**
- **Commands:**
  - `Command::Cast.spell`, `CombatCommand::{Cast.spell, Feature.feature}`;
  - `ServiceCommand::{Buy.item, Sell.item, Choose.spell, Learn.spell}`;
  - kit commands become `kit_row`.
- **The runbook's wire form:** `PutReaction.set` gets a wire `CriteriaSet` with strings, converted to the saved
  numeric form on apply. `ReactionView` and `AnswerView` show strings.
- **Rejections:**
  - the spell and feature rejections carry the string;
  - `NotEnough.item` becomes a string;
  - `UnknownItem` and `NotInStores` become `kit_row` and `stores_row`;
  - `NoSuchSpell.row` becomes `spell`;
  - `UnknownId` keeps the asked id as written.
- **Uniqueness, checked at the start of the step:**
  - if a service's stock or the party's stores can repeat an item kind, that command keeps a named row
    (`stock_row` or `stores_row`) instead of the id;
  - the pack loader refuses a duplicate in a known-spell or service-spell list, if it doesn't already.

**P2c. Events and views speak in identities**
- **Events** carry string ids for every definition:
  - `SpellCast`, `EffectApplied`, `EffectEnded`, `Concentration`, `SpellLearned`, `MonsterCast`;
  - `Equipped`, `Unequipped`, `ItemMoved`, `ItemUsed`, `Sensed`, `Bought`, `Sold`;
  - `Condition`, `ServiceEntered`, `ServiceLeft`, `Rumor`, `Door`, `RestEvent`, `EncounterStarted.stacks`,
    `Reaction.action`.
  - About 35 places build these, and each already has `Data` at hand.
- **Places:** `Moved.{from,to}`, `Here.position`, `Status.position` and `ViewportModel.map` become `Place`.
- **`Rumor.index` and `RestEvent.index`** become `rumor` and `entry`, named rows of the service's or map's list.
- **Script words** (`cast-N-mM`, …) stay positional, since they're a typing convenience:
  - `parse_script` yields words, and each is resolved against the world when applied (`Word::resolve`);
  - `omnis-cli play`, the app's dev script and the golden-replay generators resolve as they go;
  - replay files store resolved commands.

**P2d. The contract, the drift guard, the docs**
- **ARCH §4.9:** the rule above, and A17 gains "protocol 2: one meaning per name".
- **`docs/api.md`:**
  - §4 states the rule;
  - §7–§10 field tables updated;
  - §14 shrinks;
  - changelog "Protocol 2 (2026-10-xx)" with the rename table and the migration.
- **A vocabulary test**, `omnis-mcp/tests/vocabulary.rs`, collects every JSON key from:
  - the schema proof's command instances;
  - every event of the golden walk and fight;
  - every reply from a live headless world.
  It fails when:
  - one key carries different JSON types in different places (`spell` a number here, a string there);
  - a member-named key carries a number that isn't a live `CharacterId`;
  - a banned key appears (`index` outside `Monster`, bare `row`, `caster_id`, …).
  This is what stops the problem coming back.
- **Other docs:**
  - `verification.md`'s pins and the schema proof numbers if they move;
  - code-map;
  - TODO item checked.
- **`tasks/acceptance/protocol-2.md`:** a short owner check over MCP. Cast Bless by id, and see the event and
  the view use the same id.

## Critical files
- **Sim:**
  - `crates/omnis-sim/src/command.rs` (Command, Rejection, script words);
  - `combat/mod.rs` and `combat/cast.rs` (`Target`);
  - `items.rs`, `service.rs`, `tactics.rs`, `party.rs`, `rest.rs`, `dev.rs`;
  - `event.rs`, `casting.rs`, `effects.rs`;
  - `party_view.rs`, `view.rs`, `service_view.rs`, `cast_view.rs`, `query.rs`;
  - `ops.rs` (`PROTOCOL`, `Status`);
  - `omnis-rules/src/tactics.rs` (wire `CriteriaSet` beside the saved one).
- **MCP:**
  - `crates/omnis-mcp/src/schema.rs` (the hand-written input schema) and `bridge.rs` (instructions,
    `compact_tiles`);
  - `tests/{bridge.rs, schema_proof.rs}` and the new `tests/vocabulary.rs`.
- **App:**
  - the ~160 command senders: `service_panel.rs`, `debug_panel.rs`, `spell_menu.rs`, `inventory_menu.rs`,
    `combat_menu.rs`, `debug_menu.rs`, `use_menu.rs`, `tactics_panel.rs`, `feathers_service.rs` and a few more;
  - `defs.rs` already turns view ids into definitions.
- **Data:** `crates/omnis-sim/tests/replays/fight.ron` is re-recorded with `rebaseline_fight_replay`, and the
  fingerprint must come back the same.
- **Docs:** `ARCHITECTURE.md` §4.9, A17; `docs/api.md`; `tasks/knowledge/{verification,code-map}.md`.
- **Reuse:**
  - the boundary pattern of `Draft` (strings resolved on apply; `omnis-rules/src/character.rs:113`);
  - `defs.rs` in the app;
  - `the_proof_catches_a_schema_that_drifted` as the model for the vocabulary test's self-check.

## Verification
- `DEVELOPER_DIR=/Library/Developer/CommandLineTools scripts/verify.sh` prints VERIFY-GREEN at every commit.
- Both fingerprints and `SAVE_SCHEMA` 7 stay unchanged at every commit. That is the proof that only names moved.
- **Red-first:** a test that casts Bless by id, then asserts the `SpellCast` event, the `EffectView` and the
  `CastView` all carry `"base:spell:bless"` and the caster's `CharacterId`. It fails today on 1 vs 3.
- **The vocabulary test, proven both ways:** a planted numeric `spell` in one event fails it.
- **After a reorder:** reorder the party and send a command by id. It reaches the same member, where under
  protocol 1 it would have reached the member now in that slot.
- **Mutation spot checks** on `slot_of`, the criteria conversion and `Word::resolve`. A break counts only with a
  named failing test.
- **Over the real bridge,** with the `scratchpad/mcp.py` driver or the session's tools:
  - `game_status` says `protocol: 2`;
  - cast Bless by id; the event and `party_get` agree.
- Sentrux after each sub-step.
- **What you will see:**
  - in the window, nothing changes;
  - over MCP, commands name members by id and spells, items and services by their `pack:kind:name`, and events
    and views use the same names;
  - `game_status` shows protocol 2.

## Size
About 2,500 lines in all:
- the app's senders, about 700;
- the sim's wire types and conversion, about 900;
- MCP schema and tests, about 400;
- docs, about 300;
- script words, about 200.
