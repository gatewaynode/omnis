> **Superseded 2026-11-26.** The owner tried this and it was not the intent; it is removed. See
> `tasks/plans/smith-corrected.md`.

# The smith's repair service (combat in feet, step 11)

## Context

Things break and can be put right by the party itself (step 10: mending, and tools with time, a check and
supplies; the owner tried it 2026-11-25). The plan's third way is a town service: "the Smith kind gains
`Repair { map, target }`: lists broken things on the town's region's maps, priced by size, repaired at
once." This step builds that in the engine and gives it a plain row on the smith's screen, so it can be
tried. Step 12 (damaged marks, any polish of the row) and step 13 (documents) stay their own steps.

The owner's answers, 2026-11-25:
- Damaged and broken things are both taken; a thing only damaged costs half.
- A buried doorway is listed and refused ("the rubble must be cleared first"); clearing stays the party's.
- Price: ten times the supplies the job would use (supplies are 5 sp): Small 5 gp, Medium 10, Large 20,
  Huge 40, Gargantuan 80; Tiny 2 gp.

Two commits (engine with base and pins; client row). Each: tests red first, mutation checks, fmt,
workspace clippy, the gate to a log, Sentrux.

## 1. Engine (`crates/omnis-sim`, `packs/base`, `crates/omnis-mcp`, pins)

**What is named.** A thing may stand on another map and another level, which `combat::cast::Target`
cannot say (a door there has no map or level). A new type in `repair.rs`, exported through `api.rs`:
`Job::Object { map: String, object: String } | Job::Door { map: String, level, x, y, facing }`.
`ServiceCommand::Repair { job: Job }`, offered by the Smith kind only (`ServiceCommand::offered_in`).

**What is listed** (`repair::jobs(world, data, town_map) -> Vec<Job>`): the maps of the region that holds
the service's map (`data.regions`, the lookup `loader.rs:128` already does; a map in no region lists its
own things only). On each, from the saved state alone (`MapState.objects` with `damage > 0` or `broken`,
`MapState.worn_doors`), in a fixed order (map id, then objects by id, then doors by level, x, y, facing).
A thing the party has never seen cannot be in that state unless something broke it; it is listed all the
same (said below).

**One judge for both ways.** `repair::harmed` today reads the party's own map and needs the thing in
sight. Its body is split: `harmed_at(world, data, map, level, &target-shape)` does the checks that do not
depend on where the party stands (kind has durability, the map's doors can be damaged, `Buried`,
`NothingToRepair`, size, material, broken); the existing `harmed` keeps the sight and spot lookup and
calls it. `restore` takes the map and level instead of reading `world.position`, and `by:
Option<CharacterId>`.

**Events.** `ObjectRepaired.by` and `DoorRepaired.by` become `Option<CharacterId>` (`None`: the smith),
and `ObjectRepaired` gains `map` (a door's event has it already), so a repair on another map is told
truly and the client redraws only its own map. A new `Event::RepairBought { service, cost }` carries the
coin, as `Treated` and `Bought` do. No new rejection: `NothingToRepair`, `Buried`, `CannotBeDamaged`,
`NoSuchObject`, `NoDoor`, `CannotAfford`, `NotOffered` all exist.

**The deal** (`service.rs`: `quote` gains the arm, `Deal::Repair { job, cost }`, `settle` pays, calls
`repair::restore(.., by: None, mended: false)`, pushes `RepairBought`, then the usual
`service_minutes`). No check, no tool, no supplies, and a thing with no material written is repaired
too (the smith needs no named craft; by tools it stays `WrongTarget`). A job outside the service's
region is `NotOffered`.

**Price.** A rule slot in `packs/base/data/rules/services.ron`:
`"smith.repair_cost": (inputs: ["supplies", "broken"], expr: ...)` giving `max(supplies * 500, 200)`
copper, halved when `broken` is 0. `supplies` is `repair::supplies_for(size)` whatever the damage (0, 1,
2, 4, 8, 16); the exact expression follows what `omnis-expr` offers (checked at build; if it has no
`max`, a third input `tiny` or a floor in the slot's own terms). Priced through the existing
`service::price`.

**View.** `service_view::commands`' Smith arm appends one `Repair` offer per job after Buy and Sell;
`offer` quotes it as any other, so the price and the refusal (`Buried`, `CannotAfford`) come for free.
`OfferView.subject` stays `None` for it (the job is in the command).

**Words, MCP, pins.** `word.rs`: no script word (it carries data), as `Command::Repair`.
`omnis-mcp/src/schema.rs::service_schema` gains the variant (oneOf 12 to 13) and a `Job` schema;
`tests/common/commands.rs::next_service` chains it; `schema_proof.rs` and `api_doc.rs` counts move
(counted at build). Events 87 to 88; Commands, Rejections unchanged. Rule counts and the editor's
"rules" count +1. Base resealed (the seal bumps the version), goldens rebaselined and diffed (version,
hash, fingerprint only). `docs/api.md` (command, events, the two changed events, changelog; the `by`
change is a breaking change to protocol 2's event shapes and is said there), ARCHITECTURE §4.5/§4.9,
PRD §8 Objects bullet (the master plan approved the service; the owner's three answers are flagged in
the commit), code-map, `tasks/plans/combat-in-feet.md` "as built", `tasks/TODO.md`.

**Tests** (`crates/omnis-sim/tests/repair_service.rs`). The frozen `test` pack's town uses base's smith
but has nothing durable, and it may not change; so a small fixture pack is added under
`crates/omnis-sim/tests/packs/` (or the `objects` fixture gains a town map with a smith site, a region
of two maps, a durable crate and breakable doors; chosen at build by which is smaller):
- the list: a broken thing on the town's map and one on the region's other map are offered; one on a map
  outside the region is not; a whole thing is not; order is stable;
- price by size, half when only damaged, the Tiny floor; a rule override changes it;
- buying: coin taken, the thing whole (object out of its broken state, door hung shut), `RepairBought`
  and the repaired event with `by: None`, ten minutes pass, no supplies or tools touched;
- refusals leave the world unchanged (`refused` helper): too little coin, a buried doorway (`Buried`)
  until cleared, a job already whole, a job outside the region, the command at the temple;
- a thing with no material written is repaired by the smith;
- the list and the command agree for every job; save round trip.
- Mutation checks: the half price; the region filter; coin not taken; the buried refusal dropped; the
  thing left broken; `by` set to a member.

## 2. Client row (`crates/omnis-vector`)

- `screen/service.rs::lists`: the Smith's screen gains a third list, "Repairs", shown only when there
  are jobs. `labels::offer_label`: "the crate (broken), Town" / "a door (damaged), Dungeon level 2" from
  the job, the object kind's name and the map's name; `offer_caption`: "Repair"; `offer_note` gives the
  coins as for any priced offer; `service_reason` gains "the rubble must be cleared first" (the wording
  the Actions row already uses).
- `shell/text.rs`: "The smith repairs the crate for 5 gp" (from `RepairBought` with the repaired
  event); the existing "X repairs/mends ..." lines handle `by: None`. `session::reshapes` redraws only
  when the event's map is the party's.
- Tests (`tests/repair.rs` or `tests/service.rs`): the list appears with its prices and its dim reason;
  the click sends the command; the HUD line; no "Repairs" heading with no jobs.
- `presentation-ARCHITECTURE.md`, code-map, `tasks/CONTINUITY.md`.

## Verification

- Gate green after each commit; mutation runs as listed; Sentrux `check_rules`.
- Owner's try in `just qa` or their own town: break a crate and a door, walk to the smith, see both
  under "Repairs" with prices, buy one, walk back and see it whole; bury a doorway and see it dim.
  The qa gym needs a smith within its region for this; if it has none I say so in the report rather
  than touch the owner's pack.
- Not seen on screen by me unless the running game can be driven over MCP.

## Said plainly

- **The smith repairs at a distance and at once** (ten minutes of service time), a door two levels down
  included. That is the plan's line; it is an abstraction, not a visit.
- **Everything harmed in the region is listed**, whether or not the party saw it break.
- **Two event shapes change** (`by` optional, `ObjectRepaired` gains `map`): a breaking change for
  anything reading events, said in the api changelog.
- A repaired object still stands as first placed, and a rebuilt door is shut, as in step 10.
- Base is resealed once more, so saves report the pack change again.
- A new fixture pack (or a grown `objects` fixture) is test data only; the frozen `test` pack and the
  owner's pack are not touched.
