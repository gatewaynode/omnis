# Continuity notes

Written 2026-09-20 at the start of M7, for a compact. Rewrite this file every time it is used; keep
it to state, next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at
its README).

## State
- Branch `m7-tasks`. The owner pushed up to `5bce8b7` on 2026-09-20 and **its CI is green** (owner,
  same day; the build workflow runs on pull requests and on `main` only). **Every commit after
  `5bce8b7` is unpushed**, all docs: `c426d8f`, `53d5320` (M6 closed, the experiment's report),
  `184bde2` (M7 planned, PRD v0.6, ARCH v0.4), `8a664eb` (step 1's survey) and this one. If the
  owner reports a merge: `git branch --show-current`, `git log --oneline -1`, `git log HEAD..main`,
  `git log main..m7-tasks` first.
- **M6 is closed.** The Feathers experiment is a success by the owner's word; report, pattern for
  the next `bevy_ui` screen and debts in `tasks/plans/feathers-experiment.md`.
- **Owner, 2026-09-20: "That is a yes to use feathers on the editor and all user interface moving
  forward (and retrofit as convenient)."** Recorded in ARCH A11, §8.4, §15 and PRD §11.1;
  `bevy_egui` is dropped (never compiled; its workspace pin `bevy_egui = "=0.41.1"` in the root
  `Cargo.toml` is removed in step 1); the 2026-10-08 re-audit lapsed.
- **M7 is planned and approved**: items 0–13 in the M7 block of `tasks/TODO.md`, the design
  (slots, commands, events, screens) in `tasks/plans/m7-town.md`. Owner decisions: town first, the
  turn budget last (M7c, planned in plan mode when reached, save schema 6); the shipped build
  carries `bevy_ui` from step 1 and the canvas creation screen goes there; rules to level 20,
  content to level 3; ARCH §4.7's auto resolution inside the simulation stands. Step 0 is done.
- The gate is green at 376 passed, 6 ignored (its log says `tests passed 376 failed 0 ignored 6`);
  Sentrux passes (scan `/Users/john/code/omnis/crates`, rules `crates/.sentrux/rules.toml`,
  new files `git add`ed by name first; `main.rs::parse_args` sits at the 100-line limit).

## Next: M7 step 1, the shipped build carries the interface (no code changed yet)
- Before-numbers: the shipped tree (`--no-default-features`) is 320 crates and the default tree
  329, counted with `cargo tree -p omnis-app -e normal [flags] --prefix none | sed 's/ (\*)$//' |
  sort -u | wc -l`; the shipped binary is 68,490,144 bytes (68.5 MB; `cargo build -p omnis-app
  --release --no-default-features --locked`, 1 m 40 s clean). Report both again after.
- The feature goes: `crates/omnis-app/Cargo.toml` (`default = ["devtools"]`, `bevy` features gain
  `ui` and `bevy_feathers`). Its `cfg` sites: `lib.rs` 21, 36, 38, 40; `main.rs` 69, 258; `dev.rs`
  140, 166; `socket.rs` 66, 276, 320, 322, 431, 444; `tests/socket.rs` 195, 197;
  `tests/common/mod.rs` 6, 94; the heads of `tests/feathers.rs` and `tests/feathers_panel.rs`.
  `capture.rs` is reached only from `dev.rs` and `socket.rs`: it moves under `devtools`. The
  `--font`, `--ui-scale`, `--frame-stats` switches in `main.rs` (`Look`) are devtools flags today;
  check what a build without devtools still parses.
- The gate's release clippy line (`scripts/verify.sh` line 8) stays as it is and now covers
  `bevy_ui`; **`.github/workflows/ci.yml` gains the same line** (it has only the default one).
- The canvas creation screen goes: `screens.rs` `creation`, `creation_identity`,
  `creation_scores`, `creation_skills` and their tests (about 254–620); `screen.rs`
  `Target::Creation`, `Menu::Creation`, the `Skill` click arm, its tests and the `creation` entry
  of `dump_screens`; `ui.rs` 494–497 (`Menu::Covered` becomes what CreateParty always shows);
  `menus.rs` `CreationSkin`, `Screens.skin`, the `click_keys` arm near 200 and the key arm near
  333–336; `creation_menu.rs` `key`, `adjust`, `confirm`, `cursor`, `skill_cursor`, the `ROW_*`
  constants, and `lines` if nothing else reads it (its three tests move onto the named methods);
  `feathers_ui.rs` 83 and 103, `feathers_creation.rs` 431; `WidgetId::Skill` if nothing else uses it.
- Tests: `draft_fighter_by_mouse` (`tests/common/mod.rs` 304; `pointer.rs` 8 uses, `combat.rs` 3,
  and `feathers_panel.rs`'s two-paths test, which then compares the panel with the party command)
  becomes `party_by_command`: `CreationAsk(Add(draft))` then `CreationAsk(Begin)`.
  `start_new_game_by_mouse` is the title's and stays. The test count will drop; say by how many
  and why in the TODO.
- Two debts land here with tests: `CreationForm::set_name` counts characters as the input does
  (24; today bytes); the scale slider is capped at the largest scale the window holds
  (`layout_faults` names the fault today: 1.5 and 2 at 1280×720).
- Docs in the same commit: ARCH §8.1's features line (it says the feature exists until this step),
  `verification.md`, `code-map.md`, the TODO item with the numbers.

## After step 1
Step 2 (the interface kit out of `feathers_creation.rs`; find out first whether one app-wide
control id passes through `bsn!`), then data, sim, rest, ops, the service panel, camp: the TODO
block. Still owed by the owner, at acceptance a: `--frame-stats` with a panel open against the
canvas alone on their visible game. Dated: Socket re-audit of `rhai` 1.26.1 on 2026-10-10. Local
`m5-tasks`, `m6c-tasks`, `m6-closeout-tasks` are merged and can be deleted by the owner.
