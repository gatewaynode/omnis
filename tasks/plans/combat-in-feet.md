# Combat in feet, and things that break

## Context

The owner accepted targeting (2026-11-14) and named the next work: "fix the opening strike, lack of
range calculation, reach and such", destructive spells that can target objects, and hit points and
armour class on objects. Refined since:

- **One distance per stack**, no grid, no cover yet. Combat stays consistent with the SRD: 5 ft
  squares, ranges, reach and speed in feet.
- A **per-type "how many fit in a 5 ft square"** number on monsters (1 human, 2 kobolds, 3 rats). It
  limits melee attackers, sizes area spells, and caps a group on a map tile.
- Maps will need **different scales of grid** outside combat (my reading of "outside of combat we
  will likely need different scales"): a map states how many feet its tile is.
- The opening strike **rolls for surprise** (SRD Stealth against passive Perception).
- **Objects and doors** are destructible with SRD armour class by material and hit points by size;
  terrain is out. A broken container spills, and **fragile items can be lost**. Broken things are
  **repairable**: the mending spell, tools with time and materials, and a town service.

Today (researched): a fight has no distance at all. Reach is front row / first two living stacks,
`ranged: bool` reaches everything, `Tiles(n)` reaches everyone, nothing reads `speed`. The opening
strike always surprises and always hits stack 0. Objects carry `size` and `material` that nothing
reads; doors are bare edges; nothing rolls damage on the road; there are no tool proficiencies.

## Part I: the fight in feet

### Rules
- **Scale.** `Map.tile_feet` (default 5). Fight arithmetic is in feet; a square is 5 ft.
- **Distance.** `Stack.distance: u16`, feet from the party's front line; 5 is engaged. Old saves load
  at 5. Start: tiles between party and group x `tile_feet` (stepped onto or met: one tile; ambush: 5;
  opening strike: the real distance). Later stacks stand `stack_spacing_feet` (rules value, 10)
  behind the one before.
- **Monsters move.** On its turn a stack closes by its `speed` (never under 5), then attacks if in
  reach; if it cannot reach it dashes (double move, no attack). A stack with a ranged attack shoots
  instead of dashing when in range. A stack with only ranged attacks holds.
- **The party's line moves as one.** It may advance up to the slowest front-row member's speed each
  round; advancing by d brings every stack d closer. A melee attack or touch spell at a stack just
  out of reach advances automatically if the round's allowance covers it. New
  `CombatCommand::Advance` (an action, the SRD Dash) adds one more speed to the allowance and moves.
  No retreat other than Run, as today.
- **Reach.** Weapons and monster attacks get `reach` (feet, default 5). Front row reaches a stack at
  `distance <= reach`; the back row stands 5 ft behind, so a 10 ft weapon reaches an engaged stack.
- **Range.** Ranged weapons and attacks get `range: (normal, long)` in feet (default 80/320 when
  absent). Past normal: disadvantage. Past long: refused `BeyondRange`. A ranged attack by someone
  engaged (front-row member with a stack at 5; a stack at 5) has disadvantage. Spells: `Touch` is
  reach 5, `Tiles(n)` is `n x tile_feet`, `Sight` is anything in the fight.
- **Per square.** `Monster.per_square: u8`, default from size (Tiny 4, Small and Medium 1; Large and
  up take 4, 9, 16 squares each, per the SRD's space). Uses:
  1. *Frontage*: the front row presents one square per member who can fight. Engaged stacks fill
     those squares in order; a stack attacks in melee with at most `squares x per_square x
     (reach / 5)` individuals a round (at least one). The rest shoot if they can, else wait.
  2. *Area*: new `Reach::Area { squares }` hits at most `squares x per_square` individuals of the
     target stack. `Stack` and `AllStacks` stay. Base: burning hands 6, thunderwave 9, shatter 12.
  3. *Tile cap*: a group's stacks at full count must fit the tile's `(tile_feet / 5)^2` squares.
     Refused by loader and editor **only on a map that writes `tile_feet`**; an unscaled map is
     exempt, because at the default 5 ft every existing group (and the owner's pack) would fail.
- **Retired**: the rules value `monster_front_stacks`; "front stack" becomes "engaged"
  (`StackView.in_front` keeps its name, `distance` is added).
- **Opening strike.** With `opening_strike_surprise` on, the party makes an SRD group Stealth check
  against the group's best passive Perception. Success: `Surprise::Monsters` and advantage on the
  strike's attack roll. Failure: no surprise, the strike still lands first. The fight starts at the
  real distance, so melee monsters spend turns closing. New road `Command::Attack { member, target }`
  lets a ranged weapon open a fight the same way. The strike still lands on the nearest stack (the
  stacks do not exist until it is cast).

### Reused
`encounter::build/begin/struck`, `combat::start_struck`, `turn::strike`, `resolve::monster_turn`,
`reached`, `pick_attack`, `combat::weapon_for`, `targets::in_range` (`TargetView.distance` becomes
real), `checks::roll`, `passive_perception`, `RollMode` advantage, `targets/map.rs::distance`.

## Part II: things that break

### Rules
- **Durability is opt-in.** `Attributes.durability: Option<Durability { fragile, hit_points,
  armor_class, threshold, broken_state }>`. Absent means it cannot be damaged (existing objects with
  size and material written for flavour do not change). Present: AC from material (cloth, paper,
  plant 11; glass 13; wood, bone 15; stone, earth 17; metal 19), hit points from size (Tiny 2/5,
  Small 3/10, Medium 4/18, Large 5/27, fragile/resilient); Huge and up, or liquid, must write the
  numbers. Immune to poison and psychic; fails Strength and Dexterity saves; other saves do nothing.
- **Doors.** `Map.door_make` (material, size, durability); default wood, Medium, resilient (AC 15,
  18 hit points). Doors are breakable by default; a map may write `door_make: None`.
- **What may strike a thing.** `Spell.objects: bool` (base: fire bolt, acid arrow, shatter), and
  `Command::Attack` with a weapon, in touch or in range and sight. One attack per command, one
  minute. On the road only; in a fight a thing is still `WrongTarget`.
- **State.** `ObjectState.damage: u16`, `broken: bool`; `MapState.door_damage`, `broken_doors`
  (regional, travelling in the journal: `Change::Object` carries it, doors get a new `Change`).
  Schema stays 9 with serde defaults, as `links` did.
- **Broken.** An object shows `broken_state` if its kind names one, else a heap; it blocks nothing
  unless that state says so; it offers nothing but repair. Contents spill with `ground::found`;
  stacks of `Item.fragile` items are destroyed (`ItemDestroyed`) when the breaking damage is acid,
  bludgeoning, cold, fire, force, lightning or thunder. A broken door counts as open everywhere
  (`MapState::door_open` is the one place) and cannot be toggled.
- **Repair.**
  1. *Mending*: new `Utility::Mend`, base cantrip `mending`, touch; restores a Tiny or Small thing.
  2. *Tools*: `Command::Repair { member, target }`. Needs a carried item with
     `craft: Tool([materials])` for the thing's material and supplies (`craft: Supply`) by size
     (0, 1, 2, 4), a Dexterity check DC 10 (tool proficiency joins when that system exists), and
     10, 30, 60, 120 minutes by size. Base adds carpenter's, smith's and mason's tools and timber,
     iron stock and cut stone.
  3. *Service*: the Smith kind gains `Repair { map, target }`: lists broken things on the town's
     region's maps, priced by size, repaired at once.
- **Events and API.** `ObjectDamaged`, `ObjectBroken`, `ObjectRepaired`, `DoorDamaged`,
  `DoorBroken`, `DoorRepaired`; rejections `CannotBeDamaged`, `Broken`, `NeedsTool`,
  `NeedsSupplies`; `EdgeView::Door` and `ObjectView` gain damage and broken. `docs/api.md`, MCP
  schema and the pinned counts move in the step that adds each.

### Reused
`casting::validate` (the `WrongTarget` arm at L73-81 gains the thing branch), `damage_roll`,
`spell_attack`, `objects::edit`, `ground::found`, `placement::in_state`, `objects::blocks_step`,
`visibility::edge_open`, `apply::advance`, `service::offered_in/quote/settle`, `targets/map.rs`.

## Steps (one commit each: tests red first, mutation checks, fmt, clippy, gate from 1387, Sentrux)
0. **Documents**: plan to `tasks/plans/combat-in-feet.md`, `tasks/TODO.md`; PRD 8.2/8.3 and
   ARCHITECTURE 4.5/4.9 (approved by this plan, flagged in the commit); `tasks/LESSONS.md` (combat
   scale follows the SRD; I proposed 15 ft tiles and was corrected).
1. **Data, both parts**: every new field above with serde defaults and load checks; base written
   out (reach and range on weapons and monster attacks, `per_square` on kobold-sized and rat-sized
   monsters, areas, `objects` flags, fragile potions, mending, tools and supplies); base resealed
   and both goldens rebaselined once; fixture `objects` pack gains durable kinds; schema dump.
2. **Distance in the state**: `Stack.distance`, starting distances, `StackView.distance`, engaged
   replaces front stacks. No new behaviour yet beyond that.
3. **Monsters in feet**: move, dash, reach, range, disadvantage, frontage.
4. **Members in feet**: reach, range, the line's advance, `Advance`, spell range in a fight, real
   distances in `targets`.
5. **Area by squares**.
6. **Opening strike**: surprise roll, advantage, real distance, road `Command::Attack` at a group.
7. **Client**: distance beside each stack and stacks ordered by it, Advance button, Attack on the
   road's target step. **Tile cap** in loader and editor forms. Report: built, please try.
8. **Objects take damage**: state, AC and hit points, cast and attack at an object, events, break,
   spill, fragile loss, views, blocking.
9. **Doors** the same.
10. **Repair**: mending and `Command::Repair`.
11. **Repair service**.
12. **Client**: damaged and broken marks, Attack and Repair entries (mouse first), the service row.
13. **Documents**: acceptance sheet (what the owner's own maps show), code map, continuity.
    Report: built, please try.

Golden replays are rebaselined in steps 1, 3 and 4 (fight arithmetic), noted in the api changelog.

## Verification
- Sim tests on the fixture packs: starting distance per way of meeting; a melee stack closes over
  rounds and cannot hit before it arrives; a bow at long range has disadvantage and past long is
  refused; a 10 ft weapon from the back row; three rats a square against one human a square;
  an area of 6 squares hits 6 humans or 18 rats; surprise both ways with fixed seeds, and off by
  the rules value; costs untouched after every refusal; list and command agree for every target.
- Things: fire bolt at the fixture chest lowers hit points, breaks it, spills, burns a potion;
  poison does nothing; a thing without durability is refused; a broken gate stops blocking step
  and sight; save round-trip and journal; mending by size; repair with and without tool and
  supplies; the service's quote and effect.
- `scripts/verify.sh` to a log, commit only on `VERIFY-GREEN`. By hand (owner): open a fight with a
  bow from down a corridor and watch the stacks close; break and mend the fixture-style chest.

## Risks, said plainly
- **Large**: fourteen commits. Part I (steps 0-7) stands alone; Part II can stop after 8 or 10.
- **Balance moves**: monsters arriving later, frontage limiting big stacks and areas hitting fewer
  than a whole stack all change difficulty. No balance pass is in this plan; the measure harness
  is run before and after and the numbers reported.
- **The tile cap is toothless on unscaled maps** by design (see Per square, 3). If the owner wants
  it enforced at 5 ft, groups would need to spread over tiles, which is a different plan.
- **The line's advance is an abstraction**: members have no positions, so one member's charge moves
  the whole party. It is the smallest rule that lets melee reach archers.
- **Road attacks cost a minute a swing**, not six seconds; coarse, like other road actions.
- **Repair by tools is our rule**, not the SRD's, and uses a plain ability check until tool
  proficiencies exist.
- The owner's pack is not touched. Its monsters get `per_square` from size and its weapons 5 ft
  reach and 80/320 by default.

## As built, where it departs (said in the commits)
- Step 6 was built before steps 4 and 5 at the owner's word.
- **Advance** is free while the round's pace lasts and costs the action (the Dash) only after; the plan
  said "an action". SRD movement costs no action.
- **A member with a ranged weapon in range shoots**; the automatic advance is for a front-row member who
  has none. Advance, then Attack, is how the player chooses the sword.
- **An engaged stack stops the line**: a stack behind it is reached by a ranged weapon, a spell or a
  longer reach.
- No dash counter is kept: the feet advanced this round say everything.
- Step 8: a save spell flagged `objects` lands in full on a thing whatever its save's ability (shatter
  saves on Constitution; the SRD has objects in its area take the damage). `ObjectDamaged` is the event of
  every blow at a thing, a miss included (`amount` 0).

- **Step 9** (2026-11-15): one field, `MapState.worn_doors: BTreeMap<(Level, x, y, Facing), DoorWear>`,
  in place of `door_damage` and `broken_doors` (the same data in one place); the journal's change
  is `DoorWear`. No new rejection. `ViewportModel.door` says the map's door numbers. A broken door
  is drawn as two stumps. A per-door make (some doors of a map unbreakable) is not built: the make
  is the map's.
- **Step 12, in part, before step 9** (owner, 2026-11-15): a broken object is drawn as a wreck in
  the 3D view (a low heap in its footprint, crossed over its top), whatever its kind's shape and
  whether or not the kind names a broken state. The engine's look is not changed.

- **Step 11** (2026-11-25; `tasks/plans/repair-service.md`): the command is `ServiceCommand::Repair { job }`
  with a `Job` that names the map (and a door's level), not `Repair { map, target }`. Damaged things are
  taken too, at half; a buried doorway is listed and refused; the price is the slot `smith.repair_cost`.
  The service row of step 12 was built here.

## Not in this plan
A fight grid, member positions, cover, flanking, retreating or kiting monsters, thrown weapons and
ammunition, choosing the stack an opening strike lands on, attacking things during a fight,
terrain damage, fire spreading, locks on doors, tool proficiencies, damaged carried items.
