# Continuity notes

**2026-11-24.** The owner tried step 9 (doors: "do break with ranged and melee attacks") and the thunderwave
push ("does push"), and asked for four things, planned in plan mode and approved
(`tasks/plans/order-look-push-collapse.md`; the TODO item of that name in W2). **Steps 1 to 3 are built,
committed and tried by the owner** ("marching order, push spell and mouse look focus appears to be working as
described"): `f14d3b9f` (the look comes back: `movement::Resume`, the rule `look`, `Screens.resets`),
`53098e18` (`screen/order.rs`, the column's Order... button, the engine's `Reorder`), `f784b6f6` (Push,
base 0.1.6, `monster_front_stacks` gone, `SPELLS_SINCE_NAMES`), then at the owner's word `db29f69c` (Push
hurts: `Spell.dice_every_levels`, `Spell::dice_at`, 1d4 for every two caster levels rounded up, base 0.1.7,
hash 520674701000266426; **not yet tried by the owner**). Gate 1510 passed, 17 ignored. Pins: 17 commands,
82 events, 77 rejections, 20 base spells, 28 rule values, 69 rules in the editor's list, `SAVE_SCHEMA` 9.
**Next: step 4, doorways that fall in** (no code yet; the plan has the design). Read while planning:
a collapsed door keeps `broken: true` and gains `collapsed: true`, so every "broken" check already refuses
it and only `MapState::door_open` (world.rs) must answer false first; the roll goes in `breaking::strike`'s
door arm inside `if broken`, on the roller in hand; `apply::landing` needs `BlockReason::Rubble` before its
closed-door answer; `rubble_clear_minutes` goes in base's `objects.ron` (a reseal, 29 rule values and 70
rules again); existing door tests that walk through a broken door must set `collapse_percent` 0 first.
Then step 5 (the line art in `geometry.rs::edge`, `fill.rs`, the minimap's marks), then combat in feet's
steps 10 to 13. The scratchpad was emptied between sessions: the mutation runner is now
`<scratchpad>/mut.py <spec.py>` (spec: `CMD`, `MUTANTS = [(file, name, old, new)]`), rewritten 2026-11-24;
write it again if it is gone. The owner asked mid-run whether they could build: a gate or a mutation run
holds the build lock, so say when the tree is still. Open with the owner: a party-chosen front row size;
stacks rejoining; "blocks movement" implying "blocks sight"; aiming by the faced view; per-door make.

**2026-11-16, later.** The owner: the gate fix "seems to have fixed the breaking an object on the other side
of the wall", and "thunderwave does not seem to push back foes?". Planned in plan mode (choices: split the
stack; a monster's push drives the line back) and built, awaiting the owner's try: see TODO "A spell that
pushes" and the code-map's entry. 82 events, base 0.1.5 (hash 16768748016642705028), replays rebaselined for
the hash alone, `SAVE_SCHEMA` 9. A cast in a fight ends the caster's turn, so tests judge a cast by the
events before the next `Turn`. Open with the owner: stacks rejoining; whether "blocks movement" should imply
"blocks sight" for object kinds; whether aiming needs the thing in the faced view. Step 9 (doors) still
awaits the owner's try. **Next: step 10 (repair), plan mode.**

**2026-11-16.** The owner tried the redraw fix ("crates do react immediately upon destruction") and reported
"I can destroy crates on the other side of walls". Cause: an object's sight was its cell's
(`line_of_sight`, true when any corner of the cell shows). Built, awaiting the owner's try:
`visibility::object_in_sight` (lines to the object's footprint within its cell, the walk given a unit so it
runs in sixteenths), used by `targets/map.rs::object`; red test
`breaking::a_thing_behind_a_wall_is_not_seen_by_a_corner_of_its_cell` (from (5, 3) of the yard the crate is
`Obscured`). Doors and groups are still judged by cell. Sight is still all round, not only the way faced
(said to the owner as a question). The owner tried it: "I can still destroy the crate on the other side of
the wall and behind the door". Taken as the gate in the hedge's gap, which did not block sight: fixture
`objects` 0.1.10 (hash 9169258786753955413), the gate blocks sight while shut, not when open; two tests that
saw through the shut gate now open it first. Awaiting the owner's try. Step 9 (doors) still awaits the owner's try. **Next: step 10, plan mode.**

**2026-11-15, evening.** The owner tried the object targeting fix ("Object targeting and damage tracking is
working") and the Resize form ("worked but was strange": one field writable, the cursor lost on a click, no Tab
or arrows). Built since, awaiting the owner's try: `540aee75` (gate 1473), `shell/focus.rs`: every text field
carries a `TabIndex` (Bevy 0.20 gives a click's focus to nothing else), a screen's first field alone takes the
keyboard, a field's box passes a click to its field, Tab / Shift+Tab / Up / Down step fields (the screen root a
modal `TabGroup`, the editor's columns groups 0 to 3; Bevy's own Tab handler left out). Then, at the owner's
word ("a visual representation of a destroyed object"), part of step 12 pulled forward: a broken object is a
wreck in the 3D view (`shapes::WRECK`, `Out::wreck`). The owner tried both: Resize "now works as expected"; the wreck "works, but it lagged until I changed my
view", fixed by `ObjectBroken` in `session::reshapes` (awaiting the owner's try; step 9's `DoorBroken` and
`DoorDamaged` go there too if they change the picture). Not seen on screen by me. **Step 9 (doors) is built, awaiting the
owner's try** (plan approved in plan mode; TODO's step 9 line has what exists): 80 events, 77 rejections,
no pack change. Said to the owner: every door is breakable by default (18 HP of wood), so Attack is lit
wherever a door is in sight; a map opts out with `door_make: (durability: None)`, map-wide. **Next: step 10
(repair: mending, tools, `Command::Repair`; remove `monster_front_stacks` from base at that reseal), plan
mode first.**

**2026-11-15, afternoon. Combat in feet resumes at step 9 (doors).** The owner tried Part I ("Tested ranged advance
and combat, seems to work well") and the rebuilt minimap ("That fixed the minimap usability"): `0b78b7ef`, one
192-pixel square centred on the party, `minimap::Zoom` over `ZOOMS` (12 px a cell first), `Zooming` resource,
two `ZoomButton`s on the picture; lesson recorded (a "squished" report is about what the view is for). Then step 8,
`52655b5c` (gate 1470): `breaking.rs`, `ObjectState.damage`/`broken`, `objects::standing`, Cast and Attack at
`Target::Object`, `ObjectDamaged`/`ObjectBroken` (78 events), `CannotBeDamaged`/`Broken` (77 rejections), fixture
`objects` 0.1.8 (hash 9841898359379817104; crate and chest durable). Departures are in the plan's "As built".
For step 9: `MapState.door_damage`/`broken_doors` and a new journal `Change`, `MapState::door_open` the one place a
broken door counts as open, `DoorMake::numbers()`, `breaking::strike` generalised from `Thing` (an object) to a door,
`targets::durable` lists doors (a test in `breaking.rs` pins that it does not yet), `EdgeView::Door` gains damage and
broken, events `DoorDamaged`/`DoorBroken`. The client already lists and strikes durable objects through `targets`;
marks wait for step 12. **Then the owner's report** ("lists the objects ... but I can't seem to target the objects
with spells or weapons"), fixed in `991831fd` (gate 1471): `targeting::Aim::command` builds `Attack` for an
`Object` too, and `shell/text.rs::objects` words `ObjectDamaged`/`ObjectBroken` ("X hits Crate (17 against AC 15):
6 fire, 12 HP left", "Crate breaks"). **The owner is trying that fix now** (said 2026-11-15, before the compact);
wait for the result before step 9. Offered and unanswered: pull step 12's damaged and broken marks forward (a broken
crate is still drawn whole). Lesson: an engine row the client lists must work in the client in the same commit.
Nothing is pushed. Still open with the owner: the Resize form's try (`tasks/acceptance/editor-resize.md`),
the six decisions in `combat-in-feet-part-1.md`, the minimap's start zoom, unsaved zoom and button place.

**Built 2026-11-15, not yet tried by the owner: "Resize a map from the editor, on all four sides"** (`tasks/plans/editor-resize.md`,
approved in plan mode; the TODO item of that name has the steps; the owner's sheet `tasks/acceptance/editor-resize.md`).
`Edit::Extend`, `Document::losses`, notes following cells at Save, the modal `EditorResize` with its Yes / No warning;
the QA hall is inside the yard (objects 0.1.7; plain `just qa`, east through the gate at (6, 2)); a yard test that
wants the old seven-by-five yard cuts the hall off (`edit_extend.rs::yard`). The owner tried the QA hall ("advance works")
and found opening it by name odd, which led here. Combat in feet resumes at step 8 after it.

Written 2026-11-14, night. **In progress: "Combat in feet, and things that break"** (`tasks/plans/combat-in-feet.md`,
approved in plan mode 2026-11-14; the TODO item of that name in W2 has the fourteen steps ticked as they land).
Targeting is accepted by the owner (same day).

- **The owner's decisions** (asked, do not re-ask): one distance per stack, no grid, no cover; combat on the
  SRD's 5 ft square ("Combat should try to stay consistent with the SRD rules"; I had recommended 15 ft tiles
  and was corrected, lesson in `tasks/LESSONS.md`); maps outside combat will need other scales, so a map states
  `tile_feet`; a per-type number that fit a square (1 human, 2 kobolds, 3 rats) that limits melee attackers,
  sizes area spells and caps a group on a tile; opening strike rolls surprise (Stealth against passive
  Perception); objects **and doors** breakable (SRD AC by material, HP by size), terrain not; a broken
  container spills and fragile items can be lost; repair by mending, by tools with time and supplies, and by a
  town service.
- **Done and committed (not pushed)**: `acce0ed7` (0, documents), `cbf253e9` (1, data; base resealed 0.1.4,
  goldens rebaselined), `279296fc` (2, `Stack.distance`), `f32384ed` (3, monsters in feet), `3172e9ed`
  (**6, built ahead of 4 and 5 at the owner's word**: the strike's group Stealth roll `combat::unseen`,
  advantage through `state.hidden`, `Command::Attack { member, target }` in `assault.rs`, `combat::Opening`,
  `targets` rows for `Attack` on the road; 17 commands; `schema_proof.rs` 144 instances, 204 branches),
  `7c31d458` (the fight menu's Attack and Cast give the shared reason, from the owner's playtest). Gate 1423.
  Then `5308dab4` (4, members in feet: `strike_for`, `Advance`, `PartyAdvanced`, `AlreadyEngaged`,
  `CombatState.advanced`/`room`), `d7ee3a8f` (5, areas by squares), and step 7 (client and the tile cap).
  **Part I (steps 0 to 7) is built, not yet tried by the owner**: `tasks/acceptance/combat-in-feet-part-1.md`
  ends with six decisions (Advance free within the pace; bow before charge; an engaged stack stops the
  line; ammunition; balance; no editor field for `tile_feet`). Pins now: 17 commands, 76 events, 75
  rejections, `schema_proof.rs` 145 instances and 205 branches. The owner tried part I and left QA notes in
  the sheet (answered there; fixed: Attack on a member's entry in the party column, a stack's feet always
  shown; `33e87a5e`). Then, at the owner's word: `objects:map:hall` in the objects fixture (0.1.6, a new
  monster `shambler`; `just qa objects:map:hall`) and **every fixed encounter is seen**
  (`ResolvedEncounter::stood`; a client test that wants no one in sight uses `common::clear_groups`).
  **Next: the owner's try of the hall, then step 8, objects take damage.**
- **Found in step 1, fixed there**: new base items renumber every item; a save older than name tables is read
  on the table without them (`NameTable::before_names`, the list `ITEMS_SINCE_NAMES` in `save/names.rs`;
  **any definition added to a shipped pack goes in that list in the same change**), before its migrations.
- **Departures from the plan's wording, all said in commits**: the mending spell file and the fixture's
  durable kinds wait for steps 10 and 8; step 2 kept "front stacks" and step 3 replaced it with engaged;
  `monster_front_stacks` is still written in base's `combat.ron` and asserted in `load_base_pack.rs` (remove
  both at the next base reseal, step 10); monster spells' own ranges moved from step 3 to step 4.
- **Step 4's design as it was planned (built since; the plan file's "As built" lists the departures):**
  - `omnis_rules::Weapon` gains `reach` and `range` from `Item::span()` (unarmed 5).
  - `CombatState` gains `advanced: u16` and `dashes: u8` (serde default, reset in `turn::end_of_round`). The
    line's allowance a round is the slowest standing front-row member's race speed x (1 + dashes).
  - `combat::weapon_for` decides in this order: melee in reach without moving (the back row stands 5 ft
    further, so it needs a 10 ft weapon for an engaged stack); else a ranged weapon within its long range
    (disadvantage past normal, or for a front-row member with any stack engaged); else, for a front-row
    member, melee with an automatic advance if the allowance covers the gap; else `OutOfReach`,
    `BeyondRange` or `NeedsRangedWeapon`. `Plan::Attack` carries the advance and the hindrance.
  - Advancing by d takes d off every stack (never under 5): `Event::PartyAdvanced { feet }` (76 events).
  - `CombatCommand::Advance`: an action (the Dash), `dashes += 1`, then the line moves as far as the allowance
    goes toward engaging the nearest stack; a new rejection when every living stack is engaged.
  - Spells in a fight (`targets::in_range`): Touch is a reach of 5 with the same automatic advance (so
    `CastPlan` carries an advance; a back-row caster cannot touch); `Tiles(n)` is n x the map's `tile_feet`
    against the stack's distance, else `BeyondRange`; Sight anything. A member's ranged spell attack is
    hindered when engaged, as the monsters' is (`resolve::hindered`).
  - Monster casters: filter the spells offered in `monster_cast::act` by the same ranges.
  - `TargetView.distance` in a fight becomes the stack's feet (the road's is tiles; say so in `docs/api.md`).
    `CombatView` gains what is left of the advance.
  - Tests go in `crates/omnis-sim/tests/feet.rs` (helper `play` builds a fight with distances; `common::stand_back`
    places a stack). Expect the golden fight to move here, and several fight tests that attack a stack 15 ft off.
- **Then**: 5 area by squares (`cast_save` and `monster_cast` treat `Reach::Area` as the whole stack today, with
  a comment); 6 opening strike (group Stealth check, advantage, road `Command::Attack`); 7 client (distance by
  each stack, Advance button, road Attack, `rolllog` line for `StackMoved`, the tile cap in loader and editor),
  then report "built, please try"; 8 to 13 things that break (see the plan).
- **Said plainly**: no balance pass; fights are harder to read without the client showing distance (step 7);
  nothing seen on screen by me; opportunity attacks still come only from engaged stacks, not from a 10 ft reach.
- **Mutation runner**: `mut-sim.py <spec.py>` in the scratchpad; specs `mut-feet2.py`, `mut-feet3.py` show the form.

The notes below were written before this plan.

**Calmer fight colours and one targeting flow: all nine steps built and committed (not pushed)** (`tasks/plans/targeting.md`; the owner's sheet
`tasks/acceptance/targeting.md`, which ends with eight decisions; the TODO item of that name in W2 has each
step's notes and the review).
- **The owner's rule**: open menu, check requirements, show options, select option, select from possible
  targets, validate, pay, execute; kinds self, touch, range N, line of sight, link. Lesson in
  `tasks/LESSONS.md` (2026-11-14).
- **Commits**: `9b82129c` (0, documents), `bdf4f387` (1, colours), `e1aa1d13` (2, data), `4b715dd6` (3, the
  list), `b02628b1` (4, road step), `053909ef` (5, fight menu), `478f68cc` (6, doors and objects, mage hand
  as a hand, the 3D mark), `c156a83b` (7, the opening strike), `f27f2408` (8, links), then step 9's
  documents. Gate 1387.
- **Owner's try so far** (steps 1, 4, 5): "Manually tested, looking good so far. Combat colors are less
  straining on the eyes. 'Option --> select' seems to work as described." Not a final acceptance.
- **What changed in play since that try**: mage hand takes a chosen door or object (no longer the first door
  ahead); harmful spells are on the road's Spells list and, sent at a group in sight, start the fight with
  the monsters surprised and the caster's first action spent; links exist only through a dev command.
- **Where things are**: the code-map's last section, "Targeting".
- **Said plainly, still true**: nothing seen on screen by me (the dev socket did not answer; try
  `mcp__omnis__screenshot` when the owner has the game up); a dim row's outline is untested; light and
  detect magic accept any member; Give takes the column's selection; monsters' casting does not read
  targeting; a link names a thing by its name in the party's map; `Target` is not `Copy`.
- **Open with the owner**: this sheet's eight decisions; the fight-on-the-palette sheet's five; part 3's
  five (`tasks/acceptance/rendering-size-2-palette.md`); their try of loot everywhere.
- **Next, in the owner's order**: the combat plan above (the owner raised it last), or the HUD's plan or
  the minimap's, then parts 4 to 6 of rendering size 2. Ask which first.
- **Process notes**: the mutation runner is `mut-sim.py <spec>` in the scratchpad (integration tests only;
  a mutant that does not compile proves nothing); MCP pinned counts live in `tests/api_doc.rs`,
  `tests/schema_proof.rs` (136 commands, 194 branches) and `src/schema.rs`'s unit test.

**Before that** (written 2026-11-13, evening). **The fight screen and the picture window are on the palette: built, not
yet tried by the owner** (`tasks/plans/fight-on-the-palette.md`, approved in plan mode;
`tasks/acceptance/fight-on-the-palette.md`, which ends with five decisions). Commits `64e61897` (plan),
`76b3623a` (arena), `90b67da7` (picture window), `06b96af3` (theme), then the documents. Gate 1352.
- **Owner's choices**: scope is the fight screen and the picture window; party ink and hostile ink by
  side; the fight's interface follows the biosphere's interface ink, other screens green until the HUD's
  plan.
- **Built**: `tones.rs` (`Tone` with `Side`, `ink`), `cinema::Inks`, `style::Theme`, `shell/theme.rs`
  (`Ink`, `Edge`, `Fill` painted by `apply`; `follow` gives the fight's map's theme in a fight only). See
  the code-map's last section. `controls::GREEN`/`SHADES` are gone.
- **Open with the owner**: this sheet's five decisions (amber and cyan gone; dead not red; party and
  interface one hue by default; HUD or minimap next; icons); part 3's five; loot everywhere's try.
- **Next, in the owner's order**: the HUD (lift `follow`'s fight-only rule, icons, the cursor) or the
  minimap, each its own plan; then parts 4, 5, 6.
- **Process notes**: the fixture fight on a biosphere map is `common::app::met_in_the_yard`; a windowed
  game started from here did not answer its dev socket (no picture); the mutation runner only runs
  integration tests, so a mutant must be caught there.

**Before that, the same day**: the editor's forms and tips, accepted. **The owner tried part 3 and reported two things; both are built
and accepted (2026-11-13: "Label brightness is fine and the wording is pretty good for what I reviewed.
Consider this accepted."; the keyboard's tip, list rows and other screens were not answered and stay as
built)** (`tasks/plans/editor-forms-and-tips.md`, approved in plan mode; the owner's script
`tasks/acceptance/editor-forms-and-tips.md`, which ends with six decisions).
- **The report**: "no hatching on the cliffs in the meadow" were the trees terrain's *blocks*, which part 3
  made the walls' colour (the meadow has no heights). Lesson recorded: a look change's report walks the
  mapping arm by arm and says what the owner's own maps will show.
- **Built**: `Terrain.face` (`Plain`/`Hatched`; the owner sets it on their trees, their pack is untouched);
  the editor's form in two columns (`Row::field`, `Kind::Flag` tick box, group headings); the kit's
  tooltips (`Control.tip`, `shell/tips.rs` on Bevy's `Popover`); tip text for the whole editor
  (`editor/tips.rs`, listed at the plan's foot). Fixture `heights` is 0.1.2. See the code-map's last section.
- **Next**: still open with the owner: part 3's five decisions and loot everywhere's try; then,
  in the owner's order, the fight screen, minimap and HUD on the palette, then parts 4, 5, 6.
- **Process notes**: the plain headless test app has no layout and no `InputFocus`; `ui_editor_app` has a
  layout. An editor capture with a form open needs a temporary probe in `shell/editor.rs::route` (the
  capture flags cannot press a control); `--editor` refuses `--seed`. An unquoted heredoc eats backticks
  and backslash-newlines: write Rust text through a quoted heredoc or a file. If a mutation run is
  interrupted, grep the mutated strings before anything else.

The notes below were written earlier on 2026-11-13 and still hold, except "next".

Written 2026-11-13, after rendering size 2 part 3. **Part 3 (colour roles as a palette, with biospheres in
pack data) is built, not yet tried by the owner** (`tasks/plans/rendering-size-2-palette.md`, approved
2026-11-13; the owner's script `tasks/acceptance/rendering-size-2-palette.md`, which ends with five decisions
asked). The owner's order stands: parts 3, 4, 5, 6, each its own plan; X23 is applied to `presentation-PRD.md`.
- **What exists**: `omnis-data/src/biosphere.rs` (`ColourRole` nine roles, `Ink`, `BiosphereDef`, the sixteenth
  kind, held by id); `Terrain.role`, `RegionDef.biosphere`, `MapDef.biosphere`; `Data::biosphere_of` /
  `biosphere_for`; `style::Palette` (sixteen slots, a built-in default); `strokes::{Look, role, strength}`;
  `LineMaterial.palette` (binding 1) read by `lines.wesl`; `contrast.rs` (`clashes`, `APART` 0.1, `FROM_DARK`
  0.45, three simulated deficiencies). Fixtures: `objects` 0.1.5 (ember), `levels` 0.1.2 (frost, failing the
  checks on purpose; deep). Gate 1326. Commits `7fd9a30f`, `921231c4`, `1e8dea03`, `f6684eb7`, `a637c28e` and
  the documents' commit after them.
- **The look changed on every map**: floors do not take their terrain's colour; cliffs are the walls' colour;
  piles the objects'; the default's walls lean to teal and its doors to yellow, because the old hues failed
  the colour-deficiency check (measured: 0.048 against 0.1). These are in the acceptance sheet as decisions.
- **Next, in the owner's order**: (a) their try and the five decisions; (b) the plan the owner chose to follow
  part 3: the fight screen (`shell/combat.rs` `colour(Tone)`), the minimap (`minimap.rs` constants), the
  picture window (`cinema.rs`) and the interface (`style::Role`, `shell/controls.rs`) on the palette, VA11;
  (c) part 4, depth levels (`style.w`; the neighbour dim 0.2 rides in the vertex colour's alpha; X17 has no
  numbers, so they are asked); (d) part 5; (e) part 6.
- **How a palette is tuned**: the scratchpad's `contrast.py` (the arithmetic of `contrast.rs` in Python) with
  `c3.py` (a hill-climb from a starting palette to the nearest that passes). Not in the tree.
- **Process notes**: the uniform array binds as `[Vec4; 16]` on `AsBindGroup` with no trouble; the editor
  writes the last pack loaded, so an editor test on `test:map:town` must not add a fixture pack after it; a
  struct literal fixed by a script got `role: None` in the terrain form, which would have erased roles: read
  what a mechanical fix did to each site. Captures: `target/shots/p3/` (four maps, Low and High).

The notes below were written earlier on 2026-11-13 and still hold, except "next" and "not applied".

Written 2026-11-13, night, before a compact. **The owner's order: after the catchup, rendering size 2 parts
3 to 6** ("let's prepare for a compact and jump into rendering size 2 parts 3 to 6 after the catchup").

**Start there**: `tasks/plans/rendering-size-2.md` (the design: six parts; 1 and 2 built). Each part is its
own plan-mode step. What each needs before code:
- **Part 3, colour roles as a palette** (`style.z` a role index, the palette as material data): needs the
  proposed presentation-PRD amendment **X23** at the end of that file, **not applied; ask the owner to approve
  its text first**. It is M8.5 step 5's rendering half; the role set and where a biosphere is assigned are
  the owner's open questions (TODO "M8.5 step 5"). Today's colours live in `strokes::colour` by `SegKind`.
- **Part 4, depth levels** (`style.w`; the neighbour dim 0.2 is baked into the colour's alpha today): no
  amendment; the three levels' looks have no numbers yet (X17), so they are asked.
- **Part 5, instanced objects, piles and beings**: no amendment; the largest code move (`shapes.rs` to
  per-kind meshes); `aim.rs` keeps using `Body`; the aimed floor's outline (`ViewAim`) is a third line mesh.
- **Part 6, relief as displacement**: needs X23 and a CPU twin for the collision mirror; the riskiest, last.
- My recommended order in that file: 2 to 4 with M8.5 step 5, then 5, hold 6. The owner has now asked for
  3 to 6; begin by asking X23's approval and the order, in plan mode.
- Shader facts and the capture method: the 2026-11-12 notes below and `tasks/knowledge/verification.md`.
  The gate cannot see a shader. A look is judged at Low first; a channel on a shape leaves the shape whole
  (lessons 2026-11-13). `just qa [map]` is the owner's window on the fixtures. To capture on a chosen cell,
  a scratch copy of a fixture pack with the map's `start` moved, sealed with `omnis-cli pack seal`
  (`--walk` does not step the party).

**Built and tried today (2026-11-13), all committed on `W-2_look_and_engine`, nothing pushed, gate 1312**:
- Line weight and pattern (size 2 part 2): the owner "Manually tested", no fault named.
- Pile reach (`tasks/plans/pile-reach.md`): the cell's floor as the click target, its outline
  (`render::ViewAim`), the "Pick up..." button. The owner: "Better."
- Found items (`tasks/plans/found-items.md`): the engine's `GiveItem` lays a pile (`ground::found`), said by
  `ItemMoved` from the object to `Ground`; `Event::ItemFound` is no longer produced but stays in the enum
  (`PROTOCOL` 2, no pin moved). The owner: "Much better."
- Loot everywhere (`tasks/plans/loot-everywhere.md`, **not yet tried by the owner**;
  `tasks/acceptance/loot-everywhere.md`): `shell/loot.rs` holds every opener of the loot window;
  `Session.offer: Option<ItemPlace>` set by `session::finds` (an object's gift, a victory that left items)
  and `session::opens` (a container opened with something in it); Actions' "Take from <name>...".
- **Open with the owner**: gold never goes through the window; a dead member's kit goes to the stores
  unseen; the object's message on the loot window; whether the window after a fight should wait for a key;
  the earlier items below ("accepted" for Bevy 0.20 and `controls::Control`; `editor/mod.rs` 1037 lines;
  Bevy 0.20.1 on 2026-11-27; the four ECS tasks).
- Process kept today: tests red first where the plan said so (pile reach's were not, and said so); a
  surviving mutant means a missing test, not a weak mutant (the container offer); scratchpad scripts
  `mut-sim.py` (crate, filter; touches after restore) and `mut-reach.py`.

The notes below were written earlier on 2026-11-13 and still hold, except "not yet tried" and "next".

Written 2026-11-13, later still. **Found items are built, not yet tried by the owner**
(`tasks/plans/found-items.md`, approved 2026-11-13; `tasks/acceptance/found-items.md`). The owner tried the
pile reach ("Better.") and reported that the stone hiding a gem opened no loot picker; chose "Drop it, open
the loot window". Engine: the object effect `GiveItem` lays the item as a pile (`ground::found`,
`found_at`: the object's cell where the party reaches it, else the party's) and emits `ItemMoved` from the
object to `Ground`; **`Event::ItemFound` is no longer produced but stays in the enum** (no pin moves,
`PROTOCOL` 2). Client: `Session.found` (`session::finds`), `shell/aim.rs::found` opens the loot window;
log "Found gem; on the ground". Tests: sim `object_act` (two new), vector `tests/found.rs` (three). Not
tested: the window over the verbs screen (no fixture object with several verbs gives an item); a full
pile. Open to ask: the object's message on the loot window; gold as a pile; fight spoils and services
were not reviewed against "everything gained goes through the window" (lesson 2026-11-13).
**Next: the owner's try, then their decision on size 2 parts 3 to 6.**

The notes below were written earlier on 2026-11-13 and still hold, except "not yet tried" and "next".

Written 2026-11-13, later. **A ground pile that is easy to get to is built, not yet tried by the owner**
(`tasks/plans/pile-reach.md`, approved 2026-11-13; the owner's script `tasks/acceptance/pile-reach.md`), from
the owner's hand test ("Click target too small", "Underfoot pile unseen"): the floor of a pile's cell is its
click target (`aim::floor`, `inside`, `on_heap`); `Aimed` is the aimed pile's cell and `render::ViewAim` its
floor's outline (`lines::aim_mark`, `lines::hang` shared with the crests); a "Pick up..." button
(`shell/aim.rs` `PickUp`) stands on the view while the ground in reach holds anything. Tests in
`tests/pile_reach.rs` (six). Gate 1301. **The owner said "Manually tested" of patterns and weight and
`just qa` (2026-11-13), no fault named.** Captures on a pile's own cell: a scratch copy of the objects pack
with the yard's `start` moved, sealed with `omnis-cli pack seal` (`--walk` does not step the party).
**Next: the owner's try, then their decision on size 2 parts 3 to 6.**

The notes below were written earlier on 2026-11-13 and still hold, except "not yet tried" and "next".

Written 2026-11-13. **Rendering size 2, part 2 (line weight and pattern) is built, not yet tried by the owner**
(`tasks/plans/rendering-size-2-patterns.md` with an "As built" section; the owner's script
`tasks/acceptance/rendering-size-2-patterns.md`). `style::{Pattern, Weight}`, `strokes::channels(SegKind)` (a
provisional table until M8.5 step 5: grids hairline; objects and piles heavy dashed; beings heavy double; portals
dotted), drawn in `lines.wesl`: dashes 1/8 cell measured in the world and fitted to each segment, **a lighter
line between dashes, never a void** (the owner's correction; a third of the width, 0.4 of the brightness),
double's middle third open on lines of 3 pixels (High only). The owner reviewed part 1 (feathering): "looks good
so far". **`just qa [map]`** starts a windowed game on the fixtures (`--entry <map-id>`, default the yard): the
owner's way to see what the captures show; captures now load the fixture packs as they are (a scratch copy only
to turn a `start`). **Next: the owner's try, then their decision on parts 3 to 6** (3 and 6 need the proposed
PRD amendment X23). A mutation script must `touch` the file it restores (lesson 2026-11-13).

The notes below were written 2026-11-12 and still hold, except "next".

Written 2026-11-12. **Rendering size 1 is built in full and tried by the owner** ("seems to hold up, might even be significantly smoother", 2026-11-12)
(`tasks/plans/rendering-size-1.md`, approved 2026-11-12; the owner's script `tasks/acceptance/rendering-size-1.md`):
the 3D view's and the editor preview's lines are retained Bevy meshes under `LineMaterial` (Bevy's `Material`,
`shell/lines.rs`, `shell/lines.wesl` embedded, WESL), from `strokes.rs` (a quad a segment; `style` kept for size
2); crests a second mesh (`lines::hang_crests`); the gizmo renderer and `--lines` are gone (the fight's 2D lines
stay gizmos). Commits `536efc5a`, `81e127e8`, `23459e72`. Gate 1293. **Size 2, part 1 (edge feathering) is built** (shader only, checked by zoomed captures, not yet tried by
the owner; an empty crest mesh is no longer hung). **Next: the owner's try of the feathering, then the owner's
decision on the rest of size 2** (`tasks/plans/rendering-size-2.md`: six parts; recommended next is edge feathering; a
proposed presentation-PRD amendment is at its end, not applied). **How a shader is checked** (the gate cannot):
offscreen capture pairs with `--settings .omnis/test-saves/still.ron` (`(reduced_motion: true)`), because the eye
eases by real time; `<scratchpad>/pixdiff.py`, `pairs.sh`, `still.sh`, `timing.sh`; scratch copies of the fixture
packs with an `entry` under `<scratchpad>/packs/`; `tasks/knowledge/verification.md` has the method.
**Bevy 0.20 shader facts** (read from source): shaders are WESL (`import a::b;`, `@if(DEF)`), the material bind
group is `@group(constants::MATERIAL_BIND_GROUP)`, `Material` is in `bevy_pbr`, `specialize` is also called for
the prepass and shadow pipelines (so the line material turns both off), `Material::depth_bias` only sorts.

Before that, the same day: **the loot window and ceiling plan is built in full**, `tasks/plans/loot-window-and-ceiling.md`
(approved by the owner 2026-11-12): step 1, the ceiling over a dip (`ead8e8c0`, `Ground::roof`); step 2, reach
(`ee803d45`: `ground::in_reach`, `PickUp` and `ObjectsView.ground` over the party's cell then the faced cell);
step 3, the loot window and the click (`b6492d3b`: `screen/loot.rs`, `aim.rs`, `shell/aim.rs`, `Does::Loot`,
`Effect::Commands`, Tier 1 `piles_in_reach`); step 4, store and destroy (`e3467963`: `ItemCommand::Stash` and
`Destroy` with `from: ItemPlace`, `Event::ItemDestroyed` (74 events), `Rejection::NotFromThere` (72), `items::lift`
over `ground::take` and `container::take`; `LootAct::{Store, Destroy, Confirm, Cancel}`, `Loot.asking`). The
owner's script is `tasks/acceptance/loot-and-ceiling.md`. **The owner tried it 2026-11-12**: "tested piles now store and release chosen items. I think we can move on."
(the ceiling, Destroy, the pile stamp and the quiet were not named). The rendering direction followed (above). **Open with the owner**: an open
container has no way into the loot window yet (recommended: "Take from..." under an open container on the verbs
screen); Store and Destroy act on a whole row, with no count asked; `Item` has no "kept" mark, so Destroy is
never dimmed for a quest item. Pinned counts moved by step 4: `schema_proof.rs` 122 instances and 177 branches,
`schema.rs` 12 item variants, `api_doc.rs` 74 and 72.
`shell/screen.rs` stands at 1000 lines: the next addition there moves something out first.
**From the owner's try of steps 2 and 3** ("now I can get to the loot pile"), built the same day, not yet tried:
a pile stamped on a cell that has one replaces it and the inspector lists a pile's contents (`editor/stamp.rs`);
the quiet after an encounter (`encounter_quiet_steps` 8 in base, `World::quiet`, base resealed at 0.1.2, the
replays rebaselined; the roll itself measured right at 2.95%). `editor/mod.rs` is over 1000 lines (1037).
The plan came from the owner's hand test of Bevy 0.20, which went well: "the upgrade went pretty smoothly",
"the left click loss is fixed" (`74772b2a`: a button covered while pressed stayed `Pressed`; `controls::
button_up`; lesson in `tasks/LESSONS.md`). Bevy 0.20 commits on `W-2_look_and_engine`: `84edc6e9`, `a1117c72`,
`d67ba116`, `3c00f762`, `74772b2a`. Gate 1293. **Still to hear from the owner**: the word "accepted" for 0.20,
and the departure (`controls::Control` with own left-button observers, not Bevy's `ui_widgets::Button`).
The owner's edits to `packs/the_lower_kingdoms` (dungeon heights, a pile at (4, 23), version 0.2.22) are
uncommitted and theirs: never staged. After this plan: ask the owner for the size of the rendering direction
(TODO W2, BACKLOG "GPU-first"; its own plan mode). On 2026-11-27 Bevy 0.20.1 and six held crates are thirty
days old (BACKLOG, Dependencies).

**Baseline captures on 0.19.1**: `<scratchpad>/shots-019/` (title, new-game, pause, actions, inventory, log,
view, walk; made by `<scratchpad>/shots.sh <label>` into `target/shots/<label>/`). The fight and the editor
have no offscreen capture: those are read through the dev socket or by the owner's hand.

**Done and accepted before this**: the `data.*` ops, `strings.rs` as a serializer, rule priority and conflict
tracking (2026-11-11), and `DevCommand::Pass { minutes }` (accepted 2026-11-12).

**Lessons of 2026-11-11** (`tasks/LESSONS.md`, four entries): the gate fails when no test ran; look for a
rule's precedent in the tree before departing from a plan; another checkout never builds into this tree's
`target/`; a sealed pack's text change rebaselines the replays in the same commit.

Rewrite this file every time it is used. Durable knowledge lives in `tasks/knowledge/`.

## When you resume
- **State**: read `git branch --show-current`, `git status --short` and `git log --oneline -8` first; the
  branch is `W-2_look_and_engine`. The owner pushes, never me.
- **Read first**: this file, `tasks/knowledge/README.md`, `tasks/LESSONS.md`, `tasks/plans/bevy-0.20-migration.md`,
  W2 in `tasks/TODO.md`, memory `dependency-policy-match-bevy`. Find the step by the ticks in TODO and the log.
- **Dependencies**: pinned, none published after 2026-10-13 on the day of the move (30 days by the system
  date), a single-copy tree matching Bevy's own versions; Socket when in doubt.
- **Open with the owner**: whether W1 is closed (the only open W1 item is `scripts/mcp-probe.py`, "when
  needed"; I recommended closing and carrying it); ARCHITECTURE §6.1 describes pack load order as a
  topological sort with cycle errors, which is not built (build it or correct the document); the four Bevy
  ECS tasks (`tasks/TODO.md`, Open) come after 0.20 unless the owner says otherwise; party creation's remove
  or edit a member.
- **The gate's last figure**: `tests passed 1275 failed 0 ignored 16 errors 0`, VERIFY-GREEN (2026-11-12).
- **The machine**: Rust 1.98.0 with the `x86_64-pc-windows-msvc` std; `DEVELOPER_DIR=/Library/Developer/
  CommandLineTools` for cargo and git. A link error after a toolchain change is cleared with `cargo clean`.
  `gh` stays blocked in this sandbox.
- **Per commit, unchanged**: `cargo fmt --all`; clippy workspace all targets with `omnis-vector/test-fixtures`,
  `--locked`, `-D warnings`; the gate to a scratchpad file, the commit chained on `grep -q VERIFY-GREEN`, the
  count line read as a number; `git add` by name (never `-A`; `assets/` and `docs/dnd_supplemental/` are the
  owner's, never staged; owner world docs under `docs/background` and `docs/locations` commit at once,
  unedited, own commit); `git commit -F`; Sentrux `scan` on `crates/` once a session, then `rescan` +
  `check_rules`; the Co-Authored-By trailer the system gives; never push.

## Where we are
- **Branch `W-2_look_and_engine`** (the owner pushes). **M5 Editor v1 is closed** (M5.0 to M5.10,
  every step accepted; `tasks/acceptance/m5-10.md`); the **boundary tools** followed and were accepted
  (`9486183b`, `tasks/acceptance/boundary-tools.md`). Untracked and never staged: `assets/` and
  `docs/dnd_supplemental/` (the owner's). Owner world docs under `docs/background` and `docs/locations` that
  appear modified or new are committed at once, unedited, in their own commit.
- **The editor's column as a drawing GUI** (the owner's direction captured in TODO and memory
  `editor-column-as-drawing-gui`): step 1 (tool grid times palette grid, `96237cbf` and the next) and step 2
  (Fill and Move, `54161815`, `40863b52`) accepted 2026-10-25 (`c5e6dd8e`). **One open report kept past step 2's
  acceptance: Move does not carry the start marker in the owner's window** (not reproduced headless or through
  the real UI path; when taken up, ask whether the plan pans or the cursor follows and nothing lands, or read
  `editor_status` and `screen_text` over the dev socket while the owner holds the game open).
- **Step 3, placements as palette stamps (2026-10-25, `48e73b74`; `tasks/acceptance/editor-column-3.md`):**
  `Entry::Stamp(Placing)` last in the palette (icons door-open, swords, face-grinning, package, layers,
  scroll-text); a click with the brush places through the kind's own form, valid as it lands, refused where its
  kind already is, one undo step; a drag places once; the inspector's Add row gone; a right click picks a
  placement's stamp. Every placing form starts valid; `Form::keys` refuses a second portal, site or pile on a
  new form's tile; an edit under a new id goes `[Remove(old), new]` (`Place::key`). The owner's hand test
  (2026-10-31) rejected the detail's left/right choice: step 3b.
- **Step 3b, the detail as a modal picker with properties (2026-10-31, `35b3e1ac`, accepted;
  `tasks/acceptance/editor-column-3b.md`).** `editor/stamp.rs`: `Stamps([Vec<Place>; 6])`, a stamp holds what
  it drops as the data model's own places (`limit`, `details`, `short`, `rebuild`, `of_here` whole,
  `Editor::stamp_places` held-or-first, `Editor::stamp` through `Form::for_place` + `request` + `place_many`).
  `editor/pick.rs`: `Picking { kind, filter, chosen, props: Vec<Form> }` on `Editor.picking`;
  `pick_open/pick_filter/pick_toggle/pick_finish/props_open/prop/props_finish/typed_in/set_in`.
  `forms/props.rs`: `Form::for_place` (the builder with `was`, a fresh id), `Form::props_rows(n)`; `forms.rs`:
  `request` = `keys` + `value()`, `id_taken`. `acts.rs`: `EditAct::{Stamp (the icon), Pick, PickFilter,
  PickToggle(u16), PickFinish, Props, Prop(u8, u8), PropsFinish}`. `Asked::{PickDetail, Props}`;
  `ScreenId::{EditorPick, EditorProps}` (`screen/editor_pick.rs`, `screen/editor_props.rs`); `EditingCtx.pick`;
  **`Effect::Editor(act, payload)`** (text and set numbers reach the model); `shell/editor.rs::answer(nav, asked)`
  the one place the Asked's open and close overlays. Under the palette: the stamp's icon (`EditAct::Stamp`) and
  Select... (`Pick`) both open the picker (two acts: the layer syncs words by control id, so one id for two
  controls gave the button the icon's words), the sliders icon (`Props`) for Encounter/Object/Ground, a summary
  line ("giant_rat x3, goblin"; "nothing chosen, the first: <id>"). Owner's choices: the properties open after
  every Finish; piles get them too. Not offered: a per-placement loot (not in the data model). Verified: the
  search field keeps the keyboard and the caret through the rows' rebuild (`editor_pick.rs`, the keyboard test).
- **Step 5, Undo as a history list (2026-10-31, `d845eb4e`, accepted; `tasks/acceptance/editor-column-5.md`).** `edit/words.rs` names every undo step (a stroke counts its cells;
  `Document::name_step` takes the editor's richer words from `report`, fill, a run, a grab); `Document::{steps,
  at, goto}`; `EditAct::{History, HistoryTo}`; `screen/editor_history.rs` the modal (the map as opened, a row a
  step, the current one marked and dim, Undo, Redo, Close), `EditingCtx.history` from the editor's cached list
  (`read_history` in `changed`). Owner's choices: a History icon beside Undo and Redo; a modal, the plan beneath.
- **Step 4, the lasso, moving a selection, cell runs (2026-10-31, `f6537ddd` and the loop-preview fix
  `4e36a1e0`, accepted; `tasks/acceptance/editor-column-4.md`).** `editor/select.rs`: `Selection { level, cells }` on
  `Editor.selection`, the loop in `Editor.lasso` while drawn; `inside` (the loop's cells through `run::cells`
  plus the even-odd rule over the bounding box), `outline`, `moving(grid, def, sel, d)` (source paints first:
  Clear, Height 0, sides Open; then the destination's tile with its stair's rise, height, sides; boundary sides
  skipped; then the start and the placements by d; refuses off the map and a portal, site or pile on a
  destination tile not vacated), `Editor::{lasso_start, lasso_extend, lasso_end, selected, selection_outline
  (the ghost: displaced by cursor - grab.from while Held::Selection), move_selection → Document::batch}`.
  `grab.rs`: `Held::Selection` first in `under`; `grabbing` reads a loop in progress first and starts one on
  the lasso's Down. `run.rs`: `cells(a, b)`, `run_cells`, `Editor::cell_run()` = `entry.fills()`, `take_cells`,
  `Preview.cells`; a cell run's points are cells and the hover stays. `brush.rs`: `Tool::Lasso` (lasso-select,
  L), `Tool::ALL` 8. `acts.rs`: a run tool forces the wall for a stamp only; `Brush(i)` keeps a run tool for any
  non-stamp entry; `apply` drops the loop. `plan.rs`: `Source.selection`, `SELECT`, `bars`. `omnis-data`:
  `Document::batch(level, &paints, places)`. Owner's choices: a freehand loop; everything moves; the destination
  is replaced. The old run tests choose the wall first now (a terrain is a cell run); `editor_column.rs` flipped.
- **Observed, not fixed (told the owner 2026-10-25):** with the map's own file open as text and typed into, a
  paint and Save from the plan leave the text view older than the file; Save file would then write the old text
  over the paint. Narrow; a rule if the owner wants one.

## The entrances and exits enhancement (planned 2026-11-01; `tasks/TODO.md` W1; **closed, every step accepted, 2026-11-08**)
- Owner's scope (all four): loader rules on landings and starts; every map reached and every portal paired; pairs
  authored once in the editor; the start as a real entrance. Owner's choices: **the strict mirror** (the way back
  stands directly behind the landing; the party arrives facing away from it), **reachability for the maps of the
  pack that sets `entry`** (fixture packs exempt), **the landing picked on the far map's plan**. The
  Move-the-start report of 2026-10-25 is closed ("works now").
- **Step 1 built 2026-11-01 (the rules)**: `map_data.rs` `Target`, `enterable`, `standing_fault`, `arrival_errors`
  (no ground / cannot enter / a portal stands), `way_back`, `no_way_back`, `unreached`, `check_reached`;
  `Facing::{ahead_of, behind}`; the engine's `landing` and `standable` on `enterable`. A map with no portals is
  exempt from reachability (else New map could never be saved: the loader is all or nothing). The shipped stairs
  into the dungeon and the depths now land facing east (the pack 0.2.4); the replay scripts turn right after the
  stairs; `disk.rs`'s frozen-save comparisons guard on `report.fit == PackFit::Same` (the world's `packs` field
  records today's packs on a load by name, so the old guard was always true). **A portal stamped alone says "has
  no way back" until step 2**; `editor_stamp` and `editor_pick` pin that wording for now.
- **The owner's hand test of step 1 (2026-11-01)**: Save and Playtest dimmed with the loader's first error in
  full, read as "lost the test play option" and Save "overlapped by the red error text"; fixed: `view::error_
  count` gives both buttons the count alone ("1 error"), the words stay the inspector's and the message's
  (`tasks/LESSONS.md`). The test also showed that **no pair can be authored in the editor until step 2** (each
  half is refused until the other is on disk; hand-editing the files and `omnis-cli pack seal` is the only way).
- **Step 2 re-planned 2026-11-01** on the owner's direction ("an intermediary state for portals, 'unlinked'...";
  "a freehand portal_id text field... search through all available portals"); owner's choices: **one side is
  enough** (A's file says `link: Some("b")`, B's may say nothing; the editor never writes a second map), **the
  landing is derived** (ahead of the partner, facing its way; `to_*` gone from the file), **ids unique across the
  world**. The earlier plan's far-map document and `Kind::Picture` are withdrawn.
- **Step 2a built (the model)**: `map.rs` `Portal { level, x, y, id, link, facing, marker }`, `Portal::{ID_MAX,
  id_fault, landing}`, `validate` (id faults, ids shared within a map); `map_data.rs` `MapTargets::maps()`,
  `pairing` (partners: the named one and every claimant; errors: nothing loaded, itself, a third, two claimants),
  `landing` (ahead of the partner: outside, no ground, cannot enter, a portal stands), `resolve_portal` (a
  `ResolvedPortal` for linked portals only, so the sim, the floor mark, the minimap and `map.text` needed no
  change), `check_inbound` (other maps' pairings and landings here), `unreached` over linked pairs (maps without a
  linked portal exempt; **asked only of a sound world**, in the loader and in `resolve`: an erring link would
  otherwise list every map beyond it), `pub partner_of`, `unlinked_portals`, `unlinked_portals_in`. The editor:
  `forms::portal(def, data, at, was, detail, fresh)` (id, link, facing, marker; `portal_id`, `portal_id_taken`),
  `stamp::options(Portal)` = the unlinked portals of the loaded maps as the files hold them (the open map's
  unsaved ones come in 2b), `rebuild(Portal, [])` an unlinked portal, `summary` "unlinked", the choice cleared
  after a linked placement, `Mark::Portal { linked }`, the label "Portal 'id' to 'link'". Corpora: `bad-ways`
  rewritten (12-wide stand: ten rows), `bad-reach` new (one row), `broken` and `bad-objects` converted and resealed.
  Mutation checks red: the inbound claim ignored, the landing behind, reachability over every portal, the mark.
- **Step 2b built (linking in the editor)**: `pick.rs` `For::{Stamp, Field(u8)}`, `Picking { purpose, options,
  labels, .. }` built at open by `pick::options(kind, def, data, except)` (a portal's rows over
  `unlinked_portals(def, data)`, the open map's unsaved ones among them, labelled "id  -  Map (x, y)" through
  `labels::map`; `shown()` matches the label), `pick_open_for_link` (the form's Link...: the edited portal left
  out, its partner offered and shown chosen), `unlink`, `pick_finish` writing a field's choice through
  `Form::typed`; `EditAct::{LinkPick, Unlink}`; `Form::rows` adds the Link.../Unlink row for a portal form;
  `view::partner_line` under a portal's inspector row; `stamp::of_here` drops a copied portal's link. The
  picker screen reads `p.options`/`p.labels`, no longer `stamp::options`. `tests/editor_link.rs` (two flows).
  Mutation checks red: the edited portal offered, the choice kept, the field left empty.
- **The hand test of 2026-11-07**: the stamp's Finish leak (above), fixed in `route` with `Local<bool>` `stood`
  (the frame the cover lifts takes no `Down`; `Inputs` bundles the keys, the mouse and the wheel as one
  `SystemParam`, the routine being at Bevy's sixteen). Mutation check red: the guard dropped. The harness's
  `press_ui` sends no mouse button: a window report that resists a headless reproduction gets the window's input
  (`real_click`) before a second theory (`tasks/LESSONS.md`).
- **Step 2c (2026-11-07)**: `links.rs` `candidates` (= `pick::options(Portal, def, data, Some(at))` + the
  partner), `matches` → `Listed { shown, more, any }` (the whole id marks and leads; `SHOWN` 10), `with_list`,
  `Editor::{link, unlink}` through `fill_link` → `Form::set_text`; `EditAct::Link(u16)` in `LinkPick`'s place
  (the `EditAct` count unchanged); `pick.rs` without `For`/`purpose`/`pick_open_for_link`, `fits` shared.
- **Step 3 (2026-11-07, `76572568`, accepted 2026-11-08)**: `MapDef.start_level`, `Place::Start { level, .. }`,
  `validate`'s start row, `resolve_one`'s standing check on the start's level, `arrivals` under it; `World::new`
  and `places::party` on it; `SetStart` on the shown level, `grab::under`/`select` by level, `open` on the start's
  level, `Source.start` with its facing, `plan::start_pixels` (diamond + notch to the cell's side);
  `remove_level` returns a start on it to the ground. `tests/editor_start.rs`; `common::editor::scratch_of`.
  Follow-ons noted in the TODO review line: a start mark in the 3D preview, `omnis-cli`/`editor_new` starts,
  Playtest from the start.

## Next
1. Bevy 0.20 (above). Windows CI is closed and advisory (`tasks/acceptance/windows-ci.md`); a red Windows step on a
   later push is a fault for its own commit with a test (grep `.join(` against the 2026-11-08 lesson first). When the owner orders it: **the client's use of Bevy's ECS** (`tasks/TODO.md`, Open: four tasks, a
   press as a message first; plan mode, one step a commit). The frozen fixture carries the portals as they were at
   0.2.9 (the owner's strays `portal_23_0`, `portal_23_28` and `portal_bank_hidden` included): a test names them
   by content, never by count. The editor's later items otherwise (the eraser or Delete clearing a selection's
   cells, copy and paste, `editor.history` over the socket, a quantity for an object kind in the picker). The
   owner's order of 2026-10-24 otherwise: the activities schema when a second kind has engine rules; then the
   plan in `tasks/TODO.md`: the look in data, M9. W1's remaining items: Windows CI at W1's close (D31).
2. Later: the eraser or Delete clearing a selection's cells; copy and paste of a selection; Esc dropping a loop
   or a run (Esc is the menu's); a quantity for an object kind in the picker (several of one kind in a click); a smooth (sub-cell)
   pan; a fill's region and a stamp's mark previewed on hover; the map's own fields (`Edit::Map`), Resize and
   New map as forms; a random entry's creatures as rows; the slider's thumb re-synced after a refused value;
   plan edges one pixel at every zoom; the stale-text-over-paint rule above; the 4 MiB file cap stays soft
   (owner); an `editor.run` op (and a fill, a move, a stamp); the icons' keys shown somewhere; `grab.rs`'s
   `moved_place` could reuse `Form::for_place` (a tidy); a start mark in the 3D preview with the icon pass.

## The owner's way of working (keep)
- Each item ends with something visible or clickable, and the report says how to try it (numbered: where to
  stand, what to press, what appears). The owner replies "tested" or "accepted" and asks for the next; a hand
  test can also come back as a direction (step 3 → 3b), which gets plan mode and its own commit.
- Hand-test launch: `just run --windowed --pack packs/base --pack packs/the_lower_kingdoms` (the owner's world; with the
  objects fixture: `--pack packs/base --pack crates/omnis-sim/tests/packs/test --pack
  crates/omnis-sim/tests/packs/objects`, the fixture standing in for the world), New game, devtools on; teleport with Esc, Debug, the World page, to
  `objects:map:yard`. The editor: Title → Editor, or `--editor test:map:town`. A Save reseals the pack on disk:
  hand tests of the editor run on a scratch copy (`cp -R packs/the_lower_kingdoms /tmp/omnis-scratch`, named as the last pack).
- Offscreen captures: `scratchpad/yardshot.sh name x y facing walk [--screen ...]` (`cargo build -p omnis-vector`
  first). A capture cannot run a command first.
- Bug reports: fix directly, with a red test, a lesson in `tasks/LESSONS.md`, then report. Choices with real
  alternatives: AskUserQuestion with a recommended option first; larger additions get plan mode. A study's
  recommended answers become the owner's choices when the owner says "settle" (2026-10-21).
- Owner UI choices (memories `editor-pickers-are-modals`, `editor-column-as-drawing-gui`): pickers as modals
  over the editor, never a choice box over pack content (2026-10-31); a decision asked at the action, not spread
  as dim reasons; wheel zoom about the pointer to room scale; the column is a tool grid times a palette grid,
  never another row of text buttons; icons where they beat the word, words where the reason matters.
- The owner edits and saves through the editor and commits checkpoints of their own: tests never pin what those
  saves move (the pack's version, its notes); a content change the owner did not intend is asked about, not fixed.

## Pins
- **Replays:** walk `244eb411d2c9856c`, fight `961b2593313f176b` since base was resealed at 0.1.2 on 2026-11-12 for `encounter_quiet_steps` (before: `d48a913cec6c5735`, `9f52f48c5364e17f` at 0.1.1, 2026-11-11; `0370ab07b628f193`, `c81c08bde3a070e9`; against the frozen fixture at 0.2.9, hash
  14523718773761627429, since 2026-11-07; the owner's canvas moved on to 0.2.18 and beyond and moves them no more;
  earlier that day 0.2.9 for the owner's pack, three unlinked portals and four cells; 2026-11-01 for step 2a, the portals linked by id,
  `packs/test` 0.2.5, hash 15167835935440741063; earlier that day the two east arrivals at 0.2.4; 2026-10-31 the owner's depths at
  0.2.3; M5.10a at 0.2.0). A rebaseline
  records the new fingerprints here, in `docs/api.md`'s changelog and in `tasks/knowledge/verification.md`. `SAVE_SCHEMA` 9, `PROTOCOL` 2, **ops 48, MCP tools 48, `Reply` 31 (`EVERY`), `OpError` 12**
  (since the `data.*` ops, 2026-11-11), `MAX_MIGRATIONS` 1024, `MAX_STEPS` 4096 (pinned to the docs by `omnis-mcp/tests/api_doc.rs`, run as
  `--test integration api_doc`; the tool counts also in `tools.rs`, `bridge.rs` twice, `omnis-cli/tests/headless.rs`,
  README, code-map).
- **Counts:** `Command` 16, `DevCommand` 15 (`Pass`, 2026-11-11), `Event` 73, `Rejection` 71, `LoadError` 12. The palette is the
  terrains plus 10 (`brush::palette`: Wall, Door, Ramp, Height, then `Placing::ALL`'s six stamps; `tests/
  editor_fill.rs` pins WALL 9, DOOR 10, HEIGHT 12 on the town's nine terrains); `Tool::ALL` 8 (Select, Brush, Move, Lasso, Fill, Corners, Freehand, Eraser; keys V B M L F P N X);
  `Placing::ALL` 6;
  the kit's `Kind` has 7 members; `Asked` 9; `ScreenId` has EditorMenu, EditorFiles, EditorPick, EditorProps,
  EditorHistory as the editor's overlays (`nav::allowed` and `paused`). A stamp's picker limits: 1, `MAX_STACKS` 4,
  `MAX_OBJECTS_PER_CELL` 16, `MAX_PILE_STACKS` 32, `MAX_LORE_PER_CELL` 16. Base + test packs: 7 monsters, 24 items.
- **Fixtures:** `crates/omnis-sim/tests/packs/test` is **the frozen fixture** (id `test`, 0.2.9, hash
  14523718773761627429: `packs/test` as committed at `8d58c3e2`; owner 2026-11-07: never saved onto, and a test never
  loads the owner's own pack under `packs/`; `common::FIXTURE` in the vector harness);
  `crates/omnis-sim/tests/packs/{ahead,chain/v1,chain/v3,heights,levels,objects}` (the shipped
  packs have no object kinds: a test that places an object opens `objects:map:yard` with the objects pack); a
  test that edits a pack works on a scratch copy (`omnis-data/tests/common::copy_pack`,
  `omnis-cli/tests/headless.rs::scratch_pack`, `omnis-vector/tests/common/editor.rs::in_editor_scratch`) and
  reseals it; the bad-pack corpus `broken` carries `notes/bad.ron`. `packs/test` ships notes on all four maps and
  on the town's cell (11,2), which also carries the portal; the town's sites are the owner's to move (rows 0 and
  4 since 2026-11-07): a test that wants bare ground uses the street, row 2, and finds a site by its service. The dungeon's encounters carry
  dispositions and counts above one (a right click's copy is tested there).
- **Limits the editor uses:** `MAX_STRING_BYTES` 4 KiB, `MAX_FILE_BYTES` 4 MiB (soft cap by the owner's decision),
  `MAX_SAVE_BYTES` 8 MiB, `MAX_COLLECTION` 65,536, `MAX_MAP_SIDE` 256, `MAX_LEVELS` 16, `UNDO_CAP` 100.
  `Session::files()` lists `pack.ron` and `.ron` under `data`, `text`, `notes`, `migrations` only.
- **Client test helpers:** `tests/common/app.rs` (`keyboard_on`, `keyboard`, `tap`, `set`, `press_ui`, `top` (needs
  a `Session`: the editor tests use `common::editor::top_screen`), `churn_fields`, `wheel`, `ui_editor_app`),
  `tests/common/editor.rs` (`in_editor`, `in_editor_on`, `in_editor_scratch`, `press_edit`, `arrow`, `point_at`,
  `point_frac`, `leave_plan`, `mouse`, `type_into` (an `EditorPress` field), `type_ui` (a `Pressable` field of
  the top screen), `top_screen`, `hold(app, kind, ids)` (a stamp's held places as Finish would make them, the
  revision bumped), `held`, `field_entity`, `field_of`, `panel`); `editor_pick.rs` has `pick(app, kind, ids)`
  (the UI path through the picker), `props`, `count_of`, `prop_of`, `pressable` (a dim control keeps its
  `Pressable` but loses `Button`), `bare_cells`; `editor_stamp.rs` has `choose(entry)`, `detail_to` (now
  `hold`), `first`, `click`, `right_click`; `editor_select.rs` has `drag(app, button, from, through)`, `lasso_the_inn`,
  `selected`, `site_at`; `editor_run.rs::choose(tool)` chooses the wall first for a run tool. A choice control has no press entity: step it by `arrow`, never
  `press_edit`/`press_ui`. A written `RelativeCursorPosition` reaches the model on the next `app.update()`.
  Since Bevy 0.20 a test points at a control with `set(app, &marker, Pointing::{Rest, Hover, Press})`, `let_go`,
  `rest` (`Hovered` and `Pressed` by hand); Bevy's real picking reads `WindowEvent` messages, not the plain ones
  (`editor_pointer.rs`), and a harness with `TextInputPlugin` needs `PointerState`. In `ui_editor_app` tests drive the model through
  `shell::editor::act` in `ui_editor_app` tests. `screen_text` prints an icon as `[button] [icon] "name"` and a
  swatch as its glyph; an overlay's rows print after the editor's.

## Open, carried over
- `assets/README.md` font rows: the owner's file; report only, never edit.
- `docs/dnd_supplemental/`: the owner's, in progress; ask before committing anything there.
- P6 colours unconfirmed; step 5 revisits colour roles (joins, fills, crest, objects, beings).
- The automap still records the forward cone (§12.5); no objects or beings on it (owner, 2026-10-14).
- Dependencies: the rhai Socket re-audit (ask when); `smartstring` RUSTSEC-2026-0249; `ttf-parser`
  RUSTSEC-2026-0192 (informational); Bevy 0.20 not before 2026-11-07.

## Process
- **Cargo and git:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools` for both. `cargo fmt --all` first and
  again after any appended test. Workspace clippy with `--all-targets --features omnis-vector/test-fixtures --
  -D warnings` before every commit. Sim tests: `cargo test -p omnis-sim --test integration <filter>`; data `-p
  omnis-data --test integration <filter>`; client `-p omnis-vector --features test-fixtures --test integration
  <filter>` (one filter per invocation); client unit tests `--lib`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<item>.txt 2>&1`, VERIFY-GREEN, no source or pinned-document
  edits while it runs (about seven minutes; it fails fast on a clippy error, so run the workspace clippy first).
  Then `git add <paths by name>`, `git commit -F <msg>`, Sentrux `rescan` (first `scan` on
  `/Users/john/code/omnis/crates` in a new session) and `check_rules`. The scratchpad is
  `/private/tmp/claude-501/-Users-john-code-omnis/762beda3-a95d-40a0-9a67-8bf9680a2cb8/scratchpad` (absolute).
- **Tests red first; mutations** by a Python runner in the scratchpad (anchor checked unique, source restored by
  copying the backup back, then `diff -q`; `mut3b/run.py` is the pattern). No cargo command and no fixture edit
  while a mutation run or the gate runs. A mutant must undo the rule it targets, not restate it (a
  "destination before source" mutant that moved the source paints to the front was still the rule: `mut4/run.py`
  takes a list of replacements so the source paints can be moved to the end).
- **Moves and edits** by an anchored Python script in the scratchpad (every anchor asserted unique before
  writing). After `cargo fmt` the anchors move, so patch the file as it is. zsh: quote `echo "===="` (an
  unquoted `====` is a command lookup) and never pass `--include=*.rs` unquoted to grep.
- **Each new op** moves: `ops.rs`, `api_doc.rs`, `replies.rs`, `tools.rs`, `bridge.rs`, `omnis-cli/src/schema.rs`,
  `tests/headless.rs`, `docs/api.md`, README, `code-map.md`, ARCHITECTURE §9.3. Each new command, event or
  rejection also moves ARCHITECTURE §4.2's counts, `code-map.md`, `variants!` lists, the MCP `schema.rs` test and
  `schema_proof.rs`, `common/commands.rs`, `word.rs`, client `labels::refused`.
- **A change to `packs/` or a serialized `World`** rebaselines the replays in the same commit and records the
  fingerprints in `docs/api.md`'s changelog, `tasks/knowledge/verification.md` and the pins above.
- **Commits:** stage the changed files by name; never `git add -A` on `docs/`, `assets/` or the root; read
  `git status --short` before `git commit`. Messages end with the Co-Authored-By line and the Claude-Session line
  the system gives. Never push. Owner world docs (`docs/background`, `docs/locations`) go in their own commit,
  unedited. Ask before changing `PRD.md` or `ARCHITECTURE.md`; an owner's direction counts as the ask, flagged in
  the commit. Interface strings are ASCII only; every action has a mouse entry point.
- **Context:** pause and recommend a compact past 350K.
