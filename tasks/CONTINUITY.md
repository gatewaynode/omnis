# Continuity notes

Written 2026-09-20 at the close of M6 and the start of M7. Rewrite this file every time it is used; keep it to state,
next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## State
- Branch `m7-tasks`. The owner pushed up to `5bce8b7` on 2026-09-20; **`c426d8f` and the M6
  closeout docs commit after it are unpushed.** CI runs on pull requests and on `main` only; the
  owner reported the build CI for the push as still running (2026-09-20, "CI test lagged. Waiting
  on the outcome. But please go ahead with planning"). Ask for the result; a red build is the
  first task. It is the first Linux build of the `feathers` feature (`bevy_ui` and the rest).
  If the owner reports a merge: `git branch --show-current`, `git log --oneline -1`,
  `git log HEAD..main`, `git log main..m7-tasks` first.
- **M6 is closed** (owner, 2026-09-20): M6c's play-test "went well"; the Feathers experiment is
  "a success" and the owner is "committing to moving the rest of the play UI over bit by bit".
  The report, the pattern for the next `bevy_ui` screen, the debts and the stage-2
  recommendations are in `tasks/plans/feathers-experiment.md`; the summary is in
  `tasks/knowledge/horizons.md` ("Modern presentation").
- The gate is green at 376 passed, 6 ignored; Sentrux passes (scan `crates/`, rules
  `crates/.sentrux/rules.toml`).

## Next
1. **M7 is planned and approved (2026-09-20)**: the items are the M7 block of `tasks/TODO.md`, the
   design is `tasks/plans/m7-town.md`. Owner decisions: town first and the turn budget last (M7c,
   planned in plan mode when reached); the shipped build carries `bevy_ui` from step 1 and the
   canvas creation screen goes there; rules to level 20, content to level 3; ARCH §4.7's auto
   resolution stands. Step 0 (docs: PRD v0.6, ARCH v0.4, the editor on Feathers written as
   proposed only) is done. **Next is step 1**, after the CI result.
2. Every commit of this session after `5bce8b7` is unpushed.
   **Step 1's survey (done 2026-09-20, no code changed yet).** Before-numbers: the shipped tree
   (`--no-default-features`) is 320 crates and the default tree 329, counted with `cargo tree -p
   omnis-app -e normal [flags] --prefix none | sed 's/ (\*)$//' | sort -u | wc -l`; the shipped
   binary's size is 68,490,144 bytes (68.5 MB; `cargo build -p omnis-app --release
   --no-default-features --locked`, 1 m 40 s clean). The feature's `cfg` sites: `lib.rs` 21, 36, 38, 40; `main.rs`
   69, 258; `dev.rs` 140, 166; `socket.rs` 66, 276, 320, 322, 431, 444; `tests/socket.rs` 195,
   197; `tests/common/mod.rs` 6, 94; the heads of `tests/feathers.rs` and `tests/feathers_panel.rs`.
   `capture.rs` is what `dev.rs` and `socket.rs` reach: those sites become `devtools`-only, and
   `capture.rs` moves under `devtools`. The canvas creation screen to remove: `screens.rs`
   `creation`, `creation_identity`, `creation_scores`, `creation_skills` and their tests (254-620);
   `screen.rs` `Target::Creation`, `Menu::Creation`, the `Skill` click arm, its tests and the
   `creation` entry of `dump_screens`; `ui.rs` 494-497 (`Menu::Covered` becomes what CreateParty
   always shows); `menus.rs` `CreationSkin`, `Screens.skin`, the `click_keys` arm at 200 and the
   key arm at 333-336; `creation_menu.rs` `key`, `adjust`, `confirm`, `cursor`, `skill_cursor`,
   the `ROW_*` constants and `lines` if nothing else reads them (its three tests move onto the
   named methods); `feathers_ui.rs` 83 and 103, `feathers_creation.rs` 431. Tests:
   `draft_fighter_by_mouse` (`tests/common/mod.rs` 304; used by `pointer.rs` 8 times, `combat.rs`
   3, `feathers_panel.rs` for the two-paths test, which then compares the panel with the party
   command instead) becomes `party_by_command`, which writes `CreationAsk(Add(draft))` and
   `CreationAsk(Begin)`; `start_new_game_by_mouse` is the title's and stays.
3. Debts from the experiment, to place in the plan: the name limit (24 bytes in the form, 24
   characters in the input), a cap on the scale slider against the window, frame time on the
   owner's visible game (`--frame-stats`, `--script "party,create"` against `--script "party"`),
   a `screen.text` op over the text tree with the second `bevy_ui` screen.
4. Dated re-audits: `bevy_egui` 0.42.0 on 2026-10-08 (lapses if the owner agrees the editor goes
   on Feathers), Socket re-audit of `rhai` 1.26.1 on 2026-10-10. Local `m5-tasks`, `m6c-tasks`,
   `m6-closeout-tasks` are merged and can be deleted by the owner.
