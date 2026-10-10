# Continuity notes

Written 2026-10-10, mid P2d (paused so the owner can restart the terminal for the gate fix below). M8 is closed
(`8921c07`). Branch `m7a-b-tasks`. Rewrite this file every time it is used. Durable knowledge lives in
`tasks/knowledge/`.

## FIRST: the P2d work is in a stash
- `git stash list` shows **`P2d WIP: renames, vocabulary test, docs (not gated)`** (taken with `-u`, so it holds
  the untracked files too). Restore with `git stash pop` on `m7a-b-tasks` before anything else. 52 paths.
- It has NOT passed the full gate. What has passed on it: `cargo check --workspace --all-targets`; `omnis-mcp`
  `vocabulary` and `schema_proof`; `omnis-sim` `api_views`; `cargo clippy -p omnis-sim --all-targets`. A full gate
  of the tree before the last round (the `receiver` renames, the row rule, the `#n` fix) was 581/1 with only the
  vocabulary test failing, which is since fixed.

## The gate is slow because of macOS, not the tests (measured 2026-10-10)
- `scratchpad/measure_gate.py` (session scratchpad, gone after restart; the method: time each gate step, build
  with `cargo test --no-run --message-format=json`, run each test executable twice). Results:
  fmt 0.5 s, clippy workspace 113 s, clippy app 6 s, test build 137 s, **72 test binaries' first runs 2,279 s**,
  second runs 16 s in all, lints and validate 24 s. Total ≈ 43 min, 88 % of it the first launch of each freshly
  linked binary (22.7–61.7 s, median 30.5 s, 42 of 72 within 28–31 s: looks like a network timeout in macOS's
  first-launch check). Slowest real tests: app `feathers_panel` 3.3 s, `camp` 1.6 s, `tactics` 1.6 s.
- Owner ran `DevToolsSecurity -enable`; a fresh binary still took 30.7 s, so the owner is adding the terminal app
  under System Settings → Privacy & Security → Developer Tools and restarting it.
- **After the restart, check first:** `touch crates/omnis-bus/src/lib.rs`, `cargo test -p omnis-bus --lib
  --no-run`, then `/usr/bin/time -p target/debug/deps/omnis_bus-<hash>` (it was 30.7 s). Near 0 s means fixed:
  expect a ~5 min gate. Still ~30 s: the sandbox may cause it (test once with the sandbox off), and the fallback
  is one test binary per crate (`tests/main.rs` with `mod`s, 72 → ~20 binaries). Report the number to the owner.

## Protocol 2, P2d: what the stash holds
Owner, 2026-10-10: "Rename all 11" (the vocabulary test found field names with two JSON types beyond the plan's
inventory; protocol 2 is unreleased, so no extra bump; no save change).
- **Renames** (Rust field = wire name, no serde renames): `ItemCommand::Give {giver, receiver}`; `ItemCommand::Use`,
  `CombatCommand::Use` and the `ItemUsed` event: `receiver` (I first chose `on`, which clashed with
  `SetReactions.on`: lesson written); `Reply::Service.view`; `DieRoll.draw` (omnis-core); `MemberView.member`
  (was `id`), `.worn` (was `equipped`), `.in_front`; `StackView.hps`, `.in_front`; `FeatureView.pay` (was `cost`);
  `CampMember.hit_dice`, `.hit_dice_left`; `RestCommand::Short { spend }` (was `dice`, which would still clash
  with `Roll.dice`); `RestView.long_refusal`. App, MCP schema and bridge tests follow. Stale "slot" doc comments
  on public command fields and the stack/member rejections fixed.
- **`omnis-mcp/tests/vocabulary.rs`** (new; instances moved to `tests/common/commands.rs`, shared with the schema
  proof). Sources: every command instance applied to a headless world, a visit to the town smith, the golden
  walk and fight; every view op after each command. Fails on a key with two JSON types (string and object count
  as one: serde's tagged enums; capitalized variant tags are not checked), a definition key that is not a string
  or null, a member key holding a number no member has had, `caster_id`/`at`, a numeric `row`, or an `index`
  without `stack` beside it. Self-check `the_vocabulary_catches_a_name_that_drifted`. Mutation 3 of 3 caught on
  real data (member named `id` again, a die's `index`, terrain serialized as `map`).
- **`#n` bug** (party_view printed `?` for an unnamed id, everything else `#n`): `party_view.rs` uses
  `names::id_of`; red-first test `api_views.rs::a_number_no_pack_names_reads_the_same_in_the_party_view_as_everywhere`
  (red: `Some("?")`). **BUGS.md entry B6 still to write.**
- **ARCH** §4.9 "Names (protocol 2)" paragraph (the approved rule, adjusted as built: items by kind, no
  `kit_row`/`stores_row`; `stack` with `index` also in `SetMonsterHp`; `row` names only the front or back row,
  `Predicate::Row`, which is saved, so not renamed; Tier 1's `automap`, `map_text`, `step_lands`, `site_ahead`
  speak registry numbers), the vocabulary test, A17's protocol 2 sentence, status line.
- **`docs/api.md`**: protocol 2 throughout; §4 names rule and value types; §5–§10 brought to the code by a docs
  agent (rejections table has a Fields column now); §14; changelog "Protocol 2 (2026-10-10)" with the old → new
  table and migration. Counts unchanged (11 commands, 13 dev, 62 events, 62 rejections, 10 errors, 24 ops).
- `tasks/acceptance/protocol-2.md` (new): status, one cast named the same in command/event/view, reorder, window.
- `tasks/LESSONS.md`: "A new wire name is checked against the whole vocabulary first".

## Left for P2d after `git stash pop`
1. Recapture `docs/api.md` §3's transcripts (still protocol 1: `"protocol":1`, `"map":3`, `"index"` in dice,
   `{"Party":0}`) from a `Headless` world on base+test, seed 1, with the same requests (game.status,
   party.create Wren, Turn Left + Step Forward, Encounter Attack rejected, combat.get NoEncounter, and an attack's
   `AttackResolved`/`Damage`). A small throwaway test or `omnis-mcp --headless` over stdio.
2. BUGS.md B6 (the `#n` fix above; found by the docs agent during P2d).
3. Knowledge: `tasks/knowledge/verification.md` pins (test count, vocabulary test, mutation), `code-map.md`
   (`names.rs`, `word.rs`, `omnis-mcp/tests/vocabulary.rs` and `tests/common/`).
4. TODO: P2d and the parent protocol-2 item checked with detail; plan `tasks/plans/protocol-2.md` marked closed
   (note the 12 extra renames and the as-built rule changes).
5. `cargo fmt --all`, full gate (VERIFY-GREEN), replays unchanged, Sentrux (`git add` new files, scan
   `/Users/john/code/omnis/crates`, `check_rules`), then commits staged by name. Suggested split: (a) the renames
   + `#n` fix + vocabulary test + tests/common (code), (b) ARCH + docs/api.md + acceptance + BUGS + knowledge +
   TODO + plan + LESSONS (docs). Or one P2d commit if the split is awkward; say which.
6. Raise with the owner: the Socket re-audit of `rhai` 1.26.1 (due 2026-10-10).

## Pins (after P2c; P2d expected to move only the test count)
- Gate `tests passed 580 failed 0 ignored 13` before P2d (P2d adds 2 vocabulary + 1 api_views; the agent's run
  showed 581 passed + 1 failed before the `#n` test).
- Walk replay `8711507745385976768`, fight replay `5247080599556612730` (reproduced in the agent's run).
- `SAVE_SCHEMA` 7, `PROTOCOL` 2, MCP 24 tools, schema proof 94/142, dev branches 13, 62 rejections.
- Base pack tuple `(4, 4, 3, 24, 16, 18, 3, 36, 7)`. Sentrux quality 9038 (before P2d), rules pass.

## Process
- Every cargo and gate run: `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first.
- Gate: `scripts/verify.sh > <scratchpad>/gate.txt 2>&1` in the background; no edits while it runs.
- Run `cargo test -p omnis-mcp --test vocabulary` (seconds) after any wire change, before the gate.
- Mutation: apply a break, run the named tests, restore in `finally`; check the sources are clean after.
- zsh: split a file list with `${=files}` when staging by name; avoid backticks inside double quotes in commands.

## Other open TODO items (unscheduled, owner's call)
- `DevCommand::Pass { minutes }`; the `data.*` ops (labels and definitions); `scripts/mcp-probe.py`.
- Editor v1 is held "after M8, before M9" (owner, 2026-09-20).
- Possibly: one test binary per crate (see the gate section), whatever the macOS fix yields.

## Carry-over
- Not built: nothing answers `SpellCast`/`EnemyCasts`; `EnemyFlees`/`OwnTurn` have no source; region catch-up
  waits for M10.
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` 902, `save_and_replay.rs` 810, `tactics_panel.rs` 795.
- The command instances (`tests/common/commands.rs`) have no `Predicate::Row`, so the vocabulary never sees it.
