# Horizons

Everything deferred so far, with the milestone it waits for. A horizon is not a rejection
(`agreements.md`): it stays here until it is built or the owner rejects it outright.

## Dated re-audits
- 2026-10-08: `bevy_egui` 0.42.0 clears the 30-day rule (the unused `=0.41.1` pin stays for the editor).
- 2026-10-10: Socket re-audit of `rhai` 1.26.1.

## M7 (town, services, rest, progression)
Rest (pools only empty until then; the debug menu refills them); the temple for the `dead`
condition; shops (the acolyte's potion leaves the kit when shops exist); a blacksmith; `Relief`
items; string item ids before many items arrive (ids are interned
`u32`s guarded by the pack fingerprint); `doff_armor_minutes` as its own value.

## Turn budget and tactics (PRD v0.5: D21–D24, §7.9; approved 2026-09-20)
Nothing is built yet; M6 has one action per turn and shield as the only (automatic) reaction.
In or beside M7, because Second Wind and Cunning Action need bonus actions: the turn budget
(slots `turn.actions`, `turn.bonus_actions`, `turn.reactions`; several commands per turn and an
`EndTurn`), the cost field on spells, items and features, the three spell fields
(`bonus_action_available`, `preparation_available`, `preparation_required_for_bonus_action`),
declared reactions with the closed trigger list and the row/stack proximity mapping (opportunity
attacks return), `Character.tactics` replacing `auto_cast` (save schema 5), the per-member
reactions switch, a Tactics page on the sheet (the tool pad and pause overlay are full).
Later, timing open in PRD §14: criteria-set library and runbooks with encounter criteria, the
per-member auto flag and fully automated fights, a chooser any front end can call (the command
log still records plain commands), monster and hireling runbook collections, what preparation
costs, the budget curves (measured over seeds first, R12).

## Tool proficiencies (PRD §8.1, owner 2026-09-20)
Tools as pack data (id, name, default ability, the items that count as the tool), `tools` on
backgrounds, tool proficiencies on characters, `checks::roll` taking a skill or a tool. The
Explorer keeps Perception and Survival and gains cartographer's and navigator's tools; the
"stand in" comment in `explorer.ron` goes in that commit (a `packs/` change: rebaseline).
Lands with the first system that reads a tool (Cartography's map detail, M7 at the earliest);
paper and ink as charges; fast-travel modifiers wait for sectors (Phase 2); Mining, Assaying,
Refining are Omnis tools for the Prospector.

## Combat and magic
`Reach::AllStacks`, upcasting, monster spellcasting (counterspell waits for it), torches,
`Surprise::Monsters`, finesse weapons, loot and encounter budgets
(M9), floating damage numbers, MAP in a fight, saving from the pause overlay mid-fight,
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
Pixel art is placeholder only. The Feathers-in-game experiment (TODO, M6 closeout block) comes
first; then, by its result: modern fonts everywhere, Bevy UI widgets for the screens the canvas
toolkit cannot build (the tactics screen's dropdowns, number inputs, lists, scrolling), the
editor's toolkit reconsidered against `bevy_egui`, smooth instead of integer scaling, and the
open PRD §14 question of what modern means for the viewport (higher-resolution 2D, or reopening
3D) and who makes the art.

## App and tooling
A seventh tool button needs a wider right column or a menu; word labels on the pad; a fourth pad
row; `texel_scale 1`; a scrollable log; `Line::short` deletion; raw `sim:message:*` keys and
localized event text; the `screen.text` op; CC0 art; `--no-devtools` for a clean save from a dev
build; `.omnis/mcp.log` rotation; `SetRule`, `TickEco`, `SpawnEncounter` as dev commands; the
measurement policy as something other than a fixed heuristic; a rules function for age.

## Later phases
The editor (M5, deferred; `tasks/plans/editor-v1.md`), GFDL backgrounds as their own pack,
procedural generation (M9), the ecosystem (M10), the story engine (M11), packaging (M12).
