# Bevy 0.19.1 to 0.20.0 migration plan

Status: **approved 2026-11-12, in progress** on `W-2_look_and_engine`. Replaces the plan of 2026-10-10.

## Context

W2 opens with the engine move so that the shaders and materials of the rendering direction are written once,
against 0.20. The plan of 2026-10-10 (`tasks/plans/bevy-0.20-migration.md`) was audited against a tree that no
longer exists: `omnis-app` is retired, the client has an editor with text fields, a third HDR camera, picking in
the test harness, and about four times the button code. The audit was redone on 2026-11-12 against the official
0.19 to 0.20 guide (75 sections, read whole) and today's tree. Nothing was compiled: the open questions below
are settled at the first build.

## Facts checked 2026-11-12

- **Version: `bevy = "=0.20.0"`** (published 2026-10-08, 35 days old). 0.20.1 (2026-10-28) is 15 days old and
  fails the 30-day rule until 2026-11-27; its ten commits touch nothing this client uses (ruzstd, serde
  features, a buffer size fix, reflection, fog volumes, bsn). Moving to 0.20.1 later is a one-line item, noted
  in BACKLOG, not part of this work.
- MSRV 1.97.1; the tree pins toolchain 1.98.0. The workspace `rust-version = "1.95"` rises to `1.97`.
- Only `omnis-vector` uses Bevy (70 files). Features `["2d", "png", "bevy_pbr", "ui"]` all still exist.
- 0.20 requires `ron ^0.12`, `serde ^1`, `thiserror ^2`, `image ^0.25.2` (png through it), wgpu 30, glam 0.33.
  Text is parley + swash, no cosmic-text: `ttf-parser` (RUSTSEC-2026-0192) may leave the tree; the lock shows.
- No `bevy_egui` pin exists any more; nothing to move.

## What changes (all under `crates/omnis-vector/` unless a path says otherwise)

**Deprecations (errors under `-D warnings`)**
- `ui::Interaction` gives way to `picking::hover::Hovered` + `ui::Pressed`: 31 code lines in 8 src files
  (`shell/{controls,screen,combat,editor,movement,roster,notice,hud}.rs`), 82 lines in 20 test files.
- The prelude's `Button` gives way to `bevy::ui_widgets::Button`: 10 lines in `shell/{controls,screen_draw,
  notice,screen_text}.rs`, 4 in tests.
- `tests/common/app.rs:271`: `Pointer<Release>` becomes `PointerRelease` (flat pointer events).

**Silent behaviour changes**
- `Tonemapping::None` is now a full passthrough (no dither, no grading, no clamp). The three HDR cameras
  (`shell/render.rs:117`, `shell/combat.rs:467`, `shell/preview.rs:112`) take `Tonemapping::Linear`, which is
  0.19's `None`.
- A text field needs `TextInput` beside `EditableText` or typing stops: `shell/screen_draw.rs:533`,
  `shell/document.rs:164`. `TextScroll` is gone (`EditableText::viewport`); `document.rs`'s doc and
  `follow_caret` are read against it.
- Escape now also clears `InputFocus` on a focused field: `editor.rs:574` (`typing`), the `AutoFocus` fields,
  and the keeps-the-keyboard logic in `screen.rs:797` and `editor.rs:739` are tested.

**The press design** (the one decision of substance)
- Today every route polls `Changed<Interaction>` and acts on the press, button down. That stays: a press acts
  on button down, read as `Added<Pressed>`; held actions read `Has<Pressed>`; the shade reads `&Hovered` +
  `Has<Pressed>` and re-shades on `RemovedComponents<Pressed>`; "pointer over a control" (`movement.rs:125`,
  `editor.rs:644`) reads any `Hovered` true or `Pressed`. 0.20's own `Activate` event (fires on release unless
  `ActivateOnPress`) is not adopted here: it changes when a press lands and belongs with the open ECS task
  "a press as a message", which the owner orders separately.
- The filter and the over-a-control test live once, in `shell/controls.rs` (a `Pressing` filter alias and
  `over_control`), used by all seven routes, instead of being restated per file.
- Every pressable gets its components in one place: a `pressable()` bundle (`Button`, `Hovered::default()`,
  and whatever the new `Button` does not require by itself) used by `controls::button`,
  `screen_draw::{square, arrow, control_node}`, `notice.rs:76`. `hud.rs:96`'s `LogColumn` (a plain node with
  `Interaction`) takes the same bundle.
- Tests: `tests/common/app.rs::set` is rewritten over `Pressed`/`Hovered` with a small own enum (`Rest`,
  `Hover`, `Press`) so its 40 callers change a word, not a shape. The ten per-file helpers that write
  `&mut Interaction` by hand (`common/fight.rs`, `common/editor.rs`, `load_report.rs`, `inventory.rs`,
  `editor_link.rs`, `editor.rs`, `cast.rs`, `column.rs`, `screens.rs`, `editor_playtest.rs`) become calls of
  one generic `press::<C>(app, &marker)` in `common/app.rs`.

## Steps (one commit each; fmt, workspace clippy, the gate read as a number against 1256, Sentrux)

0. **Housekeeping, before any code**: `tasks/CONTINUITY.md` rewritten on this branch (the handoff commit
   stayed on W1), this plan into `tasks/TODO.md` under W2 as checkable items, the October plan file replaced
   by this one's content. Baseline captures on 0.19.1 into the scratchpad (not committed): the 3D view, the
   fight screen and the editor with its preview, through the offscreen capture path (`main.rs:77-94`), at
   1600x900; plus `screen_text` dumps of the same screens.
1. **The engine moves; behaviour held.** Root `Cargo.toml` pin and `rust-version`; `cargo update -p bevy
   --precise 0.20.0`; root pins (`png`, `serde`, `ron`, `thiserror`) moved to what 0.20 resolves so the tree
   stays single-copy; `scripts/check-duplicates.sh` and `scripts/duplicates.allow` re-read; **every new or
   moved crate in `Cargo.lock` checked for a publish date on or before 2026-10-13** (a script over the lock
   diff against the crates.io API, generic client; a younger crate is held back with `--precise` or stops the
   step and is reported); Socket on anything new and unfamiliar. Then compile fixes only: the three
   tonemappers, `TextInput`, `PointerRelease`, any struct literal that gained a field, any renamed path. The
   deprecated `Interaction` and `Button` stay in this commit under a crate-level `#![allow(deprecated)]` in
   `src/lib.rs` and the test crate root, each with a comment naming step 2. Gate green at 1256.
   New tests (red on a build without the fix): a typed character reaches a screen field and the document
   field; Escape in a focused editor field (the outcome pinned as today's); each camera's tonemapping is
   `Linear`.
2. **Buttons on `Hovered` and `Pressed`.** The press design above; both `allow(deprecated)` lines removed, so
   the compiler lists every remaining use. Mutation checks, each caught by a named test: the press filter
   widened to every frame while held (a one-shot fires twice); the held read dropped (a held move stops);
   the shade's `Pressed` arm removed (`kit_icons.rs`); `over_control` returning false (a click on a button
   also moves the party or paints); a dim control keeping its `Button` (`editor_rules.rs`, `editor_pick.rs`).
   The window path is exercised too (`real_click` in `tests/common`), per the 2026-11-07 lesson: a faked
   press is not the proof.
3. **Runtime checks and documents.** Captures and `screen_text` again, compared with step 0's; each
   difference fixed with a test or recorded. The retained-UI list read on screen through the dev socket's
   `screenshot`: minimap redraw, the editor plan after a paint, icons, hover shades, slider thumbs,
   scrollbars, `Display` toggles. The built-in font face; no F1 overlay; the capture's alpha. Documents:
   `PRD.md:78,305` and `ARCHITECTURE.md:658,668,672` (the owner's order of this migration is the ask, flagged
   in the commit), `README.md:52`, `presentation-ARCHITECTURE.md:111,355`, `presentation-PRD.md:276,321`,
   `tasks/knowledge/` where a 0.19 fact is stated as current, `tasks/BACKLOG.md` (the 0.20.1 note; the
   advisories' new state). `tasks/acceptance/bevy-0.20.md`, numbered, each "you will see" from a test or a
   capture. TODO ticked with a review line.

If step 1's compile shows the old `Interaction` is removed rather than deprecated, or the allow cannot hold,
steps 1 and 2 merge into one commit; said in the report, not decided silently.

## Open questions the first compile settles (recorded in the step 1 commit message)
- Facade paths and the feature for `ui_widgets::Button`, `Hovered`, `Pressed`, `TextInput`; whether the new
  `Button` requires `Hovered`/`Pressed`; whether the headless harness (`tests/common/app.rs:110-142,343-359`)
  needs the widgets plugin added for `Pressed` to be maintained on a real click.
- Whether `EditableTextInputPlugin`, `EditableTextGeneration`, `TextLayoutInfo::cursor` and the
  `EditableText` fields (`max_characters`, `allow_newlines`, `visible_lines`, `visible_width`) keep their
  shapes; likewise `ValueChange`, `Scrollbar::new`, `Image::new_target_texture`, `GizmoConfigStore::iter_mut`,
  `UiSystems::{Content, Prepare}`, `RenderTargetInfo`.

## Verification
- Per step: `cargo fmt --all`; clippy workspace `--all-targets --features omnis-vector/test-fixtures --locked
  -- -D warnings`; `scripts/verify.sh` to a scratchpad log, committed only on `grep -q VERIFY-GREEN` with the
  count line at 1256 plus the new tests; `lint-sim` clean (the sim crates are untouched: the replay pins walk
  `d48a913cec6c5735` and fight `9f52f48c5364e17f` must not move); Sentrux `rescan` + `check_rules`.
- By hand, the acceptance script: `just run --windowed --pack packs/base --pack packs/the_lower_kingdoms`;
  New game; walk and hold a move button; open the pause overlay with Esc; a fight (press, hover shade,
  digits); Title, Editor: paint, type in a form field and in the RON text view, Esc in a field, the preview,
  History; the Debug panel's Minutes field and Pass. Reported as "built, please try".

## After this (W2, not planned here)
The rendering direction is next and needs the owner's choice of size before a plan (TODO W2: size 1, retained
line meshes with an own WGSL material, recommended, with size 2 designed in the same presentation-PRD
amendment). Asked when 0.20 is accepted; it gets its own plan mode.
