# Piles you can find and take (a loot window), and a ceiling that stays flat over a dip

Status: **approved 2026-11-12, not started** (on `W-2_look_and_engine`).

## Context

From the owner's hand test of Bevy 0.20 (2026-11-12), two reports that are not the migration's:

1. A ground pile placed in the editor (arrows at dungeon (4, 23)) could not be found or used: "it should show up
   in search and be clickable". Today a pile is listed by Search only when the party stands on it, is picked up
   only from the Inventory screen and only from the party's own cell, and nothing in the 3D view can be
   clicked.
2. A cell with a negative height dips its ceiling too (and only half of it: its north and west lines, while
   its south and east lines are the neighbours'). "Probably not what we want (though future ceiling height
   controls are worth a note in the backlog)."

Owner's choices (asked 2026-11-12): the click opens **a loot window, reusable "for various situations where
items are available for the party to take, store, or destroy"**; a pile is in reach **as objects are** (the
party's cell and the cell it faces across an open edge); the ceiling is **flat over dips only** (a raised
cell still lifts its ceiling).

## What was found

- **Search** (`omnis-sim/src/objects/notice.rs:78`) rolls once and marks hidden object placements in `reach`
  (`objects/act.rs:149`: own cell, plus the faced cell unless a wall, shut door or cliff). Piles are never
  hidden and no event names one. The client's Search lines (`omnis-vector/src/shell/text.rs` `searched()`)
  are written from the views and already say "On the ground: ..." from `ObjectsView.ground`, which is the
  party's own cell only (`objects_view.rs:135`).
- **PickUp** (`omnis-sim/src/ground.rs:68`) takes from the party's own cell only (`here()`); `pile_at` takes any
  cell. Any member not dead or down may take. No carry limit.
- **Nothing in the 3D view is clickable.** A left click toggles mouse look (`shell/movement.rs` `look`,
  `grab_cursor`); the right button and Space are the action key (`shell/screen.rs:499`). A pile is drawn as a
  heap a quarter cell across (`shapes.rs` `PILE_LOOK`, `Body::of`), on `ground.patch(x, y).centre`.
  The camera is `pose` + an eased eye height, yaw and pitch, vertical FOV 0.96 (`shell/render.rs`).
- **The Inventory** already lists the pile and each open container in reach with Pick up / Take out
  (`screen/inventory.rs` `here()`, `counted_row`), and `ItemPlace { Member, Stores, Ground, Object(String) }`
  (`event.rs:209`) already names every place an item can be.
- **The ceiling** is `roof = p.centre + h` in `geometry.rs:398` (lines) and `fill.rs:66` (fills); a wall's top
  is the higher side's centre + h (`geometry.rs` `span`). A cell's height includes its level's base
  (`omnis-data/src/levels.rs`). No test pins a ceiling over a non-flat cell and no fixture has a dip.

## Design

### A. The ceiling (client only)
`roof = max(p.centre, base) + h`, where `base` is the level's own base height (0 on the ground level,
`map.def.levels[i].base` above and below, carried on the client's `Ground`). One function, used by the
ceiling lines, the ceiling fill, the block box and a wall's top (`span`), so a dip has no gap between its
walls and the ceiling. A raised cell is unchanged. `presentation-ARCHITECTURE.md` §8 and
`tasks/plans/m8.5-elevation-drawn.md` say "flat at the cell's height + 1.0": reworded. BACKLOG gets "ceiling
height controls" (a per-cell or per-room ceiling height in the map data and the editor).

### B. A pile is in reach as an object is (engine, additive)
- `ground::pick_up` takes from the piles in `reach`: the party's cell first, then the faced cell. `drop` still
  lays down on the party's own cell.
- `ObjectsView.ground` becomes the items of the piles in reach (same field, same type; its doc and
  `docs/api.md` say so, with a changelog entry; `PROTOCOL` stays 2: no shape changes). Search's "On the
  ground: ..." line and the Inventory's rows follow with no further change.
- `Rejection::NotOnGround` now means "no pile in reach holds it"; its words change to "there is no {item}
  within reach on the ground".
- No new `Event` and no new `Choice`: nothing on the wire moves. The golden replays pick nothing up; if a
  fingerprint moves the step stops and says why.

### C. The loot window (client)
A screen of the kit, kind-agnostic about where the items are: it is opened on an `ItemPlace` and offers what
the engine can do with that place.
- `screen/loot.rs` (pure, like `screen/inventory.rs`): `Loot { from: ItemPlace, receiver }` on `Forms`;
  `ScreenId::Loot`, a modal over the view (the game holds as under any screen). Rows: each item with its
  count, **Take** (one) and **Take all of it**; a foot row **Take everything** and **Close**; a chooser
  for who receives (the party's members able to act; starts on the column's selected member, else the first
  able). Dim buttons carry the engine's reason through `trial::why`, as everywhere.
- Sources in this version: `ItemPlace::Ground` (the piles in reach; `ItemCommand::PickUp`) and
  `ItemPlace::Object(id)` (an open container in reach; `ItemCommand::TakeOut`), reusing `inventory.rs`'s
  `counted_row` and command building, which move to a shared helper rather than being copied.
- It closes by Close or Esc, and by itself when the place is empty or no longer in reach.
- **Ways in** (mouse first): a left click on a pile in reach in the 3D view; "Pick up..." in the Actions
  menu when a pile is in reach (so the action key reaches it and Shift+action takes it as the default when it
  is the only thing there); the Inventory keeps its rows.
- **The click**: a Bevy-free `aim.rs`: project a pile's box (from `shapes::Body`) through the pose, eye
  height, yaw, pitch and FOV to a screen rectangle; `aim::pile_under(point, viewport, eye, piles in reach)`.
  `grab_cursor` asks it before toggling mouse look: a click on a reachable pile opens the window and is not
  the view's. With mouse look on, the point is the screen's centre. While the pointer is over a reachable
  pile the cursor is the pointing hand (`CursorIcon::System(SystemCursorIcon::Pointer)`), so what can be
  clicked shows it. Only piles in reach are tested (two cells at most), so walls need no occlusion test.

### D. Store and destroy (engine commands, then two buttons)
The owner's "take, store, or destroy". Neither exists for a pile or a container today.
- `ItemCommand::Stash { from: ItemPlace, item, count }`: from the ground or an open container in reach into
  the party's stores (`from` is `Ground` or `Object(id)`; any other place is refused).
- `ItemCommand::Destroy { from: ItemPlace, item, count }`: the items are gone (`Event::ItemDestroyed { item,
  count, from }`). The window asks "Destroy N x? This cannot be undone." at the press, and the button is
  dim for an item the data marks as kept (quest items, if `ItemDef` has such a mark; checked at the build
  and said in the report if it has none).
- Each new command and event moves the pinned lists (CONTINUITY's checklist: `api_doc.rs`, `schema_proof.rs`,
  `common/commands.rs`, `word.rs`, `labels::refused`, `text.rs`, `docs/api.md`, ARCHITECTURE §4.2, code-map,
  MCP `schema.rs`). `Event` goes 73 to 74; `ItemCommand` gains two variants; `PROTOCOL` stays 2 (additive).

## Steps (one commit each; red tests first, mutation checks, fmt, clippy, the gate against 1262, Sentrux)

Before step 1: this plan into `tasks/TODO.md` (W2, "from the hand test of 2026-11-12") as checkable items.

1. **The ceiling.** `Ground` carries the level's base; one `roof` function used by `geometry.rs`, `fill.rs`
   and `span`. Tests on a scratch map with a dip made through the editor's Height paint (the fixtures have
   none and the frozen fixture is not edited): the dip's ceiling lines and fill stand at the level's height;
   the walls around a two-cell dip meet the ceiling; a raised cell's ceiling is where it was; the flat-map
   oracle (`tests/geometry.rs:192`) is untouched. Mutations: the clamp removed in the lines, in the fill, in
   `span`. Docs and the BACKLOG note in the same commit.
2. **Reach.** `pick_up` and `ObjectsView.ground` over `reach`. Tests (fixture `objects:map:yard`, pile at
   (2, 2)): standing beside the pile and facing it, the view lists it and PickUp takes it; facing away it does
   not; a wall or shut door between refuses; own cell first when both hold the item; the Search lines name a
   faced pile. Mutations: the faced cell dropped; the edge check dropped; the order reversed.
3. **The loot window and its ways in.** `screen/loot.rs`, `ScreenId::Loot`, `aim.rs`, the Actions entry, the
   click and the cursor. Tests: the screen's rows and dim reasons from the pure function; Take, Take all and
   Take everything send the commands and the window closes when the pile is empty; it closes when the party
   turns away; a container's contents through the same window; `aim` unit tests (a pile ahead projects to a
   rectangle around the screen point where it is drawn; a point beside it misses; behind the camera never
   hits); in the app: a click on the pile opens the window and does not toggle mouse look, a click beside it
   toggles mouse look as before; every text measured in the harness that lays the screen out (lesson
   2026-10-31). Mutations: the hit test always true / always false; the reach filter dropped; the receiver
   ignored; the auto-close dropped.
4. **Store and destroy.** The two commands and the event with every pin, then the two buttons and the
   Destroy question. Tests per command: every refusal (not in reach, not that many, a shut container, in a
   fight), the stores' count, the pile saved as changed; in the window: Store moves to the stores, Destroy
   asks first and Cancel changes nothing.

Then `tasks/acceptance/loot-and-ceiling.md` (numbered, each "you will see" from a test), TODO ticked with a
review line, `tasks/CONTINUITY.md` rewritten. Reported as "built, please try", step by step if the owner
prefers to try the ceiling and the click before step 4.

## Verification
- Per step: `cargo fmt --all`; workspace clippy (`--all-targets --features omnis-vector/test-fixtures --locked
  -- -D warnings`); `scripts/verify.sh` to a scratchpad log, committed only on `grep -q VERIFY-GREEN`, the count
  read as a number; `lint-sim` clean (no floats in the sim: `aim.rs` lives in the client); the replay pins
  walk `d48a913cec6c5735`, fight `9f52f48c5364e17f` unmoved; Sentrux `rescan` + `check_rules`.
- By hand: `just run --windowed --pack packs/base --pack packs/the_lower_kingdoms`. In the dungeon walk to
  (4, 22) facing south (or stand on (4, 23)): Search says "On the ground: Arrows x1"; the cursor becomes a
  hand over the heap; a click opens the loot window; Take everything empties it, the heap is gone, the log
  says who picked up what. Space, "Pick up..." does the same. At (5, 22), the cell at height -2: the ceiling
  is level with its neighbours' and the walls reach it.

## Not in this plan
- Hidden piles (a pile Search must find): a pile has no `hidden` field; an object kind with a hide
  difficulty is the way today. Noted in BACKLOG with the ceiling controls.
- Loot from a won fight going through the window (it goes to the kits and stores as now).
- The owner's uncommitted edits to `packs/the_lower_kingdoms` are theirs and stay out of every commit.
