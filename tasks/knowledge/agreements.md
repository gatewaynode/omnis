# Working agreements

Standing rules observed with the owner. `CLAUDE.md` is the owner's instruction set; this file
records the practice that grew around it. A new agreement lands here the session it is made.

## Deciding
- Numbers before opinions: compute and show the comparison in a small table before recommending.
- Present both directions of any hybrid as separate options, each with what it keeps and gives up.
- A v1 choice is a horizon, not an exclusion: record the unchosen option in `horizons.md` with the
  milestone it waits for. Only owner-stated rejections become non-goals.
- Ask before changing `PRD.md` or `ARCHITECTURE.md` on drift. The "as built" paragraphs in
  ARCHITECTURE §4.5 are approved by the milestone plan and are flagged in the commit that adds them.
- A deferred milestone keeps its number and heading in `tasks/TODO.md`, marked deferred, with a
  pointer to `tasks/plans/<name>.md` and the block where it is tracked.
- Check the last push's CI (the owner reports it; `gh` is not installed) before planning the next
  milestone. Milestones are planned in plan mode; the approved plan is the scope, and wider fixes
  are follow-ups unless a test fails on them, in which case the fix ships with a test and is named
  in the commit.
- First content is measured over seeds before the owner plays it. A display decision is measured
  on the owner's 5120×1440 ultrawide.

## The app
- Every player action, dev tools included, gets a button or menu item before a key; the key is
  the shortcut. No function-key bindings (macOS). Save and load live on the pause overlay.
- A hand-over is preceded by saving by mouse. When a key binding is removed or a screen is added,
  its button or menu item lands in the same commit; a dim item with a notice beats a hidden one.

## Code
- Simulation crates: `no_std`, integers only (tests too), `BTreeMap`/`Vec`, no Bevy, clock,
  threads or network; Rust rolls the dice, Rhai rule slots add and compare
  (`data.rules.eval(slot, &[(name, Value::Int)], rng, stream)`); events carry ids, never text;
  validate then mutate; a command's dice roll on a copy of the stream written back only on success.
- Files stay near or under 1000 lines; functions under 100 lines including tests; cyclomatic
  complexity under 25; no import cycles (a path call such as `crate::x::f()` counts as an edge).
- Dependencies: exact pins, at least 30 days old, N-1 for every non-Bevy crate and matching
  Bevy's resolved versions (Bevy itself is exempt by the owner), Socket scores read before adding.
- Any change under `packs/` or to the serialized `World` rebaselines both golden replays in the
  same commit.
- Patch a file as it is after `cargo fmt`: look at the region first, replace whole functions when
  in doubt. An edit script lives in the scratchpad, is parsed whole with every anchor checked
  before it writes, anchors on the repository root, and is rerun after an anchor fix.

## Commits
- One commit per checked TODO item; each commit builds and passes the gate on its own.
- Verification runs after the last tree change: `scripts/verify.sh > log 2>&1 && git commit …`,
  never through a pipe, the status checked. Sentrux `rescan` (or `scan` on
  `/Users/john/code/omnis/crates`) then `check_rules` before every commit; a violation is fixed
  in that commit or the next one and named as debt.
- Stage by name; never `git add -A`; `.claude/` and `docs/background/*` are never staged.
- Never push. The owner pushes and opens pull requests. After the owner reports a merge, run
  `git log main..<old-branch>` before cutting the next branch and carry any tail over first.
- Trailer on every commit:
  `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01KZPdEyj5qTbbxEBX75vBuJ`.
- `tasks/LESSONS.md` gains an entry after any correction; `tasks/CONTINUITY.md` is rewritten
  every time it is used and stays short: state, next step, pointers.
