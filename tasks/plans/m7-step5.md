# M7 step 5: resting in the field (`Command::Rest`)

## Context
M7 steps 0–4b and follow-up 3c are done (HEAD `3261be1`, gate 414/7). The inn's room already
performs a long rest (`service::room` → `rest::long_rest_restore`, `Party.last_long_rest`,
`RestTooSoon` counted end to end). Step 5 (approved plan `tasks/plans/m7-town.md`) adds resting
outside town: a short rest that spends hit dice and a long rest that costs food, both of which can
be ambushed. The owner will play it only after the ambush and wipe rates are measured (LESSONS
2026-09-13). No app panel yet: the camp panel is step 8. Until then rest is reached through MCP
`sim_command` and scripts, as services were in 4b.

**Owner decisions (2026-09-27, this plan):**
1. **Both rests can be ambushed**, the short one at a lower chance.
2. **A stable member at 0 HP may spend hit dice** (SRD): the roll heals them and they wake.
3. **When an ambush comes**: a long rest is interrupted after 1d8 hours. A short rest, by the
   same rule, after 1d6 × 10 minutes.
4. **Rest only outside services** (`Mode::Explore`, any map). A town street has no random table,
   so it has no ambush, but a rest there still costs time and food. Inside a service, rest is
   refused (`WrongMode`); the inn's room is the rest indoors.

Consequence: a whole percent cannot express a short rest's lower chance (the dungeon's 3% ÷ 8 = 0).
So both ambush slots return **per mille**, and the roll is a d1000 on the encounter stream.

## Design

**Rules (`packs/base/data/rules/rest.ron`)**, integers only:
- `rest.ambush_chance(map_chance)` changes to per mille: `map_chance * 10`, so 3% becomes 30‰.
- New `rest.short_ambush_chance(map_chance)` = `map_chance * 10 / 8` (3‰ in the dungeon).
- New `rest.hit_die_heal(die, con_mod)` = `max(0, die + con_mod)`, per SRD 5.1.
- The header comment says per mille. The starting formulas are provisional: the measurement
  table below sets them, and you pick the numbers before you play.

**Sim (`omnis-sim`)**:
- `Command::Rest(RestCommand)`, with `RestCommand::{Short { dice: Vec<u8> }, Long}`. In `dice`,
  entry *i* is the number of hit dice member *i* spends; it may be shorter than the party (the
  missing members spend 0). Only `Mode::Explore` accepts it; every other mode is `WrongMode`.
- **`rest.rs` grows the command, validate then mutate, dice on a copy of the stream:**
  - A shared `too_soon(world, data) -> Result<(), Rejection>` is extracted from `service::room`,
    and the room calls it.
  - **Short: validation.** A list longer than the party is refused as `NoSuchMember`. Spending
    dice for a dead member is `MemberDead`. For a member at full HP it is `NothingToTreat`. More
    dice than a member has left (`level - hit_dice_spent`) is `NoHitDice { index, left }`.
  - **Short: the rest.** Roll the ambush. If it fires, 1d6 × 10 minutes pass, then
    `RestInterrupted { minutes }` and the encounter opens; no dice are spent. Otherwise
    `short_rest_minutes` passes and each spent die is rolled on the "rest" stream: 1d`hit_die`
    through `rest.hit_die_heal`, then `party::heal`, which emits `Healed` and wakes a downed
    member. `hit_dice_spent` goes up, and `Rested { long: false, minutes, food: 0 }` is emitted.
  - **Long: validation.** `too_soon` applies. Food needed is `rest_food_per_member` × members
    not dead; if the party has less, it is `NoFood { need, have }`.
  - **Long: the rest.** Roll the ambush. If it fires, 1d8 × 60 minutes pass, then
    `RestInterrupted` and the encounter; no food is eaten and nothing is restored. Otherwise
    `long_rest_minutes` passes, the food is eaten, `long_rest_restore` runs, `last_long_rest` is
    set, and `Rested { long: true, minutes, food }` is emitted.
  - **The ambush itself.** One d1000 on the "encounter" stream against the slot's per-mille
    chance. Maps without a random table never ambush. When it fires, the party meets an entry
    of the map's random table, picked with the existing picker in `encounter.rs`.
  - **Opening the ambush.** It goes through `encounter::begin` with a new
    `EncounterSource::Ambush`. A new variant only adds, so old saves still load and there is no
    schema change. For `Ambush`, `begin` reads `rest_ambush_surprise` in place of `surprise`
    (both 0 today). The retreat is the party's own tile, facing reversed.
- **Events:** `Rested { long, minutes, food }`, `HitDiceSpent { member, dice }` (followed by its
  `Healed`), `RestInterrupted { minutes }`. The time passing emits `TimeAdvanced` and prunes
  effects, through `apply::advance`.
- **Rejections:** new `NoFood { need, have }` and `NoHitDice { index, left }`, with text in
  `command.rs`. Existing ones reused: `RestTooSoon`, `MemberDead`, `NothingToTreat`,
  `NoSuchMember`.

**Rest events: stubs for your later ideas (your choice: a per-map table, by terrain)**
- **Data (`omnis-data/src/map.rs`).** `MapDef.rest_events: Vec<RestEventDef>` (serde default,
  empty). Each entry is `RestEventDef { terrain, rest: Short | Long | Any, chance: u16, text }`:
  - `terrain` is a terrain name on that map;
  - `chance` is per mille;
  - `text` is a text-pack key.

  The resolver turns this into `MapData.rest_events` with the terrain index.
- **Validation**, each case with a bad-pack row: an unknown terrain, a chance over 1000, a
  repeated (terrain, rest) pair, and a missing text key if the loader already checks text keys
  (otherwise the app falls back to the key).
- **Sim.** When a rest is not ambushed, each entry matching the party's terrain and the rest's
  kind rolls a d1000 on the encounter stream, in file order. It always rolls, even at chance 0,
  so the dice stay steady when a chance later changes (as the meadow's random table does now).
  A hit emits `Event::RestEvent { map, index }` and changes nothing else. The rest completes as
  usual. Whether an event can interrupt or cost something is for your later ideas.
- **Placeholders**, all at chance 0 with `Any`, each with a text-pack line in
  `packs/test/text/en/maps.ron`: the town's `street` and `green`, and the meadow's `road` and
  `grass`. The dungeon gets none; its ambush is the random table.
- **App.** The log line is the entry's text.
- **Tests.** Loader rows; in `tests/rest.rs`, a temporary copy of the loaded map with one entry
  at chance 1000 emits `RestEvent` on a rest on that terrain, not on another terrain, and not
  for the other rest kind. The placeholders at 0 emit nothing but consume their rolls.

**Edits the compiler forces** (as in 4b):
- `Command::word()`: `rest` is the long rest, `short-rest` a short rest spending no dice. A short
  rest with dice logs as `rest`.
- MCP `rest_schema()` branch: `oneOf` goes from 10 to 11, and the drift index moves. The proof's
  `next` gets a successor arm, and the instance and branch counts are recounted.
- `common::settle` in the sim tests, if its match needs it.
- App log lines in `service_text.rs`, which covers town and rest:
  - "The party rests for an hour" / "The party rests for the night and eats 4 food".
  - "Bram spends 1 hit die".
  - "Ambushed after 3 hours!"
- Refusal text for the two new rejections.

**Measurement (`tests/measure.rs`, ignored, integers):** a new `ambush_over_seeds`.
- **Parties:** two and six members at level 1, "spent" (each at a quarter of their hit points,
  at least 1, and no spell points).
- **Fights:** 300 seeds of an ambush drawn from the dungeon's random table (entry picked by
  weight, counts rolled), then fought attack-only and cast-when-possible.
- **Printed:** the wipe rate per party size. A second table gives the expected wipes per 100 long
  rests and per 100 short rests at 10‰, 30‰ and 50‰ (the chance times the wipe rate).
- **A sanity count:** over 1000 seeds of `Rest(Long)` in the dungeon, the share ambushed is
  close to the slot's chance.
- I bring you the table. You set the two chance formulas before you play, and the chosen values
  go into `rest.ron` in the same commit.

**Pins:** the pack tuple's rule slots go from 29 to 31. Both replays are rebaselined (the pack
hash moves). `SAVE_SCHEMA` stays 5.

## Files
- `packs/base/data/rules/rest.ron` (slots and header); `packs/test/data/maps/{town,meadow}.ron`
  (placeholder rest events) and `packs/test/text/en/maps.ron`; `omnis-data/src/map.rs` and the
  loader (the `rest_events` table, validation, bad-pack rows).
- `crates/omnis-sim/src/rest.rs` (the command; it may split into `rest/` if it passes about 400
  lines).
- `apply.rs` (dispatch), `command.rs` (`RestCommand`, words, rejections), `event.rs`,
  `encounter.rs` (`Ambush` and the picker made `pub(crate)`), `service.rs` (`room` uses
  `too_soon`), `lib.rs` (exports).
- Tests: new `crates/omnis-sim/tests/rest.rs`; `tests/measure.rs`; `omnis-data/tests/load_base_pack.rs`
  (the tuple); replays.
- MCP: `crates/omnis-mcp/src/schema.rs`, `tests/schema_proof.rs`.
- App: `crates/omnis-app/src/service_text.rs`.
- Docs: `tasks/TODO.md` (step 5 checked, with the measured table), `tasks/knowledge/verification.md`
  (pins and counts), `code-map.md` (rest).

## Tests (`tests/rest.rs`; each checked to fail when its rule is broken)
- **Short rest heals.** A short rest heals by 1d`hit_die` plus the Constitution modifier through
  the slot (the trace matches the healing), counts `hit_dice_spent`, and advances 60 minutes.
- **A downed member wakes** on a short rest.
- **Short-rest refusals.** Each of `NoHitDice`, `MemberDead`, `NothingToTreat` and `NoSuchMember`
  leaves the world byte-identical, streams included.
- **Long rest.** It eats food for the living members only, restores hit points and spell points,
  gives hit dice back, sets `last_long_rest` and takes 480 minutes. `NoFood` and `RestTooSoon`
  (shared with the room) refuse.
- **Ambush.** Under a seed found by search in the dungeon, the time passes (a whole number of
  hours from 1 to 8), nothing is restored, no food is eaten, and the mode is `Encounter` with
  source `Ambush` and no surprise. The same for a short rest (a multiple of 10 minutes).
- **No ambush without a table.** The town street and the meadow (chance 0) are never ambushed
  over 200 seeds.
- **Where rest works.** A rest inside a service, in an encounter or in combat is `WrongMode`.
- **Script words** `rest` and `short-rest` parse and print.

## Verification
- `scripts/verify.sh > log 2>&1`, unpiped, status read. Sentrux `scan` of `crates` and
  `check_rules`. Files under 1000 lines, functions under 100.
- The measurement run's table goes in the report and in TODO step 5. You pick the chances; I
  apply them, rebaseline and rerun the gate, then commit.
- What you can try before step 8 (through MCP `sim_command` or `--script "…,rest"`): rest in the
  dungeon and watch the log. There is no camp button yet.
