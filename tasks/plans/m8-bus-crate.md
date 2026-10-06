# The signal bus: its own crate and a written architecture (M8 step 7b)

## Context
The bus was built in M8 (`crates/omnis-sim/src/bus.rs`, 310 lines with tests) and now carries two flows: region entry to reconciliation (step 3) and combat cues to reactions (step 7, `c4baeb4`). The owner: "We'll have to expand that bus in the future. We should document an architecture for our bus design and should probably isolate it in its own crate."

**Owner decisions (2026-10-06):**
- **The crate owns the mechanism only.** `Topic`, `Subscriber`, `Signal` and `Cue` stay in `omnis-sim`, so a new topic never edits the crate.
- **The architecture designs three expansions now:**
  - saved deferred signals;
  - a topic hierarchy;
  - pack-declared subscribers.
- **Veto and ordering phases go to horizons.**

**Scope:** this step extracts the crate and writes the architecture. The three expansions are designed, not built: none has a consumer yet. Each is built with the first system that needs it, and the ARCH text names that system. This lands as step 7b, before step 8, so ARCH v0.8 includes it.

## The crate: `crates/omnis-bus`
- **Dependencies:** `no_std` with `alloc`; `serde` only. `ron` is a dev-dependency, for the save round-trip test.
- **What moves out of `bus.rs`, made generic:**
  ```rust
  pub trait Signal { type Topic: Ord + Clone; fn topic(&self) -> Self::Topic; }
  pub struct Bus<T, S> { subs: BTreeMap<T, Vec<S>> }       // Serialize/Deserialize with bounds; Default
  impl Bus { subscribe, unsubscribe, subscribers }        // unchanged semantics
  pub trait Host {
      type Signal: Signal;
      type Subscriber: Copy + Eq;
      fn bus(&self) -> &Bus<<Self::Signal as Signal>::Topic, Self::Subscriber>;
      fn deliver(&mut self, to: Self::Subscriber, signal: &Self::Signal, raised: &mut Vec<Self::Signal>);
  }
  pub fn drain<H: Host>(host: &mut H, signals: Vec<H::Signal>) -> u32   // dropped count
  pub const MAX_DEPTH: u8 = 4; pub const MAX_SIGNALS: u32 = 64;
  ```
- **Tests:**
  - the four existing unit tests (call order, save round trip, depth cap, budget) and the battle-cue cap test move to the crate;
  - they get a test-local topic, subscriber and signal, so the crate is tested without the game.
- **The module doc** keeps the crate survey's reasons for building our own bus.

## `omnis-sim` after the move
- `src/bus.rs` becomes the vocabulary:
  - `Topic`, `Subscriber`, `Signal` (with `Cue`) and `impl omnis_bus::Signal for Signal`;
  - `pub type Bus = omnis_bus::Bus<Topic, Subscriber>`, so `World.bus` and its callers keep their spelling.
- The hosts set their associated types: `time.rs::Sim` and `combat/reaction.rs::Fight`.
- **The save form must not change.** The generic struct keeps the name `Bus` and the field `subs`, so the RON is identical:
  - `SAVE_SCHEMA` stays 7;
  - both replay fingerprints stay `8711507745385976768` and `5247080599556612730`.
  - If either fingerprint moves, stop and find out why; no rebaseline.

## Workspace and rules
- `Cargo.toml`: a member, and a `[workspace.dependencies]` entry.
- `scripts/lint-sim.sh`: add the crate to `SIM_CRATES` and `NO_STD_CRATES`. The lint's self-test still passes.
- `CLAUDE.md`: add `omnis-bus` to the list of simulation crates (a one-word change; approving this plan approves it).
- Sentrux rules: check `.sentrux/rules.toml` for any crate-layer rule that needs the new crate.

## The architecture text (ARCHITECTURE.md; approving this plan approves it)
- **§3, the crates table, a new row:**
  - `omnis-bus`, a lib: "The signal bus mechanism: saved subscriptions in call order, synchronous first-in-first-out delivery, depth and budget limits. Generic over a topic, subscriber and signal that its user defines; knows no game types."
  - Allowed dependencies: `serde`.
  - It is a leaf: only `omnis-sim` depends on it.
- **§4.8, rewritten as "The signal bus (`omnis-bus`)", in these parts:**
  1. **Purpose.** Systems talk without calling each other: the raiser names a topic, and subscribers answer. It serves A13 (no global clock; things happen at contact) and the determinism rules (A5, A14).
  2. **Principles.**
     - Subscriptions are data, never closures, so they save and replay.
     - Call order is subscription order.
     - Delivery is synchronous at the raise, so moving a direct call onto the bus keeps the event order (proved in steps 3 and 7 by event diffs).
     - Nested raises are queued one level deeper.
     - Limits drop and report (`SignalsDropped`); never a panic, never a rejection after the world changed.
     - No threads, no async, no hashing.
  3. **Mechanism and vocabulary.**
     - The crate's API, as above.
     - `omnis-sim` owns the vocabulary: today `Topic::{Region, Battle}`, `Subscriber::{Reconcile, Reactions}`, `Signal::{Entered, Battle(Cue)}`.
     - A host is the simulation at a moment (time's `Sim`, combat's `Fight`) and matches each (subscriber, signal) pair to a handler.
  4. **Current subscriptions.**
     - Every region's topic goes to `Reconcile` from the world's start.
     - `Battle` goes to `Reactions` while a fight lasts.
     - Save schema 7 writes the defaults.
  5. **Designed expansions** (each built with its first consumer):
     - **Topic hierarchy.**
       - `Host::parent(&self, topic) -> Option<Topic>`, a default method returning none. The host answers it because the nesting is data: a map section's region comes from the pack.
       - `drain` delivers to the topic's subscribers, then to its parent's, up to the root; the most specific are called first. The chain is capped at 8 levels.
       - A subscriber on two levels is called once, at the most specific.
       - It lands with the first topic below a region (a map section or a city district). `Battle` stays a root.
     - **Saved deferred signals.**
       - `Bus` gains a saved queue: `held: BTreeMap<T, Vec<S>>`, in raise order, ordered by topic.
       - `hold(topic, signal)` keeps a signal until the host calls `release(topic)` at a contact (A13: no tick delivers it). The released signals are drained like any other.
       - Each topic holds at most 256 signals; past that the oldest is dropped and counted.
       - Held signals must serialize, so the save schema moves when the first is held.
       - It lands with the first news that waits for the party: rumors raised by events, or the ecosystem's catch-up in M10.
     - **Pack-declared subscribers.**
       - `Subscriber::Script(ScriptId)` in the vocabulary.
       - Packs declare `(topic, script)` pairs as data. They are checked at load (a known topic kind, a known script) and subscribed after the built-in subscribers, so the game's own answers come first.
       - A script runs under Rhai's `Script` profile (§5.1, not built), with its operation limits.
       - It cannot change the world directly: it returns signals to raise and commands from a closed list, which the host validates as it validates player commands. The bus's depth and budget bound a script that raises in a loop.
       - It lands with the `Script` profile (mods, PRD §13).
  6. **Not in the design** (to horizons): a subscriber that vetoes or rewrites a signal, and ordering phases beyond subscription order.
  7. **Why our own:** the survey of 2026-10-05, unchanged.
- **The decisions table, a new row A16:**
  - Decision: "The signal bus is its own crate, mechanism only, with the vocabulary in `omnis-sim`."
  - Rejected: a bus module inside `omnis-sim`; a crate that also holds the vocabulary; an external crate.
  - Rationale: the owner's direction to expand it (2026-10-06); a crate boundary keeps the mechanism free of game types and testable alone.
- **The status line:** v0.8 in progress (step 8 finishes it).

## Other docs
- `tasks/knowledge/horizons.md`, the M8 bus entry:
  - it is resolved;
  - veto and ordering phases are added as a horizon;
  - it points to ARCH §4.8 for the three designed expansions.
- `tasks/knowledge/code-map.md`: the new crate.
- `tasks/TODO.md`:
  - a 7b line under M8;
  - each expansion as an open item under the milestone that needs it (the map-section topic and held signals under M10, script subscribers under Phases 2–5).
- `tasks/plans/m8-time.md`: a note on step 7b.

## Commits (gate green each time, Sentrux, staged by name, never pushed)
1. **Docs:** ARCH §3, §4.8 and A16, plus horizons and TODO, in one commit, before the code, as in step 0.
2. **Code:**
   - the crate, the vocabulary move, the workspace, the lint and `CLAUDE.md`;
   - `omnis-sim` and the tests moved;
   - code-map.

## Verification
- `DEVELOPER_DIR=/Library/Developer/CommandLineTools scripts/verify.sh` prints VERIFY-GREEN with the same number of tests passing: 557, since the moved tests still run, now in the new crate.
- `scripts/lint-sim.sh` covers `omnis-bus`, and `--self-test` passes.
- `cargo tree -p omnis-bus` shows `serde` and nothing else.
- Both golden replay fingerprints are unchanged, and a mid-fight save written before the move loads after it (the existing schema-6 and reaction tests).
- `grep -rn "omnis_sim::bus\|omnis_bus" crates`: only `omnis-sim` imports `omnis_bus`.
- A mutation spot check in the crate: reverse the call order, raise the depth cap by one, drop the budget check. Each must be caught by a named test.
- Sentrux: scan `crates`, then `check_rules`.
- **What you will see:** nothing changes in play. This is a structural move plus documentation.
