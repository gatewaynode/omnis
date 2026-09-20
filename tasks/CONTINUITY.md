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
3. Debts from the experiment, to place in the plan: the name limit (24 bytes in the form, 24
   characters in the input), a cap on the scale slider against the window, frame time on the
   owner's visible game (`--frame-stats`, `--script "party,create"` against `--script "party"`),
   a `screen.text` op over the text tree with the second `bevy_ui` screen.
4. Dated re-audits: `bevy_egui` 0.42.0 on 2026-10-08 (lapses if the owner agrees the editor goes
   on Feathers), Socket re-audit of `rhai` 1.26.1 on 2026-10-10. Local `m5-tasks`, `m6c-tasks`,
   `m6-closeout-tasks` are merged and can be deleted by the owner.
