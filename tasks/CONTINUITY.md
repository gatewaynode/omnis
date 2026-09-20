# Continuity notes

Written 2026-09-20 during the Feathers experiment. Rewrite this file every time it is used; keep it
to state, next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## State
- Branch `m7-tasks`, fast-forwarded to `main` at `8e111d5`. **Nothing on it is pushed and
  `origin/m7-tasks` does not exist**: `509187a` docs, `cb2e985` step 1, `3da680b` step 2, `ce63db1`
  step 3, `130d1ee` and `6ce8b6c` docs, `1108454` the Name input tests, `400f0c7` 3a, `57c3463` 3b,
  and this commit.
- The approved plan is `~/.claude/plans/nested-growing-bear.md`; its checkable items and every
  measured number are in `tasks/TODO.md` under "The Feathers experiment" (M6 closeout block).
- Done: steps 0–3, owner look 1, 3a (the classic view is gone; the canvas creation screen is what
  a build without the Feathers plugin shows), 3b (interface scale: 0.75 per physical pixel of a
  canvas pixel until the slider is moved, so 1.5 on the owner's ultrawide; `ScaleChoice`, the
  `scale` system, `creation_panel::fitted_scale`). The gate is green at 367 passed, 6 ignored;
  Sentrux passes (scan `crates/`, the rules file is `crates/.sentrux/rules.toml`).
- **3c is open**: the owner's bug (Name inaccessible after Classic → Feathers) is **not
  reproduced**. Headless through real layout, picking and focus it works in six flows (after the
  switch, with a name typed before or on the canvas, on the ultrawide at 1.5 by click and by Tab,
  no nodes left behind); in the real app text renders after a skin flip (byte-identical capture).
  Not exercised by either: the operating system's own mouse and key events, and the macOS IME
  switch `bevy_ui_widgets` flips when a text input gains or loses the focus. The owner is asked:
  does the Name field still fail after a class change or an added member (the rebuilds that
  remain), and what is "inaccessible" exactly (no caret on click, a caret but no letters, the
  first letter lost).
- Test helpers now in `tests/feathers.rs` (step 4 builds on them): `click_node` (`PointerInput`
  move, press, release at a node's centre), `keys` (`KeyboardInput` with text, the real window
  entity), `tab`, `focus`, `ultrawide` (window resolution plus a `WindowResized` message: the
  camera's target and `WindowSize` both follow it), `pick_class_by_mouse` (opens the menu and
  clicks the item: menus work by pointer headless), `outside_the_panel` (the layout check).
- To see it without a person: `cargo run -p omnis-app -- --seed 7 --no-dev-socket --script
  "party,create" --settle 60 --screenshot-composed <file.png>` (add `--window medium`).

## Next
1. Step 4: `tests/feathers.rs` grows: every control by event; a button, checkbox, slider and
   scrollbar by `PointerInput` (the text input and a menu item already are); the layout check at
   1280×720 and 5120×1440 at the fitted scale and at forced 1, 1.5, 2 (no overlap, not only
   inside); the text tree dump under `OMNIS_DUMP_SCREENS`; the panel's fighter compared with the
   canvas path's; the refusals. The "skin switch" part of the item is void since 3a.
2. Then 5 (Inter, Alegreya Sans: OFL files from the upstream GitHub releases older than 30 days,
   hashes in `assets/README.md`; the font menu; `--frame-stats`), 6 (socket `screenshot` gains
   `target`; measure the window capture against the owner's visible game; owner look 2 on the
   ultrawide, which is also where 3c gets its answer), 7 (the report
   `tasks/plans/feathers-experiment.md`, vision edits only on the owner's word).
3. Known rough edges to fix or log: scrollbar thumbs show at full height when nothing overflows;
   the composed capture's letterbox takes the camera's default clear colour; an image target's
   scale factor is one, so a HiDPI window's capture lays the interface out at half size.
4. Still unanswered by the owner: how the M6c play-test went; whether ARCH §4.7 resolving auto
   members' turns inside the simulation stands. Then M7 planning (turn budget, declared reactions).
