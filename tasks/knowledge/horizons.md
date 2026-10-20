# Horizons

Everything deferred so far, with the milestone it waits for. A horizon is not a rejection
(`agreements.md`): it stays here until it is built or the owner rejects it outright.

## Dated re-audits
- 2026-10-10: Socket re-audit of `rhai` 1.26.1.

## M7 (town, services, rest, progression)
Planned 2026-09-20 (the M7 block, now in `tasks/archive/TODO-2026-10-20.md`). Left out of M7 by its plan, each a horizon:
swapping and hiring at the inn (D10 sidecar), donations, identify and repair (no such item state),
limited shop stock (bank interest is out: owner, 2026-10-05, "There will be no bank interest in this game"), rumors from the story engine (M11; M7's
are a static list), cantrips gained by level, ability score improvement, subclasses, Extra Attack,
3rd-level spells, rolled hit points as an option. M7b's plan (2026-10-03) adds: spell picks at
creation (known spells are still the class list's first entries), half casters' level-ups, the SRD's
full spell effects where M7b's spells simplify them (named in each spell file). Money is one copper number from save schema 5
(owner, 2026-09-27; shown as whole gold rounded down, broken out by denomination in the inventory):
**a real coin purse** (counts of gp, sp and cp held, change-making, coin weight) is a horizon; so is
**"remember my choice"** on the confirmation before entering or leaving a service (a checkbox,
owner's stretch goal, after step 4b). Waiting since before:
Rest (pools only empty until then; the debug menu refills them); the temple for the `dead`
condition; shops (the acolyte's potion leaves the kit when shops exist); a blacksmith; `Relief`
items; `doff_armor_minutes` as its own value. (String ids in saves, waiting here since M7b, were
built as M8.5 7h: below, and ARCHITECTURE §4.1.1.)

## The save model (PRD D28, M8.5 step 7)
Built in 7a–7f (ARCHITECTURE §4.1.1). Left for later, each a horizon:
- **The archive as the world's history.** Every fold keeps the journal it folds as an archive
  segment (`.omnis/worlds/<id>/archive/`), never read back today. A chronicle of the world, a
  "what happened here" for a place, or the story engine's memory (M11) can read it. The full log
  reads the party's own log instead (owner, 2026-10-18).
- **Co-op over one world** (D5): several characters' saves naming one world folder need the
  world's changes stored as shared layers, not only as each save's private journal.
- **Carrying a character to another world**, or importing one: the checkpoint is the party apart
  from any world, but its ids, flags and knowledge name one world's packs and maps (PRD §7.7).
- **Fold thresholds from the world simulation** (M10): today a safe point folds at 5,000
  interactions, a provisional rules value; story points (M11) fold too.
- **The PRD's save-model text** (owner, 2026-10-18, 7g: only §7.7's tidy line was brought to the
  build; 2026-10-20, 7h7: its changed-pack line too, and the line on the save before D28 removed): D28 and §7.7 still say the full log reads the journal and that loads replay from a base;
  §7.6, §9.3, §11.1 and §14 predate flag scope, the lay-not-replay load and the provisional fold.

## Saves by name (M8.5 7h)
Built in 7h1–7h6 (ARCHITECTURE §4.1.1; plan `tasks/plans/m8.5-saves-by-name.md`): a save keeps
the names its numbers stand for, loads on changed packs by name, and is offered the packs'
migrations. Left as it is, each a horizon:
- **Replays need identical packs.** A replay (A14) reproduces a game from its commands for tests
  and goldens; it is not remapped.
- **Snapshots are not recomputed.** A stack's or member's hit points stay as saved, so a monster
  whose maximum went down may stand above it after a load mid-fight.
- **Two events name rows:** `Event::Rumor { rumor }` and `RestEvent { entry }` are rows in file
  order, so after a pack change the party's log may retell another rumor or rest event. Cosmetic
  (`docs/api.md` §14); ids for them when either is edited in practice.
- **Stream names keep region numbers** (`time:party:0:region:3`). They are rebuilt by name on a
  renumbering; moving them to string ids would change every golden.
- **Interaction ids.** An object kind's `once` interactions are found again by verb, label and
  state; two with one signature are told apart only by order. Optional interaction ids if an edit
  makes that matter.
- **Carrying a character to another world** (above) is nearer: the name table is what a carry or
  an import would resolve a party's ids through.

## Turn budget and tactics (PRD D21–D24, §7.9; approved 2026-09-20)
**Built in M7c (2026-10-04, ARCH §4.7)**: everything in the "Planned for M7c" paragraph below, the tactics panel beside the sheet, and Bob; D24 stands over the SRD's one-spell limit (step 4b reverted in step 8a, owner 2026-10-04). What follows is the history and what stays open.
**Owner, 2026-09-20: M7c, M7's last acceptance point, planned in plan mode when reached; ARCH
§4.7's auto resolution inside the simulation is confirmed; its save schema is 6 (M7a takes 5).**
Because Second Wind and Cunning Action need bonus actions: the turn budget
(slots `turn.actions`, `turn.bonus_actions`, `turn.reactions`; several commands per turn and an
`EndTurn`), the cost field on spells, items and features, the three spell fields
(`bonus_action_available`, `preparation_available`, `preparation_required_for_bonus_action`),
declared reactions with the closed trigger list and the row/stack proximity mapping (opportunity
attacks return), `Character.tactics` replacing `auto_cast` (save schema 6), the per-member
reactions switch, a Tactics page on the sheet (the tool bar has one free cell after CAMP; the pause overlay one row).
**Planned for M7c (2026-10-03, `tasks/plans/m7c-turn-budget.md`)**: the budget, the D24 fields,
declared reactions on ARCH §4.7's whole data shape (reactions only), Second Wind, Action Surge,
Cunning Action (a bonus-action exchange without opportunity attacks, and Hide), and monsters'
opportunity attacks by a built-in rule. Left for later by the owner's choices that day: the fight
screen on `bevy_ui` (stays on the canvas; the UI branch); the runbook editor, encounter criteria
and auto play (`auto` is stored, inert); opportunity attacks as monster tactics instead of the
built-in rule; Sneak Attack's damage; `Prepare` (the two preparation fields are stored and a spell
needing preparation for its bonus action cannot take one); members' "enemy flees" and "enemy
casts" triggers firing, which waits for monsters that flee and cast.
Later, timing open in PRD §14: criteria-set library and runbooks with encounter criteria, the
per-member auto flag and fully automated fights, a chooser any front end can call (the command
log still records plain commands), monster and hireling runbook collections, what preparation
costs, the budget curves (measured over seeds first, R12).
**Monster casting after Bob (M7c, 2026-10-04)**: a caster's turn is a dice roll among its weapon
and the spells its points pay for, until monster runbooks and a priority list replace it; its
Shield is a built-in rule, as opportunity attacks are. Left for later: monster heals and buffs
(validation refuses them), Thunderwave's push (rows have no distance), anything answering
`EnemyCasts` (raised at every monster cast; the trigger field makes it answerable and testable),
the party's spells aimed past the front stacks by a policy (the harness never targets Bob behind
his rats), and Bob's points in the CLI (`combat.get` shows them per individual since step 6; no MCP or CLI test reaches a fight).
**A trigger field on every action (owner, 2026-10-03, during M7c)**: each action in the data
(spells, items, class features, the weapon attack) states the triggers it can answer as a reaction,
validated against the closed list, instead of the simulation deriving them from the effect kind
(`omnis-sim/src/tactics.rs::answers`: an armor bonus answers Attacked; a heal answers Attacked,
MemberAttacked, MemberWounded, MemberDying; the weapon answers nothing until a flee source exists, B2).
Timing not set.
**Something for martial members to declare (owner, 2026-10-04, acceptance c, B2)**: in the base pack Shield
is the only reaction, so a fighter, rogue or cleric has nothing to declare. Two SRD routes, each its own
planned step: a source for `EnemyFlees` (a stack breaking and running; the SRD has no morale rule, so the
rule is ours and moves balance) with members' opportunity attacks at a fleeing stack (PRD §8.3, "Reactions
and proximity"), resolved by `combat/reaction.rs`, which today resolves only spells; and the Ready action as
a criteria set (PRD §8.3 names it), holding the action for a declared trigger.
The trigger bus below would route by these fields.

## M8 (subjective time)
**The signal bus: resolved in M8.** Built in `omnis-sim` (survey 2026-10-05: no external crate fits),
it carries region entry to reconciliation (step 3) and combat's moments to declared reactions (step 7,
the direct `on_*` calls now raise a `Cue`). On the owner's direction to expand it (2026-10-06) the
mechanism moved to its own crate, `omnis-bus` (A16), with the vocabulary left in `omnis-sim`. Three
expansions are designed in ARCHITECTURE §4.8 and built with their first consumers: a topic hierarchy
(the first topic below a region), saved deferred signals released at a contact (M10's catch-up or event
rumors), and pack-declared subscribers (the Rhai `Script` profile, mods). Bevy's own events stay the
app's side.
**A region's catch-up waits for M10.** Reconciliation computes `delta_b` and moves the region's
clock (ARCHITECTURE §4.4 step 4), but nothing yet runs on it: the ecosystem's catch-up, respawns and
project progress arrive with `omnis-eco` (M10), most likely as saved deferred signals released at
the contact (§4.8). Until then a region left for years looks as it did, apart from its clock, its
date and its rumors.
**Veto and ordering phases (owner, 2026-10-06: not designed now)**: a subscriber that cancels or
rewrites a signal before later subscribers see it (a counterspell, a ward that turns a blow), and
explicit before/after phases beyond the subscription order. Either changes delivery from "everyone
hears the same signal" to a pipeline, so it needs its own design when a rule asks for it.

## Tool proficiencies (PRD §8.1, owner 2026-09-20)
Tools as pack data (id, name, default ability, the items that count as the tool), `tools` on
backgrounds, tool proficiencies on characters, `checks::roll` taking a skill or a tool. The
Explorer keeps Perception and Survival and gains cartographer's and navigator's tools; the
"stand in" comment in `explorer.ron` goes in that commit (a `packs/` change: rebaseline).
Lands with the first system that reads a tool (Cartography's map detail, M7 at the earliest);
paper and ink as charges; fast-travel modifiers wait for sectors (Phase 2); Mining, Assaying,
Refining are Omnis tools for the Prospector.

## Objects (M8.5 step 2a)
Stored and doing nothing yet: an object's size, material, weight, light, noise, smell, value and tags. Not
built: Detect Magic's barriers (stone, metal, lead) and naming an aura's school; casting a ritual; the
guild selling Detect Magic; more inputs for an object's rules slot than `party_level` and `party_size`;
objects and piles on the automap (owner, 2026-10-14: not yet); a pile's position chosen by the party;
sight through the corner of a gap in a wall (a ray that only touches the gap's corner sees through, so a
cell behind a hedge beside a gap is seen: found at the 2a test in the yard, 2026-10-15, as recorded for
corners in 1a). No saving mid-fight is the owner's rule (2026-10-15), built in M8.5 step 7.

## Combat and magic
`Reach::AllStacks`, upcasting, counterspell (monster spellcasting came with Bob the Rat King, M7c), torches,
`Surprise::Monsters`, finesse weapons, encounter budgets (M9) and loot tables for the base monsters
(the mechanism came in M8.5 step 2a), floating damage numbers, MAP in a fight,
`turn::act` restoring `party` as well as `state` on a `RuleError`.

## Items and the sheet
Counts other than one per move (Shift for all), item details, weight limits, proficiency and
strength penalties, charges, attunement, rarity, more equipment slots, a seventh member's tab,
the Use row's reasons beyond "not here"; the sheet's attack line, death saves when down, class and
race traits pages, scrolling past the page caps, localized long names, the sheet listing every
skill, age as a rules function.

## Sensing and the automap
Objects and creatures as knowledge layers once the automap carries them, `Persistence::Expires`,
geometries other than a ray, skills and divination as sources, cover and concealment as `sense.dc`
inputs, the reach drawn on the automap, a look from a fight, `RevealMap` as a dev command.

## Modern presentation (PRD D26, owner 2026-09-20)
> Superseded (2026-10-10, A18, X15): the game's interface is `omnis-vector`'s `bevy_ui` in the neon
> style; Feathers left the workspace with `omnis-app` at P9 (2026-10-11). The editor follows: `bevy_ui`
> (owner, 2026-10-14). Kept as history.

**The Feathers experiment is done and a success by the owner's word (2026-09-20): "committing to
moving the rest of the play UI over bit by bit."** Inter; interface scale 1.1 at canvas 1×, 1.5
on the ultrawide. The report, the pattern for the next screen, the debts (name limit in bytes
against characters, the scale slider uncapped, frame time unmeasured on a visible window, the
canvas creation screen alive until the shipped build carries `bevy_ui`, a `screen.text` op over
the text tree, vendoring decided at Bevy 0.20) and the recommendations (M7's new screens born in
`bevy_ui`) are in
`tasks/plans/feathers-experiment.md`. **Owner, 2026-09-20: Feathers for the editor and all user
interface moving forward, retrofit as convenient; `bevy_egui` is dropped (its re-audit lapsed, its
workspace pin went in M7 step 1); `tasks/plans/editor-v1.md` is re-read against Feathers when M5
returns.** What follows is the record from before it.
Pixel art is placeholder only. The Feathers-in-game experiment (TODO, M6 closeout block) came
first; then, by its result: modern fonts everywhere, Bevy UI widgets for the screens the canvas
toolkit cannot build (the tactics screen's dropdowns, number inputs, lists, scrolling), the
editor's toolkit reconsidered against `bevy_egui`, smooth instead of integer scaling, and the
open PRD §14 question of what modern means for the viewport (higher-resolution 2D, or reopening
3D) and who makes the art.
Facts already read from the Bevy 0.19.1 sources (2026-09-20), so the plan need not re-derive
them: the Bevy feature is `bevy_feathers` and it pulls in `bevy_ui_widgets`; both are part of
Bevy (no third-party crates; `bevy_egui` adds 17 and two duplicates) and both call themselves
experimental; Feathers' own docs aim it at editors and inspectors ("deliberately not intended"
for game UI; "consider copying this code into your own project") while `bevy_ui_widgets` is the
unstyled layer meant for a game's own look; Feathers controls: button, checkbox, radio, slider,
toggle switch, text input, number input, list view, menu, scrollbar, colour plane/slider/swatch,
disclosure toggle, virtual keyboard, with a dark theme, tokens and font styles. The app's `ui`
feature is off since M3b (the tree went 328 to 323 crates). What the canvas gives today and any
replacement must keep: the MCP `screenshot` and `dump_screens` show the interface (the M3 note:
window-space `bevy_ui` did not appear in canvas captures), headless tests drive every widget
through `WidgetId` hit-tests with no GPU, every action has a button before a key. The owner's
wording for the experiment: "Feathers in game. Start experimenting with feathers in the game.
See how the default styles work, and see if modern fonts can replace our pixel art based fonts."

## App and tooling
The movement pad (arrows and USE) on Feathers like the tool bar (M7 step 8a moved the tools
only; the canvas pad is the right column's last canvas widget); a fourth pad row; `texel_scale 1`; a scrollable log; `Line::short` deletion; raw `sim:message:*` keys and
localized event text; CC0 art; `--no-devtools` for a clean save from a dev
build; `.omnis/mcp.log` rotation; `SetRule`, `TickEco`, `SpawnEncounter` as dev commands; the
measurement policy as something other than a fixed heuristic; a rules function for age; a longer
name in wide scripts (the rules' `NAME_MAX_BYTES` is 32, so the form keeps 16 two-byte or 10
three-byte letters of the input's 24 characters; 96 bytes would hold 24 of any script, a rules
change for the owner to call).

## The editor (owner, 2026-10-14)
Chosen: `bevy_ui`, a top-down plan with a 3D preview, maps plus a RON text editor, a `notes` text, objects
defined first in M8.5 step 2. Not chosen for v1, each waiting for the editor's later steps or a content need:
- editing in the 3D view (picking cells and edges in first person or an orbit camera);
- form-based editors for the data tables (PRD §10's data table view);
- a map writer that keeps `//` comments in place (the `notes` text stands in);
- the editor before objects, authoring them from the start (objects are hand-written RON until the editor).

## Later phases
The editor (M5, after M8.5 step 2; the plan is written then), GFDL backgrounds as their own pack,
procedural generation (M9), the ecosystem (M10), the story engine (M11), packaging (M12).
