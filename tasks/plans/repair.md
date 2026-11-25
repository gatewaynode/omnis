# Repair: mending, and tools with time and supplies (combat in feet, step 10)

## Context

Things break (steps 8 and 9) and doorways fall in (2026-11-24), but nothing can be put right. Step 10 of
`tasks/plans/combat-in-feet.md` builds the two ways a party repairs: the mending cantrip and
`Command::Repair` with a tool, supplies and time. The data for both has been in since step 1
(`Utility::Mend { up_to }`, `Item.craft: Tool(materials) | Supply(materials)`, base's three tools and three
supplies); the engine refuses a mend today (`casting.rs:60`) and base has no mending spell. The repair
service (step 11), the damaged and broken marks (step 12) and the acceptance sheet (step 13) stay their own
steps.

The owner's answers, 2026-11-25:
- A cleared doorway can be rebuilt: rubble is cleared first, then it is a broken door like any other.
- A failed check loses the time **and the supplies**.
- The tool and the supplies may be in any member's pack.
- A thing damaged but not broken needs the tool, the check and the time, and **no supplies**.

Two commits: engine (with base's spell and the pins), then the client's entry. Each: tests red first,
mutation checks, fmt, workspace clippy, the gate to a log, Sentrux.

## 1. Engine (`crates/omnis-sim`, `packs/base`, pins)

New module `crates/omnis-sim/src/repair.rs` (kept out of `breaking.rs`, which is 393 lines of blows).

**What can be repaired** (`repair::thing`, the mirror of `breaking::thing` and `breaking::door`, reusing
`targets::map::object` / `map::door` and `breaking::make`):
- an object whose kind has durability and whose state has `damage > 0` or `broken`;
- a door of a map whose doors can be damaged, with wear or broken;
- else `Rejection::NothingToRepair`; a collapsed doorway is `Rejection::Buried` (clear it first);
  a kind or map with no durability is `CannotBeDamaged`, as for a blow.
- It must be touched (`Spot.touch`, as a melee blow at a thing is): else `OutOfReach` (existing).

**Made whole** (`repair::restore`, one place for both ways):
- object: `damage = 0`, `broken = false`, and a state that is the kind's `broken_state` is let go
  (`state = None`), so it stands as its placement first had it. Spilled contents stay where they fell.
- door: `wear_door` to `damage 0, broken false, collapsed false`; the edge is shut (the open flag cleared):
  a new door is hung closed.
- `Event::ObjectRepaired { object, by, mended }` / `Event::DoorRepaired { map, level, x, y, facing, by,
  mended }` (`mended`: by the spell, not by tools).

**Mending** (`casting.rs`): the refusal at line 60 goes. A `Utility::Mend { up_to }` cast takes
`Target::Object` or `Target::Door`, in touch (`Targeting::TOUCH` is already its default), of size at most
`up_to`: else `Rejection::TooLarge`. No roll; `cast_minutes` (1, the SRD's minute). A door's size is its
map's `door_make.size` (Medium by default, so base's mending does not rehang doors). An object with no
written size counts as Medium.
- `packs/base/data/spells/mending.ron`: cantrip, Transmutation, cleric and wizard (the SRD's lists that base
  has), `Utility(Mend(up_to: Small))`, text keys; added to the classes' cantrip lists where base lists them.
  Base resealed (the seal bumps the version itself), goldens rebaselined and diffed (hash only); the
  spell-count pin 20 to 21; `save/names.rs` since-list if spells are numbered in saves (checked first).

**Tools** (`Command::Repair { member, target: Target }`, Explore only; script word none, it carries data):
- the member can act (not down, not dead), as `assault::validate` checks;
- the thing's material: the kind's `attributes.material`, the map's `door_make.material`;
- a tool: any member carries an item with `craft: Tool` naming the material, else
  `Rejection::NeedsTool { material }`;
- supplies, only when it is broken: by size Tiny 0, Small 1, Medium 2, Large 4, Huge 8, Gargantuan 16,
  counted across every member's pack over items with `craft: Supply` naming the material, else
  `Rejection::NeedsSupplies { material, count }`; taken in marching order with `items::take_from`;
- a Dexterity check, DC 10 (rules values `repair_dc`, default 10), `omnis_rules::check` on a new `repair`
  stream, told as the existing `Event::Check { kind: CheckKind::Ability(Dexterity) }`;
- minutes by size 10, 30, 60, 120, 240, 480 through `apply::advance`, pass or fail;
- pass: `restore`. Fail: the supplies are gone and nothing is mended (the owner's choice).
- `Command::Repair` joins `trial`, the script/API surfaces and the MCP schema as `Clear` did
  (`omnis-mcp/src/schema.rs`, `tests/common/commands.rs`, `schema_proof.rs`, `api_doc.rs`).

**Views**: `query` gains what a repair of a target would take (`repair_quote(target) -> { tool: bool,
supplies: (have, need), minutes }` or the rejection), so the client and MCP can say it before the time is
spent. Exported through `api.rs`.

**Pins**: 19 Commands, 87 Events (+2), 82 Rejections (+4: `NothingToRepair`, `Buried`, `TooLarge`,
`NeedsTool`, `NeedsSupplies` is +5 = 83; counted exactly at build), rule counts and editor "rules" count
(+1, `repair_dc`), MCP instance counts. `docs/api.md` (command, events, rejections, changelog),
ARCHITECTURE §4.5/§4.9, PRD §8 Objects bullet (the plan approved it at step 0; the owner's four answers
are flagged in the commit), `tasks/knowledge/code-map.md`, `tasks/TODO.md`.

**Tests** (`crates/omnis-sim/tests/repair.rs`, on the `objects` fixture pack and its durable kinds; fixture
items for a tool and a supply added to that pack if it has none, never the frozen `test` pack):
- mending makes a broken Small thing whole and a damaged one; refuses a Medium thing and a door
  (`TooLarge`), a whole thing (`NothingToRepair`), one out of touch; costs the cast minutes;
- tools: no tool refused with the material; broken with too few supplies refused with the count; damaged
  only uses no supplies; a pass restores, takes the supplies and the size's minutes; a fail (a seed found
  for each outcome) takes supplies and minutes and mends nothing; the tool is kept; tool in one pack and
  supplies split over two;
- doors: a broken door repaired is shut and blocks step and sight again and can be struck again; a
  collapsed one is `Buried` until cleared, then repairs; wear without a break repairs without supplies;
- a repaired object leaves its broken state and offers its interactions again; contents stay spilled;
- save round trip and journal carry the repaired state; a member down or dead refused; not in a fight.
- Mutation checks: supplies taken on a pass only; none taken when damaged only; the size table; the check
  ignored; the door left open; collapsed repaired without clearing; mending ignoring size.

## 2. Client entry (`crates/omnis-vector`)

Mouse first, as the rule asks; the marks and the service row remain step 12.
- `screen/actions.rs`: a row "Repair the <thing> (<minutes> min)" for the faced or touched thing when
  `repair_quote` gives one, dim with the engine's reason ("needs carpenter's tools", "needs 2 more
  supplies for wood", "clear the rubble first"); it opens the member chooser the Attack row uses
  (`Does::Attack`'s Attacker modal, generalized to a "Who?" chooser), then sends `Command::Repair`.
  Never the default action of Space, as Clear is not.
- Mending needs no new screen: the cast screen's target step lists things in touch for a `Mend`
  (`targets` gains the Mend arm beside the strike arm; `targeting.rs` on the client).
- `shell/text.rs` and `labels.rs`: lines for `ObjectRepaired`, `DoorRepaired` ("Rik repairs the door",
  "Theala mends the chest"), the four new refusals; `session::reshapes` gains both events (a door comes
  back, a broken state goes).
- Tests (`tests/repair.rs` in the client): the row with its minutes and its dim reasons; the chooser sends
  the command; the view redraws a repaired door shut; the log lines.
- `presentation-ARCHITECTURE.md`, code-map.

## Verification

- Gate green after each commit; mutation runs as listed.
- Owner's try in `just qa`: break the chest with a weapon, cast mending on a small thing; buy or carry
  carpenter's tools and timber, repair the torch's door (60 min, 2 timber), fail and pass over reloads;
  bury a doorway, see "clear the rubble first", clear it, rebuild the door.
- Not seen on screen by me unless a scratch save can stand the party at a broken thing.

## Said plainly

- **A repaired object stands as it was first placed**, not as it was the moment before it broke: a chest the
  party had unlocked is locked again if the map placed it locked. Keeping the earlier state needs a new
  saved field; say if you want it.
- **A rebuilt door is shut**, whatever it was before.
- Base's mending restores Tiny and Small things only, so it never rehangs a door (doors are Medium). This is
  wider than the SRD's "one break no larger than a foot" in that it clears all damage at once.
- The check is plain Dexterity; tool proficiency joins when that system exists (as the plan said).
- Huge and Gargantuan things get the doubled figures (8 and 16 supplies, 4 and 8 hours); the plan gave only
  the first four sizes.
- Repair is on the road only, not in a fight and not from a town service (step 11).
- Base is resealed once more, so saves report the pack change again.
