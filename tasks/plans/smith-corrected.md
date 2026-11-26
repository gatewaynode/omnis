# The smith corrected: no repairs at a distance, a repair kit, a column that stays

## Context

Step 11 built a smith that repairs any harmed door or object in its region, at once and from afar. The
owner tried it (2026-11-26): it works, and it was not the intent. The intent:

- The smith repairs only what the party carries. Out in the world the party repairs what it broke
  itself (step 10: tool, supplies, time, a check), or pays a team to go once the place is safe.
- The smith sells repair kits: materials suitable for general repair.
- The "Repairs" column stays in place, with a placeholder when there is nothing ("The party carries no
  broken items"), and the result of a click must not be buried under the sales list.

The owner's answers (2026-11-26):
- Carried items cannot wear today (an item is an id and a count). The column stays and is empty for now;
  item wear is its own plan later, and the smith's repair of it comes with that.
- A repair kit is general supplies: one item serving any material, used up as supplies are. The
  material's tool is still needed.
- Hired repair teams get their own plan, right after this one, before steps 12 and 13.
- "Made safe", for that plan: a map names one placed encounter as the source of its random table; with
  it cleared the random rolls stop. Recorded in TODO here, built there.

Two commits. Each: tests red first, suite green then mutation checks, fmt, workspace clippy, the gate to
a log, Sentrux.

## 1. Engine, base and pins

**Removed** (everything that exists only for the at-a-distance smith):
- `service.rs`: `ServiceCommand::Repair`, its `offered_in` arm, `Deal::Repair`, `fn mend`; the wire name
  in `command.rs`.
- `event.rs`: `Event::RepairBought` (88 events back to 87).
- `repair.rs`: `Job`, `jobs`, `job_thing`, `described`, `region_maps`, and their exports in `api.rs`.
- `service_view.rs`: the Smith arm's jobs, `OfferView.broken`, the kind as `subject`.
- `packs/base/data/rules/services.ron`: the slot `smith.repair_cost`.
- `omnis-mcp`: the `Repair` variant and `Job` schema in `schema.rs`, the chain in
  `tests/common/commands.rs`.
- `crates/omnis-sim/tests/repair_service.rs`, and the client's labels, list arm and log lines for it
  (`labels.rs`, `screen/service.rs`, `shell/text.rs`, `tests/repair_service.rs`), in this commit so the
  client compiles.

**Kept, and said:** the split of `repair.rs` into `Worn` (a thing wherever it stands) and `Harmed`, and
the event shapes changed in step 11 (`ObjectRepaired.map`; `by: Option<CharacterId>` on both repaired
events). `by` is always a member until the repair teams exist; reverting it now and changing it again in
the next plan would break the event shape three times in a week. The fixture's forge map stays (a smith
beside a crate and a door: the place to try kits); its comment is corrected and the fixture resealed.

**The repair kit** (data, plus the order supplies are spent in):
- `packs/base/data/items/repair_kit.ron`: Gear, `craft: Some(Supply([every Material]))`, 1 gp, 5 lb
  (Omnis's own; the single-material supplies are 2 to 5 sp). Text key for its name. Added to
  `services/smith.ron` beside the tools and supplies.
- `repair.rs::supplies` / `spend`: the existing `serves` already accepts it. Order is made explicit: a
  supply made for the material is used before a general one (fewest materials first, then the present
  order of packs and stores), so a kit is not burnt while timber is carried.
- No change to the tool rule: a kit with no tool for the material is still `NoTool` (the existing
  rejection of step 10).

**Tests** (`crates/omnis-sim/tests/repair.rs`, base load tests):
- a kit and the right tool repair a broken wooden thing, a metal one and a stone one; one kit is used
  per supply the size needs;
- with timber and a kit both carried, the timber goes first; with too little timber the kits make up the
  rest;
- a kit and no tool is refused and nothing is used;
- the base smith stocks the kit; the smith's service view holds no repair offer; `Repair` is no longer a
  service command (schema count).
- Mutation checks: the kit not serving a material; the order reversed; the tool check dropped.

**Pins and seals:** events 87; service schema oneOf 12; schema proof and api_doc counts (counted at
build); rule counts back to `(36, 30, 5)`, "71 rules", cli "36 rule slots"; base item counts +1; base
resealed (the seal bumps the version), `objects` fixture resealed; replays rebaselined and diffed
(version, hash, fingerprint only).

**Documents:** PRD §8 Objects bullet rewritten to the owner's intent (owner direction, flagged in the
commit); ARCHITECTURE §4.5/§4.9; `docs/api.md` (the command and event removed, changelog);
`tasks/knowledge/code-map.md`; `tasks/plans/combat-in-feet.md` step 11 "as built, then corrected";
`tasks/plans/repair-service.md` marked superseded; `tasks/TODO.md`: this item, and two new open items:
"Hired repair teams" (own plan next; offered at the inn or tavern or by an NPC; only where every placed
encounter is cleared and the map's random source is cleared; a placed encounter named as the source of
the random table) and "Carried items that wear" (later; the smith's repair of them with it).

## 2. Client (`crates/omnis-vector`)

- `screen/service.rs::lists`: the Smith always has three columns: "For sale", "Your stores", "Repairs".
  `content()` gives an empty list its placeholder per heading: "The party carries no broken items" under
  "Repairs", "Nothing to offer" elsewhere as now.
- The result line (`Screen.message`, drawn in `shell/screen_draw.rs::build` after the row of panels, so
  under the longest list): moved to sit directly under the screen's title, above the panels, for every
  screen that has a message. One place, so a purchase's or refusal's line is read where the eye starts,
  whatever the lists' length. `sync` keeps rewriting it in place.
- Tests (`tests/service.rs` or a new `tests/smith_repairs.rs`): the smith's screen has the "Repairs"
  heading and its placeholder with nothing harmed anywhere and with a broken door on the map; no other
  service kind has the heading; the message node comes before the panels; the kit is in "For sale" and
  buying it says so. Mutation checks: the column dropped; the placeholder wording; the message back
  under the panels.
- `presentation-ARCHITECTURE.md`, code-map, `tasks/CONTINUITY.md` (rewritten head entry).

## Verification

- Gate green after each commit; Sentrux `check_rules`.
- Owner's try, `just qa objects:map:forge` (a smith at (0, 0), a door and a crate beside it) or their
  own town: the smith shows "Repairs" with the placeholder and no rows, whatever is broken; buy a repair
  kit and the carpenter's tools, break the crate, and repair it from the Actions screen with the kit
  used up; the line for a purchase shows under the title.
- Not seen on screen by me unless the running game can be driven over MCP.

## Said plainly

- After this the smith repairs nothing: the column is a promise until carried items can wear.
- A broken door or crate is put right only by the party (tool, supplies or kit, a check) or by Mending,
  until the repair teams are built. Next plan: the teams.
- Step 11's command and event are removed a day after they were added: a second break for anything
  reading protocol 2, said in the API changelog. Saves report the base pack change again.
- The kit's price (1 gp) and weight (5 lb) are my numbers, in pack data.
- The result line moves on every screen, not only the smith's.
