# The Feathers experiment: report (2026-09-20)

The plan was `~/.claude/plans/nested-growing-bear.md` (PRD D26, §11.1; ARCH A11 as amended): party
creation rebuilt with Bevy's Feathers widgets and a modern font, on `m7-tasks`, commits `8e111d5..5bce8b7`.
The per-step record with every measurement is in `tasks/TODO.md` under "The Feathers experiment".

## Outcome (owner, 2026-09-20)
"I'm ready to call the feathers experiment a success and committing to moving the rest of the play UI
over bit by bit." Inter is the font ("the Inter font is preferred"); the interface scale is 1.5 on the
5120×1440 ultrawide and 1.1 where the canvas fits once. The decision was made on two live looks on the
ultrawide, before this report; the report is the record, not the argument.

## Numbers
| What | Measured |
|---|---|
| Crates in the app's tree | 323 → 328 with `ui` (`bevy_ui`, `bevy_ui_render`, `bevy_ui_widgets`, `taffy` 0.10.1, `grid` 1.0.1) → 329 with `bevy_feathers`; no new duplicate; no new package in `Cargo.lock` |
| Third-party crates newly compiled | `taffy` 0.10.1 (2026-04-14; Socket supply chain 82, quality 93, the rest 100), `grid` 1.0.1 (2026-04-20; Socket 100, quality 93); both MIT, both older than 30 days |
| Clean release build, default features | 101 s → 112 s wall (853 → 1019 CPU seconds) |
| Binary, default features | 69.3 MB → 85.3 MB (+16.0 MB); fonts add 1.3 MB of that feature's data |
| Shipped build (`--no-default-features`) | unchanged by construction: 320 crates, no interface crate, clippy line in the gate |
| Clippy pedantic on `bsn!` output | accepted with no `allow`, both configurations |
| Tests | 353 → 376 passed, 6 ignored; 17 of the 23 new ones run Feathers headless with no window and no GPU (15 on the panel, 2 on probe controls) |
| Replays, pack tuple, save schema, MCP tool count | unchanged (walk `238033710167572364`, fight `7317777168019603128`, schema 4, 18 tools) |

## Legibility (physical pixels; Feathers' medium text is 14 px, its row 24 px, both times the interface scale)
Computed from each font's own `OS/2` table (x-height and cap height over units per em: Fira Sans
527 and 689 of 1000, Inter 1118 and 1490 of 2048, Alegreya Sans 458 and 641 of 1000).

| Font | Scale 1.1 (canvas 1×): size, x-height, caps, row | Scale 1.5 (canvas 2×, the ultrawide) |
|---|---|---|
| Inter | 15.4, 8.4, 11.2, 26.4 | 21.0, 11.5, 15.3, 36.0 |
| Fira Sans | 15.4, 8.1, 10.6, 26.4 | 21.0, 11.1, 14.5, 36.0 |
| Alegreya Sans | 15.4, 7.1, 9.9, 26.4 | 21.0, 9.6, 13.5, 36.0 |
| Bitmap font (5×7 at texel scale 2) | cell 10×14: caps 14 | cell 20×28: caps 28 |

Inter has the tallest x-height of the three, which agrees with the owner's pick by eye. The panel's
capitals are about half the bitmap font's height on the ultrawide and carry more text per row: the
bitmap font was large because it had to be, not because the text needed it.

## Frame time: not measured where it counts
From an agent-launched window (on no screen): 16.7 ms with vertical sync for both the panel and the
canvas alone; 43 ms (panel) and 31 ms (canvas) with it off, which a hidden macOS window explains
better than the code does. The owner's visible game was never measured (`--frame-stats` is there for
it: `--script "party,create"` against `--script "party"`). The owner saw nothing slow in two looks.
**Open**: measure once on the visible game before a screen with many rows (the inventory) moves over.

## The three guarantees
1. **An agent's screenshot shows the interface.** Kept, by new means. The canvas image cannot hold
   window-space `bevy_ui`, and a plain window capture is black for an agent-launched process (never
   measured on the owner's visible window). `capture.rs` composes canvas and interface into an image of
   the window's size; the `screenshot` op takes `target: "canvas" | "window"` and was proven on the
   running game over the dev socket. Limits: the letterbox takes the camera's default clear colour; an
   image target's scale factor is one, so a HiDPI window's capture lays the interface out at half size;
   an agent-launched window is clamped to 2560×1378, so a capture cannot stand in for the ultrawide.
2. **Screen dumps show every screen.** Lost for `bevy_ui` screens by construction (the PPM dump is a
   Bevy-free raster). The stand-in is the text tree (`creation_feathers.txt` under
   `OMNIS_DUMP_SCREENS`): one line per control, label, text and input with its rectangle and hidden
   state. It shows structure and position, not pixels.
3. **Headless tests drive every widget.** Kept, and stronger than the canvas's hit-tests: every control
   by its event (a census of the controls, every menu option, both controls of all six scores, every
   skill, the refusals by their messages), and through real layout, picking and focus by pointer and
   keys (button, checkbox, slider drag, number input, text input, menu item, scrollbar track and thumb,
   Tab order), plus a layout check (nothing outside the panel, nothing overlapping) at 1280×720 and
   5120×1440 in all three fonts. It found a real bug (a typed 99 stayed on screen while the form held 15).

## Friction log
- BSN: an enum component inside `bsn!` needs `#[derive(FromTemplate)]`; scenes hold no closures here
  (one observer per payload type looks up `PanelId`), which kept every scene function under 40 lines.
- External state everywhere: a slider's `SliderValue` is inserted, a number input is told by
  `UpdateNumberInput` and ignores it while focused, so a report that changes nothing (a clamped 99)
  needs a forced `sync`.
- There is no dropdown: a menu whose caption we own and write.
- Number and text inputs ignore `InteractionDisabled`; refusals live in the Bevy-free `apply`.
- Sliders take drags only (`TrackClick::Drag`); a click on the track does nothing.
- Fonts arrive by inheritance a frame after the spawn: `Added<TextFont>` dressed 32 of more than 40
  texts; the font system follows changes. Only Fira has a monospace face (slider values).
- The name's limit is 24 bytes in the form and 24 characters in the input: a name of multi-byte
  letters shows more than the form keeps. **A bug to fix in stage 2's first commit.**
- The scale slider allows a scale the window cannot hold (1.5 and 2 at 1280×720 push the footer out).
  **To cap in stage 2** (the layout check already names the fault).
- Scrollbar thumbs show at full height when nothing overflows.
- The 3c bug (the Name input dead after a skin switch) was never reproduced or explained; the switcher
  is gone. If a text input dies again, start from the operating system's input and the IME switch.
- Both crates call themselves experimental and will break at Bevy 0.20; Feathers' docs advise copying
  it into the project. `bevy_ui` costs 16 MB of binary.

## Recommendations for stage 2
- **More screens in `bevy_ui`: yes (the owner's decision), one screen per step, each with the panel's
  pattern**: a Bevy-free model and `apply` (the rules and refusals, unit-tested), scenes with ids,
  one observer per payload, `reconcile`/`sync`, a headless test file with a census, events, pointer,
  layout faults and a text tree. Order by what the canvas toolkit builds worst and M7 needs first:
  M7's new screens (town services, shops, level up) are born in `bevy_ui` and never get a canvas
  version; then the inventory, the character sheet, the fight's pickers, the menus; the HUD (band, log,
  pads) last, because it surrounds the viewport and decides the canvas's fate.
- **Feathers as is, or restyled**: keep Feathers' dark theme while screens move (it reads well on the
  ultrawide by the owner's eye); decide on vendoring at the Bevy 0.20 upgrade, when the cost of its
  churn is known, not before. Shared pieces (the dropdown-from-a-menu, the font system, the scale
  system, the test helpers) move out of `feathers_creation.rs` into their own module with the second
  screen, not earlier.
- **The canvas creation screen**: keep it only while the release build has no interface crates. The
  real question is whether the shipped build carries `bevy_ui` (it must, once play screens depend on
  it): when `feathers` stops being a dev feature, the canvas creation screen, its painter, its tests
  and `Menu::Covered` go in one commit.
- **Modern fonts everywhere**: direction (a), text in native-resolution `bevy_ui`, follows from the
  decision above. What stays on the canvas until its screen moves keeps the bitmap font; no CPU
  TrueType rasterizer is built for a canvas that is on its way out.
- **Smooth scaling**: the interface already scales smoothly (`UiScale`); the canvas's whole-number
  scale matters only for the viewport and belongs to the viewport spike.
- **The editor's toolkit**: Feathers, not `bevy_egui` (17 crates and two duplicates saved, one toolkit
  to learn, and the editor is what Feathers is for). The `bevy_egui` re-audit of 2026-10-08 lapses if
  the owner agrees; the editor plan (`tasks/plans/editor-v1.md`) is re-read against this when M5 returns.
- **The viewport hybrid spike** (PRD §14): unchanged, after M7's first acceptance point.
- **A `screen.text` socket op over the text tree**: build it with the second screen; agents need to
  read `bevy_ui` screens without a PNG, and the tree already exists in the tests.
- **Screen dumps**: extend `dump_screens` with a composed-capture route only if the text tree proves
  too little in review; it needs a GPU, which the gate does not have.

## Vision documents (proposed; applied only on the owner's word)
ARCH A11 and §8.4 (the experiment's outcome: `bevy_ui` with Feathers is the interface toolkit, the
canvas toolkit is kept until each screen moves; the pattern above), ARCH §8.1 (the `feathers` feature
and its plugins; when it enters the shipped build), PRD §11.1 and D26 (the outcome, Inter, the scale
rule), PRD §14 (the questions this closes and the two it leaves: frame time, vendoring).
