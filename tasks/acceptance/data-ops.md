# Acceptance: the `data.*` ops (W1; `docs/api.md` §7.12)

Built 2026-11-11, planned in plan mode on the owner's direction: "we need forward thinking flexibility above
all else, TTRPGs are filled with creative solutions and we want to capture some of that magic." Owner's choices
(asked before the plan): the definition as its pack wrote it inside a stable envelope; the author's tier, a
per-party gate left for D30; generic filters in the first version; the supplying pack recorded now.

## What was built
- **Step 1, the catalogue** (`4c5bf51b`): `omnis-data/src/catalog.rs`, one table of the 14 kinds the loader
  gathers (every kind but the rules files); `Definition`, tagged `kind` beside `def`; `links` and `referrers`,
  both from one walk over a definition's strings (`strings.rs`: a serializer that keeps the strings), so a field or a kind
  added later needs no reference code; `Data.sources`, the packs that wrote each id.
- **Step 2, the ops**: `data.kinds`, `data.list`, `data.get`, `data.text` in `omnis-sim/src/data_ops.rs`; the
  ops to 48, the MCP tools to 48, `Reply` to 31, `OpError` to 12 (`UnknownId`); `docs/api.md` §5, §6, §7.12,
  §10, §11, §14 and the changelog; ARCHITECTURE §4.9 and §9.3 (the owner's direction is the ask).
- **Step 3, no game needed**: the game's socket answers `data.*` at the title and in the editor, from the game's
  packs when one runs, else the packs loaded at start. `omnis-cli op <name> [<args>]` sends any one op from a
  shell (the plan's hand test named the CLI, which had no such command).
- Found on the way: the gate passed with no test run when every test binary failed to link (`fb69f172`,
  `scripts/test-summary.sh`; `tasks/LESSONS.md` 2026-11-11).

## Departures from the plan as approved
- The strings of a definition were first read from its compact RON. Since 2026-11-11 (the owner's choice)
  they are collected by a serializer of its own, as planned, with the two float aliases on `lint-sim: allow`
  lines as `omnis-sim/src/query/path.rs` already does. Strings now come unescaped.
- The paging field is `cursor` in the request and the reply (not `after` and `next`).
- `data.get` on an unknown language gives null words rather than an error.
- Not built: a socket test of `data.*` after a `pack.reload` onto changed packs (the routing reads the game's
  data; the test covers the title and a running game).

## Steps (a shell in the repository; nothing needs a running game)
Each "you will see" below is asserted by `omnis-cli/tests/cli.rs::op_answers_one_op_as_ron_and_says_why_not`,
`omnis-sim/tests/data_ops.rs` or `omnis-mcp/tests/replies.rs`.

1. `cargo run -q -p omnis-cli -- op data.kinds`
   You will see `reply: "kinds"`, fifteen kinds from `background` to `tileset` (fourteen until rules files became one, 2026-11-11) with their counts for
   `packs/base` and `packs/the_lower_kingdoms`, and `languages: ["en"]`.
2. `cargo run -q -p omnis-cli -- op data.list '(kind: "monster")'`
   You will see one row per monster: `id`, `kind`, `name` (the text key), `words` (the key with its text, as
   `"...goblin.name": Some("Goblin")`), `pack` and `replaces`, then `cursor: None`.
3. `cargo run -q -p omnis-cli -- op data.get '(id: "base:monster:goblin")'`
   You will see `"kind": monster`, `"def"` with the goblin's file as written (`ac: 15`, its attacks), `"words"`
   with the text of its name and its attacks' names, `"refers"` (empty for the goblin), `"pack": "base"`.
4. `cargo run -q -p omnis-cli -- op data.list '(refers_to: "base:item:dagger")'`
   You will see everything that names the dagger: the rogue and the wizard (classes), the smith (a service), and
   whatever in your world names it. Add `kind: "class"` inside the parentheses and only the classes remain.
5. `cargo run -q -p omnis-cli -- op data.list '(matching: "smithy")'`
   You will see `base:service:smith`: found by its name's text, the id says only `smith`.
6. `cargo run -q -p omnis-cli -- op data.text '(keys: ["base:text:item.mace.name", "nonsense"])'`
   You will see `words` with `Some("Mace")` and `unknown: ["nonsense"]`.
7. `cargo run -q -p omnis-cli -- op data.get '(id: "base:item:nope")'`
   You will see `omnis-cli: no loaded pack defines 'base:item:nope'` and a failed exit.
8. One of your own: `op data.list '(prefix: "the_lower_kingdoms:")'`, then `op data.get` on a map of yours
   (a map's `def` is the whole map; pipe it through `head -60`).
9. Over MCP, optional: rebuild the bridge (`cargo build -p omnis-mcp`) and restart the MCP client; the tools
   `data_kinds`, `data_list`, `data_get` and `data_text` appear and answer the same, with the game at its title.

## What it does not do yet (`tasks/knowledge/horizons.md`)
- No "what this party knows of it": everything loaded is answered (the gate comes with D30).
- A text key reports no supplying pack; rules files are not a kind (`rules.list`); `{placeholders}` are left as
  written; `refers_to` reads every definition once per call.
- Rule slots still override by rules-file id order, not by pack load order (found in the survey, not changed).
