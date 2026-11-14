# Calmer fight colours and one targeting flow: acceptance

Built 2026-11-14 (`tasks/plans/targeting.md`, nine steps). You tried steps 1, 4 and 5 the same day
("looking good so far"). Steps 6, 7 and 8 are **not tried by you yet**.

## The flow, everywhere
Open the menu, requirements are checked, the options are shown (a failed one dim, with the reason),
choose the option, choose from its possible targets, the target is validated, the cost is paid, the
option is carried out. Dodge, Run and End turn have no one to choose and stay one click. Everything
else asks, even when the only row is "self".

## What each kind of range reaches, arm by arm

| Range in the data | On the road | In a fight |
|---|---|---|
| **Own** (light, detect magic, shield, spyglass, class features) | the one acting, one row marked "self" | the same |
| **Touch** (cure wounds, guidance, potion; burning hands, thunderwave, inflict wounds) | every member; for a spell on the world, the door the party faces and an object in its cell or the one it faces; for a harmful spell, the group in the cell it faces | every member; the **front** stacks only (a back stack is listed dim, "out of reach") |
| **Tiles(n)** (mage hand 6, bless 6, healing word 12, sacred flame 12, shatter 12, fire bolt 24, magic missile 24 ...) | every member; doors, objects and groups within n tiles, counted as a king moves, in any direction | everyone (a fight has no distances) |
| **Tiles(n) that also asks a clear line** (most of the above; shatter does not) | as above, and a thing you know of but do not see is listed dim, "out of sight" | everyone |
| **Sight** (no base spell writes it; what an attack defaults to in a pack that writes nothing) | every member; things within your sight depth with a clear line | everyone |
| **Link** (no spell or item writes it yet) | only what the one acting is linked to, wherever it is, seen or not | the same |

What is a target of what: a heal or a buff takes a member; mage hand takes a door or an object; an
attack, auto-hit or save spell takes a standing group on the road and a stack in a fight. Anything
else is refused before anything is paid.

"Known" means: a door whose edge is on your automap; an object whose tile is on your automap and that
is not hidden, or that you have found. A group is a target only while you see it. What you neither
see nor know of is not listed at all, so the list tells you nothing you have not found out.

## What your own maps will show
Your pack has no objects, so on your maps the things of the map are **doors and standing groups**:
the dungeon (21 door marks, 10 standing groups), the depths (6 and 8), the town (7 doors, no groups),
the meadow (neither).
- **Mage hand** (Ilvara): its list is the doors within six tiles that you see, each as "Door, east"
  with "3 tiles"; a door you have seen and no longer see is dim, "out of sight"; in the meadow the
  Cast button is dim, "nothing in reach". It no longer opens "the first door straight ahead": you
  choose the door, in any direction.
- **Fire bolt, magic missile, sacred flame and the other harmful spells are now on the road's Spells
  list.** With no group in sight they are dim, "nothing in reach". With one in sight the row names it
  ("Giant Rat x2, north", "4 tiles"); choosing it is the **opening strike**: the spell is paid and
  lands, and the fight starts with the monsters surprised for the first round and the caster's own
  first action already used. A peaceful group struck this way is hostile. There is no bribe, hide or
  run choice before such a fight.
- While the pointer is on a row for a door or a group, that tile's floor is outlined in the 3D view.
- In a fight nothing changes from what you tried.

## Try it
1. `just run`, the dungeon. Open Spells for the wizard, Cast on Mage Hand: doors in sight are listed
   with their direction and distance. Point at a row: the tile is outlined in the view. Choose one:
   it opens, a minute passes. Turn your back to a door: it is still listed (sight is all round).
2. Walk away from a door you have seen until a wall hides it: its row is dim, "out of sight".
3. Stand where a group is in sight. Cast Fire Bolt (free) or Magic Missile at it: the fight starts,
   the log shows the damage first, the monsters do not act in round one, and on the wizard's first
   turn the action is already spent (bonus action and End turn remain).
4. Objects: `just qa` (the fixture yard). South of the hedge at the gate, Mage Hand lists the gate
   (dim until the hidden lever is pulled), the chest, the fountain and a door. The chest opens by its
   own rule: your party's Athletics check, as if by hand (see decision 3).
5. Links cannot be tried in play: nothing forges one yet. Over the dev socket, `Dev(Link { member,
   target, linked })` forges one, and a spell whose data says `range: Link` then lists it.

## Decisions for you
1. **The opening strike is strong and unbalanced**: a free spell and then a round in which the
   monsters do not act. Keep (a pack can write `opening_strike_surprise: 0` to drop the surprise), or
   make the surprise a contested roll, or cost the caster more than one action?
2. **Which stack a strike lands on** is the group's first stack in front; you do not choose. Enough,
   or should the row open the group's stacks to choose from?
3. **A hand works an object by the object's own interaction as written**: a chest that asks an
   Athletics check asks it of the party, and a key is used up, though the hand is a spell. Keep, or
   should a hand skip strength checks, or be refused where one is asked?
4. **Only spells strike from the road.** A bow or a thrown weapon does not; Attack is not on the
   road. Add a ranged weapon's opening strike, or leave it to spells?
5. **Harmful spells now lengthen the road's Spells list**, mostly dim. Keep them listed (your "dim
   with the reason" rule), or list them only while a group is in sight?
6. **More paleness in the fight** still needs the default party or hostile ink moved (the green-blind
   limit from step 1). Unchanged since you called the colours less straining; say if you want more.
7. **Give** still goes to whoever is selected in the party column; it is not a targeting of an
   effect, so I left it. Bring it into the same step?
8. **Light and detect magic** offer "self" and the engine still accepts any member for them (it
   ignores which). Tighten so only the caster is accepted?

## What is not built
Member positions or a fight grid (so "range N" means nothing inside a fight); cover or partial sight;
spells that forge links; heals or buffs on creatures and objects; monsters striking the party from a
distance; the minimap marking targets; a link that names a thing on another map.

## What I saw myself
Tests only (the gate stands at 1387; 63 mutations caught across the nine steps, one more did not
compile). The game was not running when I asked its dev socket for a picture, so I have seen none of
this on screen: not the calmer colours, not the target lists, not the outline in the view. The
outline for a **dim** row rests on Bevy keeping its hover state on a node that is not a live button;
my test covers a free row only. Sight is the engine's existing line-of-sight rule, the one the 3D
view draws by; in the fixture yard it sees a group through the gap in the hedge from the far corner,
which is right but may surprise.
