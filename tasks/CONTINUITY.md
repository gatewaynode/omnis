# Continuity notes

Written 2026-09-20 during the Feathers experiment. Rewrite this file every time it is used; keep it
to state, next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## State
- Branch `m7-tasks`, fast-forwarded to `main` at `8e111d5`. **Nothing on it is pushed and
  `origin/m7-tasks` does not exist**: `509187a` … `fdb1e85` (steps 0–3, 3a–3c, docs), `a560c32`
  step 4, `b4046e3` step 5, `07c6a01` docs, `ea35873` step 6, and this commit.
- The approved plan is `~/.claude/plans/nested-growing-bear.md`; its checkable items and every
  measured number are in `tasks/TODO.md` under "The Feathers experiment" (M6 closeout block).
- Done: steps 0–6, owner look 1, 3a–3c (step 6: `screenshot` takes `target: "canvas" | "window"`,
  the window is `capture.rs`'s composed capture, proven over the dev socket on the running game;
  `omnis-mcp::tools::screenshot_tool` was split out for Sentrux). The gate is green at 376 passed, 6 ignored; Sentrux
  passes (scan `crates/`, the rules file is `crates/.sentrux/rules.toml`; `main.rs::parse_args`
  sits at the 100-line limit: a new flag goes into `Look::take` or a helper).
- Step 4 left: `tests/common/feathers.rs` (the helpers: `control`, `controls`, `shown`, `activate`,
  `change`, `pointer`, `click_at`, `click_node`, `drag_node`, `keys`, `tab`, `resize`, `ultrawide`,
  `layout_faults` with `Fault`, `text_tree`, `draft_fighter`), `tests/feathers.rs` (8) and
  `tests/feathers_panel.rs` (9). A bug it found is fixed: every report makes the next `sync`
  write the widgets again (a typed 99 showed 99 while the form held 15).
- Step 5 left: `assets/fonts/{inter,alegreya-sans}` (OFL, hashes in `assets/README.md`),
  `feathers_fonts.rs` (`PanelFonts`, `Face`, `wear`), `FontChoice`, the footer's Font menu, and
  the switches `--font 0|1|2`, `--ui-scale <hundredths>`, `--frame-stats` (vertical sync off).
- To see it without a person: `cargo run -p omnis-app -- --seed 7 --no-dev-socket --script
  "party,create" --settle 60 --font 1 --ui-scale 150 --screenshot-composed <file.png>` (add
  `--window medium`).

## Next
1. Owner look 2 is done (2026-09-20): Inter is the pick and the panel starts in it; the scale
   stays 1.5 on the ultrawide and is 1.1 where the canvas fits once (`SCALE_FLOOR`), measured
   to fit. The M6c play-test "went well". Two numbers still need the owner's visible game, for
   the report: plain `--screenshot` against `--screenshot-composed`, and `--frame-stats` with
   and without the panel (agent-launched: 16.7 ms with vertical sync, 31 to 43 ms without).
2. Step 7: the report `tasks/plans/feathers-experiment.md`; vision edits only on the owner's word.
3. For the report's friction log, beyond TODO items 1–5: the scale slider allows a scale the
   window cannot hold (1.5 and 2 at 1280×720); the name's limit is 24 bytes in the form and 24
   characters in the input; Feathers sliders take drags only (`TrackClick::Drag`); fonts arrive by
   inheritance a frame after the spawn; only Fira has a monospace face; scrollbar thumbs show at
   full height when nothing overflows; the composed capture's letterbox takes the camera's default
   clear colour; an image target's scale factor is one, so a HiDPI window's capture lays the
   interface out at half size.
4. Still unanswered by the owner: whether ARCH §4.7 resolving auto
   members' turns inside the simulation stands. Then M7 planning (turn budget, declared reactions).
