# M8: subjective time, company-weighted, with the signal bus

**Closed 2026-10-08**: owner acceptance of M8 passed (`tasks/acceptance/m8.md`); M8 is closed.

## Context
M7 is closed (`68c4d09`). M8 (TODO, PRD §7.8, ARCH §4.4, A13) gives every holder its own clock and reconciles
clocks on contact. The owner asked for the event bus to be planned first, with external crates evaluated.

**What exists today:**
- The types are in place:
  - `Clock`, `Contact` and `HolderId` (`omnis-core/src/time.rs`, `id.rs:87`);
  - `RegionId` and `EraId`;
  - `StreamName::time(a, b)` (`id.rs:133`, with a known-answer test);
  - `World.clocks`, which holds only `PARTY`.
- Every minute the party spends goes through `apply.rs:64 advance`.
- The party changes map in one place, `apply.rs` around line 220 (`landing.through`); dev teleports change map in `dev.rs`.
- Not built yet:
  - regions in the data;
  - `contacts`;
  - reconciliation and the `Reconciled` event;
  - the calendar as data (`MINUTES_PER_DAY` is a constant);
  - filling `{arg}` placeholders in text;
  - the `time.*` operations.
- Rumors exist: `ServiceDef.rumors` holds keys, and `Event::Rumor { service, index }` carries no time.

**The event-bus crate survey (2026-10-05):** none fits. The candidates checked:
- `event-listener`, `crossbeam-channel`, `flume` and `bus` are thread primitives;
- `postage` is async;
- `signals2` keeps closures that cannot be saved;
- `shrev`'s `ReaderId` holds an mpsc sender, is not serializable and uses `unsafe`;
- `eventbus`, `event_bus`, `pubsub` and `message-bus` are abandoned;
- `evento` is a pre-release with a SQL database.

None has saved subscriptions, a fixed call order or a cap on nested raises. **A bus of about 150 lines inside `omnis-sim` adds no dependency.**

**Your decisions (2026-10-05):**
- **Regions are data files.**
- **The bus is built, carries time, and then the combat reactions move onto it.**
- **The time model, in your words:** "as more people gather together in the same place time starts to align with them … players may adventure in the wilds for years just to realize a city has only had a few months pass … player age changes constantly and doesn't reverse, but dates reconcile in villages, towns and cities". As agreed:
  - The party's **age** is its own clock and never reverses.
  - Every region has a **company** value: how many people gather there, in per mille.
  - Each minute the party lives counts toward its **shared time**, weighted by the company of the region where it was lived.
  - **Entering a settlement:**
    - the settlement catches up by the party's shared time since the last contact, with a small jitter set by its stability;
    - the settlement's coupled settlements catch up alongside it;
    - the party's **date** becomes the settlement's date.
  - Example: two years in the wilds at 100‰ is about 2.4 months in the city.
- **Visible in M8:** the calendar as data, and rumors that have an age.
- **"There will be no bank interest in this game."** PRD §7.4 and §7.8 change (the text is below).

## The model
- **Party time** (`World.party_time`):
  - `age` is `clocks[PARTY].elapsed`, unchanged;
  - `shared_milli: i64` holds minutes × company, kept exact in integers;
  - `date: i64` is the date the party believes, in minutes on the calendar;
  - `era`.
- `advance(minutes)`:
  - adds `minutes` to the age;
  - adds `minutes × company(region of the current map)` to the shared time;
  - adds `minutes` to the date, so in the wilds the party's own reckoning runs on.
  - `TimeAdvanced.day_rolled` follows the date.
- **Regions:**
  - `packs/*/data/regions/*.ron`: `RegionDef { id, name, kind: Settlement | Wild, company, stability, rule, couplings, maps }`.
  - The loader builds a map → region index. Every map belongs to exactly one region; a map listed twice, an unknown coupling, or a company outside 0..=1000 is refused.
  - **The test pack:**
    - `town`: Settlement, company 900, stability small;
    - `meadow`: Wild, 100;
    - `dungeon` plus `depths`: Wild, 50, in flux;
    - `crossroads`: a Settlement with no maps, coupled to the town by a road. It is there so coupling can be seen, through `time.clocks`.
- **Reconciliation** (`omnis-sim/src/time.rs`, the enumerated interaction points). When the party enters a region (`a` is the party, `b` the region):
  1. Read `contacts[(a, b)]`; a holder never met counts as met at the origin, both clocks at 0.
  2. Compute `lived` (the party's age since that contact) and `shared` (its shared time since then, in minutes).
  3. `delta_b` = the region's rule slot evaluated on `(lived, shared, stability)`, on stream `time:<a>:<b>`, clamped to `0..=cap`. The slots, in `packs/base/data/rules/time.ron`:
     - `time.settled` uses `shared` with jitter ±stability ‰;
     - `time.wild` uses `lived` with a larger jitter.
  4. The region's clock advances by `delta_b`.
  5. Both `Contact` records are written. `Contact` gains `self_shared`.
  6. `Event::Reconciled { a, b, delta_a: lived, delta_b, era_b }` is emitted.
  7. **For a settlement only:**
     - the party's `date` and `era` become the region's;
     - each coupled region reconciles once with `b`, by the same rule over `b`'s change since their own contact (coupling depth 1).
  - Wild regions keep clocks for the ecosystem later (M10), but they never set the party's date.
- **Not yet interaction points** (listed in `time.rs`, with tests when they arrive): actors, projects, party meetings, the start of an encounter (monsters are not holders in v1).
- **The calendar as data** (`time.ron` values):
  - `minutes_per_day` 1440 and `days_per_year` 360;
  - `night_from` 1200 and `night_to` 360;
  - `cap_minutes`, the 10 years of ARCH §4.4.
  - `MINUTES_PER_DAY` and `Clock::advance(minutes, minutes_per_day)` read them, and a `night(date)` predicate exists. Night has no rule effects yet; it is displayed.
- **Rumors with age:**
  - `ServiceDef.rumors` becomes `Vec<RumorDef { text, at }>`, where `at` is a minute on the region's clock. A plain-key form still loads, as `at: 0`.
  - A rumor whose `at` lies ahead of the town's clock is not told yet.
  - `Event::Rumor { service, index, ago }`, where `ago` is the town clock minus `at`.
  - The text takes `{ago}`. Filling `{name}` placeholders is new: a small `fill(text, &[(name, value)])` in `omnis-data/src/text.rs`. The simulation stays keys-only.

## The signal bus (`omnis-sim/src/bus.rs`)
```rust
pub enum Topic { Region(RegionId), Battle }                 // Ord, serde
pub enum Subscriber { Reconcile, Reactions }                // Ord, serde; a match calls each one's handler
pub enum Signal { Entered { region: RegionId, from: Option<RegionId> },
                  Trigger { trigger: Trigger, about: ... } } // carries what the on_* calls take today
pub struct Bus { subs: BTreeMap<Topic, Vec<Subscriber>> }   // saved; Vec order is the call order
```
- **`raise(signal)`** pushes the signal onto a queue that is not saved. **`drain(world, data, events)`** pops the queue first in, first out, and calls each subscriber of the signal's topic in `Vec` order.
- Signals raised during delivery are queued and carry `depth + 1`.
- **Two limits, `MAX_DEPTH` 4 and `MAX_SIGNALS` 64 a drain:**
  - Over either one, the rest of the queue is dropped and `Event::SignalsDropped { count }` is emitted, so a runaway loop shows in replays and logs.
  - Never a panic, never a rejection after the world has changed.
- The queue is empty at the end of every command (a debug assertion).
- **Subscriptions:**
  - The world starts with `Region(*) → Reconcile` for every region in the data.
  - Combat start adds `Battle → Reactions`, and combat end removes it.
  - A save holds the subscriptions, and `v6_to_v7` writes the defaults.
- **Reactions move last** (their own step):
  - the five `on_*` call sites raise `Signal::Trigger` and drain at once, at the same point, so the order is unchanged;
  - `fire` and `react` stay as they are; the `Reactions` subscriber calls them.

## Steps, one commit each (gate green, Sentrux, staged by name, never pushed)
0. **Docs:**
   - this plan saved as `tasks/plans/m8-time.md`;
   - the M8 block in TODO;
   - the PRD and ARCH text below.
1. **Data** (`omnis-data`):
   - `region.rs`, the map → region index and its checks;
   - `time.ron` with its values and two slots;
   - `RumorDef`, which still loads the old key list;
   - `text::fill`;
   - the test pack's four region files.
   - Tests: refusals and the fill.
2. **The bus** (`omnis-sim/src/bus.rs`):
   - Tests: the call order; the order holds through a save round trip; the depth cap and the signal budget each drop and report; the queue is empty after every command.
   - A mutation pass.
3. **Time** (`omnis-sim/src/time.rs`):
   - party time, `contacts` and `Reconciled`;
   - region entry raised on the bus from portals and dev teleports;
   - couplings;
   - the calendar and night from data;
   - `SAVE_SCHEMA 7` with `v6_to_v7`;
   - the walk and fight replays **rebaselined** (new world fields; the encounter and combat streams are not drawn from, so their event logs are compared before and after and must match).
   - **Done when** (TODO):
     - town → meadow → dungeon and the reverse route give different `Reconciled` deltas, and each replays to the same fingerprint;
     - the §7.8 examples are tests:
       - years in the wilds are months in the town;
       - a stable town drifts by less than its stability;
       - the coupled crossroads moves with the town;
       - age never reverses while the date snaps back;
       - a never-met region counts from the origin;
       - the jitter stream is `time:party:0:region:N` and no other stream moves.
   - A mutation pass.
4. **Rumors with age:**
   - `Event::Rumor.ago`;
   - a rumor not yet happened stays quiet;
   - the base tavern's three rumors get `at` values. Owner-authored content is welcome; mine are placeholders.
5. **Operations and MCP:**
   - `time.clocks`, which returns clocks, contacts and party time;
   - `time.reconcile`, which sends a new `DevCommand::Reconcile { region }`, so a replay can hold it;
   - `game.status` gains date, age, year, day and night;
   - MCP 20 → 22 tools;
   - the schema proof grows by one dev instance and one branch.
6. **App:**
   - the HUD clock reads "Year 1, day 40, 14:20 · night", with the party's age beside it;
   - the date jump shows on entering the town;
   - the tavern line fills `{ago}` ("… three days ago");
   - "A new day." follows the date.
7. **Reactions onto the bus:**
   - the five call sites and the `Battle` subscription;
   - the fight replay's event list is identical before and after;
   - a nested-raise test reaches the cap with a synthetic subscriber;
   - mutation: drop the drain, reverse the call order.
7b. **The bus in its own crate** (owner, 2026-10-06: "We'll have to expand that bus in the future. We should document an architecture for our bus design and should probably isolate it in its own crate."): `omnis-bus` holds the mechanism, generic over a topic, subscriber and signal; `omnis-sim` keeps the vocabulary; ARCH §4.8 rewritten with three designed expansions (a topic hierarchy, saved deferred signals, pack-declared subscribers), §3's row and A16; veto and ordering phases to horizons. Plan: `tasks/plans/m8-bus-crate.md`.
8. **Docs and review:**
   - ARCH v0.8 as built;
   - `code-map.md` and `horizons.md` (bank interest removed; ecosystem catch-up waits for M10);
   - `tasks/acceptance/m8.md`;
   - **owner acceptance closes M8.**

## Vision text (approving this plan approves it)
- **PRD §7.4, the Bank line** becomes: "Bank: deposit gold and gems; the bank keeps them safe and pays no
  interest."
- **PRD §7.8:**
  - **The "Clocks reconcile on contact" bullet** becomes: "**Clocks reconcile on contact, and only
    somewhat; time aligns where people gather.** Every region has a company (how many people gather
    there). The party's age is its own clock and never reverses; each minute it lives also counts toward
    its shared time, weighted by the company of where it was lived. Entering a settlement (a village, town
    or city) catches the settlement up by the party's shared time since their last contact, with a small
    drift set by its stability, and the party's date becomes the settlement's. Years in the wilds are
    months in a city. Wild places and dungeons keep their own clocks and drift by months. Never backwards
    in v1. Both remember the contact."
  - **The consequences bullet** drops "a bank balance grows by the bank's time, not the party's;".
- **ARCH §4.4:**
  - **Reconciliation step 3** names the region's rule slot and the inputs `lived`, `shared` and
    `stability`.
  - **A new paragraph, "Company and the party's date",** states the model above.
  - **`Contact`** gains `self_shared`.
- **ARCH §4.2:**
  - `World` gains `party_time`, `contacts` (already planned) and `bus`;
  - new events: `Reconciled` (already planned) and `SignalsDropped`.
- **ARCH, a new §4.x "The signal bus":** the shape above, both limits, the saved subscriptions, and the
  reason no external crate was taken.
- **ARCH §9.3:** the `time.reconcile` row says it sends a dev command.

## Verification
- On every commit:
  - `DEVELOPER_DIR=/Library/Developer/CommandLineTools scripts/verify.sh` prints VERIFY-GREEN;
  - a Sentrux scan of `crates`, then `check_rules`;
  - the simulation lints: no floats, and `BTreeMap` only.
- **Mutation passes** in steps 2, 3 and 7. Each break counts only when a named test fails.
- **Replays:**
  - rebaselined once, in step 3, and reported with the old and new values;
  - in step 7, the reactions' move is proved by an identical event list, not by the fingerprint.
- **What you will see** (step 6 onward):
  - the HUD shows year, day, time, night and the party's age;
  - walk out to the dungeon, rest there a long while, and come back: your age has grown by the full time, but the town's date has moved only by a fraction, and your date snaps to it;
  - the tavern's rumors say how long ago they happened, on the town's clock;
  - `time.clocks` over MCP lists every clock and contact, the crossroads included.
