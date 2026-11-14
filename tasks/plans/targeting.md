# Calmer fight colours, and one targeting flow for spells, items and actions

## Context

Two things from the owner's try of the fight on the palette (2026-11-14):

1. **The fight's colours are too saturated.** Answer: less glow and paler, in the fight only.
2. **Targeting.** On the road, a spell for one individual cannot be aimed at anyone but the caster
   (the target is silently the party column's selection, which starts as the caster). The owner's rule:
   every spell, item and action follows

   `open menu -> check requirements -> show options -> select option -> select from possible targets
   -> validate target -> pay costs -> execute`

   with five target kinds: **self**, **touch** (within one action's movement), **range N**, **line of
   sight** (obscured and unobscured), **link** (always targetable).

Owner's decisions (asked in plan mode):
- Saturation: less glow and paler, fight only; the 3D view's inks untouched.
- Scope: targeting declared in data, one engine list of possible targets, **and map targets now**.
- Map targets: **doors, objects and creatures**. An attack or save spell at a standing group is an
  **opening strike** (a rules addition, PRD 8.2/8.3).
- Failed requirements: the option stays listed, dim, with the reason (as today).
- Dodge, Run, End turn: one click, no target step. Everything that acts on someone or something gets
  the step, even when the only row is "self".

What exists today (research): spells carry only `Reach {One, Stack, AllStacks}` (area in a fight);
who is aimed at is inferred from the effect; no range, touch or sight anywhere. `Target` is
`Stack(u8) | Member(id)`. The engine has no "possible targets" list; the fight menu brute-forces one by
trying every candidate on a copy of the world (`combat_menu::targets`), and the road has no target step.
Members share the party's tile; a fight is two party rows and ordered stacks with no distances.
Validation already comes before any cost in all three paths.

## Design

### A. Calmer fight colours (`tones.rs`, `cinema.rs`)
Fight only. Two fixed steps applied to the party and hostile inks before the table's factors:
glow capped (the ink scaled so its brightest channel is at most a set value just over 1.0, so lines
glow but no longer flood), then mixed part-way toward white. The picture window's ink gets the same
paling. The interface ink's marks (bars, target frame) get the glow cap only. Starting values: cap
1.25, mix 0.35; tuned by eye if I can get a picture this time (`mcp__omnis__screenshot`), otherwise
said plainly as untuned. Sides must still pass `contrast` for every vision (existing test).

### B. Targeting in data (`omnis-data`, new `targeting.rs`)
```
struct Targeting { range: Range, sees: bool }      // sees: a clear line is also required
enum Range { Own, Touch, Tiles(u8), Sight, Link }
```
On `Spell` (`targeting`), `Item` (beside `use_effect`), and class features. Serde default: absent
means "as today" (derived from the effect: members for heal/buff = Touch, stacks = Sight, else Own),
so no fixture or third-party pack breaks. **`packs/base` writes every spell and usable item
explicitly, in SRD terms at 5 ft a tile** (cure wounds Touch; healing word 12 sees; bless 6; magic
missile 24 sees; sacred flame 12 sees; shatter 12; mage hand 6 sees; light Touch; detect magic,
shield, thunderwave Own; potion Touch; spyglass Own). Base is resealed (version up) and both golden
replays rebaselined in the same commit. Validation: `Tiles(n)` 1..=MAX_VISIBILITY_DEPTH; an effect
that cannot use a kind is refused at load. `Reach` (area) stays as it is.
Weapons keep `ranged: bool`; an attack's targeting is Touch (melee) or Sight (ranged).

### C. What each kind reaches (one function, both modes)
| Kind | On the road | In a fight |
|---|---|---|
| Own | the actor | the actor |
| Touch | every member; map things in `objects::act::reach` (own and faced cell) | every member; the front stacks (back stacks: out of reach) |
| Tiles(n) | every member; known map things within n tiles (king's move) | everyone (ranges collapse, PRD 8.3) |
| Sight | every member; map things within the party's sight depth with a clear line | everyone |
| Link | the actor's links, wherever they are | the same |

`sees` (and Sight) use `visibility::line_of_sight` in every direction from the party's tile, not just
the forward cone. Sight is a yes or no in this engine (no cover or half-light), so **"obscured" means
the line is blocked**: an obscured candidate is listed dim with the reason, but only if the party
already knows of it (a found object, a door on an explored tile); a creature out of sight is never
listed, so the list leaks nothing. Hidden objects not yet found are never listed.

### D. The engine's list (`omnis-sim`, new `targets.rs` + view)
- `Target` gains `Door { x, y, facing }`, `Object { object }`, `Being { being }` (additions; protocol
  stays 2). The shapes mirror `objects::Choice`.
- `targets(world, data, actor, option) -> Vec<TargetView>`; `option` names the thing chosen, kind-open:
  `Spell(id) | Item(id) | Feature(id) | Attack | Exchange`. Row: `{ target, name, distance, sight:
  Clear | Obscured, refusal: Option<Rejection> }`. Candidates come from C; each is judged by the very
  function the command path uses, so the list and the command can never disagree (a test holds that
  every unrefused row is accepted and every refused row rejected with that reason).
- Command paths (`casting::apply`, `combat::validate`, `items::validate_use`) call the same check:
  new `Rejection::OutOfRange` and `OutOfSight`. Still validate, then pay, then execute.
- Op `targets.get`, in `api.rs`, `docs/api.md`, MCP tool `targets_get`, schema for the new `Target`
  arms; the pinned counts (tools, Rejection variants) move with it.

### E. What map targets do
- **Mage hand** becomes a hand, not "first door ahead": it acts on the chosen **door** (toggle) or
  **object** (the object's own Open, Close, Push, Pull or Use interaction, through `objects::act`'s
  `admit`, so requirements and once-only still hold). `Utility::OpenDoor{range_tiles}` keeps loading
  (its range becomes the targeting when none is written).
- **Opening strike**: `Command::Cast` of an attack, auto-hit or save spell at `Being`. Checks: group
  standing, in range, clear line. Then: pay; `encounter::begin`'s stacks are rolled; the spell
  resolves on that group by its `reach` exactly as in a fight; the fight starts with
  `Surprise::Monsters` (reserved until now) and the caster's action for round 1 spent. A peaceful
  group becomes hostile. No pre-fight choices (bribe, hide, run) after a strike.
- Heal and buff reach members only for now; an object or creature is refused `WrongTarget`.
- **Link**: `World.links` (member -> set of `Target`), saved. The list and checks honour it. No spell
  forges a link yet: a dev command makes one so it can be tested; link-making spells are a later plan.

### F. The client: one target step (`omnis-vector`)
- New Bevy-free `targeting.rs`: turns the engine's rows into entries (name, distance, "out of sight",
  dim with reason) for both screens.
- **Road cast and inventory Use** (`screen/cast.rs`, `screen/inventory.rs`): pressing Cast or Use
  opens the target list in the same screen (always, even one row "self"); choosing a row sends the
  command; Back returns. The party column's selection no longer decides a target; clicking a member
  in the column during the step picks that member. The hovered map target is marked in the 3D view
  with the existing `Aimed` mark (`shell/aim.rs`).
- **Fight** (`combat_menu.rs`): `targets()` reads the engine's list instead of trials; "cast at once
  when the target does not matter" goes; arena clicks and the column list stay. Dodge, Run, End turn
  unchanged.
- Give stays as it is (not a targeting of an effect); noted for the owner.

## Steps (one commit each; tests red first, mutation checks, fmt, clippy, gate against 1352, Sentrux)
0. **Documents**: this plan to `tasks/plans/targeting.md`, `tasks/TODO.md`; PRD 8.2/8.3 and
   ARCHITECTURE 4.5/4.9 lines for targeting kinds, the opening strike and `Surprise::Monsters`
   (approved by this plan, flagged in the commit); `tasks/LESSONS.md` (a target is never implicit).
1. **Calmer colours** (A). Small and first, so the owner can re-try the fight early.
2. **Data** (B): type, defaults, validation, base written out, reseal, rebaseline, schema dump.
3. **Engine list and checks** (C, D) for members and stacks: `targets`, shared check, new
   rejections, op, MCP, api doc.
4. **Road target step** (F, cast and inventory) on step 3. This closes the reported bug.
5. **Fight menu on the list** (F).
6. **Map targets: doors and objects** (C, D, E mage hand; 3D mark).
7. **Opening strike** (E).
8. **Link** (E).
9. **Documents**: acceptance sheet (walks C's table arm by arm and says what the owner's maps show),
   code-map, presentation-ARCHITECTURE 6.5/6.7, CONTINUITY.

Steps 1 and 4 are each reported "built, please try" as they land; the rest at the end.

## Verification
- Sim tests: each row of C's table on fixtures (the `objects` pack yard has a gate, objects and
  rats; scratch copies where a spell is needed); list and command agree for every candidate; costs
  untouched after every refusal; opening strike spends points, surprises the stacks and resolves once;
  save round-trip with links. Client tests: the road cast of a heal lists every member and heals the
  one chosen (the bug's test, red first); a self-only item shows one row; the fight offers exactly the
  engine's rows.
- `scripts/verify.sh` to a log, commit only on `VERIFY-GREEN`; goldens rebaselined only in step 2.
- By hand (owner): cast cure wounds on the road at another member; mage hand at a door down a
  corridor; an attack spell at a group seen ahead; the fight's colours.

## Risks, said plainly
- **Large**: nine commits across data, engine, API and client. Steps 1-5 stand on their own if the
  owner wants to stop there.
- **Base pack behaviour changes**: with SRD ranges written, a front-row attacker's touch spells no
  longer reach back stacks; mage hand needs a chosen target. Replays are rebaselined.
- **The opening strike is new rules** with no balance pass: a free first spell plus a surprise round.
  A rules value (`opening_strike_surprise`, on by default) lets a pack turn the surprise off.
- **"Range N" in a fight means nothing yet** (no distances); it matters only on the road.
- The owner's pack (`packs/the_lower_kingdoms`) is not touched; its spells, if any omit `targeting`,
  behave as today.

## Not in this plan
Member positions or a fight grid; cover or partial sight; spells that forge links; heals or buffs on
creatures and objects; ranged weapon attacks from the road; Give; the minimap marking targets; the
HUD and minimap palette plans; the open decisions of the fight-on-the-palette sheet other than
saturation.
