# A terrain's block face, the editor's form, and tooltips: acceptance

Built 2026-11-13 after your try of rendering size 2 part 3 (`tasks/plans/editor-forms-and-tips.md`).
**Accepted by the owner, 2026-11-13**: "Label brightness is fine and the wording is pretty good for
what I reviewed. Consider this accepted." Decisions 1 and 3 are answered by that; 2, 4 and 5 were not
answered and stay as built. What I saw myself: captures at Low and High of a hatched block
(`target/shots/face/`), and one capture each of the new form and of a tip
(`target/shots/form/terrain.png`, `tip.png`), taken through a temporary change that opened the form
at start and is not in the tree. I have not seen a tip follow a real pointer.

## What you reported
- **"No hatching on the cliff faces in the meadow."** Your meadow has no cliffs (`heights: []`). What
  you saw are the `trees` terrain's blocks. Part 3 made a block with no named role the walls' colour,
  so trees looked like walls. Cliff hatching itself is unchanged.
- **The editor's form**: labels and inputs together, no tooltips.

## 1. A terrain's face (`just editor`, your meadow)
1. Choose the trees' swatch in the palette, press **Edit terrain**.
   *You will see* the form in two columns under the headings IDENTITY, TILES, MOVEMENT, LOOK, MAP COLOUR.
2. Under LOOK, step **face** from `plain` to `hatched`, press **Apply**.
   *You will see* the trees in the 3D preview with upright lines on each side, three a side, as a cliff
   has. Nothing changes on the plan.
3. Save, then `just qa the_lower_kingdoms:map:meadow` and walk to the trees.
   *You will see* hatched boxes. They are still the walls' colour: the face is the difference.
   If you want another colour too, set **role** (for example `ground`).

Your pack was not touched; `face` is yours to set. A file that names no face is plain, as before.

## 2. The form
1. Open any form (a terrain, an encounter on a cell, an object).
   *You will see* each field's name dim on the left, all in one column, and its input in its own box on
   the right, every box starting at the same place. A yes or no is a tick box: one click turns it.
2. Open an encounter and look at **Stacks**.
   *You will see* each row's fields one under another (creature, count) and then **Remove row**. The
   plan said side by side; the column is too narrow for two inputs on one line.
3. Choose a stamp, press the sliders icon.
   *You will see* the properties window's own fields in the same two columns.

## 3. Tooltips
1. Rest the pointer on a field's name or its input for about half a second.
   *You will see* a small box to the left of the row, over the plan, saying what the field means, its
   range and what empty means. It goes when the pointer leaves or you click.
2. Click into a text field.
   *You will see* that field's tip at once, and it stays while you type there.
3. Rest the pointer on a tool icon, a palette swatch or icon, a bar icon, Save, Playtest.
   *You will see* its tip under it (or over it near the window's foot).

## Covered by tests
`tests/geometry.rs` (a hatched block's lines; a plain block is its box), `omnis-data` `map.rs` (a file
without a face is plain and is written without one), `tests/editor_forms.rs` (the form keeps the face),
`tests/editor_layout.rs` (one label column, inputs at one place, groups, the tick box; on a laid-out
window), `tests/tips.rs` (rest, leave, press, the keyboard's field, inside the window and left of the
row, and that every form field and every control of the three columns has a tip). Gate 1326 to the
number in the TODO's review; mutations in each step.

## Decisions for you
1. **The labels' dimness.** They use the kit's dim green. In my capture they read, but faintly. Say if
   they should be brighter.
2. **The keyboard's tip.** A field you type in shows its tip the whole time. If that is noise, it is
   one line to make tips pointer-only.
3. **The wording.** All the tip texts are listed at the foot of the plan document, 140 lines; they are
   my reading of your rules. Mark what is wrong or unclear.
4. **Lists.** One field a line with "Remove row" under them, as built, or something denser?
5. **Other screens.** Settings and the game's own screens keep the old rows and have no tips. The
   facility is the kit's, so either can follow when you want it.
6. Part 3's five decisions (`tasks/acceptance/rendering-size-2-palette.md`) are still open; the
   second one (cliffs and piles sharing a colour) is the same family as the trees.
