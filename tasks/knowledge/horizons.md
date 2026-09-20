# Horizons

Everything deferred so far, with the milestone it waits for. A horizon is not a rejection
(`agreements.md`): it stays here until it is built or the owner rejects it outright.

## Dated re-audits
- 2026-10-08: `bevy_egui` 0.42.0 clears the 30-day rule (the unused `=0.41.1` pin stays for the editor).
- 2026-10-10: Socket re-audit of `rhai` 1.26.1.

## M7 (town, services, rest, progression)
Rest (pools only empty until then; the debug menu refills them); the temple for the `dead`
condition; shops (the acolyte's potion leaves the kit when shops exist); a blacksmith; `Relief`
items (`world.rs` still says M6); string item ids before many items arrive (ids are interned
`u32`s guarded by the pack fingerprint); `doff_armor_minutes` as its own value.

## Combat and magic
`Reach::AllStacks`, upcasting, monster spellcasting, torches, a counter for one reaction per round
(the effect enforces it today), `Surprise::Monsters`, finesse weapons, loot and encounter budgets
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

## App and tooling
A seventh tool button needs a wider right column or a menu; word labels on the pad; a fourth pad
row; `texel_scale 1`; a scrollable log; `Line::short` deletion; raw `sim:message:*` keys and
localized event text; the `screen.text` op; CC0 art; `--no-devtools` for a clean save from a dev
build; `.omnis/mcp.log` rotation; `SetRule`, `TickEco`, `SpawnEncounter` as dev commands; the
measurement policy as something other than a fixed heuristic; a rules function for age.

## Later phases
The editor (M5, deferred; `tasks/plans/editor-v1.md`), GFDL backgrounds as their own pack,
procedural generation (M9), the ecosystem (M10), the story engine (M11), packaging (M12).
