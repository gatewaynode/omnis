# Acceptance: Bevy 0.19.1 to 0.20.0 (W2's first item; `tasks/plans/bevy-0.20-migration.md`)

Built 2026-11-12 in plan mode on the owner's order ("Let's proceed with the Bevy 0.20 migration and W2 in
general"), on `W-2_look_and_engine`. The aim was the engine moved with the game behaving as it did.

## What was built
- **Step 1, the engine** (`a1117c72`): `bevy = "=0.20.0"`; all 69 `bevy_*` crates held at 0.20.0 in the lock
  (cargo chose 0.20.1, published 2026-10-28 and under thirty days old); six other crates held back for age; of
  161 moved crates none was published after 2026-10-13. `rust-version` 1.97. The three cameras share
  `render::glow()` with `Tonemapping::Linear`. Text fields carry `TextInput`. Escape leaves a field the keyboard
  (`screen_draw::keep_keyboard`).
- **Step 2, the buttons** (`d67ba116`): the deprecated `Interaction` is gone. A live control is
  `controls::Control` with Bevy's `Hovered` and `Pressed`; a press acts as the left button goes down.
- **Step 3**: the documents, this script, the duplicate check's silent exit fixed.

## Departures from the plan as approved
- **The controls are not Bevy's `ui_widgets::Button`.** The plan put Bevy's new button on every pressable. It is
  pressed by any mouse button; here the right button is the action key and the editor's pick, and the old
  `Interaction` was the left button's alone. `controls::Control` has four observers of its own that keep
  `Pressed` for the left button. Built without asking first; the other course (Bevy's button, a right click
  pressing buttons) is a small change if the owner prefers it.
- **Steps 1 and 2 stayed two commits, but not as planned**: the old `Button` is a private type in 0.20 and
  could not be kept for a commit, so step 1 already introduced `controls::pressable()`.
- **Escape in a text field** was not in the plan's list as a fault: Bevy 0.20 takes the keyboard from the field,
  and the letters typed after the editor's menu closed chose tools. Fixed in step 1, with a test.
- **The captures are not an exact baseline for the 3D view.** The view eases over its first frames by real time,
  so two builds differ by a fraction of a pixel in the floor lines. The title and new-game captures are
  byte-identical to 0.19.1's. In the game view about one channel value in seven differs by one or two levels,
  read as the dither 0.20 applies under `Linear` (0.19.1's `None` drew without it); not confirmed by eye.
- **The retained-UI checks through the dev socket were not run**: a window launched from the agent's shell
  draws no frames. They are items 3 to 6 below.

## After the first hand test (2026-11-12)
- **"I lost control of the left click button on the mouse."** A button that opens a screen over itself (Menu,
  the log, a modal's Finish) stayed pressed: the pointer's release reaches only what is under the pointer. A
  pressed control takes every click from the view and the plan. `controls::button_up` now lets every control
  go when the left button comes up. To try: click Menu, close the pause, click in the view: mouse look turns
  on. Click the running log, close it, click in the view: the same.

## Known and left
- Copy, cut and paste work in text fields now, on an in-app clipboard (nothing reaches the system clipboard).
- A held movement button through Bevy's real picking is not tested headless (item 2 below).
- BACKLOG: the move to 0.20.1 from 2026-11-27; `ttf-parser` and `webgl2` unchanged by 0.20.

## The script
Launch: `just run --windowed --pack packs/base --pack packs/the_lower_kingdoms`, New game.

1. **The view.** You will see the corridor in glowing lines as before: no banding in the faded far lines, no
   change of brightness, the minimap top right, the party column.
2. **Buttons.** Hover Forward: it lightens. Hold it with the left button: it darkens and the party walks until
   you let go; let go on the button and it returns to the hover shade. Right-click Forward: nothing is pressed
   (the right button is the action key). Click Turn left once: one quarter turn, however long you hold.
3. **The minimap** redraws as you walk (the arrow moves, new cells appear).
4. **Screens.** Esc opens the pause; Settings: step a choice with its arrows. Where a screen has a slider, drag
   it (the thumb follows); where a list is long, scroll it (the scrollbar shows and moves). Click the running
   log: the full log opens.
5. **A fight.** Walk into an encounter. The fight screen has the same glow;
   the action buttons shade under the pointer; the digit keys and a click both act.
6. **The Debug panel** (Esc, Debug, World): type in the Minutes field, press Pass; the clock moves.
7. **The editor** (Title, Editor, open a map): the tool icons shade and choose with the left button; paint a
   cell (the plan redraws); a click on a tool icon does not also paint the plan; the 3D preview shows the map
   with the game's glow.
8. **Text in the editor.** Open a cell's note and type; press Esc (the menu opens), Esc again (it closes), type
   again: the letters go on into the note and no tool changes. Open a file as text: typing, the caret and both
   scrollbars work.
9. F1 does nothing (Bevy 0.20 ships a debug overlay; it is off).

Each of 2, 7 and 8 has a headless test behind it (`editor_pointer.rs`, `column.rs`, `kit_icons.rs`,
`editor_forms.rs`, `controls.rs`); 1 and 3 to 6 rest on the existing suite (447 client tests) and on your eye.
