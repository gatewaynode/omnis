# Found items land on the ground and the loot window opens

## Context

The owner's try of the pile-reach work (2026-11-13, "Better.") found a gap: taking the loose stone that
hides a gem opens no loot picker, so a player may leave found items behind. Today the object effect
`GiveItem` puts the item straight into the party's stores (`objects/outcome.rs`, `add_to(&mut
world.party.inventory, ..)`, `Event::ItemFound`); the only sign is the log line "Found gem; into the stores".

The owner chose (2026-11-13): **"Drop it, open the loot window"**. An engine rule change: a found item lands
on the ground as a pile, and the loot window opens on it at once (Take, Store, Destroy, who receives).
Closed without taking, it stays as a visible heap with the "Pick up..." button. This is the owner's earlier
direction for the loot window: one window "for various situations where items are available for the party
to take, store, or destroy".

## Design

### A. Engine: `GiveItem` lays the item on the ground (`omnis-sim`)
- New `ground::found(world, data, placement, item, count)` in `ground.rs`, called from `outcome.rs`'s
  `GiveItem` arm in place of `add_to(.. inventory ..)`.
- **Which cell.** The object's anchor cell when it is within the party's pile reach (`ground::cells`: own
  cell, and the faced cell across an open edge); otherwise the party's own cell. The second case is real: an
  object spanning two cells can be used from a cell its anchor is not in reach of (the yard's gate), and a
  found item must never land where the party cannot take it.
- **Where in the cell.** A pile already there takes the item (its place kept). A new pile lies at the
  object's own offset in the anchor cell, or ahead of the party (`DROP_AHEAD`, as a dropped item) in the
  party's cell.
- **The stack limit.** An effect cannot be refused, so as `ground::lay` (a fight's spoils) does, a found item
  is laid even on a pile at `MAX_PILE_STACKS`. Nothing found is lost.
- **The event.** `ItemMoved { item, count, from: Object(<placement id>), to: Ground }`, an existing variant
  with its existing meaning, in place of `ItemFound`. `reshapes()` already redraws the view for it.
  `Event::ItemFound` stays in the enum and the documents, marked "not produced since 2026-11-13": removing a
  variant would bump `PROTOCOL`. No variant is added or removed, so every count pin stays (74 events).
  `docs/api.md` widens `"Ground"`'s words (also "where a found item was laid") and records the change in the
  changelog as a behaviour change of the `GiveItem` effect within protocol 2. **This is a judgement the
  approval covers**: the wire shapes do not change; what one data effect does, does.
- `DevCommand::GiveItem` (a separate path) is unchanged. `GiveGold` is unchanged: gold has no pile.
- No pack file changes (the stone's text "Under the stone lies a gem." fits). The golden replays load no
  object with `GiveItem`; the gate confirms they stay at their pins.

### B. Client: the loot window opens on what was found (`omnis-vector`)
- `Session` gains `found: bool`, set in `take_in` when the events hold an `ItemMoved` from an object to the
  ground (beside `reshape`), and cleared by the system that reads it.
- A system in `shell/aim.rs` (with the click and the "Pick up..." button, sharing `open`): when `found` is
  set and the loot window is allowed (`Nav::allowed(Loot)`: exploring, no base screen), it opens the loot
  window on the ground. It opens over the screen that issued the verb (the verbs screen stays open after a
  verb today); closing the loot window returns there. `shell/screen.rs` (1000 lines) is not touched.
- The log line: "Found gem; on the ground" for an `ItemMoved` from an object to the ground (`shell/text.rs`).
  The object's own message ("Under the stone lies a gem.") is the notice, seen when the window closes; if
  the window can carry the newest line as its message without a new mechanism, it does.

### C. What does not change
The loot window itself, reach, `PickUp`, containers (`contents`), the wire shapes and pins, pack data.

## Steps (one commit each; red tests first this time; mutation checks with a touch after restore; fmt, clippy, the gate against 1301, Sentrux)

0. This plan to `tasks/plans/found-items.md` and into `tasks/TODO.md`.
1. **The engine rule.** Red first in sim `tests/object_act.rs`: taking the stone leaves the stores without
   the gem, a pile with the gem on (0, 2) at the stone's offset, one `ItemMoved` from the object to the
   ground and no `ItemFound`; `PickUp` then takes it; with an item dropped on the cell first, the gem joins
   that pile. A unit test of the cell choice (anchor in reach, anchor out of reach, a full pile). Then
   `ground::found` and the arm. Existing tests that read the old rule are moved to the new one
   (`object_act::a_once_interaction_succeeds_once`, vector `object_lines`, and any in `objects_view` that
   count the ground). Mutations: back into the stores; always the anchor cell; the limit refusing; the wrong
   event.
2. **The window opens.** Red first in vector `tests/pile_reach.rs` (or a new `tests/found.rs` if it passes
   400 lines): the stone taken through the verbs screen opens the loot window listing the gem; Take
   everything puts it with a member and the window closes; taken with Shift+Space, the same; closed without
   taking, the heap is drawn and "Pick up..." stands; an ordinary pick-up or drop opens nothing. Then
   `Session.found`, the system, the log line. Mutations: never opens; opens on any ground move; the flag
   never cleared.
3. **Documents.** `ARCHITECTURE.md` (the effects line and the event list's note; changed under this approved
   plan and flagged in the commit), `docs/api.md` (`ItemFound`, `"Ground"`, the changelog),
   presentation-ARCHITECTURE (the found line), code-map, `tasks/acceptance/found-items.md`, TODO review,
   `tasks/LESSONS.md` (items that reach the party go through the loot window; found by the owner's try, not
   by me), `tasks/CONTINUITY.md`.

Reported as "built, please try".

## Verification
- Per commit: `cargo fmt --all`; workspace clippy (`--all-targets --features omnis-vector/test-fixtures
  --locked -- -D warnings`); the sim lint (no floats, no hashed maps) is part of the gate; `scripts/verify.sh`
  to a scratchpad log, committed only on `VERIFY-GREEN`, the count read as a number; Sentrux `rescan` +
  `check_rules`.
- By hand (owner), `just qa`: at the start, Actions, the loose stone, Take. The loot window opens with the
  gem. Take it, or close: a heap lies where the stone was and "Pick up..." stands at the bottom.

## Risks, said plainly
- **A rule change in the engine**: a pack author's `GiveItem` no longer fills the stores. Only the objects
  fixture uses it today; the owner's world has no objects yet.
- **`ItemFound` is left unproduced** to avoid a protocol bump. If the owner prefers a clean removal with
  `PROTOCOL` 3, that is a separate, larger change (every pin).
- **Over the verbs screen**: I have not yet read how `Nav` stacks two overlays; if the loot window cannot
  open over the verbs screen, the verb's screen is closed first, and I say so in the report.
- Not seen in a window by me: checked headless, and by a capture where one can show it.

## Not in this plan
Gold as a pile; a container's `contents` opening the loot window on Open (still open with the owner); a
count asked for Store and Destroy; removing `ItemFound`; the owner's uncommitted edits to
`packs/the_lower_kingdoms` stay out of every commit.
