# Continuity notes

Written 2026-10-05, mid-M8. M7 is closed (`68c4d09`). M8, subjective time with the signal bus, is
planned and approved (`tasks/plans/m8-time.md`, the same text as `/Users/john/.claude/plans/snoopy-kindling-origami.md`).
Steps 0–6 are committed. Rewrite this file every time it is used; the durable knowledge lives in
`tasks/knowledge/` (start at its README).

## On resuming
- Run `/catchup`, then continue at **step 7** (below). The owner said "do commit 8c and continue":
  carry on through the M8 steps, one commit each, without asking again unless something needs a
  decision.
- **Toolchain:** every cargo and gate run needs `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.
  The gate: `scripts/verify.sh > <scratchpad>/gate.txt 2>&1`, then grep `VERIFY|tests passed|FAILED`.
- **Sentrux:** `git add` new files, scan `/Users/john/code/omnis/crates`, then `check_rules`.
- **Mutation passes:**
  - recreate `<scratchpad>/mutate.py` each session. Its arguments are a breaks JSON file of
    `[name, file, old, new]`, then the crates joined by commas, then extra cargo args such as
    `--test time`;
  - it prints CAUGHT, SURVIVED or BUILD ERROR for each break;
  - it applies each break, runs `cargo test`, and restores the file in a `finally`.
- **Proving a refactor keeps the replays' events:** a throwaway test dumps the golden replays'
  events. Run it in a `git worktree` of HEAD (with its own `CARGO_TARGET_DIR`) and in the tree,
  diff the two dumps, then remove the worktree. This was done in step 3, and step 7 needs it again.
- **Seeing the canvas headless:** write `common::frame(&app).frame.raster` as a PPM, convert it
  with `sips`, and Read the PNG.

## State
- Branch `m7a-b-tasks`, **not pushed since `4b1bd14`**.
- **Commits since `51c38ff`:**
  - `68c4d09`: 8c, M7 closed;
  - `f852eff`: M8 step 0, docs. It covers PRD v0.7 (§7.4 no bank interest; §7.8 "time aligns
    where people gather") and ARCH §4.4, §4.8 the bus, §9.3;
  - `dadcdb0`: step 1, data;
  - `0bcb02f`: step 2, the bus;
  - `0bc94d2`: step 3, time;
  - `6f6cd4a`: step 4, rumors;
  - `26437cd`: step 5, the ops and MCP;
  - `df4a9b7`: these notes, first version;
  - `54c3ab2`: step 6, the HUD.
- Gate: `tests passed 553 failed 0 ignored 12`.
- **Pins:**
  - tuple `(4, 4, 3, 24, 16, 18, 3, 36, 7)` (rule slots 34 → 36);
  - walk replay `8711507745385976768`, fight replay `5247080599556612730`;
  - `SAVE_SCHEMA 7`, MCP 22 tools, proof 94/142.
- **Owner decisions, 2026-10-05:**
  - Regions are data files.
  - The bus carries time; then the reactions move onto it (step 7).
  - Time model: a region's company weighs the party's shared time. A settlement catches up by
    the shared time and sets the party's date; a wild region catches up by lived time. The
    party's age never reverses. Memory: `time-aligns-where-people-gather`.
  - Visible in M8: the calendar as data, and rumors with an age.
  - "There will be no bank interest in this game."
- **As built so far:**
  - **Data:**
    - `omnis-data/src/region.rs` (`RegionDef`, `Region`, `RegionKind`) and `MapData.region`;
    - `Data::calendar()`, and `omnis_core::{Calendar, Date}` with `night`;
    - `packs/base/data/rules/time.ron`: the slots `time.settled` (inputs `shared_time`,
      `stability`) and `time.wild` (inputs `lived`, `stability`). `shared` is a Rhai keyword, and
      dice are `d(1, n)`;
    - the test pack's regions: town (Settlement 900/980, coupled to the crossroads), meadow (Wild
      100/800), dungeon with the depths (Wild 50/600), and crossroads (a mapless Settlement
      700/950);
    - `RumorDef { text, at }`; a bare key still loads, at 0.
  - **`omnis-sim/src/bus.rs`:** `Topic`, `Subscriber { Reconcile, Reactions }`, `Signal::Entered`,
    the `Host` trait, and `drain` (depth 4, 64 signals, returns the count dropped).
  - **`omnis-sim/src/time.rs`:**
    - `PartyTime { shared_milli, date, era }`;
    - `live` (called from `apply::advance`, which now takes `data`);
    - `moved` (portals, retreat, dev teleport), `enter`, the `Sim` host, `reconcile_party` and
      `reconcile_coupled`;
    - `region_clock`, `subscriptions`.
  - `World.{contacts, party_time, bus}`; `v6_to_v7` counts the past in full.
  - **Events:** `Reconciled`, `SignalsDropped`, and `Rumor.ago`.
  - **`time_view.rs`:** `TimeView`, `DateView`, and `Status.date`.
  - `DevCommand::Reconcile`; `Op::{TimeClocks, TimeReconcile}`.
  - **App:**
    - `text::ago_text`; `Names::rumor(id, index, ago)` fills `{ago}`;
    - the HUD clock line `Year 1 day 40 14:20 night  age 2y 40d` (`panels::clock_text(date, age, calendar)`);
    - the sheet's years use the calendar;
    - `MINUTES_PER_DAY` is gone.

## Next steps
- **7, reactions onto the bus.**
  - Add `Signal::Trigger { .. }` (carrying what `combat/reaction.rs`'s `on_attack`, `on_wound`,
    `on_cast`, `on_missile` and `on_enemy_cast` take) on `Topic::Battle`.
  - The `Reactions` subscriber calls the existing `fire`/`react`.
  - Combat start subscribes `Battle → Reactions` and the end unsubscribes. Saves made mid-fight
    need the subscription: `v6_to_v7` or `begin_after_load`.
  - Prove it: the fight replay's event list is identical (the worktree diff), plus a nested-raise
    cap test with a synthetic subscriber and a mutation pass.
  - The `Sim` host in `time.rs` may move to `bus.rs` or its own file.
- **8, docs and review.**
  - ARCH v0.8 as built. Fix the §4.4 wording: the inputs are `lived`, `shared_time` and
    `stability`.
  - **Ask the owner** about PRD §7.8's consequences bullet, "a region left for a subjective year
    has changed by roughly a year": it reads against the new model and was not in the approved
    edit.
  - `code-map.md`, `horizons.md` (the bus's later uses; ecosystem catch-up waits for M10),
    `verification.md` pins, and the README if it lists MCP tools.
  - `tasks/acceptance/m8.md`, then owner acceptance closes M8.

## Carry-over and watch-outs
- **Log anomaly (from M7):** not reproduced. It becomes B4 only if the owner's screen capture
  confirms it.
- **Not built:**
  - `SpellCast` and `EnemyCasts` are raised but nothing answers them;
  - `EnemyFlees` and `OwnTurn` have no source;
  - items, features and the weapon declare no reactions;
  - no pack declares flags.
- **Large files:** `plan.rs` 953, `bake.rs` 934, `screen.rs` 880, `loader.rs` 805,
  `tactics_panel.rs` 778, `command.rs` 761.
- **Dated:** the Socket re-audit of `rhai` 1.26.1 is due 2026-10-10.
