# Marching order, the look that comes back, Push, and doorways that fall in

## Context

The owner's hand test, 2026-11-24, of step 9 (doors) and the thunderwave push. Both work. Four things came out
of it, with the owner's answers to this plan's questions:

1. **No way to change the marching order.** The engine has had `PartyCommand::Reorder` since M7 (refused only
   in a fight, `WrongMode`); no screen sends it. The wizard cannot be put in front to cast thunderwave.
2. **Mouse look is lost after every screen.** One bool (`Intent.looking`) and nothing that remembers it was
   on. Owner's choice: **it comes back after every screen and after a fight**; a left click in the view is
   the deliberate way to turn it off.
3. **A "Push" spell** (1st level wizard, ranged, one individual, 50 ft back). Owner's choice: **add it to the
   base game** as well as the order screen. It is not an SRD spell: a departure that adds, at the owner's word.
4. **A broken door shows no debris, and a frame can come down.** Owner's choices: **25% by default** that
   breaking a door buries the doorway (a map sets its own figure); **rubble is cleared with time**.

Four commits, in this order (the first two unblock the owner's testing). Each: tests red first, mutation
checks, fmt, workspace clippy, the gate to a log, Sentrux. Steps 10 to 13 of combat in feet follow after.

## 1. The look comes back (`crates/omnis-vector`)

- `shell/movement.rs`: `Intent` gains `resume: Resume` (`No`, `Pending`, `Yes`). The pure rule `look` takes and
  returns both (it stays unit-tested):
  - a screen, the fight or the editor covers the view while looking, or while `Pending`: looking off, `Yes`;
  - Esc, Space or a right click while looking: looking off, `Pending` (the screen those keys open is seen one
    frame later, because `Showing` is written after `Grab`); the next frame with nothing covering: `No` (a
    key that opened nothing, such as Shift+Space running its default action, stays a release);
  - nothing covers and `Yes`: looking on again, the cursor locked, `No`;
  - a left click in the view while looking: off, `No` (deliberate).
- Cleared (`No`) when a game is loaded, started or left for the title, and when the editor is entered, so a
  loaded game never starts with the pointer taken.
- `hud.rs` hint becomes "Mouse look on: a click gives the pointer back" (Space and Esc now only lend it).
- Tests: `movement.rs` unit cases for each arm; `tests/overlay.rs` (camp opened by R while looking, closed:
  looking and `Locked` again; the same through a fight's `ViewState`); `tests/loot.rs:690` flips (Esc, Esc
  ends looking); a click-off then a screen does not turn it on; a load clears it.
- `presentation-ARCHITECTURE.md` lines 353 and 556.

## 2. The marching order screen (`crates/omnis-vector`; no engine change, no pin moves)

- `screen/order.rs`, modelled on `screen/camp.rs`: `ScreenId::Order`, a modal. Rows in marching order under
  "front" and "back" notes (from `MemberView.in_front`): the member's name, class and level, then **Up** and
  **Down** (`ControlId::Shift { member: CharacterId, up: bool }`), dim at the ends. A press sends
  `Command::Party(PartyCommand::Reorder { order })` with two neighbours swapped
  (`Effect::Command { close: false }`), so the list and the party column redraw at once. Close button.
- Mouse entry: an **Order...** button at the foot of the party column (`screen/roster.rs::content`,
  `ControlId::Order`), dim with the engine's reason from `world.trial` in a fight. `nav.allowed`: wherever the
  engine accepts `Reorder` (the road and a town).
- Tests: `tests/order.rs` (pure: rows, dim ends, a press gives the swapped order, a member crossing the line
  changes group; app: the column's button opens it, a press moves the member in `party_view`, dim in a fight).
- Documents: presentation-ARCHITECTURE's screen list, code-map.

## 3. Push (`crates/omnis-data`, `crates/omnis-sim`, `packs/base`)

- `packs/base/data/spells/push.ron`: level 1, wizard, action, `Save(ability: Strength, dice: none,
  push_feet: 50)`, `reach: One`, `targeting: (range: Tiles(12), sees: true)` (60 ft). Text keys in base's
  language file. No damage.
- `spell.rs`: `MAX_PUSH_FEET` 30 to 60; a `Save` with a zero-count die is allowed when `push_feet > 0` (today
  refused), and `cast_save` / the monster arm skip the damage roll and its `Damage` event for it.
- The push itself is the code built for thunderwave (`combat/push.rs::thrown`): `Reach::One` catches the
  stack's nearest individual; failing, it is split off 50 ft back (or the stack moves when it is alone).
- Base resealed (0.1.6), the two golden replays rebaselined and diffed (hash and fingerprint only).
  **At this reseal `monster_front_stacks` leaves base's `combat.ron`** and its assertion in
  `load_base_pack.rs` (owed since step 3); rule-count pins follow. If saves number spells as they do items,
  the spell goes in the since-names list in `save/names.rs` in the same change (checked first).
- PRD §8.3's departures table gains the line (owner's direction, flagged in the commit).
- Tests (`tests/push.rs`): a failed save splits one individual 50 ft back with no `Damage` event; a pass does
  nothing; a lone individual moves whole; out of range refused; data: zero dice without a push refused, 65 ft
  refused. Spell-count pins updated.

## 4. Doorways that fall in (`crates/omnis-data`, `crates/omnis-sim`, `crates/omnis-vector`)

Engine, with the words, action and redraw in the client in the same commit; the line art in a second commit.

- **Data**: `DoorMake.collapse_percent: u8`, default 25, at most 100 (checked at load), unwritten at the
  default. Set in a map's file; no editor field (as `tile_feet`).
- **State**: `DoorWear.collapsed: bool` (serde default, unwritten when false; `SAVE_SCHEMA` stays 9, the
  journal's `Change::DoorWear` carries it already).
- **The roll**: in `breaking::strike`'s door arm where the door breaks, d100 on the stream in hand (`strike`
  or `cast`) against the map's percent; no draw at 0. `Event::DoorCollapsed` after `DoorBroken`.
- **Blocking**: `MapState::door_open` is false for a collapsed edge, which closes steps, sight, reach and the
  client's collision and fills in one place. `apply::landing` says `BlockReason::Rubble` (new). Everything
  that treats a broken door as nothing to open, shut, strike or hand treats a collapsed one the same.
- **Clearing**: `Command::Clear` (the faced edge; 18 commands): takes `rubble_clear_minutes` (rules value in
  base's `objects.ron`, 60; in the reseal of commit 3 if it lands first, else its own) through
  `apply::advance`, so the time rolls what time rolls; then `collapsed` is false and the doorway stands open
  for good. `Event::DoorCleared`. `Rejection::NothingToClear`. `Interact` offers it (`Choice::Rubble`).
- **Views**: `EdgeView::Door` gains `collapsed`; `map_text` a glyph; `query::door_collapsed`.
- **Client words**: Actions row "Clear the rubble (60 min)"; log "The frame gives way: the doorway is
  buried", "The rubble is cleared"; the step's refusal "Rubble blocks the way"; `session::reshapes` gains
  both events.
- **Line art** (`geometry.rs::edge`, second commit):
  - *broken, passable*: the posts snapped at unequal heights with splintered ends, a hanging fragment at the
    head, and planks lying across the sill on both sides (short lines, placed by the edge's coordinates so
    they never flicker);
  - *collapsed*: the opening filled, not cut (`fill.rs`): a mound outline to about two thirds of the height,
    boulder shapes within it, the lintel fallen across it at a slant;
  - the minimap (`minimap.rs`) reads the live state of known door edges: a broken door is drawn as a gap in
    its bar, a collapsed one as a crossed bar in the danger colour. This is the cave-in marker on the map.
- Pins: 18 commands, **84** events, **78** rejections, `schema_proof.rs` counts, `api_doc.rs`, ARCHITECTURE,
  `docs/api.md` (events, `EdgeView`, `Clear`, changelog). PRD §8 gains the collapse rule (owner's direction).
- Tests (`tests/breaking_doors.rs`, fixture maps with `collapse_percent` set in memory to 0 and 100): at 100
  a broken door is collapsed, blocks the step with `Rubble`, hides what is beyond, is refused by Door, hand
  and Attack; at 0 no roll is drawn (stream position pinned); `Clear` takes the minutes and opens the way;
  nothing to clear refused; save and journal round trip; a seed at 25 for each outcome. Client:
  `geometry.rs` segment tests for both pictures, the fill not cut, the Actions row, the log lines, minimap.
- Existing door tests break doors at the default 25%: those that then walk through set 0 first.

## Verification

- Gate green after each commit, mutation runs per commit (the look's arms; a swap's direction; push without
  a save; collapse roll ignored, drawn at 0, sight through rubble, clearing free).
- Captures of the two doorway pictures at Low and High if a scratch save can stand the party before them;
  otherwise said as not seen by me.
- Owner's try in `just qa`: order the wizard to the front and cast thunderwave; cast Push from range; break
  the torch's door several times over reloads to see both outcomes, then clear the rubble; open and close
  screens and a fight with the look on.

## Said plainly

- **The front row's size stays a rule** (`front_row`, 3): the order screen changes who is in it, not how many.
  A party-chosen split (two and two) is an engine change; say if you want it.
- Push has no damage and catches the stack's nearest individual: individuals in a stack cannot be picked
  anywhere in the game. A monster caster given Push would drive the whole line back 50 ft.
- With 25% by default, every existing map's doors can now bury their doorway, the owner's town included.
- A collapse blocks sight too. Clearing is uninterrupted time: an encounter the hour rolls comes after.
- Rubble cannot be repaired into a door; step 10 (repair) will say what it does with a cleared doorway.
- Esc and Space no longer turn the look off for good; only a click does.
- The minimap learns of a break or a collapse at once, since only the party's own blow causes one.
