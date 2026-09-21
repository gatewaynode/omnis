# Continuity notes

Written 2026-09-20 after M7 step 2. Rewrite this file every time it is used; keep it to state,
next step and pointers. The durable knowledge lives in `tasks/knowledge/` (start at its README).

## State
- Branch `m7-tasks`. The owner pushed up to `5bce8b7` on 2026-09-20 and its CI is green (owner;
  the build workflow runs on pull requests and on `main` only). **Every commit after `5bce8b7` is
  unpushed**: the docs commits `c426d8f`, `53d5320`, `184bde2`, `8a664eb`, `d3b163a`, then M7
  step 1 (`f8e2879`; it changes `.github/workflows/ci.yml`, so the next CI run is the first to
  lint the shipped configuration on Linux) and step 2 (the interface kit). If the owner reports a merge:
  `git branch --show-current`, `git log --oneline -1`, `git log HEAD..main`,
  `git log main..m7-tasks` first.
- M6 is closed; Feathers is the toolkit for the game, the editor and all interface going forward
  (owner, 2026-09-20; ARCH A11, §8.4, §15, PRD §11.1); `bevy_egui` is dropped and its pin removed.
- **M7 is planned and approved** (items 0–13 in the M7 block of `tasks/TODO.md`, the design in
  `tasks/plans/m7-town.md`): town first, the turn budget last (M7c, its own plan in plan mode,
  save schema 6); rules to level 20, content to level 3; ARCH §4.7 stands. **Steps 0, 1 and 2
  are done.**
- Step 1 as built: no `feathers` cargo feature (`ui`, `bevy_feathers` in every build; `devtools`
  is the only real feature and holds `capture.rs`); the canvas creation screen, the form's key
  path, `CreationSkin`, `WidgetId::Skill`, `Kind::Toggle` are gone; CreateParty paints
  `Menu::Covered` and only the panel (`CreationAsk`) drives it; `MinimalPlugins` tests use
  `party_by_command`. Shipped tree 320 → 326 crates, shipped binary 68,490,144 → 86,843,056 bytes.
  The name is held to 24 characters and 32 bytes; the scale slider ends at `scale_cap` (1.25 per
  canvas pixel, measured).
- Step 2 as built: `ui_model.rs` and `ui_kit.rs` (`code-map.md` has the inventory and the recipe
  for a new screen). A nested id passes through `bsn!`. The observers publish `UiReport`; each
  screen reads its own in `UiSet::Dispatch`. The panel's text tree is byte-identical to before.
- The gate is green at 379 passed, 6 ignored (`tests passed 379 failed 0 ignored 6`); Sentrux
  passes (scan `/Users/john/code/omnis/crates`, rules `crates/.sentrux/rules.toml`, new files
  `git add`ed by name first; `main.rs::parse_args` sits at the 100-line limit). Pins unmoved:
  tuple `(4, 4, 3, 24, 16, 11, 3, 19)`, walk `238033710167572364`, fight `7317777168019603128`,
  save schema 4, 18 MCP tools.

## Next: M7 step 3, data
`omnis-data`: `ServiceDef { schema, id, name, kind: Inn|Temple|Trainer|Smith|Tavern|Bank|Guild,
items, spells, rumors }`; `MapDef.sites: Vec<Site { x, y, service }>` (serde default);
validations (unknown service, a site on an impassable tile, stock naming unknown items or spells)
with bad-pack rows. Rules files `services.ron` and `rest.ron` (slots and values in
`tasks/plans/m7-town.md`). Base pack: seven services. Test pack: `town.ron` (kind `Town`, one room
behind a door per service, a terrain per service), portals to and from the meadow, **the game
starts in town**. The tuple gains a ninth number (services) and moves with the new slots; **both
replays are rebaselined in the same commit** (`cargo test -p omnis-sim rebaseline -- --ignored`).
Simulation-crate lints apply (integers, `BTreeMap`/`Vec`). Read first: `loader.rs::load_one` (92
lines, near the cap), `content.rs`, `registry.rs`, how `MapDef` and portals are validated, where
the start map is chosen, and the bad-pack test rows. About 450 lines; one commit.

## After step 3
Sim (step 4: save schema 5, `Mode::Town`, `ServiceCommand`), rest, ops, the service panel, camp:
the TODO block. Still owed by the owner, at acceptance a: `--frame-stats` with a panel open against
the canvas alone on their visible game. A call for the owner when convenient: a longer name limit
for wide scripts (`horizons.md`, App and tooling). Dated: Socket re-audit of `rhai` 1.26.1 on
2026-10-10. Local `m5-tasks`, `m6c-tasks`, `m6-closeout-tasks` are merged and can be deleted by
the owner.
