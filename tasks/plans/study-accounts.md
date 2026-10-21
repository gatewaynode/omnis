# Study: cryptographically secure user accounts

Read-only (W0.5, owner 2026-10-21). Nothing here is built until the owner chooses from it; the choice enters
`PRD.md` as D29 first. Four purposes, four designs: (1) identity in multiplayer, (2) signed saves and world
files, (3) signed packs and mod provenance, (4) a hosted service. Dates are checked against 2026-10-21.

## 1. What the documents say today

- No account, login, cloud, profile or server exists in `PRD.md`, `ARCHITECTURE.md`, `presentation-PRD.md` or
  `presentation-ARCHITECTURE.md`. "Server" means the MCP bridge (ARCH §9) and nothing else.
- PRD §2: players are "single-player, desktop"; "explicit non-consumers for v1: ... multiplayer groups". PRD §5:
  "Multiplayer of any kind (but see goal 7)". Goal 7 and D5 keep co-op possible; §13 Later lists "networked
  co-op on the deterministic sim" and, since 2026-10-21, this study.
- PRD §7.7: "Saves are files the player owns. There is no cloud dependency." Also: the save format is versioned
  and migratable; a tidy never removes a file on its own, "because saves are the player's files".
- PRD §11.1 and §11.3: dependency rules (pin, N-1, 30 days, minimize, own helpers, Socket); "treat mod packs
  as untrusted input (path traversal, size limits, no code execution)".
- ARCH §4.1.1: the world id is "sixteen hex digits of its manifest's hash"; every world-folder file "is named
  by the fnv64 hash of its text and never overwritten"; a load checks each base's text against its name
  (`LoadError::BadBase`). The character save has no such check of its own.
- ARCH §4.6: a `Replay` is seed, packs, settings, commands, fingerprint; "networked co-op later is 'share the
  command stream', which is why commands carry no client-side state".
- ARCH §4.9: the host ops (`save.write`, `save.read`, `save.check`, `pack.reload`) are I/O-free rules; "each
  host writes only the file and window code around them".
- ARCH §9.1: the dev socket is "loopback only, one client at a time, no auth beyond loopback in v1; an optional
  shared token file is a later addition".
- ARCH §11: "Hashing is our own. FNV-1a 64 and splitmix64 are a few lines each in `omnis-core`"; "the
  simulation never touches entropy"; "Later: multiplayer shares the world seed and gives each party its own
  `party:<id>` stream". `omnis-core/src/rng.rs:12` documents `fnv1a64` as "never for security".
- ARCH §12: the seal is "the FNV-1a 64 of the pack's content at its version"; the loader refuses an unsealed or
  changed pack. §12 has no network or user threat model. §13 keeps `rand`, `tokio` and any MCP SDK out.
- presentation-ARCH §10.5: the only per-player file is `.omnis/settings.ron`; §15 question 4: the platform's
  config directory "arrives with packaging (M12)". §12: every file read is bounded and symlink-free.

## 2. What the code constrains

- **The seal** (`omnis-data/src/manifest.rs:21`, `loader.rs:335-360`, `seal.rs`): `PackManifest.hash: u64`;
  `PackHasher` takes fnv64 of each data and text file, sorts the (path, hash) pairs, and fnv64s the list.
  `seal::seal` rewrites only the `version` and `hash` lines of `pack.ron`, so new fields (`signer`,
  `signature`) can sit beside `hash` and, like it, outside the hashed content. FNV-1a 64 is not collision
  resistant (a second preimage is cheap), so a signature over the present seal proves nothing: signing needs a
  cryptographic content hash first, and `PackFingerprint.hash` is what world manifests record, so changing it
  makes every existing save load as "other packs, same names" (`PackFit::Content`), which is allowed.
- **Saves** (`omnis-sim/src/save/disk.rs`, `character.rs:66`): `save_files` returns `Vec<FileOut>` texts,
  `load_save` takes a reader; the simulation never touches a file. The natural place for a signature is the
  host at write time: a sidecar `<save>.sig` written from the same texts, or a `signed.ron` per world folder
  listing each hash-named file. A field inside `CharacterSave` would bump `SAVE_SCHEMA` (9) and put a
  signature where the simulation would have to carry it; a sidecar keeps the schema still.
- **The command stream**: `Command` has no envelope and `Replay.commands` is `Vec<Command>`; the socket line is
  `{"id","op","args"}`. A signed stream is an envelope around the line (party key, sequence, signature over the
  bytes as sent), handled by the transport host. The simulation gains at most a party id (the multiplayer
  study's `party:<id>`); it never holds a key.
- **The lint** (`scripts/lint-sim.sh`): simulation crates, `omnis-data` included, may carry no float, hashed
  collection, clock, thread, network or (outside `omnis-data`) file I/O in their own source; dependencies are
  checked only for Bevy edges and rhai's features. Hashing and signature verification are deterministic
  integer code, so a crate for them is lint-clean. `omnis-sim` is `no_std`; `omnis-data` is std.
- **Where verification runs**: pack signatures in `omnis-data`'s loader, which is already the pack's gate
  (`load_one`); save signatures in the hosts (`omnis-vector`'s `game_io.rs`, `omnis-cli`) around `save_files`
  and `load_save`; command signatures in whichever host carries the stream. Key generation and signing stay in
  the hosts: they need entropy, which the simulation may not touch.
- **Entropy**: `omnis-vector/src/shell/screen.rs:280` draws a blank seed from the clock's nanoseconds through
  `splitmix64`, not OS entropy as ARCH §11 says (a doc drift for `tasks/BACKLOG.md`; harmless for a seed,
  unusable for a key). `getrandom` 0.4.3 is already in the tree (via `uuid`, `accesskit`, `winit`), so a
  CSPRNG costs no new crate.
- **The key store**: `.omnis/` is the development path and `.omnis/settings.ron` the only player file; a key
  file beside it (`.omnis/identity.ron`, 0600, read through `ron_io::read_text`) moves with it to the platform
  config directory at M12. An OS keychain is a dependency tree of its own and is not needed.
- **Already in the tree** (`cargo tree -i`): `blake3` 1.8.7 via `bevy_asset` (features `default`, `std`),
  `getrandom` 0.4.3, `cpufeatures` 0.3.1, `cfg-if`, `libc`, `cc`. No signature, SHA-2 or TLS crate.

## 3. The four purposes

### Dependencies (checked 2026-10-21; rule status against CLAUDE.md and ARCH §12)
- `blake3` 1.8.7, 2026-08-20, 62 days; the version Bevy resolves (rule 1, match Bevy): 0 new crates. CC0-1.0
  OR Apache-2.0. Has a `cc` build script for assembly (`cc` already in the tree). Clears.
- `getrandom` 0.4.3, 2026-06-17, 126 days; Bevy's resolution: 0 new crates. Clears.
- `ed25519-compact` 2.4.2, 2026-08-19, 63 days; latest is 2.6.0 (2026-09-25, 26 days, fails 30 days), 2.5.0 the
  same day; 2.4.2 is N-2 by minor. MIT, jedisct1 (libsodium's author), `no_std` without `std`; 0 dependencies
  without `random`, with `random` only `getrandom` 0.4 (in tree). 1 new crate. Socket score not yet read (no
  CLI in this session): read before adding. Clears N-1 and 30 days.
- `ed25519-dalek` 2.2.0, 2025-07-09, N-1 behind 3.0.0 (2026-07-06, 107 days). BSD-3-Clause. Minimal tree 15
  crates, 12 new; `cpufeatures` 0.2.17 (2025-01-25) duplicates Bevy's 0.3.1, failing `check-duplicates.sh`
  without an exception; `zeroize` resolves to 1.9.1 (2026-10-06, 15 days) unless pinned to 1.8.2 (2025-09-29);
  `rand_core` 0.6.4 would duplicate Bevy's 0.10.1 if the `rand_core` feature is used (not needed: a 32-byte
  secret from `getrandom` suffices). Clears N-1 only with two pins and a duplicate exception.
- `ed25519-dalek` 3.0.0, 2026-07-06: N, fails N-1; 14 crates; its `sha2` 0.11.1 (2026-10-12, 9 days) needs a
  pin to 0.11.0 (2026-03-25). `ed25519-zebra` 4.2.0 (2026-03-20): 36 crates with a proc-macro stack. Rejected.
- `sha2` 0.10.9, 2025-04-30, N-1 behind 0.11.x; 10 crates, 7 new, the same `cpufeatures` duplicate. Only if
  blake3 is refused, and blake3 is already Bevy's.
- **Own SHA-256**: about 150 lines plus 50 of FIPS 180-4 vectors, one day, low risk (public data, no timing
  concern), in the project's practice (own PCG32, FNV, MCP, noise). Not needed while blake3 is in the tree.
- **Own Ed25519**: field arithmetic mod 2^255-19 (about 300 lines; `u128` products are lint-clean integers),
  SHA-512 (150), scalars mod L (120), extended-coordinate point ops and a windowed multiply (230),
  decompression (60), sign and verify (80), RFC 8032 vectors (100): 1100 to 1300 lines in three files, two to
  four weeks with review. Risks: carry and reduction bugs that pass the vectors; verification edge cases
  (non-canonical encodings, small-order points, cofactored versus cofactorless); timing on the signing path.
  Against one zero-dependency crate of about 2500 reviewed lines, the security rule outweighs the
  self-supporting rule; own code is the horizon if that crate ever lapses.

### (1) Identity in multiplayer: a keypair naming a party, signing its commands
- Needs: an Ed25519 keypair per player; a party id = the public key (or its blake3-256 cut to sixteen hex, the
  world id's shape); each command in an envelope `{party, seq, prev, line, sig}` signed over the world id, the
  sequence and the line's bytes as sent (no JSON canonicalization), `prev` chaining the order; a transport,
  which does not exist (no `tokio`, no async; the dev socket is the only line protocol); a roster per world;
  no trust root (peers vouch on joining); revocation = a new key is a new party.
- Smallest design: the envelope on the socket wire, verified by the host before `apply`; the simulation gains
  a `PartyId` and the `party:<id>` stream (ARCH §11). About 250 host lines, none in the simulation beyond
  the multiplayer study's own.
- Documents: PRD D5 and §5 move with the multiplayer study; ARCH §4.6 (the envelope), §9.1 (the token line
  becomes "a signed envelope"), §12 (a network threat model: impersonation, replay, reordering).
- Threats answered: a peer impersonating another party; a replayed, dropped or reordered command; a host
  rewriting a command. Not answered: a modified client (a signed command is still the player's choice; the
  simulation runs on every machine, so cheating is a multiplayer-design question, not a signature one);
  denial of service; a key copied from the player's disk.
- Earliest phase: after W5, with the multiplayer study's transport; the key format and envelope can be fixed
  on paper in D29 now at no cost.

### (2) Signed saves and world files
- Needs: the same keypair; blake3-256 of each file `save_files` returns; one Ed25519 signature per save over
  (world id, the file list with hashes); a sidecar `<save>.sig` and `worlds/<id>/signed.ron`; verification in
  the hosts before `load_save`. No trust root: the key is the player's own.
- Honest framing: the player holds the key, so a signature detects change by anyone but the player, and PRD
  §7.7 makes the files the player's: a player editing a save is not a threat by the owner's own word. What a
  signature adds is provenance when a save leaves the machine (a co-op partner's save, an upload in (4), a bug
  report) and a check the character save lacks today (bases are checked by name; `quick.ron` is not).
- Smallest design, two sizes: (a) integrity only, a blake3-256 sidecar, 0 new crates, about 60 lines in the
  hosts, warns on mismatch and never refuses; (b) provenance, (a) plus an Ed25519 signature, 1 crate, about
  150 lines plus the key store of (3). Loading a save that does not verify warns and loads: the file is still
  the player's.
- Documents: PRD §7.7 one bullet; ARCH §4.1.1 writing and loading, §6.2 (the sidecar's limits), §12.
- Threats answered: silent corruption; a save altered by another program or person; a save passed off as
  another player's. Not answered: the player's own edits; a stolen key; a rolled-back save (that is a game
  rule, D17).
- Earliest phase: (a) any time; (b) W5 with the key store (M12 packaging decides the config directory).

### (3) Signed packs and mod provenance: the seal becomes a signature
- Needs: the author's keypair; `pack.ron` gains `signer` and `signature`, outside the hashed content like
  `hash`; the content hash becomes blake3-256 (a `pack.ron` schema bump, every pack resealed, both goldens
  rebaselined); the loader verifies when a signature is present and reports "signed by <key>", "unsigned" or
  "does not verify"; a trust list `.omnis/trust.ron`, trust on first use (TOFU, as `known_hosts`): a pack id
  seen with a new signer is a warning; the project's public key compiled in as a pinned root for `base` only.
  Revocation: none without a service; a new key under TOFU is a warning, the honest ceiling. "Whether it
  changed": the seal already fixes content under one version; the signature adds who sealed it and covers
  `migrations/`, which are sealed content.
- Smallest design: `omnis-cli pack seal --sign <key>` (60 lines), manifest fields and verification in
  `omnis-data` (150), the report row through `load_packs` to the title and Load screens (40), the trust list
  (80), key generation in the CLI (40): about 370 lines, 1 new crate. The editor reseals when it writes a pack;
  signing from the editor needs the key on that machine, which is the modder's own.
- Documents: ARCH §6.1 (two fields), §6.2 (the checks), §6.3 (attribution gains the signer), §12 (the seal
  paragraph); PRD §10 or §11.3 one line; the mod documentation (M12).
- Threats answered: a pack altered after its author sealed it (a mirror, a bundle, a transit); a pack claiming
  an author without that author's key; the base pack's authenticity in a shipped build. Not answered: a
  malicious author (a signature says who, not whether it is safe; the loader's limits and no code execution
  stay the defence, PRD §11.3); a first-load impostor under TOFU; a stolen key; a safe pack whose author lost
  the key (a new key, a warning).
- Earliest phase: W1, since the editor's seal work touches the same code; recommended W5, where the mod
  documentation and the "a modder outside the team ships a pack" criterion (PRD §13 Phase 5) give it a
  consumer. Unsigned packs are never refused: PRD §2 ranks modders second and TOFU cannot bootstrap otherwise.

### (4) A hosted service: login, profiles, cloud saves
- Needs: a server (TLS, storage, accounts, abuse handling, privacy law, an operator on call); a client with
  TLS and HTTP (an `ureq` or `reqwest` tree is 60 to 120 crates, uncosted because the choice is far; `tokio`
  is deliberately absent); passwords or passkeys unless the account is the keypair of (1); money every month.
- Smallest design, two directions: (a) no own service: "cloud saves" are the player's own folder sync. The
  world folder is content-addressed (hash-named files never overwritten, ARCH §4.1.1), so iCloud, Dropbox or
  Syncthing already sync it without conflicts; the character save is one file; a profile is the public key of
  (1); login is possession of the key. 0 crates, 0 lines, a paragraph of documentation. (b) a platform's
  service (Steam Cloud, GOG Galaxy) at W5 packaging: a platform SDK, no own server, the local files stay
  canonical. (c) an own server: the direction this study does not recommend at any phase.
- Documents: PRD §7.7 amendment (below); §2 a note; ARCH §12 a line if (b) is ever chosen.
- Threats answered: a lost disk; play across machines. Threats introduced: account takeover, a server breach
  holding every player's saves, availability, and the "files the player owns" promise bent.
- Earliest phase: (a) now, as words; (b) W5; an own service never, unless the owner states a reason that
  outweighs the operating cost.

## 4. Comparison

| Purpose | New crates | Own lines | Document changes | Earliest phase | If never built |
|---|---|---|---|---|---|
| (1) Multiplayer identity | 1 (`ed25519-compact`) | ~250 host | PRD D5/§5 (with multiplayer), ARCH §4.6, §9.1, §12 | after W5, with a transport | Co-op trusts the socket: fine on a LAN, not on the internet |
| (2a) Save integrity | 0 (`blake3`) | ~60 host | PRD §7.7, ARCH §4.1.1 | any | Corruption of `quick.ron` is found only by its parser |
| (2b) Save provenance | 1 | ~150 + key store | as (2a) plus §12 | W5 (key store) | A shared save has no author; nothing else |
| (3) Signed packs | 1 | ~370 | ARCH §6.1-6.3, §12; PRD §11.3; mod docs | W1 possible, W5 recommended | Mods are sealed, not attributed; a mirror can alter a pack and reseal it |
| (4a) Folder sync | 0 | 0 | PRD §7.7 one line | now | Nothing: it is the status quo, written down |
| (4b) Platform cloud | SDK | ~200 | PRD §7.7, §2; ARCH §12 | W5 | Players sync by hand (4a) |
| (4c) Own service | 60-120 client, a server | thousands | PRD §2, §5, §7.7, §11, §12; ARCH §12, §13 | never recommended | Nothing is lost |

One keypair serves (1), (2b) and (3); a modder's key and a player's key are the same kind of file.

## 5. Proposed PRD rows (drafts; blanks are the owner's)

- **D29 | Accounts and signatures |** Choice: an account is a local Ed25519 keypair the player or modder owns
  (`identity.ron` beside the settings file, the platform config directory from M12); it names a party in co-op,
  signs packs at `pack seal --sign`, and may sign saves; verification is in the pack loader and the hosts, never
  the simulation; content hashes move from FNV-1a 64 to blake3-256 where a signature covers them; trust on
  first use with the project's key pinned for `base`; unsigned packs and unverified saves load with a warning.
  First build: ____ (recommended: (3) at W5 with the mod documentation; (2a) with it; (1) with multiplayer).
  Rejected alternatives: an own Ed25519 (1100+ lines, review risk, security rule over self-supporting rule;
  a horizon if the crate lapses); `ed25519-dalek` (12 crates, a duplicate `cpufeatures`, two pins); a signature
  registry or revocation service (needs (4c)); an own hosted service (operating cost against PRD §7.7's
  promise). Rationale: one crate with zero dependencies, zero new hashing crates, the deterministic simulation
  untouched, and every file stays the player's. Owner: ____ (date).
- **§2 consumers note:** "Builders and modders: a pack may carry its author's signature; the loader shows who
  sealed it and whether it verifies, and never refuses an unsigned pack."
- **§5 non-goal or §13 Later line** (one or the other, the owner picks): §5: "An account held by a server, or a
  login, in v1." §13 Later: "Signed packs, saves and commands on a local keypair (D29): packs at W5, saves with
  them, commands with co-op."
- **§7.7 amendment:** "Saves are files the player owns. There is no cloud dependency. The world folder is
  content-addressed and the character save one file, so any folder sync the player already runs carries them;
  a platform's cloud save may be offered at packaging as an option that never becomes a requirement, and the
  local files remain canonical. A save may carry its maker's signature (D29); one that does not verify loads
  with a warning."

## 6. Questions for the owner (with a recommended answer)

1. Is a player editing their own save a threat Omnis defends against? Recommended: no (PRD §7.7); signatures on
   saves are provenance for files that leave the machine, never anti-cheat.
2. A crate or own code for Ed25519? Recommended: `ed25519-compact` 2.4.2 after a Socket read; own code stays a
   horizon. Hashing: `blake3`, already Bevy's, no own SHA-256.
3. Trust model for mods: TOFU plus the project's pinned key for `base`, no registry? Recommended: yes; a
   registry needs a service.
4. Unsigned or unverified packs: warn or refuse? Recommended: warn, always load; refusing would turn away the
   second-ranked consumer.
5. Does the seal's hash move to blake3-256 now, ahead of signing (a `pack.ron` schema bump, every pack
   resealed, goldens rebaselined)? Recommended: no; move it in the same change as the signature, so packs are
   resealed once.
6. Which purpose is built first, and at which phase? Recommended: (3) at W5 with the mod documentation; (2a)
   alongside at no dependency cost; (1) when the multiplayer study sets a transport; (4) as words now.
7. A hosted service: own, a platform's, or neither? Recommended: a platform's at W5 if the store asks for it;
   never an own server.
8. Is the dev socket's "optional shared token file" (ARCH §9.1) part of this? Recommended: no; it is 32 bytes
   from `getrandom` in a file when the first non-loopback client is wanted, and stays its own line.
9. Should the seed's clock-based entropy (`screen.rs:280`) be brought to ARCH §11's "OS entropy"? Recommended:
   a BACKLOG item; harmless today, and `getrandom` is already in the tree for the day a key needs it.

## 7. Recommendation

Do nothing in code now. Record D29 as a horizon with the shapes fixed on paper (Ed25519 keypair, blake3-256
content hashes, a sidecar for saves, `signer` and `signature` in `pack.ron`, TOFU with a pinned base key), so
no phase between here and W5 builds against it, and write the §7.7 amendment, which costs nothing and answers
the cloud question honestly: folder sync of content-addressed files is already the cloud save. The first real
build is signed packs at W5, when a modder outside the team is the acceptance criterion: one crate of zero
dependencies, about 370 lines, a pack schema bump and a reseal of every pack. Save integrity rides along for
60 lines and no crate. Multiplayer identity waits for the multiplayer study's transport, which is the larger
and harder design. An own hosted service is not recommended at any phase: it is the one purpose whose
dependency tree and operating cost cannot meet the project's rules, and the one that strains PRD §7.7's
promise rather than keeping it.
