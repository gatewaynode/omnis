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

## 2026-09-19 — Every action needs a mouse entry point, and no function keys on macOS
- **What happened**: M6a shipped casting on the road behind the C key, the debug menu behind the backtick or F1, and save and load behind F5 and F9. The owner's first play: F5 is taken by macOS, the testing party could not be saved between runs, and remote sensing, items, spells and character sheets had no way in outside a fight but a key.
- **Rule**: Every player action gets a button on the screen (a pad button, a menu item, an action row) before it gets a key; the key is the shortcut, never the only door. Save and load are menu items on the pause overlay, not function keys.
- **Rule**: Function keys are not bindings on macOS (the system and the hardware take F1–F12); a shortcut is a letter, and the help line names it.
- **Rule**: A test party the owner builds by hand must survive the session: saving is the first thing to verify by mouse before a milestone is handed over.

## 2026-09-19 — The Sentrux pass runs on every commit's tree, not once per milestone
- **What happened**: After M6a's twelve commits the first Sentrux `check_rules` of the entry-points step found four violations the M6a commits had introduced (`screen::click` at cyclomatic 27, `Rejection`'s Display at 26, two base-pack tests at 198 and 123 lines). The per-task review had been skipped while the milestone was being pushed through; a refactor commit followed.
- **Rule**: `scan` and `check_rules` run before every commit, alongside the verification script, and a violation is fixed in that commit or the next one, named as debt; the awk long-function check covers tests as well as sources.

## 2026-09-19 — A dev tool is a player action too
- **What happened**: The entry-points step gave every player screen a button and unbound F1, but left the debug menu on the backtick alone. The owner accepted M6b and then could not find the debug menu.
- **Rule**: The buttons-before-keys rule covers dev tools and every other screen a person can open: when a key binding is removed or a screen is added, its button or menu item lands in the same commit, and a dim item with a notice beats a hidden one.

## 2026-09-19 — A commit made after the push is stranded when the PR merges
- **What happened**: The debug-menu pause item (`4e5973a`) was committed on `m5-tasks` after the owner had pushed the branch and opened PR #6. The PR merged the pushed state; the next branch was cut from `main`, so the commit, its tests and its LESSONS entry were in no build until a `/catchup` after the compact noticed the tree had six pause items.
- **Rule**: After the owner reports a merge, run `git log main..<old-branch>` before touching the new branch; any tail is cherry-picked first, verified, and named in CONTINUITY. A commit made after a push is noted in CONTINUITY as unpushed the moment it lands.

## 2026-09-20 — Drift closes toward the SRD, not away from it
- **What happened**: On the first drift item (PRD §8.3, action economy) I offered "change the document to one action per turn" as an equal option and recommended rewriting the reactions row to describe the automatic shield as built. The owner: "We actually need to get closer to the SRD, rather than drift away." Their own departures add to the SRD (more than one bonus action, player-composed reactions); they never subtract from it.
- **Rule**: Where the built game is simpler than the SRD, the recommendation is that the code catches up, with the milestone named; PRD §8 is not rewritten to bless a shortcut. A document change is offered only for plain facts (names, lists, versions) or for an adaptation the owner originated, and §8.2 needs a crawl-loop reason for it.
- **Rule**: An SRD rule that depends on miniature proximity is answered with a gridless mapping (rows, stacks, the lead individual) before it is called out of scope.

## 2026-09-20 — The owner's world-building documents are committed when they appear
- **What happened**: I had carried "`docs/background/*` is never staged" as a standing rule and left the owner's new `mechanics_and_rules.md` untracked; the owner committed it by hand and corrected the rule.
- **Rule**: A supporting world or game building document the owner adds is committed as soon as it shows up, unedited, in its own commit, even when its subject is a horizon far from the current work. `.claude/` stays unstaged.

## 2026-09-20 — A style word in a first draft is a claim about the owner's intent; confirm it
- **What happened**: The PRD's first draft (mine) said "deliberately retro: 2D pixel art" and D2 gave "most faithful" as a rationale. For nine days display, font and toolkit decisions (the raster canvas, the 5×7 bitmap font, integer scaling, canvas sprites over `bevy_ui`) leaned on that framing. The owner: pixel art was never the intent; Omnis is a modern tactics and strategy re-imagining, the art of the era is not what worked, and no compromise is made for a pixel-art feel (PRD D26).
- **Rule**: An aesthetic or genre adjective in a vision document ("retro", "faithful", "pixel art", "minimal") is asked as a question with its alternatives before it is written as a decision, exactly like a mechanic. A placeholder source (16×16 CC0 tiles) says nothing about the goal.
- **Rule**: When a decision is justified by preserving a look or a feel, name that justification to the owner in the recommendation, so a wrong premise surfaces at the first decision and not the tenth.

## 2026-09-20 — After a merge, look at the checkout before exploring it
- **What happened**: The owner merged PR #9 and checked out an old local `m7-tasks` (at the PR #6 merge). I launched three explorations without looking; two of them read a tree from before M6b, M6c and PRD v0.5, and reported "D26 does not exist" and a six-item pause overlay. The contradiction with my own commit is what gave it away.
- **Rule**: When the owner reports CI or a merge, run `git branch --show-current`, `git log --oneline -1` and `git log HEAD..main` before any exploration or planning; a checkout behind `main` is reported and fixed first. An exploration's claim that contradicts a commit made in the same session is checked against `git show main:<path>` before it is believed.

## 2026-09-20 — A fact in an option's description is a claim: verify it
- **What happened**: Offering party creation as the experiment's screen, I wrote that it "sits before a world exists". It does not (`PlayState::CreateParty`; the new-game form makes the world). The owner chose that option; the error made it sound harder than it was, and I corrected it in the plan.
- **Rule**: Every factual clause in an `AskUserQuestion` option is checked against the code before it is sent, exactly like a claim in a report; what is not checked is worded as unknown.


## 2026-09-27 — A step the owner can play needs a "what you will see" line
- **What happened**: M7 step 3 moved the game's start into a town of seven services, all data. The owner played it and found nothing to interact with and no visible way in or out; the report had given the tests and pins but not that services open only in step 4, nor that no portal on any map is drawn.
- **Rule**: When a commit changes what a new game shows, the report says what the owner will see and what does not work yet, with the step that makes it work.
- **Rule**: Before building on a mechanic in a new place, check that the player can perceive it (portals were invisible on every map since M1; the town made it obvious).

## 2026-09-27 — A restart must not leave a tree that cannot run
- **What happened**: Continuity for a client restart was written with step 3b's data rule (every portal needs a marker) uncommitted in the tree, before any pack had markers. The app built but refused the test pack at start; two `bad_packs` tests failed. The owner launched it, saw a crash, and could not tell whether the tests had been run.
- **Rule** (revised 2026-10-10, see below): Before a restart, compact or hand-over, the working tree either passes the gate or the unfinished work is committed as a labelled WIP commit, and the continuity note says which, with the commit.
- **Rule**: A validation rule that the shipped packs cannot yet meet lands in the same change as the data that meets it, never ahead of it.

## 2026-10-02 — A rule break counts only when a named test fails
- **What happened**: The first run of M7 step 7's fourteen rule breaks reported every one "caught" from a non-zero exit. Every build had in fact failed to link (an Xcode update left its license unaccepted), and then zsh passed `--test service` as one word; no test had run at all.
- **Rule**: A broken rule is recorded as caught only with the name of the test that failed; an exit code alone proves nothing. Run one break by hand and read its output before trusting a loop.
- **Rule**: In zsh a variable holding several arguments is split with `${=var}`.

## 2026-10-02 — A gate with nothing to link proves nothing about the linker
- **What happened**: After the owner accepted the Xcode license, the gate ran green without the `DEVELOPER_DIR` override and was reported as "green on Xcode's toolchain". Every artifact was already built, so nothing was linked. The first fresh build (a scratch worktree) failed to link: inside the sandbox `xcodebuild -find clang` cannot read Xcode's license plist, accepted or not. The claim had to be withdrawn.
- **Rule**: A toolchain fix is verified with a fresh link (a new target directory, a scratch worktree, or a touched crate that links a binary), never with a cached gate.
- **Rule**: When a sandbox blocks a file a tool reads, assume the tool behaves as if the file were missing, and test it from inside the sandbox before claiming it works there.

## 2026-10-03 — A capability in a vision document needs the owner's ask behind it
- **What happened**: ARCHITECTURE §8.1 said since its first draft (mine) that `InputPlugin` maps the gamepad to commands. No gamepad code was ever written and the PRD never names one; the M7a sync found it, and the owner: "I never mentioned any gamepad. This will definitely be a keyboard and mouse game first. Other control methods are stretch goals."
- **Rule**: A platform, device or capability (gamepad, touch, controller, network play) goes into a vision document only when the owner asked for it; a sync pass checks every such word against the code and the PRD and asks about any that neither supports.

## 2026-10-04 — An SRD rule the PRD set aside is not "missing"
- **What happened**: Writing M7c step 5's policy I found the SRD's one-spell rule absent and added it as step 4b (`25ce447`), calling it missing. PRD D24 and the §8.3 casting-time row reject exactly that rule (the owner's departure: two spells a turn within the budget). Nobody was asked; three later steps built on it. Found in step 8's planning; the owner chose to revert it.
- **Rule**: Before adding an SRD rule the code lacks, search the PRD's decisions (D-table "Rejected" column) and §8.3 for it. "SRD is the default direction" applies only where the owner has not departed; a departure the owner wrote is never undone without an ask.

## 2026-10-04 — A rule that keeps a turn open is counted in play, not only asserted
- **What happened**: M7c's turn-ending rule held a member's turn while any feature could still be spent. Tests
  asserted it, and the acceptance script even called it "by design", but Second Wind (level 1, once a rest)
  and Cunning Action (no limit) made every fighter and rogue press End nearly every round. The measurement
  policy used every feature before the action, so it never saw the chore. The owner found it in the first
  fight of acceptance c (B1).
- **Rule**: When a rule decides whether the player must press something, count how often it fires in a
  measured or scripted fight (End presses a turn, by class) before shipping, and put the number in the report.
- **Rule**: A test helper or fixture that works around a rule (here `END` pressed "for Second Wind" in two
  app tests and a replay) is a sign that players will hit the same thing; surface it to the owner.

## 2026-10-07 — A killed mutation run leaves its break in the source
- **What happened**: M8 step 8c's mutation runner ran under `timeout` piped into `tail`. The timeout killed
  Python mid-run: its buffered output was lost and its `finally` never restored the last break, so the app's
  save read was left unguarded in the working tree. Found by grepping for the break before going on.
- **Rule**: Run `mutate.py` with `python3 -u`, output to a file, never under `timeout` or `| tail`; for a long
  run use `run_in_background` and wait for its notification.
- **Rule**: After any mutation run, grep the sources for each break's new text (or `git diff` against the
  last known state) before testing, committing or editing further.

## 2026-10-10 — A new wire name is checked against the whole vocabulary first
- **What happened**: Renaming protocol 2's colliding fields, I gave `ItemCommand::Use`'s potion recipient the
  name `on`, which `SetReactions.on` and `ReactionsSwitched.on` already carry as a bool. The vocabulary test
  caught it after a 45-minute gate. I had also missed `CombatCommand::Use.target`, the same field in the fight.
- **Rule**: Before proposing a wire field name, grep the wire types for it (`grep -rn "\bname:" crates/omnis-sim/src
  crates/omnis-core/src`) and run the vocabulary test (`cargo test -p omnis-mcp --test integration vocabulary::`, seconds)
  before the full gate. A rename covers every sibling with the same meaning (the command outside a fight and in
  one, and the event it raises).

## 2026-10-10 — Unfinished work is committed, not stashed, before a restart
- **What happened**: Before the owner restarted the terminal (and pushed), I stashed P2d's ungated work, as the
  2026-09 rule said. Owner: "Don't stash, we need to capture this work." A stash is local only: the push would
  not carry it, and nothing outside this machine would hold it.
- **Rule**: Unfinished work before a restart, compact or push is committed by name as its own commit whose
  subject says `WIP` and whose body says it has not passed the full gate and what has passed. The next commit
  completes it; nothing is squashed or amended once the owner may have pushed. Never stash work the owner has
  not seen committed.
