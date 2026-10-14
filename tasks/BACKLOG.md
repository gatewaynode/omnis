# Backlog: small findings and unused assets

Started 2026-10-11 (owner: "add all the small findings and extra assets to a BACKLOG.md file to review later for
addressing and removing"). Each entry says what the player or the reader sees, what is actually so, and where it was
found. An entry leaves this file when it is fixed (with a test, for a bug), moved into `tasks/TODO.md` as planned
work, or dropped by the owner.

Features and planned work stay in `tasks/TODO.md`: Look and Map, removing or editing a party member,
`DevCommand::Pass`, the `data.*` ops, `scripts/mcp-probe.py`, the rhai Socket re-audit.

## Interface
- **The HUD clock line has lost the date** (found at P9).
  - The player sees "Day 40 14:20", counted from the party's age.
  - The retired client showed the date and the age: "Year 1 day 40 14:20 night  age 2y 40d", with narrower forms for
    a narrow window. In a town the date snapped back while the age never went back (M8 Part 1, accepted).
  - `Here.date` already carries the date, so the fix needs no engine change. The old `panels::clock_text` and its
    tests: `git show 736558c:crates/omnis-app/src/panels.rs`.
- **At 200% the party column's entries wrap** (P8).
  - The column keeps a quarter of the window, so long entries break over two lines at the largest scale.
- **The picture window draws every monster as the rat** (P7).
  - Whatever the fight is against, the picture shows the rat.
- **Two of P6's calls are unconfirmed by the owner:**
  - down members show in red;
  - dead members stay dim, with "dead" in red.

## Tools and developer features
- **An offscreen `--screenshot` into a missing directory writes nothing and exits 0** (found at P9; older than P9).
  - `cargo run -p omnis-vector -- --screenshot .omnis/new/x.png` logs "Cannot save screenshot, IO error: No such
    file or directory" and the run succeeds.
  - Fix: create the parent directory, or exit non-zero.
- **A flag's Set on the debug panel cannot be tried** (P8).
  - The shipped packs declare no flags, so the World page shows "The packs declare no flags". The Set path is
    tested only through the model.
- **Pack text is not checked for non-ASCII.**
  - Screen words are kept ASCII by rule (presentation-ARCHITECTURE §7.4). Text that comes from a pack is not
    checked, so a pack could show glyphs the fonts lack.

## Docs and wording
- **The editor's toolkit is contradicted** (found at P9). **Resolved 2026-10-14: the owner chose `bevy_ui`; the docs agree.**
  - ARCHITECTURE A11 chose `bevy_ui` with Feathers for the editor; presentation-PRD §8 says the editor keeps
    `bevy_egui` until revisited. Feathers left the workspace with `omnis-app`.
  - ARCHITECTURE §8.2 marks it open. Decide it when the editor is planned (`tasks/plans/editor-v1.md`).
- **`assets/README.md` still says Inter and Alegreya Sans are compiled into the Feathers client only** (P8).
  - Since P8 every build of the game compiles them in (`omnis-vector/src/shell/fonts.rs`). This is the owner's file.
    `ATTRIBUTION.md` was corrected at P9.
- **`Op::Screenshot`'s doc still says "the canvas"** (`omnis-sim/src/ops.rs:123-129`, found at P9).
  - The protocol keeps both targets, `canvas` and `window`, and the game captures its window for both. A simulation
    crate, so it was left alone at P9.
- **Test names still say `other_client`** (found at P9).
  - For example `tests/inventory.rs::the_kit_s_events_read_as_the_other_client_s_lines`. The comments say "the
    retired client" since P9; the function names were left as they were.

## Dependencies
- **`smartstring` RUSTSEC-2026-0249** (through rhai): to add to ARCHITECTURE §13's dependency notes.
- **`ttf-parser` 0.25.1 RUSTSEC-2026-0192**: reached only on Linux, through winit's Wayland window decorations
  (`winit` → `sctk-adwaita` → `ab_glyph` → `owned_ttf_parser`; `cargo tree --target all -i ttf-parser`). It is not in
  the macOS build. To review against the Bevy 0.20 move (not before 2026-11-07).

## Assets no code uses (review for removal; the owner's files)
Found at P9, when `omnis-app` left. Nothing here is deleted without the owner.
- `assets/fonts/inter/Inter-Bold.ttf` and `assets/fonts/alegreya-sans/AlegreyaSans-Bold.ttf`.
  - Only the retired client compiled them in (`feathers_fonts.rs`). The game uses the Regular faces.
- The baked tileset sprites in `packs/test/assets/tilesets/`.
  - Only the retired client drew them (`plan.rs`). The tileset data under `packs/test/data/tiles` is still loaded
    and validated by `omnis-data` and `omnis-cli validate`, and maps name a tileset, so removing the sprites means
    deciding what the tileset data is for.
- `assets/openrtp-tiles/*.png`.
  - No client loads them. They are the source sheets for `omnis-cli tileset bake` (`packs/test/bake/*.ron`), so they
    stand or fall with the baked tilesets above.
- Not code-referenced yet, but the owner's work in progress (listed so the review is complete, not proposed for
  removal): `assets/basic-glb-components/`, `assets/basic-svg-components/`, `assets/private/time-fantasy-icons/`.
