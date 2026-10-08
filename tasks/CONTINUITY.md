# Continuity notes

Written 2026-10-07, late in M8 step 8. M7 is closed (`68c4d09`). M8 (subjective time with the signal bus) has
steps 0–7, 7b and 8a–8e committed. Step 8f and the owner's acceptance remain. Plans:
- `tasks/plans/m8-time.md` (the milestone);
- `tasks/plans/m8-bus-crate.md` (7b);
- `tasks/plans/m8-api.md` (step 8, the engine's API).

Rewrite this file every time it is used. The durable knowledge lives in `tasks/knowledge/`.

## Owner direction for step 8 (2026-10-07)
- "as we wrap up the docs and review we are creating the API that any number of clients might be interacting
  with. The sim is basically an SRD variant engine that other tracks of development will have to interface with
  while the sim itself is under development." (memory: `sim-is-an-engine-api`)
- **Decisions:**
  - fix the API's gaps now;
  - the contract lives in ARCH §4.9 (with A17) and in a new `docs/api.md`;
  - two tiers: the Rust library (`omnis_sim::api`, views only; `World`'s fields are not API) and the JSON op
    protocol (`ops`, `PROTOCOL` 1, tagged replies).

## Step 8 commits (none pushed; branch `m7a-b-tasks`, not pushed since `4b1bd14`)
- `dfc24a1` **8a, docs as built:**
  - ARCH §4.1–4.4, §9 and §10;
  - §4.9 and A17;
  - README, verification, code-map, horizons, TODO.
- `f25503d` **8b, the views:**
  - `query::here` and `flags`;
  - the sheet's fields and `EffectView` on `MemberView`;
  - `StackView.refusal`, `CombatView.bribe`, `FeatureView.choices` (`ChoiceKind`), `OfferView.subject`;
  - `cast_view`;
  - `tests/api_views.rs`.
- `7b0f3b4` **8c, the protocol:**
  - `PROTOCOL` 1, tagged `Reply`, `rest.get` and `cast.get` (24 tools);
  - the host ops' rules once in `ops.rs`, which fixed **B4** (the app read saves without the loader's limits);
  - `omnis_sim::api`, and `core::error::Error` on the API's errors;
  - `omnis-mcp/tests/replies.rs`;
  - `tests/saves/v6.ron` captured from a worktree of `0bcb02f`.
- `d5be33d`: lesson, a killed mutation run leaves its break in the source.
- **8d** (`34f341f`, `3b74476`, `1e900b7`, `2a0c5b6`) and **8e** (`76c5078`), done by a subagent and reviewed:
  - the app reads a `Views` resource;
  - `SimWorld`'s world is private, with fixtures behind the feature `test-fixtures`;
  - `MemberView.modifiers` added;
  - `defs.rs`.
- `69d48a9`: TODO and code-map for 8d and 8e.

## Next: step 8f
1. **`docs/api.md` and its drift test.** A subagent was writing them when this file was written (it does not
   commit):
   - the drift test is `omnis-cli/tests/api_doc.rs` or `omnis-mcp/tests/`;
   - the agent may also add `EdgeView` and `ViewTile` to `api.rs` and switch `plan.rs`'s import.
   Review its report, then read the document. Check its known gaps; the main one is that JSON clients have no
   op for pack definitions or labels. Run the gate and commit.
2. **ARCH:**
   - the status line to "v0.8, matches M8 as built";
   - check §4.9 against the build: `api.rs` also re-exports `step_lands`, `site_ahead`, `Known`, `layer` and
     `MapId`, and the app's `SimWorld` has methods.
   - Approved with the plan as "ARCH v0.8 as built".
3. **`tasks/acceptance/m8.md`:** drafted, untracked. Check that the test names it cites exist (grep each), then
   commit it with 8f.
4. **TODO:** 8f marked done, plus a review section for M8. Then **ask the owner to run the acceptance**.
   Part 2 needs me to pass about 30 days in the meadow over MCP (`sim_script` with 720 short rests).
5. **Proposals to put to the owner:**
   - a dev command to pass time, for testing time by hand;
   - a `data.*` op family so JSON clients can read pack definitions and labels.
   Neither has been asked for; do not build either unasked.

## Pins (after 8e)
- Gate `tests passed 570 failed 0 ignored 13`. The 13th ignored test is `capture_schema_6_fixture`.
- Walk replay `8711507745385976768` and fight replay `5247080599556612730` (unchanged through step 8).
- `SAVE_SCHEMA` 7, `PROTOCOL` 1, MCP 24 tools, schema proof 94/142, dev branches 13.
- Base pack tuple `(4, 4, 3, 24, 16, 18, 3, 36, 7)`.

## Process
- **Toolchain:** every cargo and gate run needs `DEVELOPER_DIR=/Library/Developer/CommandLineTools`. Run
  `cargo fmt --all` before the gate.
- **The gate:** `scripts/verify.sh > <scratchpad>/gate.txt 2>&1`, run in the background. It takes about 5–10
  minutes. Never edit sources while it runs.
- **Sentrux:** `git add` new files, scan `/Users/john/code/omnis/crates`, then `check_rules`. The rules file
  `crates/.sentrux/rules.toml` is local and gitignored.
- **Mutation runs:** `python3 -u <scratchpad>/mutate.py breaks.json crates [cargo args] > log`, never under
  `timeout` or `| tail`. Afterwards grep the sources for leftover breaks.

## Carry-over
- **Log anomaly (M7):** not reproduced. It becomes B5 if the owner's capture confirms it; B4 is now the
  save-read fix.
- **Not built:**
  - `SpellCast` and `EnemyCasts` are raised, but nothing answers them;
  - `EnemyFlees` and `OwnTurn` have no source;
  - region catch-up waits for M10.
- **Large files:** `plan.rs` 978, `bake.rs` 934, `screen.rs` 898, `loader.rs` 805, `tactics_panel.rs` 778.
- **Dated:** the Socket re-audit of `rhai` 1.26.1 is due 2026-10-10.
