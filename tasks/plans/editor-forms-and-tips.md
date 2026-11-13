# A terrain's block face, the editor form's layout, and tooltips

Approved by the owner 2026-11-13 (plan mode). The tip texts are listed at the foot of this document when
step 4 writes them (one table, `crates/omnis-vector/src/editor/tips.rs`).

## Context

The owner tried rendering size 2 part 3 (2026-11-13) and reported two things.

1. **"No hatching on the cliff faces in the meadow."** The meadow has no cliffs (`heights: []`). What the
   screenshot shows are the `trees` terrain's solid **blocks**. Before part 3 a block took its terrain's
   colour (dark green); since part 3 a block with no named role is *structure*, so trees are drawn exactly
   like walls and read as rock faces. Cliff hatching itself still draws (capture of `heights:map:hills`).
   My report named the merges of floors, cliffs and piles but not blocks; that goes to `tasks/LESSONS.md`.
2. **The editor's form in the right column is unclear**: label and input share one bordered box in one
   colour, inputs start at different places, and nothing explains a field.

The owner's decisions (asked in plan mode, 2026-11-13):
- **Blocks**: a terrain picks its block's face (plain box, or hatched like a cliff).
- **Layout**: two aligned columns, dim labels left, each input in its own box right, fields grouped under
  small headings (the chosen preview also shows a flag as a tick box).
- **Tooltip**: a floating box beside the control after the pointer rests on it; the field holding the
  keyboard shows the same box.
- **Scope of tip text**: the whole editor (every form field, tool icons, palette entries, bar and file
  buttons). The facility is the kit's, so any screen can use it later.

## Design

### A. A terrain's face (`omnis-data`, `omnis-vector/src/geometry.rs`)
- `omnis-data/src/map.rs`: `enum Face { Plain, Hatched }` (default `Plain`), `Terrain.face: Face` with
  `#[serde(default, skip_serializing_if = ...)]`, so **no existing seal moves** and `PROTOCOL` stays 2.
  An enum, not a flag, so further faces can be added.
- `geometry.rs::cells`: a `Hatched` block adds verticals at the cliff's quarter-cell spacing (`HATCH`) on
  each of the box's four sides, from the block's base to its top, as `SegKind::Block` (so its role, weight
  and pattern stay a block's). The box's twelve edges stay whole.
- Terrain form (`editor/forms.rs::terrain`, `terrain_fields`): a `face` choice. The editor's 3D preview
  shows it at once.
- Fixture: one block terrain of a fixture pack other than `test`/`base` (the `heights` pack) is hatched;
  resealed, version bumped. `docs/api.md` changelog, `omnis-cli/src/schema.rs` terrain example.
- **The owner's pack is not touched**: the owner sets `face` on `trees` in the terrain form.

### B. The form's layout (kit and editor)
- `screen/mod.rs`: `Row::field(label, control)`: a row whose label is a dim span in a **label column of
  fixed width** and whose control is drawn **without its label inside its box** (`Row.aligned`, in
  `Screen::shape`). The control keeps `label` for the text readers and routing.
- `screen/mod.rs`: `Kind::Flag(bool)`: a tick box, one press turns it. `form::Value::Flag` maps to it
  instead of a no/yes stepper. Routed as a press (`Payload::Press`), drawn in `shell/screen_draw.rs`.
- `shell/screen_draw.rs::rows` / `control_node`: the aligned row; text inputs take the rest of the row's
  width. Other screens (Settings and the rest) keep today's rows.
- `editor/form.rs`: `Form.sections: Vec<(usize, &'static str)>` (a heading before field `i`; field indices
  and `EditAct::Field(i)` do not move). `Form::rows` emits headings and `Row::field`. In a list's row each
  input keeps a dim label before it and its own box. The properties modal (`screen/editor_props.rs`) uses
  the same rows.
- Sections for every form live in a new `editor/tips.rs` (below), keeping `forms.rs` (978 lines) under
  1000; if the `face` choice pushes it over, the terrain form moves to its own file.

### C. Tooltips (kit, shell, editor)
- `screen/mod.rs`: `Control.tip: Option<String>` (its presence in `shape`).
- `shell/tips.rs` (new), adopting **Bevy 0.20's `ui_widgets::Popover`** (already in the tree; positions a
  child beside its parent and keeps it inside the window): a control node with a tip carries `Tip`. After
  the pointer has rested on it about 0.4 s, or while it holds the keyboard (`InputFocus`), a small bordered
  box of body text is spawned as its child with `Popover` (left of the control first, then right, below,
  above, so the right column's tips fall over the plan, not over other fields), above every screen
  (`GlobalZIndex`). It goes when the pointer leaves, the control is pressed, or focus moves. A dim control
  shows its tip too. The box is ordinary text, so `screen.text` reads it when shown; no format change.
- `editor/tips.rs` (new, Bevy-free): `field(kind, list, label) -> &'static str` and
  `sections(kind)`; tips for tools, palette entries (edge, height, stamp, terrain swatch), the bar, file
  buttons and the form's Apply/Cancel/Remove/Add row. Each field's tip says what it means, its range, and
  what empty means. ASCII only. The full text is listed in the plan document for the owner to correct.

## Steps (one commit each; tests red first; mutation checks; fmt, clippy, the gate against 1326, Sentrux)

0. **Documents.** This plan to `tasks/plans/editor-forms-and-tips.md` (with the tip texts) and
   `tasks/TODO.md`; the lesson in `tasks/LESSONS.md`; if `presentation-PRD.md` or
   `presentation-ARCHITECTURE.md` describes the kit's controls, a line for tooltips and the tick box
   (flagged in the commit as approved by this plan).
1. **Face.** Data, geometry, the form's choice, fixture and reseal, api.md, schema. Tests: a hatched block
   has the verticals on four sides and a plain one none; the old RON loads as `Plain` and re-serialises
   unchanged; the form keeps `face` across an edit. Mutations: face ignored; hatching on plain. Captures at
   Low then High of the hatched fixture block.
2. **Aligned rows and the tick box.** Kit, drawing, `Form::rows`, sections. Headless tests by `screen_text`
   rectangles: every top field's label starts at one x and every input at one x; a label is not inside its
   input's box; a tick box turns on one press; typing keeps its caret across a redraw (existing tests stay
   green). Mutations: label column width dropped; flag press ignored.
3. **The tooltip facility.** `Control.tip`, `shell/tips.rs`. Tests: no tip before the rest time; shown
   after; gone on leave and on press; the focused field shows its; the box lies inside the window at the
   column's edge. Mutations: delay skipped; not removed on leave.
4. **The editor's tip text.** `editor/tips.rs` wired into every form, the tool grid, palette, bar and
   files. Test: every control the editor draws (each form kind opened on the fixture, each tool) has a
   non-empty ASCII tip. Mutation: one tip emptied.
5. **Documents.** code-map, `tasks/acceptance/editor-forms-and-tips.md`, TODO review, CONTINUITY.

Reported as "built, please try".

## Verification
- Per commit: `cargo fmt --all`; workspace clippy; `scripts/verify.sh` to a scratchpad log, committed only
  on `VERIFY-GREEN`, the count read as a number; Sentrux `rescan` and `check_rules`.
- Layout and tips: headless tests through `screen_text` rectangles; a capture of the editor with the
  terrain form open if the capture flags reach it, otherwise I say it was not seen.
- By hand (owner): `just editor`, open the trees terrain, see the grouped two-column form, rest the pointer
  on `climb` and on a tool icon, set `face` to hatched, Apply, and see the trees hatched in the preview and
  in `just qa`.

## Risks, said plainly
- **Hover in headless tests**: the repo's tests press by inserting `Pressed`; hover is Bevy picking's
  `Hovered`. If a test cannot drive it faithfully, the timing logic is tested Bevy-free and the wiring is
  checked by hand, and I say so.
- **`Popover` is new to this crate**; I read its source first. If it cannot hang from a control inside a
  scrolling column (clipping), the alternative (a root-level box placed from the control's rectangle) is
  asked before it is built.
- **The focused field's tip** may feel noisy while typing down a form; it is one constant to turn off, and
  the acceptance sheet asks.
- Forty-odd tip texts are my wording of your rules; expect corrections.
- Every block-hatched map gains four to twelve lines a block cell; part 5 (instancing) is where line count
  is dealt with.

## Not in this plan
Tips on the game's own screens; the new layout on Settings and other screens; a `[tip]` listing in
`screen.text` for agents; the fight screen, minimap and HUD on the palette (still next, then part 4); the
five decisions from part 3's sheet, which stay open; `packs/the_lower_kingdoms` (the owner's).
