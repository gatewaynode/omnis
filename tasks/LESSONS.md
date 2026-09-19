# Lessons

## 2026-09-11 — Non-goals must separate mechanics from information
- **What happened**: I wrote "free look" as a PRD non-goal. The owner intends remote sensing (spyglass, scouting skill, divination spells) that reveals distant tiles into the automap, which my wording excluded.
- **Rule**: When writing a non-goal about a control or camera mechanic, state only the mechanic being excluded (movement model, viewport behavior). Do not let it imply exclusion of the information the player can obtain by other systems. Ask "what does the player learn, and how" separately from "how does the player move and look".
- **Rule**: In a systemic game PRD, treat player knowledge (automap, fog of war, rumors) as a first-class system with its own section, not as a side effect of movement.

## 2026-09-11 — A "v1" choice is a horizon, not a permanent exclusion
- **What happened**: The owner chose "regional simulation, coarse tick" for v1 over "agent-level simulation". I wrote agent-level simulation as a flat non-goal. The owner intends free agency for a limited set of important NPCs as a long-term goal.
- **Rule**: When a decision was posed as "what is in v1", record the unchosen options as deferred with a horizon, and ask which are rejected outright. Only owner-stated rejections become non-goals.
- **Rule**: For every deferred capability, add a forward-compatibility requirement to the relevant v1 section so the v1 design does not preclude it.

## 2026-09-11 — Offer both directions of a hybrid
- **What happened**: I offered "SRD content on MM2 structure" as the only hybrid option. The owner chose it, then inverted it after seeing the draft: SRD structure with MM2 adaptations limited to what serves the crawl loop.
- **Rule**: When two systems could be combined, present both directions as separate options with what each keeps and gives up. A single "hybrid" option hides the real decision.
- **Rule**: When adapting a mechanic from a secondary source, require a stated reason tied to the core loop. If no reason exists, do not adopt it.

## 2026-09-12 — Verification runs after the last change, never alongside it
- **What happened**: I launched clippy and tests in the same step as moving a directory into `crates/`. The checks ran against the tree before the move, passed, and I reported M0 green. CI then failed because the workspace glob matched the new directory.
- **Rule**: The verification run that backs a "done" claim starts only after the final edit to the tree, including moves, renames, and config-only changes. Never run checks concurrently with edits.
- **Rule**: Anything that changes what the workspace contains (new directory, member list, feature flags) gets a fresh `cargo metadata` before any other check, because every cargo command depends on it.

## 2026-09-12 — Patch the file as it is, not as it was written
- **What happened**: Four patches in a row failed or misfired because `cargo fmt` had rewrapped the lines I anchored on since I last saw them, and one assertion-less run left a file half-edited.
- **Rule**: Before any text replacement, look at the current text of the region (a `sed -n` of the lines is enough). After every `cargo fmt`, treat earlier views of that file as stale.
- **Rule**: When more than a few lines change, rewrite the whole function or file rather than anchoring on formatter-controlled lines.

## 2026-09-12 — A game loop under test must keep looping
- **What happened**: The socket test sent a request, ran one frame, and waited for the reply; loopback delivery is not synchronous, so the request sometimes arrived after the frame and the test timed out. A megabyte written in one blocking call then deadlocked: the game reads only between frames, and the test could not run frames while blocked in the write.
- **Rule**: A test peer of a polled server sends, then alternates short reads with `app.update()` until the reply arrives; large payloads are written from another thread.

## 2026-09-13 — A test never reads Bevy's message buffers after an unknown number of frames
- **What happened**: The socket test asserted `PartyChanged` by reading `Messages<SimEvent>` after a second round trip. Bevy keeps a message for two frames; loopback delivery sometimes needs a second frame, so the message was gone on a slow CI runner while every local run passed.
- **Rule**: A headless test that asserts on messages collects them into a resource with a reader system ordered after the writer (`after(SimSet::Publish)`); it never reads `Messages<M>` directly.
- **Rule**: When a CI-only failure involves timing, reproduce it deterministically first (extra `app.update()` calls, a forced delay) and keep that reproduction in the test as the guard.

## 2026-09-13 — Check the last push's CI before planning the next milestone
- **What happened**: With the M3 branch pushed and its CI red, I entered plan mode for M4. The owner had to interrupt the plan to point at the failure ("a bit premature to enter planning").
- **Rule**: At a session start or after the owner reports a push, ask whether CI passed (or read the failure they paste) before planning or starting the next milestone; a red build is the first task.

## 2026-09-13 — First content is measured before the owner plays it
- **What happened**: The M4 acceptance placement was three goblins and two rats with a surprise check at the trigger. The owner's two-member party stepped onto it and the fight ended inside that step, before a single turn: the first thing they saw of combat was the defeat modal. Measured afterwards: a 94% wipe rate for two members, 7% of fights over before any turn.
- **Rule**: Before an encounter reaches the owner, measure it: wipe rate over a few hundred seeds for a two-member level-one party, fighting every turn. The first placements stay trivial (a rat or two) until the systems have been played, whatever the golden fight needs.
- **Rule**: A mechanic that can end a fight before the player's first turn (surprise at the trigger, monster turns resolved inside the trigger command) ships off by default as a rules value, and the screen that follows a fast end shows what happened.

## 2026-09-13 — A commit is gated on its checks, and an edit is proven by the diff
- **What happened**: I chained the checks and the commit with `;`, so a commit went in with a parse error the checks had just printed (amended before anything was pushed). Earlier, an edit script with relative paths ran from the wrong directory, edited nothing, and the tests "passed" on the untouched tree.
- **Rule**: The commit follows its checks with `&&`, or runs in a separate call after the check output has been read; never `;`. A commit made in the same command as its checks is proven by building HEAD.
- **Rule**: Edit scripts anchor on the repository root (`cd` first or absolute paths), and a green run after an edit counts only when `git status` shows the files changed.

## 2026-09-13 — A display target is measured on the owner's primary display
- **What happened**: The display rework sized the canvas for a 16:9 4K monitor because the PRD named one. The owner's primary display is a 5120×1440 ultrawide; the fixed 16:9 canvas left half of it empty and every windowed class fell to 1× under the menu bar. A second rework followed the same day.
- **Rule**: Before a decision about display size, scale, or aspect, ask which display is primary and read the machine (`system_profiler SPDisplaysDataType` on macOS: physical and "looks like" sizes for every panel), then measure the design on every panel listed, windowed and fullscreen, before proposing it.
- **Rule**: A layout is designed for the aspect range the hardware shows, not for one canvas: state what fills the screen on each panel and what stays empty, with numbers, so the owner decides on the bars before the code exists.

## 2026-09-19 — A deferred milestone keeps its number
- **What happened**: When the owner deferred the editor (M5), I removed the `### M5` block from the TODO and moved its content into the Phases 2–5 block, leaving a vacant number and a "renumber?" question. The owner's rule: leave the numbering alone and say in the description that the milestone is deferred, with a pointer to where it went.
- **Rule**: A whole milestone that is deferred keeps its heading and number in `tasks/TODO.md`; the heading gains "(deferred)" and its one-line body names the date, the saved plan (`tasks/plans/<name>.md`) and the block where it is now tracked. Nothing after it is renumbered.

## 2026-09-19 — The gate's exit status must reach the commit
- **What happened**: I ran the verification script through a pipe (`scripts/verify.sh | tail`) and chained the commit on the pipe's status, which is `tail`'s. The sim lint had failed on `f64` in a new test file; the commit went in anyway and was amended once noticed. Earlier the same day a test summary piped through `awk` had hidden a failed test until the script grew an exit code.
- **Rule**: The command that gates a commit is the verification script itself, never a pipeline over it: `scripts/verify.sh > log 2>&1 && git commit …`, with the log read on failure. Any summary or filter runs on a saved log after the status is known.
- **Rule**: A new file in a simulation crate, test or not, is written with integers; percentages and means print as tenths and hundredths.
