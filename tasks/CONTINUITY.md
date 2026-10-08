# Continuity notes

Written 2026-10-08. **M8 is closed** (owner acceptance passed 2026-10-08). M7 closed at `68c4d09`. Branch
`m7a-b-tasks`, not pushed since `4b1bd14`. Rewrite this file every time it is used. The durable knowledge lives in
`tasks/knowledge/`.

## M8 acceptance (2026-10-08)
- The owner played parts 1, 3, 4 and 6 in the window. Parts 2 and 5 ran through the real `omnis-mcp` binary over
  stdio against the running game, because this session's `/mcp` reconnect never launched the bridge (its log
  `.omnis/mcp.log` untouched since 2026-10-02). Driver: `<scratchpad>/mcp.py` (`Bridge(log).call(tool, args)`).
- Numbers: 720 short rests in the meadow, age +43,200, shared +4,320; town entry snapped the date day 30 → day 3
  (HUD day 4, it counts from 1); fingerprint unchanged across `rest_get`/`cast_get`.
- The M7 log anomaly did not show: dropped, no B6.

## Next: protocol 2, one meaning per field name (owner, 2026-10-08)
- Owner: "Close it, and fix the field names now before protocol 2". TODO item at the top of "M9–M12".
- The finding: in commands `caster` is a marching-order slot and `spell` a row in the caster's list; in events
  and views `caster` is a `CharacterId` and `spell` a `SpellId` (registry index). Casting Bless with `spell: 3`
  produced `SpellCast { spell: 1 }`. `docs/api.md` §3 documents it, but the wire names collide.
- **Plan mode first.** Open questions for the plan:
  - the full inventory of colliding names (`caster`, `spell`, `member`, `target`, `item`, `index`, `stack`…)
    across `Command`, `Event`, `Rejection`, the views and `Op` args;
  - whether saves hold commands (the replay log) and what `SAVE_SCHEMA` 7 → 8 and the replay files need (serde
    `alias` for old names on read, or a migration); replay fingerprints must stay or be re-pinned with reason;
  - `PROTOCOL` 1 → 2, changelog with the migration, `api_doc.rs` drift test, MCP schemas, schema proof numbers.
- Then plan M9. Note TODO line ~165: **Editor v1 is held "after M8, before M9"** (owner, 2026-09-20).

## Other open TODO items (unscheduled, owner's call)
- `DevCommand::Pass { minutes }`; the `data.*` ops (labels, definitions); `scripts/mcp-probe.py`.

## Pins (after 8f, unchanged by acceptance)
- Gate `tests passed 572 failed 0 ignored 13`.
- Walk replay `8711507745385976768`, fight replay `5247080599556612730`.
- `SAVE_SCHEMA` 7, `PROTOCOL` 1, MCP 24 tools, schema proof 94/142, dev branches 13.
- Base pack tuple `(4, 4, 3, 24, 16, 18, 3, 36, 7)`.

## Process
- Every cargo and gate run: `DEVELOPER_DIR=/Library/Developer/CommandLineTools`; `cargo fmt --all` first.
- Gate: `scripts/verify.sh > <scratchpad>/gate.txt 2>&1` in the background (5–10 min); no edits while it runs.
- Sentrux: `git add` new files, scan `/Users/john/code/omnis/crates`, then `check_rules`.
- Mutation runs: `python3 -u <scratchpad>/mutate.py …> log`, never under `timeout` or `| tail`; grep for leftovers.

## Carry-over
- Not built: nothing answers `SpellCast`/`EnemyCasts`; `EnemyFlees`/`OwnTurn` have no source; region catch-up
  waits for M10.
- Large files: `plan.rs` 978, `bake.rs` 934, `screen.rs` 898, `loader.rs` 805, `tactics_panel.rs` 778.
- Dated: Socket re-audit of `rhai` 1.26.1 due 2026-10-10.
