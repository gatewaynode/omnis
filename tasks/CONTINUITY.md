# Continuity notes

Written 2026-10-11, after P7 was built, before the owner's acceptance run. Rewrite this file every time it
is used. Durable knowledge lives in `tasks/knowledge/`.

## Where we are
- **Branch `vector-adoption`**, not pushed (the owner pushes).
- **Accepted by the owner:** P2, P3, the party column, P4, P5, P6 (all 2026-10-11; P6: "P6 accepted, move on
  to planning P7").
- **P7 built:** plan `tasks/plans/p7.md` (with "As built"), commits `5ed2b36` (plan), `ab06bed` (P7a engine),
  `340f8c3` (P7b, P7c), then the docs commit (P7d).
  - Owner answers: Features its own entry; React a choice "Reactions < on >" under the budget line; the
    budget line at the top of the action column; the pickers a panel over the picture.
  - Engine: `SpellView.pay`; `CombatView.items` (`FightItemView` with the refusal `Use` would get).
  - Client: `ScreenId::Fight` (`screen/fight.rs`), drawn and routed by `shell/combat.rs` (`FightPress`,
    `Mark::Fight`), never on the Nav stack; `Forms.fight` with `settle` (the red-first fix: a turn passed
    elsewhere took the menu back to the top); `arena::picker`.
- **Next:** the owner runs `tasks/acceptance/p7.md`. On acceptance, tick P7 in TODO with their words (as P6's
  line). Then P8 (debug panel; settings), planned in plan mode with layout questions first.

## Facts
- **Fight words:** "Round R: X's turn"; "Action 1   Bonus 1   Reaction 1 (on)"; pickers add "SP s/max" for a
  caster; spell rows "Bless  1 pt", notes in order "reaction" (dim, "declare it in tactics"), the refusal, "in
  force", "bonus action"; feature rows the name with "1 use, bonus action" after; "Cunning Action: swap"/"hide";
  "Spyglass: not from a fight"; "Monsters ahead: hostile"; "Victory! N XP each, G gold".
- Healing Word's definition costs an action (`bonus_action_available`); Cunning Action is a rogue's from level 2.
  Durin's turn: Bless after Healing Word leaves his bonus action, so his turn goes on until End turn.
- The fixed party carries a potion each. Town sites: inn (1,1), temple (4,1), trainer (7,1), guild (10,1);
  smith (1,3), tavern (4,3), bank (7,3). Start (10,2) West, 60 gp, 40 food.

## Open, carried over
- Unconfirmed calls from P6: down members red; dead members dim with "dead" in red.
- rhai Socket re-audit after P9; `smartstring` RUSTSEC-2026-0249 to add to ARCH §13; `ttf-parser` 0.25.1
  RUSTSEC-2026-0192. Bevy 0.20 not before 2026-11-07. Editor's toolkit open until Editor v1 is planned.
- Backlog: removing or editing a party member. Unscheduled: `DevCommand::Pass`; the `data.*` ops;
  `scripts/mcp-probe.py`. Small: pack text not checked for non-ASCII; the health colours not checked for
  colour deficiency; the picture window draws every monster as the rat (art, the style step).
- The owner's asset work (`.gitignore`, `assets/`, two untracked `cutout-basic-*.svg`) is theirs. Never stage it.

## Pins
- Gate `tests passed 798 failed 0 ignored 13`, VERIFY-GREEN at `340f8c3`. Vector 171 integration + 32 unit.
- Walk replay `78e57e238dcb93c0`, fight `48d160014c6a627a`; `SAVE_SCHEMA` 7, `PROTOCOL` 2, 25 ops, MCP 25 tools.
- Sentrux: scan `crates/`: rules pass, quality 8992.

## Process
- **Cargo:** `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, `cargo fmt --all` first. Vector clippy, each
  written out: `--lib`; `--no-default-features --lib`; `--features test-fixtures --tests`.
- **Tests:** `--test integration` with `--features test-fixtures` (`tests/main.rs` lists the files). Fight app
  helpers in `tests/common/fight.rs`: `entries`, `step`, `enabled`, `press` (by `Act`). Model: `turn_of(name)`,
  then `fight::content` over `common::ctx`.
- **Gate:** `scripts/verify.sh > <scratchpad>/gate-<step>.txt 2>&1` in the background, a new file each run, no
  edits while it runs.
- **Captures:** temporary `PROBE-TEMP` hooks (back the files up to the scratchpad first, restore after, grep
  `PROBE`): for a fight, `open_start` builds an encounter with `omnis_sim::combat::start` and dodges to the
  member; `shell/combat.rs::enter` sets the step after `fight::settle`. Build with `--features test-fixtures`;
  `./target/debug/omnis-vector --seed 1 --screenshot .omnis/x.png --size WxH`.
- **Mutations:** `python3 -u` to a file, never under `timeout`; check the sources are restored.
- **Unfinished work** before a compact is committed as `WIP`, never stashed. Stage by name, never `.claude/`.
- `cargo fmt` reflows lines: a Python `replace` written before formatting may silently miss; assert counts.

## Carry-over
- Large files: `plan.rs` 980, `bake.rs` 934, `screen.rs` (omnis-app) 902, `omnis-vector/tests/screens.rs`
  about 840, `save_and_replay.rs` 810, `shell/screen.rs` (vector) 754, `combat_menu.rs` 719, `tests/service.rs`
  about 700, `screen/mod.rs` (vector) 689, `screen/tactics.rs` 588.
