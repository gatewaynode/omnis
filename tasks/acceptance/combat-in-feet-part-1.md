# Combat in feet, part I: the fight in feet

Built 2026-11-14 (`tasks/plans/combat-in-feet.md`, steps 0 to 7). Not seen on screen by me: every
"you will see" below has a test behind it, none has a picture.

## How to start

`just qa` (the yard of the fixtures). The two pack rats stand at (5, 1); the party starts in sight
of them. Your own world works too: any group that stands in view.

## What to try

1. **A weapon opens a fight.** With the rats in sight, open the square's actions (Space). "Attack..."
   is there. Press it: "Choose a target" lists "Pack rat x2, east" with the tiles between. Beside
   Back, "Attacker" steps round the party. Choose: the fight starts and the log shows the party's
   Stealth rolls, then the blow, then round 1.
   - Out of sight of any group, "Attack..." is not offered.
   - A member with no ranged weapon (Ilvara), from more than a tile away: the row is dim with the
     reason. (The engine refuses it; the row's wording has no test of its own.)
   
   QA Note: There is no functionality to shoot a mundane weapon like Pip's shortbow from a range and start a fight at a distance.

2. **Surprise is rolled.** Open several fights with a spell or a weapon. Sometimes the log says "the
   monsters are surprised", sometimes only "The fight begins". When they are surprised, the opening
   attack rolled two dice (advantage) and no monster acts in round 1.

   QA Note: Works as described.  Backlog a future order UI widget.

3. **Distances on the fight screen.** Open a fight from two or three tiles away. The rats are named
   with their feet ("Pack rat  10 ft") and the log says "Pack rat closes with the party" when they
   arrive. A stack that is engaged shows no distance.

   QA Note: This works as described, but again there is no UI for distance, like a small text indicator next to foes indicating how far away they are at any given time.

4. **Advance.** While no stack is engaged the turn's menu offers "Advance 25 ft" (the dwarf in the
   front row sets the pace; 30 ft without him). It costs no action. Once the round's pace is used it
   reads "Advance (dash)" and costs the action. Engaged, it is dim: "the party is engaged already".

   QA Note: Option visible, but no chance exists to use it after trying 3 combats in the yard.


5. **Who reaches what.** A member with a bow shoots a stack that is not engaged; a front-row member
   without one walks the line up to it if the round's pace covers the gap, and the log says "The party
   advances 10 ft". A stack behind an engaged one cannot be walked to. The back row cannot use a
   touch spell on a stack.

   QA Note: Works as designed.

6. **Your screenshot's case.** After a spell opens the fight, the caster's first turn says "the turn's
   action is spent" on Attack, Cast and Dodge.

   QA Note: Confirmed.

## What changed that you did not ask for by name

- A front-row caster's ranged spell attack (fire bolt) has disadvantage while a stack is engaged, as
  a bow's has. Ilvara stands in the front row in the usual six.
- Burning hands catches as many as fit six squares (18 giant rats, 12 goblins), not the whole stack.
- A map that writes `tile_feet` is refused when a group does not fit one tile. No map writes it yet.

## Decisions for you

1. **Advance is free within the round's pace**, the action only as a dash. The plan said "an action";
   I followed the SRD. Say if you want it to cost the action always.
2. **A member with a bow shoots rather than charges.** To use the sword: Advance, then Attack. The
   other way round (charge by default) is a one-line change.
3. **An engaged stack stops the line**, so archers behind a front rank are safe from swords.
4. **Ammunition is not counted.** Arrows and bolts are carried and never spent.
5. **Difficulty**: no balance pass was made. Say what feels wrong after a few fights.
6. **`tile_feet` and `door_make` have no editor field**; they are written in the map's file.

## Answers to the QA notes (2026-11-14)

- **Note 1, no way to shoot Pip's shortbow.** It was under Space, "Attack...", which is not where you
  looked. Built, please try: select Pip in the party column; beside Sheet, Inventory and Spells there
  is now **Attack**. It opens the target list with Pip as the attacker; Choose starts the fight. It is
  dim with the reason when no group is in sight or nothing of theirs reaches. The entry under Space
  stays.
- **Note 2, order widget.** In `tasks/BACKLOG.md`, Interface.
- **Note 3, no distance indicator.** The feet showed only on stacks that were not engaged, and the
  yard's rats are engaged before the first turn. Built, please try: every stack's label now ends with
  its feet at all times ("Pack rat  5 ft").
- **Note 4, no chance to use Advance.** True in the yard: the rats move 30 ft a turn and the yard is
  35 ft long, so they are upon the party before anyone acts. Advance matters against a group that
  holds off (archers) or starts far away. No fixture has one. Recommended: a small QA map (a long hall
  on 10 ft tiles with archers at the far end); say the word and I build it.
