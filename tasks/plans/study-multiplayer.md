# Study: multiplayer (W0.5)

A study, not a plan to build (owner, 2026-10-21, `tasks/TODO.md` W0.5). It reads what the documents say, what
the code as built constrains, four models with their costs, the PRD rows the owner would fill in, and the
earliest phase. Nothing is built before the owner chooses. Written 2026-10-21 against protocol 2, `SAVE_SCHEMA`
9 and the `M8.5-tasks` branch.

## 1. What the documents say today
- **A v1 non-goal, kept possible.** PRD §5: "Multiplayer of any kind (but see goal 7)". Goal 7: the deterministic
  sim makes "networked co-op a future addition instead of a rewrite". D5: desktop single-player, "co-op later",
  rejecting "single-player only forever". §13 Later: "Networked co-op on the deterministic sim"; since 2026-10-21
  "multiplayer ... under study ... nothing is built before" a decision row.
- **The time model was designed for it.** D25: one mechanism serves "NPC agency, multiplayer, construction,
  travel, and aging". PRD §7.8 names "other players' parties" as holders of their own clocks and lists
  multiplayer among the things the shape buys. ARCHITECTURE §4.4: "each party is a holder; parties reconcile when
  they meet, and no global tick has to be agreed between players"; "two parties meet" is an enumerated
  interaction point; A13 says the same.
- **Co-op is the command stream.** ARCHITECTURE §4.6: "Networked co-op later is 'share the command stream',
  which is why commands carry no client-side state." A2: "Replay, tests, MCP, and co-op all reduce to the same
  stream." §11: "Later: multiplayer shares the world seed and gives each party its own `party:<id>` stream."
- **The save model admits several parties.** D28 and §7.7: a character save per party over a world folder the
  simulation owns. `tasks/plans/world-state.md`: "Co-op (D5) and other players' parties (§7.8): several
  character saves over one world file"; its hard row: two saves "must not both own the same change".
  `tasks/knowledge/horizons.md`: co-op needs "the world's changes stored as shared layers, not only as each
  save's private journal". §7.7: saves are the player's files, "no cloud dependency".
- **The transport posture.** ARCHITECTURE §9.1: the dev socket is loopback TCP, newline JSON, one client, no
  auth, polled by a Bevy system each frame, "no async runtime"; §13: `tokio` is "deliberately absent"; A3: no
  async runtime in the game. §4.9: two API tiers, protocol 2; `docs/api.md` §14: "No push. Clients poll."

## 2. What the code constrains
- **One party, one place, one mode.** `World` (`omnis-sim/src/world.rs:257`) holds `party`, `position`, `mode`,
  `party_time`, `automap`, `known`, `log` and `flags` as single fields; `PARTY` is the constant
  `HolderId::Party(PartyId(0))` (`lib.rs:106`), used at 11 sites; `world.party` and `world.position` are read at
  266 sites in 46 of `omnis-sim`'s files (20,389 lines). `CharacterId` is allocated from `party.next_character`
  (`party.rs:123`), so two parties would mint the same ids. `Mode` is one for the world: a second party could
  not be in a fight while the first explores.
- **Holders already fit several parties.** `HolderId::Party(PartyId)` exists (`omnis-core/src/id.rs:91`);
  clocks and contacts are keyed by holder and by holder pair; `StreamName::time(a, b)` is canonical for any
  pair, so `time:party:0:party:1` derives with no new code. `time.rs` reconciles only the party with a region
  (`reconcile_party`, hard-wired to `PARTY`) and documents the missing sites: "meeting an actor, inspecting a
  project, two parties meeting, the start of an encounter". A party-to-party contact needs a rule slot
  (`time.party`, like `time.settled`), a `delta` for each side (today `b` catches up and `a` moves nothing), and
  the greeting: `Event::Reconciled { a, b, delta_a, delta_b }` already carries exactly the two deltas of the lore.
- **The save already bundles the party.** `save::Checkpoint` (`save/mod.rs`) is the party's state apart from
  the world: party, party_time, party flags, its holdings (clock, contacts, the streams no region owns), bus,
  position, automap, known, mode, settings, turn. That is the `PartyState` a multi-party `World` would hold
  in memory; today it exists only on disk. `stream_owner` (`save/split.rs`) already parses a `party` kind in a
  stream name and makes bare names (`combat`, `encounter`, `cast`, `items`, `town`, `rest`, `object`, `party`)
  the party's, so each character save carries its own fight dice. The `party:<id>` stream of §11 is a naming
  change, not a mechanism: `combat` becomes `combat:party:1` or the checkpoint keeps bare names per save.
- **The journal and the fold under two parties.** `journal::Entry` is `{ minute, at, command, changes }` with
  no party; `Change` is the value after (door open, object state, pile, cleared, world flag, a non-party clock,
  contact or stream), found by diffing `Held` before and after each command (`apply.rs`), "so laying an entry
  twice is harmless". Two journals over one base are therefore safe per key and wrong across keys: a region
  clock both parties advanced takes the last lay's value, not the sum; a region's `time:` stream and the world
  flags are order-dependent. A live host has no problem (one `World`, one apply order, one journal). Two
  separate sessions have: the merge is a fold (`save::fold`: bases from `split(world)`, journal to the archive)
  with an agreed order, and `Entry.command` is kept, so the other party's interactions can be re-applied over
  the new base with the changed-pack machinery ("skipped and reported", §7.7). The world folder is append-only
  and hash-named (`save::save_files`, "never overwritten"), so two writers cannot clobber a base; the character
  save's `fingerprint` check (`LoadError::Diverged`) must then accept a merged world.
- **"Share the command stream" concretely.** `Command` has 16 variants (`Step`, `Turn`, `Interact`, `Door`,
  `Search`, `Climb`, `Party(Create|Reorder|Tactics)`, `Encounter`, `Combat`, `Cast`, `Item`, `Service`, `Rest`,
  `Dev`, `Act`, `Approach`); members by `CharacterId`, definitions by string id, no slot, no client state
  (protocol 2). `Replay { seed, packs, settings, commands, fingerprint }` rebuilds a game from them. A shared
  stream is `Vec<(PartyId, Command)>` in host arrival order; with one party it is the binder log the vector
  client already keeps (`host.rs::accepted` appends socket commands to it). Named streams in use: `party`,
  `combat`, `encounter`, `cast`, `items`, `town`, `rest`, `object`, `time:<a>:<b>`.
- **No async; a transport lives outside the sim crates.** `Cargo.lock` has no `tokio`. The dev socket
  (`omnis-vector/src/shell/socket.rs`, 422 lines) is `std::net::TcpListener` set non-blocking, one client,
  polled from a Bevy system; `host.rs` (100 lines) routes host ops and sends the rest to `dispatch`;
  `omnis_cli::Headless` (119 lines) is the same host with no window. A multiplayer transport is this code
  with several streams, a line framing it already has, a thread or non-blocking poll per connection, and auth
  from the accounts study; the sim crates see only `Command`s and `Op`s. The lint bans threads and network only
  in the nine simulation crates.
- **Where a second party's view comes from.** The client reads only views (`SimWorld`, private world,
  `presentation-ARCHITECTURE.md` §4.1): `here`, `party_view`, `combat_view`, `viewport`. Every view takes
  `&World` and reads `world.party`/`world.position`; a guest needs either views computed by the host and sent as
  JSON (Tier 2 has every view as an op; `docs/api.md` §7) or a mirror `World` kept in step by the same command
  stream (lockstep; deterministic by A14, so the mirror's fingerprint checks the link). Events reach "whoever
  sent" the command; a push or a per-party `events.tail` cursor is needed either way. Other parties on a map
  are drawn as beings (`ViewportModel.beings`, M8.5 2c) once a view lists them.

## 3. The models
Each on its own, with what it keeps and gives up. Sizes are orders of magnitude in lines against `omnis-sim`
20,389, `omnis-vector` 19,533, `omnis-cli` 2,467, `omnis-mcp` 1,489.

### (a) Co-op on one party
Several players, one party, one `World` on the host. Either every player may send any command (the table's
"whoever has the mouse"), or each player owns members (`CombatCommand` names its actor by `CharacterId`, so
"your turn" is a host rule) and out of a fight the host's player, or a rotating lead, moves.
- Engine: none to `World`; optionally `Party.owners: BTreeMap<CharacterId, PlayerId>` with a `Rejection::NotYours`
  checked in `apply` (so it holds in replay), 100 to 200 lines, additive under protocol 2.
- Host and transport: the dev socket generalized (several clients, a bind address beyond loopback, auth,
  event broadcast); guests run the client in a "guest" mode that renders from views the host answers or from a
  lockstep mirror. 1,000 to 2,000 lines in `omnis-vector`, or a std-only `omnis-net` crate shared with the hosts.
- Save: unchanged; the host's character save and world folder; a guest keeps nothing.
- Determinism and anti-cheat: the host is authoritative; a mirror that diverges is detected by fingerprint; a
  guest cannot roll dice. What replay does not give: hiding information between players (one party knows one
  automap) and latency hiding (every command is a round trip).
- Documents: PRD §5, D5, a D30 row, §7.1 (who commands the party); ARCHITECTURE §9 (the socket as a transport).
- Earliest phase: after W1 (nothing in W2 to W4 blocks it; it rides on the socket). Keeps: the whole engine as
  is, saves as is. Gives up: separate parties, meeting, the time model's multiplayer promise.

### (b) Several parties in one world, meeting by the time model
Each player a party in one `World` on a host; each party keeps its own clock, so a command of party B never
waits for A (no lockstep, no global tick: §4.4 made literal). Parties on one map see each other as beings;
meeting reconciles both clocks by a `time.party` rule and raises `Reconciled` with the lore's two deltas.
- Engine: `World.parties: BTreeMap<PartyId, PartyState>` (the `Checkpoint` shape in memory) replacing the single
  fields; `apply(world, data, party, command)`; `Entry.party`; world-unique `CharacterId` (a counter on the
  world); bare streams named `combat:party:<id>`; `Mode` per party; a fight is one party's (two parties in one
  fight is a horizon: initiative and the encounter clock are per fight today); `reconcile_party` taking its
  party; the party meeting site. A mechanical refactor of 266 sites plus 1,500 to 3,000 new lines; save schema
  bump (the character save stays one checkpoint). Protocol stays 2 if `party` defaults to 0 on the wire.
- Host and transport: as (a), each connection bound to a party; the host may be a player's client or the
  headless host. Guests render from views (the simplest) since each party's views are cheap and per party.
- Save: one character save per party, one world folder; the host writes all; the journal is the world's (every
  party's interactions in apply order) and each save names it or holds its own slice; a fold is one event.
- Determinism and anti-cheat: the host's interleaved stream replays exactly; views give each party only what
  it knows (automap, known, log are per party by construction).
- Documents: PRD §5, D5, D30, §7.1, §7.8 (the sentence stands); ARCHITECTURE §4.1 (the struct), §4.2 (party in
  commands and entries), §4.4 (the site built), §4.9, §11 (streams), §12.
- Earliest phase: the refactor's cost grows with every system that adds per-party state, so the cheapest window
  is between W1 and W3 (before M9 and M10 add region state that must know which party made contact); the
  meeting rule and the transport, W4 at the earliest. Keeps: the time model's promise, player-owned saves per
  party, no lockstep. Gives up: a shared fight (horizon), and the single-party code's simplicity.

### (c) Asynchronous play over a shared world file
No live connection. Players share one world folder (files, a sync folder, or the accounts study's service),
each with their own character save, playing at different times; folds are the sync points where the host of
the fold lays every journal over the base in an agreed order.
- Engine: a merge fold (`fold` with several journals: lay the first, re-apply the others' `Entry.command`s
  with the changed-pack check, report what no longer fits), a `Verdict::Merged`, and the fingerprint check
  relaxed to the merged world; per-party `Entry` and `CharacterId` as in (b) so journals name their party. 1,000
  to 2,500 lines in `save/` and `journal.rs`; `omnis-cli world merge`; the Load screen's report.
- Host and transport: none; file sharing is outside the game. A service for the folder is the accounts study.
- Save: as D28, several character saves over one folder; the append-only hash-named files already tolerate
  two writers; only the merge is new.
- Determinism and anti-cheat: a merge is deterministic given the order; a player can edit their own character
  save (true today; signed saves are the accounts study). Parties never meet live, so the time model gives the
  fiction for free: "been moons on dae road" is the other party's journal, laid later.
- Documents: PRD §5, D5, D30, §7.7 (a merge fold); ARCHITECTURE §4.1.1 (the merge), §4.6.
- Earliest phase: W4, since a merge is a fold and the fold's thresholds wait for M10. Keeps: no server, no
  transport, no auth, files the player owns. Gives up: seeing, trading with or fighting beside another party.

### (d) A hosted authoritative server with thin clients
`omnis_cli::Headless` behind a listener, run by a player or a service, owning the `World` and the saves;
clients send ops and receive views and events; (a) or (b) inside.
- Engine: (b)'s, plus sliced long ops and a push (`docs/api.md` §14 lists both as gaps).
- Host and transport: a new std crate `omnis-server` (3,000 to 5,000 lines: listener, sessions, auth from the
  accounts study, broadcast, persistence), no async runtime needed at co-op scale (a thread per connection and a
  channel to the one apply loop); the client's guest mode as in (a).
- Save: the server's files; a player's download of their character save keeps §7.7 true only if the service
  is optional (the accounts study's proposed row).
- Determinism and anti-cheat: complete, the server rolls every die and validates every command; the cost is
  hosting, uptime and the security surface (§12's limits exist: 1 MiB lines, an op allowlist, no panics).
- Documents: PRD §2 (a server consumer), §5, D5, D30, §7.7 ("no cloud dependency" qualified), §11.1;
  ARCHITECTURE §2, §9, §12, §13 (dependencies), a new §; the accounts decision first.
- Earliest phase: W5 or after. Keeps: full authority. Gives up: the no-server posture, and it waits on accounts.

## 4. Comparison
| | (a) one party | (b) parties, time model | (c) async world file | (d) hosted server |
|---|---|---|---|---|
| Players see each other | yes, one view | yes, as beings | no | as (a) or (b) |
| Turn sync | lockstep per command | none (own clocks) | none | as inside |
| `World` change | none | parties map, refactor | per-party entries | as (b) |
| Transport | socket generalized | socket generalized | none | new crate |
| Needs accounts study | auth only | auth only | no (files) | yes |
| Save model | unchanged | save per party | save per party + merge fold | server owns |
| Anti-cheat | host authoritative | host authoritative | none beyond signing | full |
| Earliest phase | after W1 | refactor W1-W3, play W4 | W4 | W5+ |
| Size (lines) | 1-2k client, 0-200 engine | 3-5k engine, 1-2k client | 1-2.5k engine | 5-8k + (b) |
| Serves D25's promise | no | yes | in fiction | as inside |

## 5. What every phase must keep (forward compatibility, PRD §9.2 style)
1. Commands carry no client state and name members and definitions by identity. True now (protocol 2).
2. Every die is rolled by the host from a named stream; validate then mutate. True now (A5, A14, §4.2).
3. Every party is a holder with its own clock, reconciled by pair. True in type (`HolderId::Party(PartyId)`,
   `StreamName::time`); not yet in code: `PARTY` is a constant at 11 sites and `reconcile_party` takes none.
4. The party's state is one bundle the world holds per party. Not yet: `Checkpoint` is that bundle on disk;
   in memory it is eight `World` fields read at 266 sites.
5. `CharacterId` is unique across parties. Not yet: minted per party (`party.next_character`).
6. A journal entry names its party. Not yet: `Entry { minute, at, command, changes }`.
7. World changes are values after, laid idempotently; the world folder is append-only and hash-named. True now.
8. The bare streams are the party's, nameable per party. Half: `stream_owner` parses `party:<id>`; nothing
   emits it.
9. Transport, threads and files stay outside the simulation crates; no async runtime. True now (the lint,
   `socket.rs`, no `tokio`).
10. Tier 2 answers every view, and the host is a thin shell over `dispatch`. True now (27 ops; `Headless`).
11. Events are addressable per party (a push or a cursor per party). Not yet ("No push", api.md §14).
Rule for W1 to W5: no new per-party state goes into `World` as a bare field; it goes where `Checkpoint` keeps
it, and the first system to open `World` for its own reason (M9's regions or M10's state) does 4 and 5 then.

## 6. Proposed PRD rows
- **D30 Multiplayer** | Choice: `[ (a) co-op on one party | (b) several parties meeting by the time model |
  (c) asynchronous play over a shared world | (d) hosted server ]`, entering the plan at phase `[ W_ ]`;
  until then the eleven requirements of `tasks/plans/study-multiplayer.md` §5 hold, and 3 to 6 are done
  `[ with M9 | with M10 | on their own ]` | Rejected: `[ the models the owner rejects outright ]` | Rationale:
  goal 7 and D25 were built for it; the cost today is a refactor of the one-party `World` whose price rises with
  every system added, and a transport that lives outside the simulation; the accounts decision `[ D29 ]` gates any
  model that leaves loopback.
- **§5 non-goal, reworded**: "Multiplayer in v1 (D30 names the model it becomes and the phase; goal 7, D25 and
  D28 keep the door open and §9.2-style requirements in the study hold every phase)."
- **§13 Later, the line**: "Multiplayer as D30 chooses: `[ model ]` on the deterministic sim, each party a
  holder of its own clock, a character save per party over one world folder."
- **§7.8, if it changes**: "... construction projects and other players' parties (D30)." The sentence stands
  as written; only the pointer is added.

## 7. Questions for the owner (each with a recommended answer)
1. Which experience is the goal: friends at one table (a), friends in one world who need not be online
   together (b or c), or a service (d)? Recommended: (b) as the horizon, (a) never alone (it serves no promise
   the documents make), (d) only if the accounts study chooses a service.
2. May two parties share a fight? Recommended: no in the first multiplayer step; a horizon with its own row
   (initiative across parties, the encounter clock, loot).
3. Does a party's `CharacterId` become world-unique now (a save schema bump with a migration) or with the
   refactor? Recommended: with the refactor, in one schema bump.
4. Is a live link required, or is (c)'s file-sharing play acceptable first? Recommended: accept (c) as the
   first playable step of (b): it needs no transport and no auth, and its merge fold is work (b) needs anyway.
5. Who hosts: a player's client or a headless process? Recommended: both from day one, since `Headless` and the
   window's host already share `dispatch`; a server (d) is a later shell, not a different engine.
6. Protocol: may `party` default to 0 so protocol 2 stands? Recommended: yes; an absent `party` is party 0.

## 8. Recommendation
Do nothing playable until W4, and do the two cheap things that keep the price flat: write D30 as a horizon
naming (b) with (c) as its first step, and fold requirements 4 and 5 of §5 (the `PartyState` bundle and a
world-unique `CharacterId`) into the first milestone that opens `World` anyway (M9 in W3), as a refactor with a
schema bump and no new behaviour; requirements 3, 6 and 8 follow in the same change at a few hundred lines. The
honest cost of (b) is 3,000 to 5,000 engine lines of which most is the refactor whose price is paid once, plus a
transport of 1,000 to 2,000 lines outside the simulation that the accounts decision gates; (a) alone is cheaper
but builds nothing D25 promised, (d) is a hosting business before it is a feature. No Bevy networking crate is
needed for any model: the transport is the newline-JSON socket the game already has, and the ecosystem crates
(lightyear 0.29 for Bevy 0.19; bevy_replicon's 0.19 line unconfirmed on 2026-10-21) replicate ECS state, which
the engine deliberately does not keep in the ECS (A10); any such crate would fail the N-1 and 30-day check at
each Bevy migration and add nothing the command stream does not already give.
