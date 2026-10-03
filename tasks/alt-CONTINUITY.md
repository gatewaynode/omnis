# Alt continuity notes (the 3D experiment)

Written 2026-10-03, before a compact in the middle of Phase B. Rewrite this file every time it
is used; keep it to state, next step, pointers and gotchas. It is kept apart from
`tasks/CONTINUITY.md`, which belongs to the main build (stale; it describes `m6-closeout-tasks`).

## State
- **Branch:** `gui-3d-experiment`, cut from `main` at `8e111d5`.
  - **The owner pushes:** the agent's sandbox has no GitHub access. See memory `owner-pushes`.
  - The owner pushed through `5083a2c`. Commits after that are local until the owner pushes again. Check with `git status -sb`.
- **Clone:** `/Users/john/code/omnis-alt/omnis`. The main checkout is `/Users/john/code/omnis`.
- **Phase A is complete** (A0–A8). The A8 report is in `tasks/alt-TODO.md`. The gate is green at 407 passed and 6 ignored, about 11 s warm.
- **Phase B, the 2D combat screen:** the plan was approved on 2026-10-03.
  - The plan: `~/.claude/plans/snug-munching-gray.md`. **Read it first.**
  - B0 is checked, and B1–B7 are listed in `tasks/alt-TODO.md`. They were committed with this file.
  - **B1 has not started in code.** Research is done (see Next).
- **The owner's Phase B decisions:**
  - alt-PRD §10.3: **write fresh**; no `omnis-app` reuse.
  - **Glowing vector-line figures** for the monster stacks and party members, animatable later.
  - **Every fight action**, with targets picked by clicking: Fight, Bribe, Hide, Run; then Attack, Cast (on a stack or a member), Use an item, Dodge, Swap, Flee.
  - **A short roll log** written fresh in omnis-vector.
- **Standing owner rules:**
  - This branch is UI only. Mechanics changes go to the mechanics branch.
  - **Rebase onto the mechanics work before merging** (B7).
  - Buttons before keys.
  - One commit per item, the gate and Sentrux before each.
- **Earlier owner decisions** (alt-PRD X1–X7 and Phase A) are in `alt-PRD.md` and in the TODO's reviews.
  - Keys: extended WASD (W/S, A/D, Q/E), Space or a right click is the action key (with Shift: the default action), and a left click toggles mouse look.
- **Owner displays:**
  - They work on the ultrawide, a Samsung LS49AG95: 5120×1440 at 144 Hz.
  - macOS's main display is an LS27A800U: 60 Hz, 4K scaled to 1920×1080.
  - Measured: 280–310 fps with vsync off on the ultrawide.
- **A message in the first session looked like a password.** It was not used or stored anywhere; do not repeat it.

## Next: B1 (the roll log), then B2–B6 in the plan's order
- **`src/rolllog.rs`** (Bevy-free): `describe(&Event, &Names, &Data) -> Option<String>`.
  - Fight events and their fields are in `crates/omnis-sim/src/event.rs`, lines 229–495:
    - `EncounterStarted{stacks, disposition, noticed}`, `Check{actor, kind: CheckKind::{Stealth,Hide,Run,Flee,Save}, dc, success}`, `Bribed{cost}`
    - `CombatStarted{surprised: Surprise::{None,Party,Monsters}}`, `Initiative{order}`, `RoundStarted{round}`, `Turn{actor}`, `Waited`, `Dodging`, `Exchanged{a, b}` (slots)
    - `AttackResolved{attacker, target, roll, ac, hit, crit}`, `Damage{target, amount, kind, adjust}`
    - `SpellCast{caster: CharacterId, spell: SpellId, points}`, `EffectApplied` / `EffectEnded{target: EffectTarget::{Member,Party}, spell}`, `Concentration`
    - `ItemUsed{member, item, target}`, `Healed{target, amount, hp}`, `Down{target}`, `Wounded`, `DeathSave{result}`
    - `Condition{target: ActorRef, condition: ConditionId, applied}`, `Death{target}`, `CombatEnded{outcome, xp, gold, fallen}`
  - Names: `data.label("en", &data.spells[&id].name)`; likewise `data.conditions[..].name`, `data.items[..].name` and `data.monsters[..].name`.
  - `ActorRef::{Member(CharacterId), Stack(u8), Monster{stack, index}}`. `index` counts among the living at that moment.
- **The `Names` timing trap:**
  - The final command of a fight sets `world.mode = Explore`, and `bury` removes members who died for good. After that command the stacks and the dead are gone from the world.
  - So `Session` keeps a `Names`, rebuilt at the start of `note()` only while the world is in Encounter or Combat. It is built from the stacks (monster labels) and the party (id → name, slot → name). Describe with it, so the ending events still name everyone.
  - `EncounterStarted` arrives while the world is already in Encounter, so the rebuild picks up the new stacks.
- **The feed:** `Session::note` appends to a `fight_log: Vec<String>` (keep the last 40), cleared on `EncounterStarted`. `shell/text.rs::describe` keeps the exploration HUD lines.
- **Tests:** drive the placed group fight (as `tests/fight.rs` does: dungeon (3, 6) facing South, walk into (3, 8); seed 1 fights, seed 2 runs), then assert the log has lines for initiative, attacks or damage, and the ending.
- **Then B2–B6:**
  - `combat_menu.rs`: pending Cast, Use or Swap, targets, cancel. Everything is tried on a clone via `shell/notice.rs::{choice, refusal}`; move those into the core if B2 needs them outside the shell. Usable items are `equipment` rows with `use_effect`, checked by trial. Spells come from `combat_view().spells`, with `targets_members` and `blocked`. Members from `omnis_sim::ops::party_view` (`MemberView`: hp, front, down, dead, conditions).
  - `arena.rs`: layout in logical pixels, `segments`, `pick`.
  - `shell/combat.rs`: the screen, per the plan's design §4–5.
    - A `ViewState` state needs `StatesPlugin` in headless tests (MinimalPlugins lacks it).
    - Swap the camera's `IsDefaultUiCamera`.
    - Use `CombatGizmos` on `RenderLayers` 1.
    - Copy the offscreen `RenderTarget` to the 2D camera.
    - Hide the movement pad and the minimap while in Fight.
    - Call `follow()` on exit.
  - Retire the A7b notice.

## Gotchas
- **Builds need `export DEVELOPER_DIR=/Library/Developer/CommandLineTools`.** The Xcode licence is unaccepted.
- **The gate:** run `scripts/verify.sh > log 2>&1 && git commit …` unpiped. Then Sentrux `scan` and `check_rules`; check `use super` cycles in `src/shell` by hand.
- **Windows get no frames from the agent's shell.** Use the offscreen capture: `./target/debug/omnis-vector --screenshot .omnis/shot.png --walk N [--size WxH]`, then Read the PNG.
- **To capture a panel or a fight,** add a temporary placement in `Session::start`. Edit a backup copy, restore it, and never commit the placement. The probe's minimap is blank, since the placement skips the look.
- **`~/Desktop` is blocked for the agent.** The owner copies files into `.omnis/`.
- **Headless input:** keys and mouse buttons go in as `KeyboardInput` and `MouseButtonInput` messages. Buttons get their `Interaction` set directly (`tests/common/app.rs::set`). The app builders are `tests/common/app.rs::{app, app_logging_to, app_with(log, seed)}`.
- **The binder logs only `Ok` commands.** Choices are tried on a clone first.
- **`Interact` with nothing there returns `Ok`** with `NothingHere`.
- **A test that places the party directly cannot replay its log.**
- **Python edit scripts:** assert that each anchor exists. `cargo fmt` rewraps lines, so read the current text before anchoring on it.
- **Commit trailer:** `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Pointers
- Vision: `alt-PRD.md` (Phase B in §6, X7, §10.3) and `alt-ARCHITECTURE.md` (§6, §8, §9).
- Plan and reviews: `tasks/alt-TODO.md`. The Phase B plan file is listed above.
- Core: `crates/omnis-vector/src/{geom,grid,pose,collide,bind,geometry,minimap,party}.rs`.
- Shell:
  - `src/shell/{mod,session,movement,controls,actions,notice,fight,panel,render,hud,minimap,text,capture}.rs`
  - `fight.rs` holds the A7b notice that Phase B replaces.
  - `panel.rs::current` shows it.
- Tests: `tests/{agreement,binding,shell,fight,actions,minimap}.rs` and `tests/common/{mod,app}.rs`.
- Simulation reads:
  - `omnis_sim::{combat_view, CombatView, StackView, SpellView, bribe_cost, CombatCommand, Target, EncounterChoice}`
  - `omnis_sim::ops::party_view`
  - `omnis_sim::combat::state::can_fight`
- Rules: `tasks/LESSONS.md` (entries 2026-10-02 and 2026-10-03), `tasks/knowledge/agreements.md`.
