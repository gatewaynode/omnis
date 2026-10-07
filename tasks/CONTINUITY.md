# Continuity notes

Written 2026-10-07, late M8. M7 is closed (`68c4d09`). M8, subjective time with the signal bus, has
steps 0–7 and 7b committed; only **step 8** (docs and review, then owner acceptance) is left. Plans:
`tasks/plans/m8-time.md` (the milestone) and `tasks/plans/m8-bus-crate.md` (7b). Rewrite this file
every time it is used; the durable knowledge lives in `tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then do **step 8** (below). The owner's standing "continue" covers it: one commit
  per item, no asking again unless something needs a decision. ARCH and PRD edits for step 8 were
  approved with the M8 plan ("ARCH v0.8 as built"); anything beyond as-built text is asked first.
- **Toolchain:** every cargo and gate run needs `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.
  The gate: `scripts/verify.sh > <scratchpad>/gate.txt 2>&1`, then grep `VERIFY|tests passed|FAILED`.
  The gate runs `cargo fmt --check` first: run `cargo fmt --all` before it.
- **Sentrux:** `git add` new files, scan `/Users/john/code/omnis/crates`, then `check_rules`. Its rules
  file `crates/.sentrux/rules.toml` is gitignored (local); it now has a `bus` layer (order 0) and
  boundaries keeping `omnis-bus` a leaf.
- **Mutation passes:** `<scratchpad>/mutate.py breaks.json <crates,comma> [cargo args]` still exists
  in this session's scratchpad (recreate if gone: apply each `[name, file, old, new]`, run
  `cargo test -p ... --no-fail-fast`, print CAUGHT by test names / SURVIVED / BUILD ERROR, restore
  in `finally`). A break counts only with a named failing test.

## State
- Branch `m7a-b-tasks`, **not pushed since `4b1bd14`**. Tree clean after the continuity commit.
- **Commits since the last continuity (`5d9dcfd`):**
  - `c4baeb4`: step 7, combat reactions on the bus;
  - `f3d428d`: 7b docs (ARCH §3 row, §4.8 rewritten, A16; horizons; TODO; `m8-bus-crate.md`);
  - `c36006f`: 7b code, the `omnis-bus` crate.
- Gate: `tests passed 557 failed 0 ignored 12`.
- **Pins:** tuple `(4, 4, 3, 24, 16, 18, 3, 36, 7)`; walk replay `8711507745385976768`, fight replay
  `5247080599556612730` (both unchanged by steps 7 and 7b); `SAVE_SCHEMA 7`, MCP 22 tools, proof
  94/142.
- **Step 7 as built:** `Signal::Battle(Cue)`, `Cue::{Attack, Missile, EnemyCast, Wound, Cast}` on
  `Topic::Battle`; the five `reaction::on_*` raise their cue and drain at once through
  `combat/reaction.rs::Fight` (holds the attack roll; returns the first `RuleError`); `turn.rs`
  `start` subscribes `Battle → Reactions` before round 1, `finish` unsubscribes; `v6_to_v7`
  subscribes a fight saved mid-way. Proof: both replays plus 80 scripted fights (360 reactions)
  identical events vs the previous commit. New tests in `tests/reactions.rs` (subscription held for
  the fight, schema-6 mid-fight save, a heal for a fall answers falls only). Mutation 8 of 10; the 2
  survivors are unobservable today (error guard needs a second battle subscriber; `SpellCast` has no
  answering action).
- **Step 7b as built (owner, 2026-10-06: "We'll have to expand that bus in the future. We should
  document an architecture for our bus design and should probably isolate it in its own crate."):**
  - Owner choices: the crate is **mechanism only** (vocabulary stays in `omnis-sim`); the
    architecture designs **saved deferred signals, a topic hierarchy, pack-declared subscribers**;
    veto and ordering phases went to horizons.
  - `crates/omnis-bus`: `no_std`, `serde` only (`ron` dev-dep); `Bus<T, S>`, traits `Signal` (assoc
    `Topic`) and `Host` (assoc `Signal`, `Subscriber`), `drain`, `MAX_DEPTH` 4, `MAX_SIGNALS` 64;
    five tests on their own vocabulary. `subscribers(topic)` takes the topic by value.
  - `omnis-sim/src/bus.rs` is the vocabulary: `Topic`, `Subscriber`, `Signal`, `Cue`,
    `type Bus = omnis_bus::Bus<Topic, Subscriber>`, re-exports `Host`, `drain`, the limits.
  - Workspace member and dep, `scripts/lint-sim.sh` both lists, `CLAUDE.md` crate list.
  - The three expansions are open TODO items under "M9–M12" (each with its first consumer).

## Next: step 8, docs and review
- **ARCH v0.8 as built.** Status line to v0.8. §4.4: the rule inputs are `lived`, `shared_time` and
  `stability` (the text still says `shared`); check §4.1/§4.2 against the built `World` fields and
  events (`Reconciled`, `SignalsDropped`, `Rumor.ago`), §9.3 the `time.clocks`/`time.reconcile`
  rows, the calendar values in `rules/time.ron`. §4.8 is already current (7b).
- `tasks/knowledge/code-map.md`: the M8 files (`time.rs`, `time_view.rs`, `omnis-data/src/region.rs`,
  `Data::calendar`, `text::fill`, app `panels::clock_text`, `text::ago_text`); the bus lines are in.
- `tasks/knowledge/horizons.md`: ecosystem catch-up waits for M10 (bus entry already resolved).
- `tasks/knowledge/verification.md`: the pins above; README if it lists MCP tools (22).
- `tasks/acceptance/m8.md`: an owner script with "what you will see" — HUD clock line, years in the
  meadow vs the town's date snapping on entry, age never reversing, tavern rumors with their age,
  `time_clocks` over MCP listing the crossroads, a mid-fight save and load keeping reactions.
- Then owner acceptance closes M8 (TODO M8 heading closed, plan closed).

## Carry-over and watch-outs
- **Log anomaly (from M7):** not reproduced; becomes B4 only if the owner's screen capture confirms.
- **Not built:** `SpellCast` and `EnemyCasts` raised but nothing answers; `EnemyFlees` and `OwnTurn`
  have no source; items, features and the weapon declare no reactions; no pack declares flags.
- **Large files:** `plan.rs` 953, `bake.rs` 934, `screen.rs` 880, `loader.rs` 805,
  `tactics_panel.rs` 778, `command.rs` 761.
- **Dated:** the Socket re-audit of `rhai` 1.26.1 is due 2026-10-10.
