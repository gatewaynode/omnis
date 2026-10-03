<!-- Approved in plan mode 2026-09-27 (M7 step 4, split into 4a and 4b); copied from the session plan file so it outlives the session. -->

# M7 step 4, split: 4a (save schema 5, money in copper) and 4b (the town mode and services)

## Context
M7 steps 0–3b are done; the game starts in town and every portal is visible (owner tested 3b by
hand, 2026-09-27; follow-up TODO 3c, the meadow's town-entrance marker, is worked after step 4).
The saved M7 plan (`tasks/plans/m7-town.md`, step 4) puts save schema 5, `Mode::Town` and eleven
service commands in one ~700-line step; the owner split it in two and settled money:

**Owner decisions (2026-09-27, plan mode):**
1. Two commits: **4a** schema 5 and money, **4b** the town mode and services.
2. **Money is stored as copper**, the smallest coin (100 cp = 1 gp, 10 cp = 1 sp). The field keeps
   its name `gold` (no `_cp` rename, no wire-name change); its doc says copper.
3. **Display**: whole gold **rounded down** on every screen (never promises more than the party
   has); the **inventory shows the breakdown** by denomination (15 gp 3 sp 7 cp), a view derived
   from the one copper number. A real coin purse (coins as items, change-making) is a horizon.
4. **Pack files as today**: backgrounds and monster drops in whole gold, item prices `cost_cp`;
   the sim converts gold to copper where it reads it. No floats anywhere: the simulation lint
   holds and display is integer arithmetic.
5. **Entering a site** from the game: always a log line and a **confirmation** before entering and
   before leaving; walking out of a service leaves it (play never gets stuck before the step 7
   panel). "Remember my choice" is a later stretch goal (horizon).

Facts this rests on (explorations, 2026-09-27): `Party.gold: u32` whole gp (`party.rs:20-38`),
credited at creation (`party.rs:152-162`), loot (`combat/turn.rs:344-360`, `CombatState.gold`),
bribes (`encounter.rs:359-410`, `bribe.cost` in `combat.rs` rules); `SAVE_SCHEMA = 4`
(`world.rs:22`), `World::from_ron` chains `v1_to_v2`, `v2_to_v3`, `v3_to_v4(data)`; schemas 2–4
parse straight into the current `World`, so a unit change needs its own step keyed on the number;
fixtures `saves/v1..v3.ron`, no `v4.ron`. `Mode` has three variants (`world.rs:127`), matched
exhaustively in `world.rs:141`, `omnis-app/src/sim.rs:75` (`PlayState::for_mode`),
`omnis-app/src/text.rs:46`, `omnis-sim/tests/common/mod.rs` (`settle`). `Command::word()` and
`omnis-mcp/tests/schema_proof.rs::next()` are exhaustive over `Command`; the schema pins oneOf 9,
64 instances, 94 branches, the drift test's `/oneOf/9`. Service data exists: `ServiceDef`,
`ServiceKind`, `MapData::site_at`, `services.ron` slots (all in copper), `rest.ron` values.

---

## 4a. Save schema 5 and money in copper (~350 lines, one commit)

0. **Fixture first, at HEAD**: an ignored `capture_schema_4_fixture` (as `capture_schema_3_fixture`,
   `save_and_replay.rs:397-463`) writes `tests/saves/v4.ron` from the current code (acolyte, 15 gp,
   mid-dungeon), committed with 4a.
1. **`omnis-core/src/money.rs`** (integers, no Bevy): `CP_PER_GP = 100`, `CP_PER_SP = 10`,
   `gp_floor(cp) -> u32`, `Coins { gp, sp, cp }` with `Coins::of(cp)`, `from_gp(u32) -> u32`
   (saturating). Unit tests: 1537 → 15/3/7, 1599 floors to 15, 0, saturation.
2. **Schema 5** (`world.rs`, `migrate.rs`):
   - `Party.gold` doc: copper. New `#[serde(default)]` fields: `Character.hit_dice_spent: u8`,
     `Party.bank: u32` (copper), `Party.last_long_rest: Option<…>` (the party clock's type).
   - `SAVE_SCHEMA = 5`; `v4_to_v5(world)`: `party.gold` ×100, `CombatState.gold` ×100 when saved
     mid-fight; every older arm chains into it. Tests: v4 fixture loads with gold 1500; v1–v3
     tests assert schema 5 and their gold ×100; `loads_are_checked` edits `schema: 5`.
3. **Where copper is written**: creation adds `from_gp(background.gold)`; loot adds
   `from_gp(rolled)` (the `Death.gold` roll trace stays the gp dice); `bribe.cost` in
   `packs/base/data/rules/combat.ron` returns copper (×100 in the formula, so every price a rule
   returns is copper; the header comment says so); `DevCommand::SetGold { gold }` is copper.
   `Rejection::CannotAfford` text shows gold rounded down: "that costs 3 gold; the party has 2".
4. **Display** (app): whole gold rounded down in the bribe label and notice (`combat_menu.rs`),
   log lines (`combat_text.rs`: bribe, death drop, victory), the debug Gold row (steps of 1 gp);
   the inventory summary shows the breakdown ("15 gp 3 sp 7 cp  food 10", `inventory_menu.rs:93`).
   MCP: `party.get` `gold` documented as copper (`ops.rs` `PartyView`, `tools.rs` descriptions,
   `bridge.rs` 77 stays a copper value); CLI `schema dump` header says schema 5.
5. **Tests and pins**: gold expectations ×100 in `omnis-sim/tests/{party,combat,encounter,dev}.rs`
   and app fixtures; both replays rebaselined; the tuple unchanged.

**4a done when**: gate green; v4 fixture migrates; the inventory shows 15 gp 0 sp 0 cp for an
acolyte; a bribe the party can't afford reads correctly; Sentrux clean; TODO 4a checked.

---

## 4b. The town mode and services (~1000 lines, one commit)

1. **Mode** (`world.rs`): `Mode::Town(ServiceState { service: ServiceId, kind: ServiceKind })`
   (`kind` stored so `may_save` needs no data); `ModeKind::Town`; `World::may_save()` passes
   "in an inn" to `Settings::may_save`, so `SaveRule::InnOnly` and `Relief` mean something.
2. **Entering and leaving** (`apply.rs`):
   - A step that ends on a site tile (`MapData::site_at`, after the portal check in `r#move`)
     enters it: `Mode::Town`, `Event::ServiceEntered { service }`; no encounter roll there.
   - `Interact` on a site tile in `Explore` enters again (before the door check).
   - In `Town`: `Step` emits `ServiceLeft` then moves as in Explore; `Turn` stays inside;
     `Service(Leave)` leaves in place; everything else is `WrongMode` as today.
3. **`Command::Service(ServiceCommand)`** (`service.rs` in omnis-sim, validate then mutate; dice
   on a copy of the stream; every price through its `services.ron` slot, in copper; each advances
   `service_minutes`): `Leave`, `Room`, `BuyFood { count }`, `Rumor`, `Heal { member }`,
   `Cure { member }`, `Raise { member }`, `Buy { item, count }`, `Sell { item, count }`,
   `Deposit { amount }`, `Withdraw { amount }`.
   - Refusals (new `Rejection` arms with text in `fmt_play`): `NotOffered` (wrong kind of service,
     or an item or spell not in its stock), `CannotAfford` (exists), `NothingToTreat`, `NotDead`,
     `BankShort`, `NotEnough` (exists, for selling).
   - `Room`: a long rest with no food and no ambush: hit points and spell points full, half the
     spent hit dice back, `last_long_rest` set, the clock by `long_rest_minutes`, refused inside
     `long_rest_every_minutes`; the restore is `rest::long_rest_restore`, which step 5 reuses.
   - `Rumor`: free, one of the tavern's rumors picked by a die on the stream.
   - `Heal`: a living member to full at `temple.heal_cost(missing)`; `Cure`: every condition but
     `dead` and `unconscious` at `temple.cure_cost`; `Raise`: a dead member (not buried) back at
     1 hit point at `temple.raise_cost(level)`; `Buy`/`Sell` at the smith, into and out of the
     stores; `Deposit`/`Withdraw` move copper between `gold` and `bank`.
   - Events (ids and numbers only): `ServiceEntered`, `ServiceLeft`, `RoomTaken { cost }`,
     `FoodBought { count, cost }`, `Rumor { service, index }`, `Treated { member, cost }`,
     `Raised { member, cost }`, `Bought { item, count, cost }`, `Sold { item, count, price }`,
     `Banked { amount, deposit }`; restoration reuses `Healed` and `Condition`.
4. **Forced edits**: `Command::word()` ("service"), `settle()` in the sim tests, `text.rs`,
   `PlayState::for_mode(Town) = Explore` until step 7, the MCP schema branch (`service_schema`)
   with proof successors and the pinned counts moved (oneOf 10, instances and branches recounted,
   the drift index), script words `leave` and `room`.
5. **App**:
   - Log lines for every new event (`service_text.rs`, strings in the text pack).
   - **The confirmation**: before a step that would enter a site (a `query::site_ahead`) and before
     a step out of a service, a small `bevy_ui` panel on the kit (`confirm_panel.rs` model +
     scenes): "Enter the Inn?" / "Leave the Inn?" with Go and Stay buttons, Enter and Escape as the
     shortcuts; Go sends the step, Stay sends nothing. Mouse first, per the app agreements.
   - "Remember my choice" goes to `horizons.md` as a stretch goal.
6. **Tests**: `tests/town.rs` (enter by step and by Interact, no encounter on a site, step out
   leaves, turn stays, Leave in place, may_save under InnOnly in an inn and not outside),
   `tests/services.rs` (each command's success and each refusal, prices from the slots, the
   stream untouched by a refusal, the clock advanced), the confirm panel headless (pointer Go and
   Stay, Escape, layout at both sizes); rebaseline if the fingerprints move.

**4b done when**: gate green; the owner can walk into the inn, see the log line and the
confirmation, and walk out; services work through MCP `sim_command` and scripts (the panel is
step 7); Sentrux clean; TODO 4b checked; a "what you will see / what does not work yet" line.

---

## First, before any code (the owner asked to prepare for a compact)
- Save this plan as `tasks/plans/m7-step4.md`; in `tasks/TODO.md` replace item 4 with 4a and 4b
  pointing at it, with the money decisions; horizons (a real coin purse; "remember my choice")
  in `tasks/knowledge/horizons.md`; `tasks/CONTINUITY.md` rewritten for the compact; one docs
  commit. Then pause for the owner's compact before 4a.

## Verification (each of 4a and 4b)
- `scripts/verify.sh > log 2>&1`, status read; replays rebaselined in the same commit when they move.
- Sentrux `scan` of `crates` and `check_rules`; files under 1000 lines, functions under 100.
- The owner's manual check with the report's "what you will see" line (agent windows draw no
  frames on this machine as of 2026-09-27, so captures may not be possible).
