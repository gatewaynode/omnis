# TODO

The development execution plan (owner, 2026-10-21). Alignment flows from the vision documents, `PRD.md`,
`ARCHITECTURE.md`, `presentation-PRD.md` and `presentation-ARCHITECTURE.md`: every item names the section it
serves, and where the plan needs a document to change, the change is listed under W0 and the owner approves it
before it is made. What was built before 2026-10-20 (M0 to M8, protocol 2, vector adoption P0 to P9, M8.5 steps 1
to 4 and 7) is recorded in `tasks/archive/TODO-2026-10-20.md`. Small findings and unused assets: `tasks/BACKLOG.md`.
Bugs: `tasks/BUGS.md` (B1 to B7, all closed).

## The goal: ready for world building
The MVP is proven: a party is created, walks a 3D world drawn in lines, fights, levels, shops, rests, saves and
loads by name across changed packs, and every system is driven by data, by the CLI and by an agent over MCP. The
next goal is that the owner builds Toel's first locality, **the Three Kingdoms of the lower Blood Sea**
(`docs/locations/`: Karinguard, Shoentran and Resdie; Foloen and the Sunken Grove uncharted; the Old Dam, the
Great Goblin Head, the ruins and the catacombs; biospheres piedmont, coastline and mountains), with the shipped
tools and in pack data, nothing hard-coded (PRD goals 2, 3 and 6, D4, §11.3).

**Ready means (owner, 2026-10-21): the end of W1.** Every map and level is authored in the editor and
round-trips, and the owner starts on the first settlement, Resdie. The world's look as data a region names (W2)
and the locality's regions, settlements and wilds laid out on the continent (W3) are built while that content
grows, each against what the content needs (PRD R5). The living world (W4) makes what was authored move.

**The order and why.** The old order ran tools last (PRD §13 put the editor first, D4 moved it to close Phase 1,
and the milestones built the loop, the time model, the API, the presentation and the save model before it).
That was right for proving the loop; it leaves every map hand-written. From here the order is what world
building needs first:
1. **W0** clean-up and alignment, so the plan and the vision documents agree;
2. **W1** the authoring tools (M5), since nothing else the owner authors should be hand-written RON;
3. **W2** the look and the engine under it (Bevy 0.20, the rendering direction, M8.5 steps 5 and 6, set pieces),
   since the owner's regions each name biospheres and the lore rests on ambient mana;
4. **W3** the world's structure (M9), since the locality is regions on a continent with wilds to fill;
5. **W4** the living world (M10, M11), since factions, rumours and quests are the lore's second half;
6. **W5** release (M12).

Milestone numbers are kept inside the phases (M5, M8.5, M9 to M12); a milestone never changes number. Each phase
is planned in plan mode when it is reached, one sub-step at a time, each ending in something the owner can try.

## Standing rules (from the delivery plan, 2026-09-12)
Tests on real `packs/test` data, no mocks; sim-crate lint green (CLAUDE.md); Sentrux review; `LESSONS.md` updated
after any correction; one commit per checked item or small group; MCP tools and CLI subcommands ship with the
system they expose, never later; every player action, dev tools included, has a button before a key; the client
reads the engine through `omnis_sim::api`, never `World` fields; the owner pushes.

## W0 — Clean-up and alignment (now)
Done when the plan, the vision documents, the README and the notes all tell one story, and the three far
horizons have a study each.
- [x] W0.1 This plan (2026-10-21): `tasks/TODO.md` refactored (this commit); `tasks/CONTINUITY.md` and
  `tasks/knowledge/README.md` repointed.
- [ ] W0.2 `tasks/alt-TODO.md` archived to `tasks/archive/alt-TODO-2026-10-21.md`: every item in it is done
  (Phases A and B, 2026-10-02 to 2026-10-10) or tracked here (Bevy 0.20). presentation-PRD §11's "Report in
  `tasks/alt-TODO.md`" follows the move (owner approves).
- [ ] W0.3 `README.md`: the Status table stops at M6 and says "Next is M7"; "`tasks/TODO.md` is ... the record of
  what each milestone delivered" now names the archive.
- [ ] W0.4 The vision documents brought to this plan, each change approved by the owner first:
  - PRD §13's note of 2026-09-12 ("revisit this section once the milestones settle"): revisited; it names this
    plan's phases and which PRD phase each closes; §13 Later gains the three far horizons as the studies leave
    them.
  - ARCHITECTURE §16 phase mapping: a status column (0 and 1 built but the editor; 2 to 5 planned); the Phase 1
    row still names the retired `omnis-app`; the build-order note points at the archive. §4.7's "the tables are
    in `tasks/TODO.md`" and §8.3's "`tasks/TODO.md` M1 review" point at the archive. The Date row is the
    creation date.
  - presentation-PRD §11: steps 1 (the merge, 2026-10-10) and 4 (height and cliffs, M8.5 1a to 1c and 4) marked
    done; the list reordered to W2's order.
  - presentation-ARCHITECTURE: the "Vector adoption" pointer in its status line names the archive; §4.3's
    labels are built (`labels.rs`), not "to build"; §7's style system is partly built (the type scale and fonts).
- [ ] W0.5 Three studies for the far horizons the MVP makes thinkable (owner, 2026-10-21), each a
  `tasks/plans/study-<name>.md` that reads only: what the documents say today, what the code constrains, the
  options with their costs, the proposed PRD rows (a decision, the §2 consumers, §5 non-goals, §13 Later) and
  the earliest phase the work could enter. Nothing is built until the owner chooses from a study.
  - **Cryptographically secure user accounts.** Today: no account, login, cloud or server exists in any
    document; PRD §7.7 says saves are the player's files with no cloud dependency; every hash the engine uses
    (the world id, base names, the pack seal) is FNV-1a 64, which is not cryptographic; the dev socket is
    loopback only with no auth. The study covers the four purposes the owner named (2026-10-21), since each is a
    different design and a different dependency audit: identity in multiplayer (a player keypair naming a party
    and signing its commands); signed saves and world files (tampering detectable); signed packs and mod
    provenance (the seal becomes a signature); a hosted service (login, profiles, cloud saves), which PRD §7.7's
    "no cloud dependency" admits only as optional, a row the study proposes.
  - **Multiplayer.** Today: a v1 non-goal (PRD §5) kept possible by goal 7 and D5 (co-op later, §13 Later);
    ARCHITECTURE §4.6 "share the command stream", §11 a `party:<id>` stream per party, §4.4 each party a holder
    of its own clock (D25); `World` holds one party and one position; no async runtime (`tokio` deliberately
    absent). The study maps co-op onto the character save and world folder of D28 (several character saves over
    one world file is already named in `tasks/plans/world-state.md`).
  - **Multi-platform.** Today: D5 names macOS, Linux and Windows; browser and mobile are non-goals, with §14
    asking the owner which are rejected outright; CI runs macOS only (Linux deferred by the owner 2026-09-12 until
    needed; Windows once Phase 1 is playable, ARCHITECTURE §14); packaging and the settings directory arrive
    with M12. The study costs Linux and Windows CI now against at M12, and names what a web or mobile build would
    give up (the lint's own rules make the simulation portable; the client is Bevy's reach).

## W1 — The authoring tools (closes PRD §13 Phase 1, D4)
Done when the Phase 1 content (`packs/test`'s maps and levels) is re-authored in the editor and round-trips with
no loss, and every later map is built with it.
- [ ] **M5 Editor v1** (deferred 2026-09-19; settled 2026-10-14: `bevy_ui` in the neon style, a top-down plan one
  level at a time with the 3D view as preview and Playtest, maps plus a RON text editor checked by the loader for
  every other file, a `notes` text on maps and levels; PRD §10, ARCHITECTURE §8.2; `tasks/plans/editor-v1.md` is
  superseded for planning and keeps §A, the document model, and §E's kinds and limits to reuse). Planned in
  plan mode first, against the format as built (heights, ramps, levels, joins, stairs, portals, encounters by
  id, sites, objects, the pack seal). The plan sets the sub-steps; the expected shape: the document model with
  undo and redo and a save refused while invalid; the editor mode and the plan view; painting; the preview and
  Playtest; the RON editor; `editor.*` ops and their MCP tools; the re-authoring; the documents.
- [ ] Ops for pack definitions and text labels (owner, 2026-10-08): a `data.*` op family so a JSON client, an
  agent authoring content among them, turns ids and text keys into names and definitions; the main known gap in
  `docs/api.md` §14; additive, protocol stays 2. With the editor's ops, since both read the same registry.
- [ ] A dev command to pass time (owner, 2026-10-08): `DevCommand::Pass { minutes }` lives the minutes at the
  region's company, bounded, refused in a fight; a debug-panel field and the MCP schema (`dev_schema` 14 moves).
  Replaces the 720 short rests in `tasks/acceptance/m8.md`. When a test of time needs it.
- [ ] A bridge probe in the repo: `scripts/mcp-probe.py` drives `target/debug/omnis-mcp` over stdio so a session
  whose MCP client did not launch the bridge still runs the MCP acceptance steps. When needed.

## W2 — The look, and the engine under it (presentation-PRD §11 steps 3 and 5; no PRD §13 phase)
Done when a region's biosphere sets what the player sees from pack data, the first authored set piece stands in a
map, mana is in the PRD and in the packs, and the performance floor is read against the finished geometry (X20).
- [ ] Bevy 0.20 (`tasks/plans/bevy-0.20-migration.md`, not before 2026-11-07). First, so shaders are written once.
- [ ] The rendering direction (BACKLOG, 2026-10-20, "GPU-first"): the owner chooses a size; the recommendation
  is size 1 (retained line meshes with an own WGSL material) straight after 0.20, with size 2 designed in the
  same presentation-PRD amendment so width, fog and relief are in the shader from the start. A plan under
  `tasks/plans/` and the amendment text come with the choice.
- [ ] **M8.5 step 5, the colour scheme and biosphere palettes** (owner's 2 and 5; presentation-PRD §5.2, X18, X19,
  §11 step 3): the fixed colour roles, a biosphere's palette and style set in pack data, the relief noise per
  terrain kind (presentation only; as displacement on the GPU if size 2 is designed in), the first non-colour
  channels (line pattern, weight, glyph markers, motion), applied to the 3D view, the fight screen and the HUD,
  with a simulated colour-deficiency check (XR8; it covers the health colours). The owner's regions name their
  biospheres already (piedmont, coastline, mountains; mystic isles, volcanic; archipelago, jungle, savannah,
  desert). Open for the owner: the role set and glyph vocabulary (§12.3); whether a biosphere is assigned per
  region, per map or per terrain, and whether it also picks the relief noise and shape generators (§12.2). Living
  things and resources stay in M10.
- [ ] Shaped structures and set pieces (presentation-PRD §11 step 5, X12, §12.4): shape generators per terrain
  kind, then the first authored set piece. The owner's locality names the first two: the Old Dam across the P'ta
  and the Great Goblin Head. The set-piece format (a RON line model, SVG, glTF or an in-editor tool) is decided
  here; the editor places them.
- [ ] **M8.5 step 6, magic and mana in the map packs** (owner's 6). Neither the PRD nor ARCHITECTURE has mana in a
  place (members have spell points), while the lore does: Toel's abundant ambient mana, the taint Resdie's
  wizards cleanse from the fields. Needs the owner's intent before a plan, and a PRD section with it: what a
  place's mana does (spell power, recovery, wild or dead zones, taint), whether it is per cell, per map or per
  region, and whether it changes over time (M10). Shown in the magic colour role from step 5.
- [ ] Interface polish the owner sets a time for: Look and Map in vector (deferred at P9: Look, a button beside
  Camp and L, the first spyglass a member who can act carries; Map, the large automap over the view, M); the art
  update (a figure per monster, the fight screen's scale on large windows, deferred 2026-10-03); the distant
  level and the blueprint automap (presentation-PRD §12.5, §12.7).
- [ ] Dependencies: the rhai Socket re-audit (owner: after the migration); `smartstring` RUSTSEC-2026-0249 and
  `ttf-parser` RUSTSEC-2026-0192 reviewed with 0.20 (BACKLOG).

## W3 — The world's structure (closes PRD §13 Phase 2)
Done when a seed produces a traversable world of at least 100 regions with towns and dungeons the player can
complete, regenerating one layer keeps the edits in the others, and the Three Kingdoms' locality is laid out as
regions on Parva with its authored places in them.
- [ ] **M9 procedural world** (`omnis-gen`, ARCHITECTURE §7, §16): first the world's layout as data, since the
  owner's documents are shaped world, continent, locality, region (climate, biospheres) and today a region names
  only its kind, company, stability, rule, couplings and maps, and maps join by portals; regions laid out with
  edge links; travel between a locality's regions. Then the layered pipeline (terrain, hydrology and roads,
  settlements, dungeons, encounters and loot, quest hooks), each layer a seeded pure function over the LockMask so
  regeneration keeps hand edits; lazy `Pending` and `Materialized` maps and regions in the world folder; the
  editor's procgen panel; `gen region`; goldens seed to fingerprint. Generated maps may carry levels and joins.
  The ruins strewn over the Three Kingdoms are what the dungeon and settlement layers fill between the authored
  set pieces.

## W4 — The living world (closes PRD §13 Phases 3 and 4)
Done when clearing a lair measurably changes encounters and prices in coupled regions on the next contact, visible
through rumours on the teller's clock; the main quest is a graph with a static completability proof; generated
quests appear in every generated town.
- [ ] **M10 ecosystem** (`omnis-eco`, ARCHITECTURE §7.2, PRD §9.2): the simplified 4X engine the owner named
  (2026-10-17) driving biospheres, settlements, regions and the world; `RegionState` (populations, factions,
  resources, prosperity, danger, weather, actors, stability); typed `RegionEvent`s; the ordered tick run through
  reconciliation catch-up; derived outputs (encounter weights, prices, rumours, quest availability); the fold
  thresholds of D28; `eco.*` ops and `bench eco`; the biosphere's living things and resources (split from M8.5
  step 5). The lore's factions come first: the Wardens, House Tourien, the merchants guilds, the clans of
  Karinguard, the Van Trahruns.
- [ ] The bus's designed expansions (ARCHITECTURE §4.8), each with its first consumer: a topic hierarchy with the
  first topic below a region (a map section or a city district); saved deferred signals with the first news that
  waits for the party (event rumours or M10's catch-up); pack-declared subscribers with the Rhai `Script` profile.
- [ ] **M11 story engine** (`omnis-story`, ARCHITECTURE §7.3, PRD §9.3): quest graphs with Rhai guards and the
  static check at pack load; templates bound to towns, dungeons and populations; the journal over `QuestState`;
  dialogue as data; story points that fold (D28); `story.*` ops; the editor's graph view.

## W5 — Release (closes PRD §13 Phase 5)
Done when external playtesters complete the opening act and a modder outside the team ships a working pack from
the documentation alone.
- [ ] **M12 base campaign and release**: the campaign's content (whether the Three Kingdoms is its own pack over
  `packs/base`'s SRD material or `base` itself is decided here), the balancing pass, packaging with the
  platform's settings directory, mod documentation, the licensing audit, Windows and Linux CI, the credits.
- [ ] GFDL backgrounds (Farmer, Squire, Metal Worker, Adventurer, Cook in
  `docs/background/opensource_backgrounds_and_skills.md`) as a separate `packs/backgrounds-gfdl` pack with its own
  license text and the GFDL copy; the base pack stays CC-BY-4.0 (owner, 2026-09-19).

## Far horizons (after the W0.5 studies; the owner schedules them)
- **Accounts, multiplayer, multi-platform**: each enters the plan as its study proposes and the owner chooses,
  as a PRD amendment first. Until then every phase keeps what the documents already keep for them: commands carry
  no client state (ARCHITECTURE §4.6); every party is a holder of its own clock (D25); the character save and the
  world file are apart (D28); the simulation crates stay portable by the lint.
- From the PRD's §13 Later, unchanged: free agency for important non-player characters (after M10), the hireling
  sidecar (D10), aging (D13), general scripting for mods on the sandboxed engine.

## Open, outside the phases (the owner sets the order)
- [ ] Party creation: remove or edit a member after adding them (owner, 2026-10-11; found in P3 as "not in P3").
  Today Add member is final: the CreateParty screen can only add, and Back drops the whole unfinished game. Neither
  client could ever do this, and the engine has no command for it (`PartyCommand` is `Create`, `Reorder`,
  `Tactics`). Plan in plan mode when reached; open questions for the owner then:
  - **Where the change lives.** Option 1: engine commands (for example `PartyCommand::Remove { member }`, and Edit
    as remove plus create, or its own command) under protocol 2, additive, with an op, an MCP tool and
    `docs/api.md`. Option 2: the client keeps drafts until Begin and sends the `Create`s only then (no engine
    change, but agents creating through MCP get nothing).
  - **When it is allowed.** Only during creation, before Begin (a new `creating` state in the world, or a rule
    such as "no turns taken yet"), or also later, as dismissing a member in town (gear to the party inventory, as
    permadeath does).
  - **Start again.** `Session::drafts()` re-creates the members from the `Party(Create)` commands in the binder
    log; a removal or edit must change what Start again re-creates (fold the log, or keep the final drafts).
  - **Identity.** A removed member's `CharacterId` is never reused (the vocabulary test checks member keys against
    ids ever held); the slots and marching order close the gap.
  - **The screen.** A member row in the Party list becomes pressable: Edit loads the draft back into the form
    (name, race, class, background, alignment, bought scores, picks; the racial bonus is not part of the draft),
    Remove asks to confirm. The member's draft must be recoverable (from the log, or a `MemberView` field the view
    does not yet carry).
  - Tests: a removed member is gone from the views and from Start again; an edited member keeps their slot; the
    party-full reason clears after a removal; the replays and `SAVE_SCHEMA` 9 do not move (or the save schema
    bumps with a migration if a world state is added).
- Tactics beyond declared reactions (auto play, further runbooks, encounter criteria, the library editor, monster
  runbooks): unscheduled since M7c (PRD §14, `tasks/knowledge/horizons.md`).
- Small findings and unused assets: `tasks/BACKLOG.md`, to review for fixing or removing.
