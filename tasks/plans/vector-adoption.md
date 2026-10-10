# Plan: adopt `omnis-vector` as the game client

**Approved by the owner 2026-10-10.** Progress is tracked in `tasks/TODO.md` ("Vector adoption").

## Context
You played the 3D vector experiment (`origin/gui-3d-experiment`, crate `omnis-vector`) and decided to adopt it (alt-PRD X8). You have reviewed alt-PRD v0.3 and accepted it, and you chose to replace `omnis-app` **after parity**, as X14 says. The reason: today `omnis-vector` only walks, opens doors and fights. It has no character creation, saving or loading, pause screen, town services, inventory, rest or dev socket, so deleting `omnis-app` now would make the game unplayable and break window-mode MCP.

This plan covers the **adoption itself**: bring the crate onto `vector-adoption`, port it to protocol 2, make it the default client, and fold its documents into the main ones. Parity (rebuilding every screen) and retiring `omnis-app` come afterwards as a roadmap. Each screen gets its own plan in plan mode.

What I found:
- **The experiment touches nothing outside its crate** except the workspace member line, `Cargo.lock` (11 lines), `tasks/LESSONS.md` and its own docs. It depends only on `omnis-sim` and Bevy `=0.19.1`. No version conflicts; every Bevy subcrate it needs is already in our lock.
- **Porting it to protocol 2 is about 40 compile errors in roughly 30 places.** They are in `combat_menu.rs`, `arena.rs`, `rolllog.rs`, `cinema.rs`/`hud.rs`, and 4 test files:
  - Spell and item list rows become string ids.
  - `Pick::Member(slot)` becomes a `CharacterId`.
  - Renames: `StackView.index`→`stack`, `hp`→`hps`, `front`→`in_front`; `MemberView.index` is gone, so `member` is used instead.
  - `Event::Exchanged{member, with}`.
  - The command gains a `pay` field.
  - A new arm for `Mode::Town`.
  - `CombatCommand` is no longer `Copy`.
- **Three behaviour changes the compiler won't catch:**
  - A turn now ends only when the budget is spent or on `EndTurn`, so the fight menu needs End turn.
  - The test pack now starts the party in the town at (10,2) facing West, and the guild is one step north.
  - Stepping onto a site enters `Mode::Town`, which vector has no screen for.
- **Code we can reuse** from `omnis-app`, already on protocol 2:
  - `spell_menu.rs:95-131` (a cast with a string id, `row.pay()`, `Target::Member(id)`)
  - `combat_menu.rs:63-72,326,370-392` (the id list, the Exchange partner, EndTurn)
  - `use_menu.rs:140-170`
  - `defs.rs` (looking up a definition from its string id)
  - `spell_text.rs:51-58`

## Steps

### V0. Bookkeeping (docs only, one commit)
- `tasks/TODO.md`:
  - Check P2d and the parent protocol-2 item: Part 3 was played by you on 2026-10-10, "looks good".
  - Close `tasks/plans/protocol-2.md`.
- Add an "Adoption (V0–V5)" section to `tasks/TODO.md` with checkable items, and the parity roadmap below.
- Write the plan to `tasks/plans/vector-adoption.md`.

### V1. Bring the crate in (merge commit)
- Run `git merge origin/gui-3d-experiment` on `vector-adoption`. A merge keeps the experiment's 28 commits and their history. Resolve the conflicts:
  - Workspace `members`: keep both `omnis-bus` and `omnis-vector`.
  - `Cargo.lock`: regenerate it, don't hand-merge it (alt-PRD §9). Check that no new external crate appears, only the `omnis-vector` entry, and that every checksum still matches.
  - `tasks/LESSONS.md`: keep both sides' entries, in date order.
- This commit will not compile `omnis-vector`: it is still on protocol 1. The commit message says so, and V2 follows straight after. I won't run the gate on this commit alone.

### V2. Port `omnis-vector` to protocol 2
- **Mechanical renames:**
  - `StackView.stack`/`hps`/`in_front`
  - `MemberView.member`/`in_front`
  - `Event::ItemUsed{receiver}` and `Exchanged{member, with}`
  - `Mode::Town` arms in `cinema.rs` and `shell/hud.rs`
- **Slot to id:**
  - `Pick::Member(CharacterId)`, which stays `Copy`.
  - `Target::Member(id)`, `Exchange{with: id}`, `Use{receiver}`.
  - `rolllog::Names` keys members by id; `Names::slot` goes.
- **Row to string id:**
  - `Action::Cast(u8)`/`Use(u8)` stay as row indexes in the menu, so the menu stays `Copy`.
  - `command()` turns them into the string id and the `pay` from the row, reading the views, the same way `omnis-app/src/spell_menu.rs:95-131` does.
  - `rolllog` looks up spells, items, conditions and monsters by string id. The lookup is a small helper in vector modelled on `omnis-app/src/defs.rs`, not a dependency on `omnis-app`.
- **Turn budget:**
  - Add **End turn** (`CombatCommand::EndTurn`) to the top menu.
  - Spells take their `pay` from the row, so a bonus-action spell costs a bonus action.
  - The HUD status shows the budget from `CombatView.budget`.
  - Features, React and the full Use picker are fight-screen parity work (roadmap item P7).
- **Town stopgap:**
  - Inside `Mode::Town`, vector shows a notice: "Town services are not in this client yet. Use `omnis-app`." It has a **Leave** button that sends the leave command `omnis-app` already uses.
  - Otherwise the player is stuck one step north of the start.
  - The notice is replaced by the services screen at parity.
- **Tests:**
  - Move them into one `integration` binary: `autotests = false`, a `[[test]] integration` target, and `tests/main.rs` declaring the modules and `common`. `scripts/check-test-modules.sh` must pass.
  - Fix the starting assumptions: tests that expect the meadow start get a `common::place(map, x, y, facing)` helper, or walk out explicitly.
  - Fix the one-command-per-turn assumptions in `tests/common/fight.rs`, `rolllog.rs` and `combat*.rs`, so each turn ends with `EndTurn` where a test needs it.
  - The agreement test (the collision mirror against `apply(Step)` on every edge of every test map) must pass on the bigger maps with their portals, sites and new doors. If it doesn't, that's a real disagreement to fix in `collide.rs`, not a test to loosen.
- **New tests, one per behaviour change:**
  - End turn passes the turn.
  - A bonus-action Healing Word leaves the action available.
  - After a reorder, a member picked in the arena is the same member that acts.
  - Leave from the town notice returns to exploring.

### V3. Make it the default client
- `justfile`: `run` starts `omnis-vector`, and a new `run-app` keeps `omnis-app` (`cargo run -p omnis-app`).
- `scripts/verify.sh` and CI:
  - `omnis-vector` is covered by the workspace clippy and test runs.
  - Record the gate's run time with Bevy's 3D features unified across the workspace (alt-PRD §9, XR3). It was 222 s before.
- `README.md`:
  - The run commands: vector is the game; `omnis-app` stays until parity.
  - Add the crate to the crate table.

### V4. Fold the documents (your approval of this plan is the CLAUDE.md "ask before editing" for these)
- **`PRD.md`:** apply alt-PRD §2's amendment table:
  - Strike the §5 "3D rendering" non-goal.
  - Amend D2, §1/§7.2, D16, D20/§11.1, D26/§14 and R10/R13, each pointing to alt-PRD.
  - Add a decision line recording the adoption (X8, X14) and your acceptance of alt-PRD v0.3 on 2026-10-10.
- **`ARCHITECTURE.md`:**
  - §2, §3: add the `omnis-vector` row; both clients may import Bevy.
  - §8.1: Bevy's 3D features for the client.
  - §8.2/A11: `bevy_ui` in the neon style for the game; the editor's toolkit as alt-PRD §8 says.
  - §9: the dev socket's host moves to vector at parity.
  - §15: the layout.
  - Each points to `alt-ARCHITECTURE.md`.
- **Experiment docs:**
  - `alt-PRD.md` and `alt-ARCHITECTURE.md` stay at the root as the presentation documents (X9).
  - `tasks/alt-TODO.md`: its open items B7 and Bevy 0.20 go into `tasks/TODO.md`. The file stays as the experiment's history.
  - `tasks/alt-CONTINUITY.md` is deleted; there is one continuity file.
  - B7 is closed by V1–V2.
  - Its "B6" label is left as it is, in the experiment's history; BUGS.md B6 is separate.
- **Knowledge files:**
  - `tasks/knowledge/code-map.md`: add an `omnis-vector` section.
  - `verification.md`: the vector test layout and the capture command.
  - Memory `modern-look-not-pixel-art`: it says Feathers is the UI going forward, which X15 replaces with `bevy_ui` neon. It needs updating.
- **Decision for you at this step:** alt-PRD §8 says "the editor keeps `bevy_egui` (ARCH A11)". Our ARCH dropped `bevy_egui` for Feathers (D26). The editor plan is deferred, so I'll write it as "the editor's toolkit is decided when Editor v1 is planned" unless you say otherwise.

### V5. Close the adoption
- Run the gate, replays, Sentrux and the acceptance script (below). Write `tasks/acceptance/vector-adoption.md`, add a review section in TODO, and rewrite CONTINUITY. Commit.

## Afterwards: the parity roadmap (each item gets its own plan)
I recommend this order: tools first, so I can verify every later screen myself; then a real game loop; then the rest.
- **P1: dev socket in vector.** `.omnis/dev.addr`, newline JSON, protocol-2 ops, and `Screenshot`/`ScreenText` from vector's UI, so window-mode MCP works against vector. Port `omnis-app/src/socket.rs` and its tests.
- **P2: title screen, new game, pause, save and load.** This replaces the fixed party at start-up.
- **P3: character creation.**
- **P4: town services and the confirmation step.** This replaces the V2 stopgap.
- **P5: inventory, give and use, spells outside a fight.**
- **P6: camp and rest; character sheet; tactics panel.**
- **P7: fight-screen parity.** Features, React, the Use picker, the budget line.
- **P8: debug panel; settings** (reduced motion, grid visibility, quality, font and scale).
- **P9: retire `omnis-app`.**
  - Delete the crate, and rename vector's binary to `omnis`.
  - Remove the `-p omnis-app` clippy line from `verify.sh` and `ci.yml`.
  - Update the justfile and docs.
  - The acceptance docs m7a, m7b, m7c, m8 and protocol-2 cite `omnis-app` tests as coverage. Point them at vector tests, or mark them historical.
  - The error string at `omnis-mcp/src/backend.rs:110`.
  - Then the rhai Socket re-audit (your call: after the migration).
- alt-PRD §11's visual steps (occluding fills, the style system, height and cliffs, shaped structures) and Bevy 0.20 (not before 2026-11-07) run alongside or after the parity items; you set the order after V5.

## Critical files
- New in this tree (via the merge): `crates/omnis-vector/**`, `alt-PRD.md`, `alt-ARCHITECTURE.md`, `tasks/alt-TODO.md`, `tasks/plans/bevy-0.20-migration.md`.
- To edit in V2:
  - `crates/omnis-vector/src/{combat_menu,arena,rolllog,cinema}.rs`
  - `src/shell/{hud,combat,notice}.rs`
  - `tests/*` → `tests/main.rs`, `tests/common/{mod,app,fight}.rs`
  - `crates/omnis-vector/Cargo.toml`
- To edit in V0, V3 and V4: `tasks/TODO.md`, `tasks/plans/protocol-2.md`, `justfile`, `scripts/verify.sh` (only if needed), `.github/workflows/ci.yml` (only if needed), `README.md`, `PRD.md`, `ARCHITECTURE.md`, `tasks/knowledge/{code-map,verification}.md`.
- Untouched: every simulation crate, `omnis-app`, `omnis-mcp`, `omnis-cli`.

## Verification
- **Gate:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools scripts/verify.sh` is green (VERIFY-GREEN). The test count is 583 + vector's tests, with failed 0, and I record the run time.
- **Replays unchanged:** walk `8711507745385976768`, fight `5247080599556612730`; `SAVE_SCHEMA` 7, `PROTOCOL` 2. No simulation file changes, which I check with `git diff --stat main -- crates/omnis-{core,bus,expr,data,rules,gen,eco,story,sim}` showing nothing.
- **Tests:** `check-test-modules.sh` and `lint-sim.sh` pass. The agreement test passes on every map of the current test pack.
- **Sentrux:** run scan then check_rules; rules pass. Note the quality score against 9003.
- **Capture:**
  - `cargo run -p omnis-vector -- --screenshot <scratchpad>/a.png --walk 60` renders the town.
  - A second capture placed in the dungeon renders.
  - I look at both images.
- **Acceptance (you, in the window), `tasks/acceptance/vector-adoption.md`:**
  - `just run` opens vector in the town.
  - Walk north onto the guild: the notice appears and Leave works.
  - Walk out to the meadow or dungeon and start a fight.
  - Cast Healing Word as a bonus action (or another bonus-action spell the fixed party knows), then attack in the same turn, then End turn.
  - Swap with another member.
  - The roll log names spells and members correctly.
  - `just run-app` still plays the old client unchanged.
