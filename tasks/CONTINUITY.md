# Continuity notes

Written 2026-09-20 during the Feathers experiment. Rewrite this file every time it is used; keep it
to state, next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## State
- Branch `m7-tasks`, 17 commits past `main` (`8e111d5`), **pushed by the owner on 2026-09-20 up
  to `5bce8b7`** (`origin/m7-tasks` equals it, checked with `git fetch`). **This continuity commit
  is made after that push and is unpushed.** The push's CI result is not known: `gh` is not
  installed, so ask the owner before planning anything (a red build is the first task). No PR
  was mentioned; if the owner reports a merge, run `git branch --show-current`,
  `git log --oneline -1`, `git log HEAD..main` and `git log main..m7-tasks` first.
- The approved plan is `~/.claude/plans/nested-growing-bear.md`; its checkable items and every
  measured number are in `tasks/TODO.md` under "The Feathers experiment" (M6 closeout block).
- Done: steps 0–6, owner looks 1 and 2, 3a–3c. Only step 7 (the report) and the closeout item's
  review are open. The gate is green at 376 passed, 6 ignored; Sentrux passes (scan `crates/`,
  the rules file is `crates/.sentrux/rules.toml`; `main.rs::parse_args` sits at the 100-line
  limit: a new flag goes into `Look::take` or a helper).
- Owner look 2 (2026-09-20): "the Inter font is preferred. Ultrawide test is clean and starts at
  the right scale. The scale for ultrawide should stay 1.5, scale for medium should be 1.1."
  Built in `5bce8b7`: `FontChoice` defaults to Inter (`--font` overrides only when given);
  `creation_panel::SCALE_FLOOR` 110 under `fitted_scale` (1.1, 1.5, 2.25, 3.0 for a canvas at
  1× to 4×), measured to fit the 960×540 viewport for every class in all three fonts. The small
  1280×720 window gets 1.1 too (same viewport); said to the owner, no objection yet.
- The owner also said the M6c play-test "went well".
- Where things are: `tests/common/feathers.rs` (helpers: `control`, `controls`, `shown`,
  `activate`, `change`, `pointer`, `click_at`, `click_node`, `drag_node`, `keys`, `tab`, `resize`,
  `ultrawide`, `layout_faults` with `Fault`, `text_tree`, `draft_fighter`), `tests/feathers.rs` (8),
  `tests/feathers_panel.rs` (9); `feathers_fonts.rs` (`PanelFonts`, `Face`, `wear`),
  `assets/fonts/{inter,alegreya-sans}` (OFL, hashes in `assets/README.md`); `screenshot` takes
  `target: "canvas" | "window"` (`ops::ShotTarget`; the window is `capture.rs`'s composed capture
  through `socket::window_shot` and `composed_saved`); switches `--font 0|1|2`,
  `--ui-scale <hundredths>`, `--frame-stats` (vertical sync off).
- To see it without a person: `cargo run -p omnis-app -- --seed 7 --no-dev-socket --script
  "party,create" --settle 60 --screenshot-composed <file.png>` (add `--window medium`; an
  agent-launched medium window is clamped to 2560×1378, so the canvas fits once there, never
  twice as on the owner's ultrawide: forced scales in captures say nothing about the ultrawide).

## Next
1. Ask the owner for the push's CI result, and for two numbers only their visible game can give
   (asked once already, 2026-09-20, not yet answered): `frame_time` from `cargo run -p omnis-app
   -- --seed 7 --script "party,create" --frame-stats` against `--script "party"`, and whether a
   plain `--screenshot <file.png>` of a visible window is black (agent-launched: black; 16.7 ms
   with vertical sync, a worse 31 to 43 ms without, not to be trusted). The report can be written
   with those two marked as pending.
2. Step 7: the report `tasks/plans/feathers-experiment.md`: every number in TODO items 1–6; the
   legibility table (x-height and row height in physical pixels for Inter at 1.1 and 1.5, beside
   Fira and Alegreya, against the bitmap font's 10×14); frame time; the three guarantees now (MCP
   screenshot: the window target, proven; screen dumps: a PPM cannot show `bevy_ui`, the text
   tree stands in; headless: every control by event and by pointer); the friction log; a
   recommendation per stage-2 item and for the fate of the canvas creation screen. Then the
   closeout TODO item checked with a review, `horizons.md`, `verification.md` (test count 376),
   `code-map.md`. Vision edits (ARCH A11, §8.4, §8.1; PRD §11.1, D26 outcome) proposed and applied
   only on the owner's word.
3. For the friction log, beyond TODO items 1–6: the scale slider allows a scale the window cannot
   hold (1.5 and 2 at 1280×720); the name's limit is 24 bytes in the form and 24 characters in
   the input; Feathers sliders take drags only (`TrackClick::Drag`); fonts arrive by inheritance a
   frame after the spawn (`Added<TextFont>` missed all but 32 texts); only Fira has a monospace
   face; no dropdown (a menu whose caption we keep); inputs ignore `InteractionDisabled`; a report
   that changes nothing needs a forced `sync` (the typed 99); scrollbar thumbs show at full height
   when nothing overflows; the composed capture's letterbox takes the camera's default clear
   colour; an image target's scale factor is one, so a HiDPI window's capture lays the interface
   out at half size; both crates call themselves experimental and will break at Bevy 0.20.
4. Still unanswered by the owner: whether ARCH §4.7 resolving auto members' turns inside the
   simulation stands. Then M7 planning in plan mode (town, services, rest, progression; the turn
   budget and declared reactions). Dated re-audits: `bevy_egui` 0.42.0 on 2026-10-08, Socket
   re-audit of `rhai` 1.26.1 on 2026-10-10. Local `m5-tasks`, `m6c-tasks`, `m6-closeout-tasks`
   are merged and can be deleted by the owner.
