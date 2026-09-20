# Continuity notes

Written 2026-09-20 during the Feathers experiment. Rewrite this file every time it is used; keep it
to state, next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## State
- Branch `m7-tasks`, fast-forwarded to `main` at `8e111d5` (PR #9 merged the M6 closeout;
  `git log main..m6-closeout-tasks` was empty). On it, unpushed: `509187a` docs (the plan, PRD §14
  viewport = a 2D/3D hybrid), `cb2e985` step 1, `3da680b` step 2, `ce63db1` step 3, and this commit.
  **Check `git log origin/m7-tasks..m7-tasks` first** (the remote branch may not exist).
- The approved plan is `~/.claude/plans/nested-growing-bear.md`; its checkable items and every
  measured number are in `tasks/TODO.md` under "The Feathers experiment" (M6 closeout block).
- Done: steps 0–3. The gate is green at 363 passed, 6 ignored; Sentrux passes (scan `crates/`, the
  rules file is `crates/.sentrux/rules.toml`); the 30 PPM dumps are byte-identical to `main`.
- What exists: cargo feature `feathers` (default on, release unchanged); `feathers_ui.rs` (plugin),
  `feathers_creation.rs` (scenes, `reconcile`/`place`/`sync`, observers), `creation_panel.rs`
  (Bevy-free `PanelId`/`Payload`/`apply`), `creation_menu.rs` (the model moved out of `menu.rs`,
  named rule methods), `capture.rs` (`--screenshot-composed`, because the window capture is black
  from an agent-launched process), script word `create`, `feathers_app()` in `tests/common/mod.rs`
  (Bevy's no-renderer route), `tests/feathers.rs` (4 tests).
- To see it without a person: `cargo run -p omnis-app -- --seed 7 --no-dev-socket --script
  "party,create" --settle 60 --screenshot-composed <file.png>` (add `--window medium`).

## Next
1. **Owner look 1 is done** (2026-09-20, ultrawide): "Looks good so far." Three updates, recorded as
   3a–3c in the TODO and done before step 4: **drop the classic view** (remove the skin switcher;
   the canvas creation screen remains only for builds without the `feathers` feature until the
   report), **interface scale 1.5 by default** (the owner's preference on 5120×1440), and a **bug:
   after switching Classic → Feathers the Name field is inaccessible**. Root-cause the bug first
   (reproduce headless by respawning the panel, since a class change and an added member respawn it
   too), fix it with a failing-first test, then remove the switcher.
2. Step 4: `tests/feathers.rs` grows: every control by event, a button, checkbox, menu item, slider
   and scrollbar by `PointerInput` through real layout and picking, the layout check at 1280×720 and
   5120×1440 at UI scale 1, 1.5, 2 (1.5 is the reference), the text tree dump, the panel's fighter
   compared with the canvas path's, refusals. Then 5 (Inter, Alegreya Sans: OFL files from the
   upstream GitHub releases, hashes in `assets/README.md`; the font menu; `--frame-stats`), 6 (socket
   `screenshot` gains `target`; measure the window capture against the owner's visible game; owner
   look 2 on the ultrawide), 7 (the report `tasks/plans/feathers-experiment.md`, vision edits only
   on the owner's word).
3. Known rough edges to fix or log: scrollbar thumbs show at full height when nothing overflows;
   the composed capture's letterbox takes the camera's default clear colour; an image target's
   scale factor is one, so a HiDPI window's capture lays the interface out at half size.
4. Still unanswered by the owner: how the M6c play-test went; whether ARCH §4.7 resolving auto
   members' turns inside the simulation stands. Then M7 planning (turn budget, declared reactions).
