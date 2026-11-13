# A ground pile that is easy to get to

## Context

The owner's hand test of 2026-11-13 found the ground pile hard to reach, and named two causes:
- **The click target is too small.** Today the target is the drawn heap's screen rectangle (a quarter of a
  cell across, an eighth high) grown by 12 pixels (`aim.rs::pile_under`, `MARGIN`).
- **A pile underfoot is unseen.** On the pile's own cell the heap is below the view; nothing on the screen
  says it is there or offers a way in, short of the Actions menu or Shift+Space.

Nothing in the 3D view shows what the pointer is on except the hand cursor, and the exploring HUD has no
control that appears only when it applies (Explore report: `shell/controls.rs`, `shell/hud.rs`,
`shell/notice.rs`).

Outcome: a pile in the faced cell is clicked anywhere on that cell's floor, and the floor lights up under
the pointer; whenever anything on the ground is in reach, a "Pick up..." button is on the view. The engine's
reach rule, the loot window and every wire shape stay as they are. Client only (`omnis-vector`).

## Design

### A. The click target: the pile's cell floor (`aim.rs`, Bevy-free)
- New `aim::floor(eye, size, ground, x, y) -> Option<[(f32, f32); 4]>`: the four corners of cell `(x, y)` at
  the ground's own heights, projected with the existing `aim::project`; `None` when a corner is behind the
  eye (always so for the party's own cell).
- New `aim::inside(point, quad) -> bool`: point in a convex quad (same-sign cross products).
- `pile_under` answers yes when the point is in the heap's grown rectangle (as today) **or** inside the
  floor quad of the pile's cell. The own cell keeps today's behaviour (the heap, when looked down on).
- Effect in play: with a pile in the faced cell, a left click on that cell's floor opens the loot window
  instead of taking the mouse look. A click anywhere else is the view's, as now. With the mouse look on,
  the middle of the view stands for the pointer, as now.

### B. The highlight: the cell's floor outline (`shell/aim.rs`, `shell/render.rs`, `shell/lines.rs`)
- `Aimed(pub bool)` becomes `Aimed(pub Option<(u16, u16)>)`: the cell of the pile under the pointer.
- A third line entity, `ViewAim`, spawned in `render.rs::spawn_lines` beside `ViewCrests` with the same
  `lines::bundle` and material. A system after `follow` hangs a mesh on it when `Aimed` changes, by the
  pattern of `lines::hang_crests` (replace in place, remove when there is none): four lines, the aimed
  cell's floor square inset by an eighth of a cell, at the ground's heights, in the pile's colour, solid,
  normal weight (`Strokes::push` with `PLAIN`).
- A separate outline, not a restyle of the heap: the scene mesh is not rebuilt on hover, and two lines in
  one place would fight over depth. It also shows exactly where a click counts.

### C. The prompt: a "Pick up..." button on the view (`shell/aim.rs`)
- One button, bottom centre, above the HUD's bottom row, built with `controls::button`; its marker a new
  `PickUp` component. The label is "Pick up...", the Actions menu's own words.
- Shown and pressable while the party explores, no screen is up, no fight, not the editor, and
  `session.views.objects.ground` is not empty (the same condition the Actions menu's entry uses, so it
  covers the own cell and the faced cell). Otherwise `Visibility::Hidden` and not pressable (the pattern of
  `controls::light`, so a hidden button takes no click).
- A press opens the loot window the way the click does (`screens.forms.loot = Loot::default();
  nav.open(ScreenId::Loot, ..)`), shared in one function with the click.
- No new key: Shift+Space (the default action) and the Actions menu stay.

### D. What does not change
`piles_in_reach`, `PickUp`, the loot window, the sim crates, the wire shapes, the pins. `shell/screen.rs`
(1000 lines) is not touched.

## Steps (one commit each; red tests first, mutation checks with a touch after restore, fmt, clippy, the gate against 1295, Sentrux)

0. This plan to `tasks/plans/pile-reach.md` and into `tasks/TODO.md` as checkable items.
1. **The floor as the target.** `aim::floor`, `aim::inside`, `pile_under`. Tests in a new
   `tests/pile_reach.rs` (`tests/loot.rs` is at 886 lines), on the yard fixture (pile at (2, 2), party at
   (1, 2) east): a floor point of the faced cell away from the heap is on the pile; a point on the cell
   beyond, on a wall above it and beside it are not; a pile underfoot at level pitch is not hit and is hit
   on the heap when looking down; in the app, a click on the floor opens the window and a click outside the
   cell takes the mouse look. The existing "a click beside it is the view's" test moves its point outside
   the cell if it now falls on the floor. Mutations: the floor dropped from `pile_under`; the quad test
   always true; the wrong cell's floor.
2. **The highlight.** `Aimed` as the cell, `ViewAim`, the mesh. Tests: hovering the floor hangs a mesh of
   four lines on the aimed cell, plain style; moving off removes it; a screen opened over it removes it.
   Mutations: the mesh never removed; the outline on the party's cell. Looked at in offscreen captures
   cannot show a hover (no pointer), so this one is checked headless and by the owner's hand.
3. **The prompt.** The button and its system. Tests: standing on the pile's cell it is visible and
   pressable and a press opens the loot window; facing the pile likewise; with nothing in reach, with a
   screen up, and in a fight it is hidden and takes no press; after Take everything it goes. Mutations: the
   condition always true; hidden but still pressable; the press opening nothing.
4. **Documents.** presentation-ARCHITECTURE where the pointer on a pile is described (kept current; if
   presentation-PRD's text on the click needs a word, it is changed under this approved plan and flagged in
   the commit); code-map; `tasks/acceptance/pile-reach.md`; TODO review; `tasks/CONTINUITY.md` rewritten.

Reported as "built, please try".

## Verification
- Per commit: `cargo fmt --all`; workspace clippy (`--all-targets --features omnis-vector/test-fixtures
  --locked -- -D warnings`); `scripts/verify.sh` to a scratchpad log, committed only on `VERIFY-GREEN`, the
  count read as a number; Sentrux `rescan` + `check_rules`.
- By hand (owner), `just qa` (the yard): walk to face the arrows; the pointer anywhere on their cell's floor
  is a hand and the floor's outline lights; a click opens the loot window; "Pick up..." is at the bottom of
  the view. Step onto the pile's cell: the button is still there and opens the window. Walk away: it goes.

## Risks, said plainly
- **A click on the faced cell's floor no longer takes the mouse look while a pile is there.** The outline
  and the hand say so before the click; if it gets in the way, the target shrinks to a part of the cell.
- **The highlight is not seen by me in a window**: no capture has a pointer. Headless tests pin the mesh;
  its look (colour, inset) is the owner's to judge.
- **The button's place** (bottom centre) is my choice; it must not cover the movement pad or the side
  buttons at 1280 pixels. Checked by a capture standing on the pile.
- A pile underfoot is still not clickable in the view at level pitch; the button is its way in.

## Not in this plan
A longer reach or walking to a far pile; the window opening by itself on a step; objects and containers as
click targets (the open container's way into the loot window stays open with the owner); a count asked for
Store and Destroy; the owner's uncommitted edits to `packs/the_lower_kingdoms` stay out of every commit.
