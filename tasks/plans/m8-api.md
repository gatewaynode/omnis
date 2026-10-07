# M8 step 8: the engine's API, the docs as built, and acceptance

## Context
Step 8 was meant to be docs and review. The owner, 2026-10-07: "as we wrap up the docs and review we are creating the API that any number of clients might be interacting with. The sim is basically an SRD variant engine that other tracks of development will have to interface with while the sim itself is under development."

The inventory of the client surface found five gaps.

**No version.**
- The ops carry no version, and neither do `game.status` or the socket.
- `Reply` is untagged. A client cannot decode a reply unless it knows which op it sent, and the variant order in `Reply` is load-bearing.

**Unanswered views.** `RestView` has no op. The explore-time spell rows are built in the app by rebuilding a `Pcg32` from `world.seed`.

**Duplicated host code.** The app socket and `Headless` both implement the host ops. The save rule, the reload tile check and `rules.set` are written twice.

**The app reaches into `World`.**
- About 60 non-test reads of its fields, plus calls to sim internals: `weapon_for`, `bribe_cost`, `cast::check`, `omnis_rules::*`.
- So the World's field layout is in effect part of the API.

**The docs have drifted.**
- ARCH §4.2:
  - lists `Journal`, `Region`, `Quest`, `Saved`, `Loaded` and `Message{key,args}`, which do not exist;
  - says the log is "unbounded in dev", but it is always 4096.
- §4.3 names `query::party`, `region` and `journal`, which do not exist, and omits the five view functions.
- §9.1 promises sliced long ops, which are not built.
- §9.3 lists `pack.validate`, `eco.*`, `story.*` and `editor.*` as if they existed, and mixes dotted op names with underscored tool names.
- §10 lists CLI subcommands that do not exist.
- The README says "twenty tools".
- `verification.md` says schema 6, 20 tools and 12 dev branches.

**Owner decisions (2026-10-07):**
- Fix the gaps now, before acceptance.
- The contract lives in ARCH (its shape and rules) and in a new `docs/api.md` (the reference other tracks read).
- There are two named tiers:
  - the **Rust library** for in-process Rust clients;
  - the **JSON op protocol** for everything else.
- `World`'s fields are not part of the API; clients read through views.

**What stays fixed through the whole step:**
- No save, world or replay change: `SAVE_SCHEMA` 7, walk `8711507745385976768`, fight `5247080599556612730`.
- Each sub-step is checked against these, and the build is a no-op in play apart from the new ops.

## The contract (ARCH §4.9 "The engine's API", new; approving this plan approves the text)

**Tier 1, the Rust library (`omnis_sim::api`):**
- A new façade module re-exports exactly the contract:
  - `World::new`, `apply`, `World::{to_ron, from_ron, fingerprint}`, `Replay`;
  - `Command` and its sub-commands, `Event`, `Rejection`;
  - `ModeKind`, `Settings`;
  - the views (below), `Data` and `load_packs`.
- Anything outside `api` is internal and may change without notice.
- **The views are the only way a client reads state:**
  - `query::here` (new), `party_view`, `combat_view`, `service_view`, `rest_view`, `time_view`, `cast_view` (new);
  - `query::{viewport, automap, map_text}`.
- `World`'s fields are public only for the sim's own tests and tools. Clients must not read them.

**Tier 2, the JSON op protocol (`ops::{Op, Reply, OpError}`):**
- Transports: the socket, `Headless`, and MCP (tool = op name with `_` for `.`).
- Every Tier 1 view has an op.
- A client calls `game.status` first and checks `protocol`.

**Stability rules (both tiers):**
- Additions do not bump the version: a new op, a new optional field (`serde(default)`), a new event or rejection variant. Clients must ignore unknown fields and variants they do not handle.
- A rename, a removal, or a change of meaning or units bumps `ops::PROTOCOL`. It is listed in the changelog in `docs/api.md` with the migration.
- Saves have their own version (`SAVE_SCHEMA`) and packs theirs (`omnis_data::SCHEMA`). The three are independent.
- `world.query` (path reads into the World's serialized shape) is debug-only and outside the contract.
- Wire forms:
  - `Op`: `{"op","args"}`;
  - `Reply`: internally tagged `{"reply": "<name>", ...}`;
  - `OpError`: `{"kind", ...}`;
  - `Event`, `Command` and `Rejection`: serde's external tagging `{"Variant": {...}}`.
- Limits: `MAX_SCRIPT` 10,000, `MAX_LINE` 1 MiB, strings bounded, paths relative under the working directory.

**A17, a new decision row.**
- Decision: two tiers, views only, one protocol version and a changelog.
- Rejected:
  - the World's fields as the API;
  - the op protocol as the only tier;
  - generated API docs with no drift test.

## Sub-steps (one commit each unless split; gate green, Sentrux, staged by name, never pushed)

**8a. Docs as built (ARCH and the knowledge files).**
- ARCH:
  - §4.1 and §4.2 rewritten to the code: 11 commands, 62 events grouped by milestone, the log always 4096;
  - §4.3: the views;
  - §4.4: the inputs `lived`, `shared_time` and `stability`;
  - §9.1: the sliced long ops marked not built;
  - §9.3: dotted op names with the MCP spelling stated, and unbuilt rows marked "later";
  - §10: the real subcommands (validate, schema dump, tileset bake, map text, play, replay);
  - the new §4.9 and A17 (above).
- README: the tool count and list.
- `verification.md`: schema 7, the pins, 22 tools, the dev branches at 13.
- `code-map.md`: the M8 files.
- `horizons.md`: the ecosystem catch-up waits for M10.
- The status line stays "v0.8 in progress".

**8b. Sim: the views the contract needs.** All additive, with `serde(default)`, integers, `BTreeMap`/`Vec`.
1. `query::here(world, data) -> Here`:
   - `mode`, `position`, `service`, `date`, `age`, `turn`;
   - `may_save`, `seed`, `settings`;
   - plus `query::flags`.
   - `Status` is built from `Here`, plus packs, the fingerprint and the groups cleared, so it gains `may_save`, `seed` and `settings`. `Status` serializes the world for its fingerprint, so `Here` is the cheap per-frame read.
2. `MemberView` gains:
   - background, alignment, age in years, proficiency, saves, skills, casting ability;
   - `effects: Vec<EffectView { spell, caster, minutes_left }>` (also on `PartyView`).
   All of these come from the same `omnis_rules` calls the sheet makes today.
3. `StackView.refusal: Option<Rejection>` (keeping `reachable`), `CombatView.bribe: Option<u32>`, `FeatureView.cunning`, `OfferView.subject` (the item or spell id).
4. `cast_view(world, data) -> Vec<CastView>`: the spells castable outside a fight, with cost and refusal, quoted on a copy of the `cast` stream (the `service_view` pattern).
5. Tests:
   - each new field against the rule it mirrors;
   - `cast_view` and `here` leave the fingerprint unchanged.
- Likely two commits (here and member fields, then combat, offers and `cast_view`).

**8c. Sim and protocol: version, tags, new ops, one host code path.**
1. `pub const PROTOCOL: u32 = 1` in `ops.rs`:
   - `Status.protocol`;
   - the schema dump's header;
   - the MCP server info's instructions line.
2. `Reply` becomes `#[serde(tag = "reply", rename_all = "snake_case")]`:
   - the existing fields stay where they are, so the bridge sees the change only as an added key;
   - the "Value and Done stay last" hazard goes away.
3. New ops: `rest.get` (`Reply::Rest`) and `cast.get` (`Reply::Casts`). This brings MCP to 24 tools:
   - hand-written schemas;
   - the count asserted in `tools.rs`, `bridge.rs` and `headless.rs`;
   - the schema dump.
4. Host ops: I/O-free helpers in `ops.rs`:
   - `save_text(world)`: the save rule, then `to_ron`;
   - `load_text(text, data, force)`;
   - `check_reload(world, &fresh)`;
   - `rules_set(data, slot, source)`.
   `Headless::handle` and the app's `socket::handle` keep only the file and window I/O.
5. `omnis_sim::api`, the façade module.
6. The schema dump gains a "protocol replies" section, one example per `Reply` variant, parsed back in `headless.rs`'s test.
- Tests:
  - a reply round trip per variant, now unambiguous;
  - `game.status` carries `protocol: 1`;
  - the bridge tests updated for the new key and the new tools.

**8d. App: read through views.**
- The `Views` resource:
  - it lives in `sim.rs` and holds `here`, `party`, `combat`, `service`, `rest`, `casts`, and `flags` when devtools is on;
  - `refresh_views` updates it at the end of `SimSet::Apply`, only when `SimWorld` or `PackData` changed or was added.
- `PlayState::for_kind(ModeKind)`.
- The files then move in four commits, each green:
  1. mode readers;
  2. position, the HUD, the automap, the viewport, the socket, the tool bar, the look, the roster;
  3. `sheet_menu`, the `ui` member rows, `inventory_menu`;
  4. `combat_menu`, `use_menu`, `spell_menu`, `text.rs`, the service, confirm, camp, debug and pause panels.
- The app stops calling `weapon_for`, `bribe_cost`, `cast::check` and `omnis_rules::*`.
- Labels stay in the app, resolved through `Data`.
- Watch `plan.rs` (953 lines): its automap change must be net-neutral or move code out.
- Per frame this does less work than today, which rebuilds `combat_view` every frame.

**8e. Enforcement.**
- `SimWorld`'s field becomes private to `sim.rs`. Its methods mirror the contract: `new`, `apply`, `dispatch`, `save`, `replace`, `status`, `automap`.
- Test fixtures use `fixture()`/`fixture_mut()` behind `cfg(any(test, feature = "test-fixtures"))`, enabled by a self dev-dependency. The gate's existing `clippy -p omnis-app --no-default-features --lib` proves non-test code cannot reach them.
- Fallback if the feature trick misbehaves: `#[doc(hidden)]` accessors plus a check in `scripts/` that greps for them outside tests. I'll tell you if that fallback is used.

**8f. `docs/api.md`, the drift test, ARCH v0.8, acceptance.**
- `docs/api.md` contents:
  - the two tiers, with a quick start for each (Rust: load, new, apply, a view; JSON: `game.status`, `sim.command`, a reply);
  - every op with its args and reply;
  - every view;
  - the commands, events, rejections and `OpError` kinds;
  - the limits, the versions and the changelog (protocol 1, today's baseline).
- `omnis-cli/tests/api_doc.rs` reads `docs/api.md` and fails when it lacks:
  - any op name;
  - any `Reply` tag;
  - any `OpError` kind;
  - any `Event`, `Command` or `Rejection` variant.
  Names come from exhaustive `match` helpers, so a new variant breaks the build until it is documented, as the schema proof does.
- ARCH status line to v0.8 as built.
- `tasks/acceptance/m8.md` with "what you will see":
  - the HUD clock line;
  - years in the meadow while the town's date snaps forward on entry;
  - age never reversing;
  - tavern rumors with their age;
  - `time_clocks` over MCP;
  - a save and load mid-fight that keeps its reactions;
  - **API checks:** `game_status` shows `protocol: 1`, `rest_get` and `cast_get` answer in the meadow, and a reply carries its `reply` tag;
  - the app plays exactly as before: the sheet, the spells outside a fight, the fight's greyed stacks, bribe prices, the service rows.
- TODO, continuity and code-map; then **your acceptance closes M8**.

## Verification (each sub-step)
- `DEVELOPER_DIR=/Library/Developer/CommandLineTools scripts/verify.sh` prints VERIFY-GREEN, with the count of passing tests rising by the new tests only.
- Both replay fingerprints and `SAVE_SCHEMA` 7 are unchanged at every commit, since nothing here touches the world.
- The schema proof and the bridge tests pass with 24 tools; `omnis-cli schema dump` parses back with replies.
- For 8d:
  - the app tests (`ui.rs`, the menus, the Feathers screens) pass unchanged in what they assert;
  - a headless screen capture compared before and after for the sheet, the fight and the town.
- For 8e: `cargo clippy -p omnis-app --no-default-features --lib` fails to compile if a fixture accessor is used in non-test code. I'll prove that once by planting a use and removing it.
- Mutation spot checks on the new view fields and on `rules_set` and `check_reload` in their shared form: a break counts only with a named failing test.
- Sentrux after each sub-step.
- **What you will see:** nothing changes in play. Over MCP you get `rest_get` and `cast_get`, `game_status` shows `protocol: 1` and `may_save`, and every reply names itself.

## Size
About 2,300 lines in total: 8b–8c about 650, 8d–8e about 1,200, and the docs.
