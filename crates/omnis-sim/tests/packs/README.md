# The fixture packs

`test/` is **the frozen fixture**: `packs/test` as committed at `8d58c3e2` (id `test`, 0.2.9), the content set every
test and the two golden replays run against (owner, 2026-11-07). It is sealed once and never saved onto: a test
that writes works on a scratch copy (`copy_pack`, `scratch_pack`, `in_editor_scratch`). Its `pack.ron` is part of
the seal, comments included, so even a comment there moves the hash. A deliberate change re-baselines the replays
in the same commit (`cargo test -p omnis-sim rebaseline -- --ignored`) and records the fingerprints in
`tasks/knowledge/verification.md`. The owner's world under `packs/` is never a test's.

The others (`ahead`, `chain/v1`, `chain/v3`, `heights`, `levels`, `objects`) are the engine's own fixtures, each
depending on `base` and `test` for the tilesets, loaded by the helpers in each crate's `tests/common`.
