# The fight screen and the picture window on the palette

## Context

Rendering size 2 part 3 gave the 3D view a palette of colour roles that a map's biosphere can change
(`style::Palette`, `omnis_data::ColourRole`). Its plan left the fight screen, picture window, minimap and
HUD for later. Today the fight ignores the palette:

- **The arena** (`arena.rs` `Tone`, `shell/combat.rs::colour`) draws party and monsters in one fixed green,
  with fixed amber, cyan, red and white marks. `Tone` does not know a figure's side.
- **The picture window** (`cinema.rs`) is painted in fixed green bytes (`LINE`, `GLOW`, `BACKGROUND`).
- **The fight's interface** (title, menu, roll log, labels, party column) uses the fixed interface green
  (`style::Role::rgb`, `controls::GREEN`, `style::SHADES`).

The owner's decisions (asked in plan mode, 2026-11-13):
- **Scope**: the fight screen and the picture window. The minimap and the HUD keep their own plans.
- **Sides**: party members in the palette's party ink, monsters in its hostile ink.
- **Interface**: the fight screen's text, borders and panels follow the biosphere's interface ink. Other
  screens stay green until the HUD's plan, so the two differ meanwhile (accepted by that choice).

## Design

### A. Where the fight's palette comes from
The palette of the map the party stands on, exactly as the 3D view takes it
(`Palette::of(session.data.biosphere_of(session.views.position.map))`, `shell/render.rs::tune`). A fight
never changes map, so it is fixed for the fight.

### B. The arena's lines (`arena.rs`, new `tones.rs`, `shell/combat.rs`)
`Tone` learns the side: `Figure(Side)`, `Down(Side)`, `Dead(Side)`, `Acting(Side)`; `Bar`, `Health`,
`Target`, `Hover` as now. A Bevy-free `tones::ink(tone, &Palette) -> [f32; 3]` replaces the fixed table:

| Line | Today | Built |
|---|---|---|
| A standing party member | bright green | **Party** ink |
| A standing monster | the same green | **Hostile** ink |
| A figure that is down, and its slash | dim green | its side's ink at a quarter |
| A dead figure, and its cross | red | its side's ink at an eighth |
| The brackets round whoever acts | amber | the acting side's ink, full |
| A target's frame | cyan | **Interface** ink, brightened to bloom |
| The hovered target's frame | white | white (unchanged) |
| A health bar's frame / fill | two greens | **Interface** ink, dim / bright |

Down, dead, acting and target keep their shapes (slash, cross, brackets, full frame), so none of them is
told by colour alone. The lines stay Bevy gizmos: one width, no patterns. Moving the arena onto the line
material (patterns, weights) is not in this plan.

### C. The picture window (`cinema.rs`, `shell/cinema.rs`)
`paint(scene, w, h, inks)`: the enemy's drawing in the **Hostile** ink (core at the ink's colour without
its glow, the halo a quarter of it), on the interface's darkest panel shade. The cached image is keyed by
scene and inks.

### D. The fight's interface (`style.rs`, new `shell/theme.rs`, the fight's drawing code)
- `style::Theme` (Bevy-free): the interface's colours as sRGB: `rgb(Role)` and `shade(i)`.
  `Theme::default()` is **today's values exactly**. `Theme::of(&Palette)`: when the palette's Interface ink
  is the default's, the default theme; otherwise Interface and Title are the ink (glow taken out, so text
  never clips), Dim and the three panel shades are that ink darkened by fixed factors, and **Caution and
  Alert stay as they are** (they are warnings, not the interface's own colour).
- `shell/theme.rs`: a `Theme` resource, set each frame: in `ViewState::Fight` the map's palette's theme,
  otherwise the default. The HUD's plan later removes that "otherwise" and every screen follows.
- Every node drawn while a fight is on takes its colour from the resource instead of `colour(role)`,
  `GREEN`, `SHADES`: `shell/combat.rs` (title, panels, log, labels), `shell/screen_draw.rs::build` (the
  theme passed in with the height and mark; other screens pass the same resource, which is the default
  outside a fight), `shell/roster.rs`, the status line of `shell/hud.rs`, and `shell/tips.rs`/`notice.rs`
  if they can show in a fight. The fight screen's rebuild key gains the theme.
- `screen_draw.rs` (903 lines) and `screen.rs` (960) stay under 1000; if the theme pushes one over, its
  colour helpers move to `shell/theme.rs`.

### E. Seeing it
The fixture needs a fight on a map whose biosphere changes the Party, Hostile and Interface inks. The
`test` and `base` packs are not touched (goldens). If no other fixture map has both a biosphere and an
encounter, a hostile encounter is added to one (resealed, version bumped).
There is no offscreen capture of a fight. I will try a picture through the running game's dev socket
(`screenshot`); if that does not work I say the screen was not seen by me.

## Steps (one commit each; tests red first; mutation checks; fmt, clippy, the gate against 1339, Sentrux)

0. **Documents.** This plan to `tasks/plans/fight-on-the-palette.md` and `tasks/TODO.md`; `style.rs`'s
   header line and, if presentation-PRD/ARCHITECTURE describe the fight's colours, a line there (flagged
   in the commit as approved by this plan).
1. **The arena's inks.** `Tone` with sides, `tones::ink`, `shell/combat.rs::draw` reading the palette.
   Tests: a party figure's tone differs from a monster's; each row of the table against a palette with
   changed inks; hover is white under any palette; the default palette's Party, Hostile and Interface inks
   pass `contrast::clashes` (if they do not, I report it and ask, not retune). Mutations: side ignored;
   palette ignored (default used).
2. **The picture window.** Tests: the core pixels are the Hostile ink's colour under two palettes; the
   image is painted again when the inks change, not when they do not. Mutation: inks ignored.
3. **The fight's interface.** Tests: `Theme::default()` equals today's constants; `Theme::of` under another
   interface ink (text unclipped, Dim darker than Interface, Caution and Alert unchanged); in a fight on
   the biosphere map no text or border keeps the default green and the title has the theme's; out of the
   fight the screens are the default green again. Mutations: theme not applied in a fight; applied outside.
4. **Documents.** `tasks/acceptance/fight-on-the-palette.md`, code-map, TODO review, CONTINUITY.

Reported as "built, please try", walking the table above arm by arm.

## What the owner's own maps will show
If `packs/the_lower_kingdoms` names no biosphere: party figures teal-green and monsters orange-red (the
default Party and Hostile inks), marks as in the table, the picture window's rat orange-red, and the
fight's text and panels **unchanged** (the default interface ink is today's green). The interface only
changes on a map whose biosphere sets an Interface ink.

## Verification
- Per commit: `cargo fmt --all`; workspace clippy; `scripts/verify.sh` to a scratchpad log, committed only
  on `VERIFY-GREEN`, the count read as a number; Sentrux `rescan` and `check_rules`.
- By hand (owner): `just qa`, walk into a fight on the default palette and on the biosphere fixture map;
  see the two sides apart, whose turn it is, a target and its hover, a fallen figure, the picture window,
  and the menu and log in the biosphere's interface colour.

## Risks, said plainly
- **Amber and cyan go.** The acting brackets and target frames lose their own hues; brackets and frames
  differ by shape and by ink, but the screen has fewer colours. The acceptance sheet asks.
- **Dead is no longer red.** A dead figure is its side's ink, very dim, with its cross. Asked too.
- **A dark interface ink** in a pack would make dim text hard to read; the data check does not guard
  contrast today. I note it; a check is the HUD plan's.
- **Two interface colours meanwhile**: leaving a fight on a biosphere map, the HUD is green again until
  the HUD's plan.
- Bloom on the fight's 2D camera makes bright inks spread; the factors in the table are tuned by eye only
  if I get a picture.

## Not in this plan
The minimap; the HUD and every other screen on the theme; the arena on the line material (patterns and
weights); drawings per monster in the picture window; a colour-deficiency preset; part 3's five open
decisions; `packs/the_lower_kingdoms` (the owner's).
