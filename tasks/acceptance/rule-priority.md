# Acceptance: rule priority and conflict tracking (W1; `docs/api.md` §7.13, ARCHITECTURE §5.5)

Built 2026-11-11 in plan mode on the owner's direction: "We do need to start tracking rule conflicts and consider
all rules have a required priority value that is used to determine rule precedence." Owner's choices (asked
before the plan): a required `priority` on every slot, value and table, none on a file; two definitions of one
name at one priority are always a load error; an override declares the inputs of the rule it replaces; shown in
`rules.list`/`rules.get`, `omnis-cli validate`, `data.*` and an editor rules view.

## What was built
- **Step 1, the format and the merge** (`90dcd81c`): every entry of a rules file states `priority`; a value is
  `(priority, value)`, a table `(priority, rows)`. Of the definitions of one name the highest stands, whatever
  the file's id or the pack's place in the load order (`omnis-data/src/rules.rs` `merge`). Refused: a tie, an
  override with other inputs, a formula that does not compile, standing or not. `Data.standings` keeps what
  stands and what it beat. `base` writes 0 throughout and is resealed at 0.1.1.
- **Step 2, the API and the CLI** (`f315a8b5`): `origins` on `rules.list`, `origin` on `rules.get` and
  `rules.set`; rules files the fifteenth kind of `data.*`; `override:` lines from `omnis-cli validate`.
- **Step 3, the editor**: a Rules... button under the file buttons opens a list of every rule with its file and
  priority, a rule that beat another marked OVERRIDES with what it beat beneath, a search, and the file opened
  as text when the pack being edited wrote it.

## Departures from the plan as approved
- **The golden replays were rebaselined.** The plan said they would stay at their pins. They record base's
  version and hash, and the world's fingerprint covers the packs it was made on, so resealing base moved both.
  The compiled rule set is byte for byte the one before (`rules.list` compared across the two commits). New
  pins: walk `d48a913cec6c5735`, fight `9f52f48c5364e17f`. A save made on base 0.1.0 meets base 0.1.1 as any
  save meets a changed pack.
- **An entry without a priority is refused by the parser**, which names the file, the line and the field
  `priority`, not the entry's name.
- **The mark in the editor is the word OVERRIDES, not an icon**: the kit's text rows carry no icon.
- **A rule's file opens from the list only when it is named as its id ends** (`pack:rules:house` in
  `data/rules/house.ron`), which every rules file in the tree is. Otherwise the editor says so.
- **The wire field for what a rule is, is `kind`** (`slot`, `value`, `table`), not `rule`: `rule` already
  carries a slot object in the `rule` reply.
- **The list's texts were not measured headless** against the modal's width (the lesson of 2026-10-31 asks
  it of a column). The longest is a file id with its priority. Look for a clipped row in step 8.

## Steps
Each "you will see" is asserted by `omnis-data/tests/rule_priority.rs`, `omnis-cli/tests/cli.rs`
(`an_override_is_listed_by_validate...`), `omnis-sim/tests/rules_ops.rs` or `omnis-vector/tests/editor_rules.rs`.

A house pack to try with, written once (`.omnis/` is not tracked):

```sh
mkdir -p .omnis/house/data/rules
cat > .omnis/house/pack.ron <<'RON'
(schema: 1, id: "house", version: "0.1.0", name: "House rules", license: "MIT",
 attribution: [], depends: ["base"], entry: None)
RON
cat > .omnis/house/data/rules/table.ron <<'RON'
(schema: 1, id: "house:rules:table",
 slots: {"attack.total": (priority: 200, inputs: ["die", "bonus", "proficiency"], expr: "die + bonus")},
 values: {"death_save_dc": (priority: 200, value: 12)})
RON
cargo run -q -p omnis-cli -- pack seal .omnis/house
```

1. `cargo run -q -p omnis-cli -- validate packs/base packs/the_lower_kingdoms .omnis/house`
   You will see the `ok:` line with 3 packs, then two lines: `override: slot attack.total: house:rules:table
   (pack house, priority 200) over base:rules:combat (pack base, priority 0)` and the same for `value
   death_save_dc`.
2. `cargo run -q -p omnis-cli -- validate packs/base packs/the_lower_kingdoms`
   You will see the `ok:` line alone: nothing overrides anything.
3. `cargo run -q -p omnis-cli -- op rules.get '(slot: "attack.total")' --pack packs/base --pack packs/the_lower_kingdoms --pack .omnis/house`
   You will see `source: "die + bonus"` and an `origin` with `file: "house:rules:table"`, `pack: "house"`,
   `priority: 200` and `overrides` holding `base:rules:combat` at priority 0.
4. The same with `op rules.list`: `origins` holds one row per slot, value and table, slots first.
5. The same with `op data.list '(kind: "rules")'`: base's ten rules files and `house:rules:table`; and
   `op data.get '(id: "house:rules:table")'` gives the file as written with `"pack": "house"`.
6. Edit `.omnis/house/data/rules/table.ron`: set the value's priority to 0, reseal (`pack seal`), validate as in
   step 1. You will see it refused: `value 'death_save_dc': priority 0 in both 'base:rules:combat' and
   'house:rules:table'; two definitions of one rule need different priorities`. Put 200 back.
7. Change the slot's inputs to `["die", "bonus"]`, reseal, validate. You will see it refused: `slot
   'attack.total': inputs [die, bonus] are not the [die, bonus, proficiency] of 'base:rules:combat'`. Put
   them back and reseal.
8. In the game: `cargo run -p omnis-vector -- --windowed --pack packs/base --pack packs/the_lower_kingdoms
   --pack .omnis/house --editor`. Press Rules... under the file buttons. You will see `69 rules, 2 overriding
   another`, each rule with its file and priority on a button, and `OVERRIDES  slot attack.total` with `over
   base:rules:combat  priority 0` beneath. Type `overrides` in the search: the two remain. The button of
   `house:rules:table` opens the file as text and closes the list; a base rule's button is dim (base is not
   the pack being edited).

## Not built
- Pack load order as a topological sort with cycle errors (ARCHITECTURE §6.1 says so; the code takes the order
  given and checks only that `depends` came earlier). Reported to the owner, its own item if wanted.
- A mark on a slot whose formula a `rules.set` swapped: its `origin` stays as loaded.
