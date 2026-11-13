# The loot window opens on fight spoils and on an opened container

## Context

The owner's direction for the loot window is one window "for various situations where items are available
for the party to take, store, or destroy". On 2026-11-13 the owner found that a found gem opened no window
and chose that finds open it at once; that is built and tried ("Much better."). The lesson written then:
list every way the party gains items and say for each whether it goes through the window. Two do not yet:

- **Fight spoils.** A won fight lays the dead's items on the party's cell (`ground::lay`) with no window;
  the only signs are the Victory line and the heap underfoot.
- **Containers.** An opened chest has no way into the loot window at all; nothing in the client ever opens
  it on an object. Its contents are reachable only from the Inventory's "Take out" rows.

The owner asked for this pass (2026-11-13: "Yes, plan the fight spoils and containers pass").
Outcome: the window opens by itself after a won fight that left items, and when a container is opened with
something in it; an open container can be gone back to from the Actions menu. **Client only**: no engine
rule, wire shape, pin or pack changes.

## Every way the party gains items or gold, and what this plan does with it

| Path | Where it goes today | After this plan |
|---|---|---|
| Object `GiveItem` | A pile; the window opens | Unchanged (built 2026-11-13) |
| A pile on the map, a dropped item | The floor click, "Pick up...", Actions | Unchanged |
| **Fight spoils, items** | A pile underfoot, no window | **The window opens after the fight** |
| **A container's contents** | Inventory "Take out" only | **The window opens when it is opened; "Take from ..." in Actions** |
| Fight gold, object `GiveGold` | Straight to the party's gold, a log line | Unchanged; gold has no pile (open with the owner) |
| A dead member's kit (permadeath) | Into the stores, silently, at the fight's end | Unchanged; named here for the owner, not changed |
| Shop Buy, tavern food, bank | The stores, food or gold, at the counter | Unchanged: the player asked for it by name |
| Party creation's starting kit | The member's kit | Unchanged |

## Design

### A. One place that opens the window: `shell/loot.rs` (new; moved out of `shell/aim.rs`)
- `shell/aim.rs` keeps the pointer (the aim, the floor click). The window's openers move to a new
  `shell/loot.rs`: `open`, the `found` system, the "Pick up..." button, and from `shell/screen.rs` the
  Shift+Space handler `action_keys` (a move, no behaviour change), which brings `shell/screen.rs` under its
  1000 lines before anything is added there.
- `Session.found: bool` becomes `Session.offer: Option<ItemPlace>`: the place the window should open on.
  `open(screens, views, from)` sets `forms.loot = Loot { from, ..default }` and opens `ScreenId::Loot`
  (`Forms::shown` already keeps the form when the new top screen is the loot window).

### B. Fight spoils
- `session::finds` also answers for `CombatEnded` with `outcome: Victory` and a non-empty `loot`: the offer
  is `Ground`.
- The `found` system waits while the view is still the fight's (`ViewState::Fight` lasts one more frame
  after the winning command) and opens the frame the party is back on the map. The Victory line is in the
  log and stands as the notice once the window closes.
- A fight that left nothing, a flight and a defeat open nothing.

### C. Containers
- In `Session::take_in`, after the views are refreshed: an `ObjectState` event whose object now shows
  `contents` (the engine's own "open container", `ObjectView.contents: Some`) with something in them sets
  the offer to `Object(id)`. Closing one, or opening an empty one, sets nothing: an empty chest says so in
  the Inventory as today, and the log has its "is now open" line. A find in the same command wins (`Ground`).
- **The way back in.** The Actions menu gains "Take from <name>..." for each open container in reach that
  holds something, after "Pick up...". `Does::Loot` carries the place (`Does::Loot(ItemPlace)`), and
  `Effect` gains `Loot(ItemPlace)` carried out through `shell/loot.rs::open`; "Pick up..." and Shift+Space
  use the same path with `Ground`.
- The window itself already lists a container, takes with `TakeOut`, stores and destroys, and closes when
  the container empties or shuts (`loot::place`, `gone`).

### D. What does not change
The engine, the events, `docs/api.md`, the packs, the Inventory's rows, gold, the click on an object in the
3D view (objects are not click targets yet).

## Steps (one commit each; red tests first; mutation checks with a touch after restore; fmt, clippy, the gate against 1306, Sentrux)

0. This plan to `tasks/plans/loot-everywhere.md` and into `tasks/TODO.md`.
1. **The move.** `shell/loot.rs` from `shell/aim.rs` and `action_keys` from `shell/screen.rs`; `found` to
   `offer: Option<ItemPlace>`. No behaviour change: the existing tests (`found.rs`, `pile_reach.rs`,
   `loot.rs`) pass unchanged but for import paths.
2. **Fight spoils.** Red first in `tests/found.rs`: on the yard, the party meets the placed pack rats
   (5, 1), wins by the fight's own commands, and the loot window is up on the spoils once the view is the
   map's; Take everything empties the pile. A fight that leaves nothing (the test dungeon's giant rats) and
   a flight open nothing. A unit test of `finds` over built events (victory with loot, victory without,
   fled, defeat). Mutations: any `CombatEnded` opens; never opens; opens during the fight frame.
3. **Containers.** Red first: the chest at (5, 3) opened by its own verb opens the window titled with the
   chest's name, listing the dagger; Take moves it and the emptied window closes itself; closed without
   taking, Actions lists "Take from Chest..." and it opens the same window; shut again, the entry is gone;
   a torch's state change opens nothing; an empty open chest opens nothing and has no entry. Mutations: any
   `ObjectState` opens; the entry offered for a shut or empty container; the entry opening the ground.
4. **Documents.** presentation-ARCHITECTURE (the loot window's openers), code-map,
   `tasks/acceptance/loot-everywhere.md`, TODO review, `tasks/CONTINUITY.md`.

Reported as "built, please try".

## Verification
- Per commit: `cargo fmt --all`; workspace clippy (`--all-targets --features omnis-vector/test-fixtures
  --locked -- -D warnings`); `scripts/verify.sh` to a scratchpad log, committed only on `VERIFY-GREEN`, the
  count read as a number; Sentrux `rescan` + `check_rules`.
- By hand (owner), `just qa`: walk to the chest (east side of the yard), Actions, Use..., force or pick it:
  the window opens on the chest. Close it; Actions shows "Take from Chest...". Walk into the pack rats and
  win: the window opens on what they dropped.

## Risks, said plainly
- **Winning a fight in a headless test with real commands** depends on the dice. I will pin a seed where
  the rats die and drop something; if none of a small range does, I say so and fall back to building the
  fight as the sim's own loot tests do (rats at 1 hit point).
- **The frame the fight ends**: the window must not open over the fight screen while it is being taken
  down. Covered by a test that the top screen is not the loot window while `ViewState` is still `Fight`.
- **Two windows wanted at once** (a chest whose opening also gives an item): the find on the ground wins;
  the chest is one press away in Actions.
- **`shell/screen.rs`** is at 1000 lines; step 1's move comes first so later steps do not break the limit.
- Not seen in a window by me; checked headless.

## Not in this plan
Gold as a pile; the dead member's kit going to the stores unseen; clicking an object or a chest in the 3D
view; a count asked for Store and Destroy; the object's message on the loot window; the owner's uncommitted
edits to `packs/the_lower_kingdoms` stay out of every commit.
