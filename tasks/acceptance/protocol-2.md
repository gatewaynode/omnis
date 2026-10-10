# Acceptance: protocol 2 (one meaning per field name)

The script for your acceptance of protocol 2 (`tasks/plans/protocol-2.md`). It covers the engine's API only:
every field name on the wire has one meaning. A member is its `CharacterId`, a definition its string id
(`pack:kind:name`), a place a `Place` with the map's string id, and the rest positions in named lists
(ARCHITECTURE §4.9, `docs/api.md` §4).

**What you will see.** In the window, nothing changes: the app sends the same actions, now addressed by id.
Over MCP, commands name members by id and spells, items and services by their string ids, and the events and
views name them the same way. `game_status` says protocol 2.

Parts 1 and 2 use the MCP bridge (`.mcp.json`; restart the bridge after building): ask me to run them, or call
the tools yourself. Part 3 is in the window (`cargo run -p omnis-app`).

## Setup
**New game** in the window (a dev build), with a cleric who knows Bless in the party, or a headless bridge with a
party created through `party_create`.

## Part 1: the status
1. Call `game_status`.
   *You will see* `"protocol": 2`, and `position` as `{"map": "test:map:town", "x", "y", "facing"}`: the map by
   its string id, not a number. There is no separate `map` field any more.
   Covered by `omnis-mcp/tests/replies.rs::every_reply_names_itself_and_reads_back_without_its_op` and
   `omnis-sim/tests/movement.rs::a_step_through_a_portal_names_where_it_lands_as_every_view_does`.

## Part 2: one cast, one name
1. Call `party_get` and note the cleric's `member` (a number) and that `spells` lists `"base:spell:bless"`.
2. Call `cast_get`.
   *You will see* a row with `"caster"` the same number and `"spell": "base:spell:bless"`.
3. Call `sim_command` with
   `{"Cast": {"caster": <that number>, "spell": "base:spell:bless", "target": {"Member": <that number>}}}`.
   *You will see* a `SpellCast` event with the same `caster` and `"spell": "base:spell:bless"`, and an
   `EffectApplied` naming the same spell. Under protocol 1 the event said `"spell": 1` while the command said
   `3`.
4. Call `party_get` again.
   *You will see* the member's `effects` row with `"spell": "base:spell:bless"` and the same `caster`.
   Covered by `omnis-sim/tests/api_views.rs::bless_is_cast_by_its_id_and_the_row_carries_that_id`.
5. Reorder the party (`sim_command` `{"Party": {"Reorder": {"order": [...]}}}` with the ids in another order),
   then repeat step 3.
   *You will see* the same member cast, wherever it now stands.
   Covered by `omnis-sim/tests/party.rs::a_command_reaches_the_same_member_after_a_reorder`.

## Part 3: the window
1. Play a few minutes as usual: walk, open a door, buy at the smith, cast outside and inside a fight, give an
   item, rest.
   *You will see* nothing different from before. Any difference is a bug.

## Covered throughout
`omnis-mcp/tests/vocabulary.rs::every_key_on_the_wire_has_one_meaning` replays every command variant, the golden
walk and the golden fight, asks every view after each command, and fails if any key carries two meanings.
