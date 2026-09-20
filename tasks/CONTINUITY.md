# Continuity notes

Written 2026-09-20 after M7 step 1. Rewrite this file every time it is used; keep it to state,
next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## State
- Branch `m7-tasks`. The owner pushed up to `5bce8b7` on 2026-09-20 and its CI is green (owner;
  the build workflow runs on pull requests and on `main` only). **Every commit after `5bce8b7` is
  unpushed**: the docs commits `c426d8f`, `53d5320`, `184bde2`, `8a664eb`, `d3b163a`, then M7
  step 1 (the first code commit of M7; it changes `.github/workflows/ci.yml`, so the next CI run
  is the first to lint the shipped configuration on Linux). If the owner reports a merge:
  `git branch --show-current`, `git log --oneline -1`, `git log HEAD..main`,
  `git log main..m7-tasks` first.
- M6 is closed; Feathers is the toolkit for the game, the editor and all interface going forward
  (owner, 2026-09-20; ARCH A11, §8.4, §15, PRD §11.1); `bevy_egui` is dropped and its pin removed.
- **M7 is planned and approved** (items 0–13 in the M7 block of `tasks/TODO.md`, the design in
  `tasks/plans/m7-town.md`): town first, the turn budget last (M7c, its own plan in plan mode,
  save schema 6); rules to level 20, content to level 3; ARCH §4.7 stands. **Steps 0 and 1 are
  done.**
- Step 1 as built: no `feathers` cargo feature (`ui`, `bevy_feathers` in every build; `devtools`
  is the only real feature and holds `capture.rs`); the canvas creation screen, the form's key
  path, `CreationSkin`, `WidgetId::Skill`, `Kind::Toggle` are gone; CreateParty paints
  `Menu::Covered` and only the panel (`CreationAsk`) drives it; `MinimalPlugins` tests use
  `party_by_command`. Shipped tree 320 → 326 crates, shipped binary 68,490,144 → 86,843,056 bytes.
  The name is held to 24 characters and 32 bytes; the scale slider ends at `scale_cap` (1.25 per
  canvas pixel, measured).
- The gate is green at 378 passed, 6 ignored (`tests passed 378 failed 0 ignored 6`); Sentrux
  passes (scan `/Users/john/code/omnis/crates`, rules `crates/.sentrux/rules.toml`, new files
  `git add`ed by name first; `main.rs::parse_args` sits at the 100-line limit). Pins unmoved:
  tuple `(4, 4, 3, 24, 16, 11, 3, 19)`, walk `238033710167572364`, fight `7317777168019603128`,
  save schema 4, 18 MCP tools.

## Next: M7 step 2, the interface kit
Out of `feathers_creation.rs` into `ui_kit.rs` (and `ui_kit_scenes.rs` if size asks): the row,
column, button, title and dropdown-from-a-menu scenes; one `PanelRoot`; `place`, `scale` and the
fonts on every panel (`feathers_fonts::wear` looks only under the creation root today); one
app-wide id (`UiId::{Creation(PanelId), Service(..), Camp(..)}` on `Control`/`Shown`) so the five
observers stay single. **Find out first whether the nested enum passes through `bsn!`** (an enum
component inside `bsn!` needs `#[derive(FromTemplate)]`); the fallback is per-screen marker
components with a shared observer helper. Test helpers in `tests/common/feathers.rs` become
id-generic. No behaviour change: the panel's tests pass unedited except for imports. About 400
lines moved; one commit.

## After step 2
Data (step 3: `ServiceDef`, `MapDef.sites`, `services.ron`, `rest.ron`, `town.ron`, the game
starts in town; tuple and both replays rebaselined), then sim, rest, ops, the service panel, camp:
the TODO block. Still owed by the owner, at acceptance a: `--frame-stats` with a panel open against
the canvas alone on their visible game. A call for the owner when convenient: a longer name limit
for wide scripts (`horizons.md`, App and tooling). Dated: Socket re-audit of `rhai` 1.26.1 on
2026-10-10. Local `m5-tasks`, `m6c-tasks`, `m6-closeout-tasks` are merged and can be deleted by
the owner.
