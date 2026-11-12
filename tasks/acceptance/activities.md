# Acceptance: activities in the editor (after M5.7)

The owner's category (2026-10-24): "a category of map item that the town services fall under, something like
activities", the things that take the party out of its stride into a decision, each with a representation
in the game. Decided at the M5.7 planning: an editor grouping now, the data's `sites` kept until a second
kind has engine rules.

**Accepted by the owner 2026-10-24** ("much better placeholders").

## What was built
- **Add → Activity.** The inspector's Add choice reads Portal, Encounter, **Activity**, Object, Ground pile,
  Lore; "Site" is gone from it. An activity's form names its **kind** (a choice: "Service site" today; the list
  the category grows by) and what the kind needs (the service's id); Apply places a site as before. The
  inspector lists it as `Service <id>` with Edit and Remove.
- **An icon per service kind** on the plan's mark and the 3D floor, from the Lucide set: inn `bed`, temple
  `church`, trainer `dumbbell`, smith `anvil`, tavern `beer`, bank `coins`, guild `shield`; a service the
  loaded packs lack keeps the grinning face. The game draws the same floor icons.

## Steps
1. Editor, open the town, zoom in.
   *You will see* on each service's cell its kind's icon: a bed on the inn, a church on the temple, a dumbbell
   on the trainer, an anvil on the smith, a beer on the tavern, coins on the bank, a shield on the guild.
2. Click a street cell, step Add to Activity, **Add here**.
   *You will see* "Activity at (x, y)" with kind "Service site" and a service choice; Apply: the inspector lists
   "Service base:service:…", the cell carries that service's icon.
3. New game, face a service's door.
   *You will see* the same icon on the cell's floor.

Covered by `tests/editor_forms.rs` (the Add choice, the form, the placed site and its mark) and
`tests/geometry.rs` (each site's floor icon is its kind's, line for line).

## Known and expected
- The icon names are a first pass; renaming one is a one-line change in `icons::service_icon`.
- The data keeps `sites`; `activities` as a placement of its own waits for a second kind with engine rules.
