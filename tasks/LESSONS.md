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
- **Rule** (2026-10-03): "Primary" is two things on macOS. One is the display the owner works on. The other is the *main display* (the menu bar), which can pace frames. Ask for both, with each display's refresh rate and its scaling (a 4K panel can show "1920×1080"). System Settings screenshots work when `system_profiler` lists no displays from the agent's shell.
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

## 2026-10-02 — An overlay is judged over the busiest frame, and a destructive button stands apart
- **What happened**: The 3D viewer's first button pad had translucent backgrounds and put Save log and Quit in the movement pad, one row under Back. My checks were offscreen captures where no line happened to cross a label. The owner's first play: "buttons need some work". In their screenshot a floor line ran through Save log and Quit, and a slip off Back could quit.
- **Rule**: UI drawn over a moving 3D or line-art view gets opaque backgrounds unless translucency is asked for. The visual check uses a frame where scene lines pass behind the overlay.
- **Rule**: A destructive or session-ending action (quit, restart, delete) never shares a group with frequent actions. It goes in its own group, away from where the hand rests.

## 2026-10-03 — A mode that takes the pointer shows its way out, and the click that entered it leaves it
- **What happened**: The 3D viewer locked and hid the pointer on any left click in the view. Only Esc released it, and nothing on screen said so. The owner was trapped in the window and got out with Space, they think by accident. Headless tests could not see this, because the grab needs a window.
- **Rule**: Any mode that captures input (pointer lock, a modal, full-screen capture) is left the way it was entered: a second click leaves a click-entered mode. It also shows how to leave while it is on.
- **Rule**: The decision behind a window-only behaviour is a pure function with unit tests (`movement::look`), so the part the headless suite cannot reach is as small as possible.

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

## 2026-10-11 — A defect is asked about in what the player sees, not in field names
- **What happened**: Planning P6, I asked when to fix "the fight's SpellView.cost still uses point_cost()". The
  owner answered "Describe this problem again please. Magic is supposed to have a point cost." The question read
  as if spells should lose their cost; the defect was that the price shown could differ from the price charged.
- **Rule**: When a question to the owner turns on a defect, state it as shown versus actual for the player first
  ("the fight shows 1 pt, the cast takes 2"), say when it happens and when it does not, and put the code names
  after that.

## 2026-10-11 — A capture checks the screen against a full HUD, and a key-opened screen against mouse look
- **What happened**: P6's captures showed Camp, the sheet and Tactics with "no overlap", but the HUD had no
  recent lines in them; in play, ten lines ran under the panel and drew over it. A screen opened by R while the
  mouse looked left the cursor locked, because the grab system was gated off by the very pause the screen set.
  The test shell had no render plugin, so nothing ran the grab.
- **Rule**: A new screen's capture is taken with the HUD's recent lines filled and long. A new way to open a
  screen (a key, a button) is tested with mouse look on. A system the shell gates on `playing` is checked for
  whether it must also run while a screen holds the game.

## 2026-10-16 — A layout constant sized for one set of buttons goes stale when the set grows; outcomes of chosen acts stand front and centre
- **What happened**: The explore message line reserved room for two right-hand buttons (`message.rs` `RIGHT`);
  Camp (P6) and Level (step 4b) joined them and nobody re-measured, so long lines ran under four buttons. The
  owner, hand-testing 2b's verbs, found a plaque's words cut off, and asked that the result of anything started
  on purpose stand centred where the actions panel stands, be repeated in the log, and fade as the party walks
  away from where it was done (owner, 2026-10-16: only the result fades; gone by 1.5 cells; the bottom line
  retired).
- **Rule**: A node placed beside another group is placed from that group's own measure, or a test checks the two
  never overlap at the sizes the window allows; when a group gains a button, grep for the constants that sized it.
  A result the player asked for is shown where they were looking (the screens' centre), not in a band at the edge.

## 2026-10-16 — A command the player starts always answers, even when it changed nothing
- **What happened**: 2b's Search said only "{name} searches: Investigation n". The engine's `Searched` event
  reports the roll, and a find follows as `Noticed`; with nothing hidden there, nothing else followed. The owner,
  hand-testing, searched a cell with a pile of arrows and saw no word of what was there.
- **Rule**: For each command the player can start, list what its outcome lines say in the empty case ("nothing
  hidden here") and what the player would expect it to mention (what lies in the cell), and test both. A line
  that reports only the dice is not an outcome.

## 2026-10-17 — A new panel must be checked against every node already on that part of the screen
- **What happened**: The outcome notice (2026-10-16) was placed where the screens stand, but the HUD's running
  log, a single unbounded text at the top left, ran under it when a line was long. The owner found the overlap
  by hand, and asked that the log be made less prominent now that the notice carries the important words. The
  lesson of 2026-10-16 (place a node from the other group's measure) was written about the bottom line and not
  applied to the log beside the new notice.
- **Rule**: When a node is added to the screen, list the nodes already in its band (top left, centre, right
  column, bottom) and give each the other's measure, or derive both from one constant with a test of the
  relation. Unbounded text never shares a band with a panel.

## 2026-10-17 — A fade is sized to the step that ends it, and anything that stands over the view can be put away
- **What happened**: The outcome notice was whole for half a cell and gone at a cell and a half, so it outlasted
  the step away from where it was done; and it took no clicks, so a long one could only be walked off. The owner,
  hand-testing, asked that it fade from the first movement, be gone by the cell's edge, and carry a close button.
- **Rule**: A transient's lifetime is stated in the player's units (one step, the cell's edge), not in round
  numbers, and checked against the motion that ends it. A panel over the view that holds the player's attention
  has a close control from its first build.


## 2026-10-17 — When a request will not fit the save, question the save before offering a way to fit it
- **What happened**: The owner asked that the full log be kept in the save. The save is the engine's whole
  `World`, so I offered two ways to put the log there (worded lines in a journal field, or the events). The
  owner stopped me: the save was over-coupled. It should centre on the characters and their interactions,
  replayed against a world the simulation owns (PRD D28).
- **Rule**: When a feature has no natural home in a core structure (the save, a view, a crate), say that before
  proposing where to wedge it. Name the coupling ("the save is the whole world, so a client note must become
  engine state"), and ask whether the structure is right. Only then offer the options inside it.

## 2026-10-17 — A rebuild must carry over what the player holds: the keyboard, not only the words
- **What happened**: The owner emptied the debug panel's Value field to type a new number and could not: the
  cursor went away. An empty field dims its Set button ("type a number"), a dim button is part of a screen's
  shape, and a change of shape rebuilds the panel. Every text field was spawned with `AutoFocus`, so the
  rebuild handed the keyboard to whichever field spawned last. The bug was in every screen with a field and a
  button that dims on it; the debug page was where it was met. A headless test could not press Backspace (it
  needs the text's layout, which has no fonts there), so the test empties the field as the backspaces would.
- **Rule**: When a screen is rebuilt from scratch, list the state the player holds in its nodes (focus, cursor,
  scroll, a half-typed word) and carry each over, or rebuild in place. Test it the way the player meets it:
  focus, empty, type. Carrying focus must give way when the new screen lacks that field, so a move to the next
  screen still focuses its own field.

## 2026-10-17 — An acceptance step is checked against the fixture it names
- **What happened**: 7c's script told the owner to "pull the lever" in the yard. The yard's lever is placed
  `hidden: Some(12)` and must be found by a search first, so the owner found no lever to pull. I wrote the
  step from memory of the lever's interaction, not from the map file.
- **Rule**: For each step of an acceptance script that names a thing in a fixture (a lever, a door, a
  monster), read its placement in the map file and write what stands in the way: hidden, locked, on another
  level, needing an item. Where a step depends on a roll, say so and give a sure way round (the debug panel).

## 2026-10-17 — Ask what a cost buys before defending it, and check budgets against the genre
- **What happened**: The owner asked whether visited squares could be deduplicated in the journal. I answered
  that every step had to be kept because a step passes time and may draw an encounter roll, and shrank only the
  encoding. Then I set the fold rule from a one-second load. The owner: CRPGs load in 4–5 s, and movement does
  not change the world besides eating time, so a save should capture interactions. Reading `apply.rs` showed
  that everything a step changes is the party's own state (position, clocks, automap, its RNG streams), apart
  from the interactions it triggers. The "every step is needed" claim held only because the save re-derived
  party state by replay, which was a choice, not a fact.
- **Rule**: When defending a cost ("this must be kept"), list what the kept thing actually changes and whose
  state that is, and say what design choice makes it necessary. A budget (load time, file size, frame time)
  is set against what players of the genre accept, stated with its source, and offered to the owner as a
  number to confirm, never assumed.

## 2026-10-18 — A writer checks every file against the limit its reader enforces
- **What happened**: 7f gave the character save its own limit and checked it in `save_files`. The 7g doc
  audit found that the same function wrote world bases and archive segments with no check, though the hosts
  read them with `read_under` and refuse anything over `MAX_FILE_BYTES`. A large enough fold would have
  written a save that could not load.
- **Rule**: When a reader caps a file, its writer refuses the same cap for every file it produces, not only
  the one under discussion. Test each kind of file the writer emits, not just one.

## 2026-10-18 — A mutation runner restores a file by rewriting it, never by moving a backup back
- **What happened**: the mutation runners copied each source to `.bak`, wrote the mutant, built, then
  `shutil.move`d the backup back. The restored file kept the backup's mtime, which is older than the mutated
  build, so cargo (mtime freshness for path crates) treated the mutated artifact as up to date. A later
  `cargo build -p omnis-cli` linked the last mutant ("version not rehashed") and the 7h1 demo failed: a
  resealed pack was still refused. The gate itself builds with other features, so it rebuilt; a plain build
  did not.
- **Rule**: restore with `open(f, 'w').write(original)` (a fresh mtime), or `touch` the file after restoring.
  After any mutation run, `touch` the mutated files before the gate or a demo. When a demo disagrees with the
  tests, suspect a stale artifact before the code.
- **Broken again 2026-10-22**: the continuity notes abbreviated this rule to "rewriting its text and `os.utime`",
  and the runner put the *old* mtime back, so the B8 gate tested the mutant and went red on the very test
  that had caught it. The mtime after a restore must be new: never `os.utime` back to the original. A
  continuity note quotes a rule's operative words or points at the lesson; it never paraphrases one.

## 2026-10-19 — Context is measured, not estimated
- **What happened**: asked to check context before 7h4, I estimated 200–250k from memory of what I had read;
  `/context` measured 297.7k, with the messages alone at 262k. Tool output (test failures, file reads, long
  diffs) costs more than it feels.
- **Rule**: when the owner asks about context, say plainly that I cannot read it and ask for `/context`, or
  give a range that leans high. Before a large sub-step, keep broad reading in subagents (the 7h3 id inventory
  by an Explore agent cost the main context one report), and cap test output with `grep`/`cut`.

## 2026-10-19 — Interface words are ASCII unless the face is known to draw them
- **What happened**: The 7h4 Load screen wrote "saved → loaded" and "…and N more". The built-in face has neither glyph, so the owner's walk showed boxes. The tests read the strings, not the glyphs, so nothing failed.
- **Rule**: Text a screen builds in code uses ASCII ("saved 0.1.1, loaded none", "and 3 more"). A new screen's test asserts its texts are ASCII. Arrows, ellipses and dashes belong in docs and comments, never in `Span`s, until a face's coverage is checked.

## 2026-10-19 — Stage by path, never a directory that holds the owner's files
- **What happened**: committing 7h5 I ran `git add -A crates docs tasks ARCHITECTURE.md`. `docs` swept in the
  owner's untracked `docs/dnd_supplemental/dnd_5e_combat_flow.md`, which must never be staged. Seen in the
  status line after the commit; the commit was unpushed, so it was amended without the file (left on disk).
- **Rule**: stage the files I changed by name (`git add <paths>`, or `git add -u` plus each new file I made).
  Never `git add -A`/`.` on `docs/`, `assets/` or the root. Read `git status --short` before `git commit`, not
  after, and stop if anything under `assets/` or `docs/dnd_supplemental/` is staged.

## 2026-10-19 — Nothing builds, and no fixture changes, while a mutation run is going
- **What happened**: during the 7h6 mutation run I edited the `chain` fixture's files and ran `cargo run -p
  omnis-cli -- pack seal`. The run mutates sources in place, so my build compiled whatever mutant was in place
  (a warning showed one), and between my edit and the reseal the fixture packs were unsealed, so any test of
  that moment failed for my reason, not the mutant's: a false "caught". The run's results were discarded and
  rerun.
- **Rule**: once a mutation runner starts, no `cargo` command and no edit to a file any of its tests read
  (sources, fixtures, packs) until it ends. Write docs and plans meanwhile. Settle fixtures before the run.

## 2026-10-21 — A text for an author is sized to how authors work, and addressed to what it is about
- **What happened**: M5.1 added one optional `notes: Option<String>` of 4 KiB to each map and level, the
  shape ARCHITECTURE §8.2 had named. The owner: campaign notes are copious, the editor is also the game
  master's desk, and each square, object and item may need its own 4 KiB; a keyed notes file is the right
  shape. The hand-written maps already showed it: their comments sit on particular cells, services and
  portals, which one text per map cannot address.
- **Rule**: when a field holds text for a person (an author, a game master, a modder), size it against how
  that person works, not against the string limit, and address it to the thing it is about (a key per
  square, object or item, in a store of its own) rather than hanging one string on the file. Before
  building a field a vision document named in passing, check the real material it must hold.

## 2026-10-21 — A gate run in the background is read before the commit, and the commit is chained on its verdict
- **What happened**: M5.2's gate ran in the background and the task runner reported "exit code 0", the
  wrapper shell's. The gate itself had exited 101 on a clippy error in a test. I committed in a separate
  command that printed the log's `exit 101` and went on to `git commit` regardless; the commit had to be
  amended. The 2026-09-19 lesson ("the gate's exit status must reach the commit") was broken by the
  background pattern, where the status sits in a file.
- **Rule**: a background gate writes `VERIFY-GREEN` and its exit code to its log; the commit command is
  `grep -q VERIFY-GREEN <log> && git commit …` (or the whole gate-and-commit chain runs as one background
  command), never a commit in a command of its own after reading the log by eye. The runner's exit code is
  the wrapper's, not the gate's.

## 2026-10-22 — A control the owner chose against the convention is accepted by stumbling into it, not only by seeking it
- **What happened**: on 2026-10-12 the owner asked for an unlimited camera pitch, a free head that turns over
  past straight up. The acceptance script had them look over the top on purpose, and they accepted it. Ten
  days later, during an unrelated acceptance, mouse look was locked, a mouse move carried the view over the
  top unnoticed, and W walked the party away from what the screen showed. The report read as a bug in the
  keys. The convention every first-person game keeps (a clamp short of vertical) exists for exactly that
  accident, and the first acceptance only tested the deliberate case.
- **Rule**: when the owner chooses a control behaviour against the genre's convention, name in the plan the
  accident the convention prevents, and put that accident in the acceptance script: arrive at the state
  without meaning to, then try the ordinary keys. If the state is confusing arrived at by accident, say so
  before building, and offer the convention as the recommended option. When the owner later reports the
  accident as a bug, the answer is their earlier choice and the alternatives, not a fix to the keys.


## 2026-10-22 — A game window launched from the agent's shell never ticks; its socket hangs
- **What happened**: to look at the editor's new tools column, the game was launched windowed from the
  sandboxed shell in the background and asked for a screenshot over the dev socket. The listener accepted
  the connection (the kernel's backlog) and no reply ever came, in the editor and at the title alike; a
  quarter of an hour went on process hunting (`ps` and `pgrep` are refused in the sandbox) and on a
  suspected freeze in the new code. The control run at the title hung the same way: the process has no
  window-server session (`Connection Invalid error for service com.apple.hiservices-xpcservice`), so winit
  never runs the frame loop, and the socket's system never runs.
- **Rule**: a visual check of the real window is the owner's, through the acceptance script, or an MCP call
  on a game the owner started. From this shell, verify with the headless tests and `screen_text`; a
  windowed launch from here proves only that the binary starts without a panic. Do not debug a socket that
  gives no reply from such a launch.

## 2026-10-24 — A test that writes a framework's component must use the framework's convention
- **What happened**: the editor's pointer tests wrote `RelativeCursorPosition.normalized` as 0..1 from the
  pane's top left, and the router read it the same way. Bevy fills it from −0.5 to 0.5 about the centre, so
  in the real window every click fell off the map and the mouse did nothing; fifteen tests were green on a
  convention nobody had checked. The owner found it in the first hand test. A test through Bevy's own
  `UiPlugin`, layout and focus systems (`tests/editor_pointer.rs`, no renderer: a `Window`, a `Camera2d` and
  a stand-in for the camera's target info) went red at once.
- **Rule**: when a test writes a component the framework normally fills, read the framework's source for its
  convention first and quote it in the helper's doc. Keep one test per such path that goes through the
  framework's own systems, so the hand-written helper is checked against what the framework really writes.
- **Also**: the owner reached for the game's movement keys (WASD) in the editor. Wherever a cursor moves,
  bind the game's keys beside the arrows.

## 2026-10-24 — A mouse action must never depend on a hidden state
- **What happened**: an edge brush painted the side the pointer was near only within a thin band; anywhere
  else in the cell it painted the side the cursor *faced*, set by Q and E. The owner found doors "very
  careful about the direction" and could not remove them, and walls hard: the rule was invisible, and the
  outcome depended on a state the pointer gave no sign of.
- **Rule**: what a click will do must be visible before the click (a hover of the aim), and a pointer action
  decides from the pointer alone; keyboard state steers keyboard actions. When a report says a tool is
  "fiddly" or "direction-sensitive", look first for a hidden input in the rule.

## 2026-10-24 — A layer that rebuilds its nodes must carry their transient state across
- **What happened**: the editor's tools column is rebuilt whenever its screen changes; the eighths choice is
  part of that screen, so each step of the height rebuilt the column unscrolled and the owner had to scroll
  back to the Height row for the next step, or step blind from the keys.
- **Rule**: when a change redraws nodes from a description, the state that is not in the description (a
  panel's scroll, the field with the keyboard, a hover) is read before the despawn and given to what is built
  in its place. Check it with a test that scrolls, changes the content and reads the new panel.

## 2026-10-24 — A creation form starts valid and keeps the loader's keys
- **What happened**: the editor's Add → Encounter opened an empty form: Apply with nothing typed gave an
  encounter with no id and no stacks, which the loader refused, so the map could not be saved until the
  author had filled every field right; and an id already on the map would have replaced the other encounter
  silently, since the document places by upsert. The owner asked for a minimal default and for the
  validation to be checked at creation, "so we can be sure it saves before the editing is done".
- **Rule**: a form that creates a thing opens filled with defaults the loader takes (a free id, the one
  required row, the plain choice), so Apply at once leaves the map saveable; and the form checks the
  loader's keys itself before the store's upsert (a taken id, a cell that holds one already), refusing in
  one line with the form standing. After any Apply, say the loader's verdict at once rather than leaving it
  to the Save button's dim reason.

## 2026-10-24 — A system added to a set inherits the set's run conditions
- **What happened**: the editor's preview systems were put `in_set(VectorSet::Draw)` beside the view's, and
  `VectorSet::Draw` runs only while a game session exists. In the editor there is none, so the preview's
  camera never switched on and the picture stayed black; the owner saw "an empty viewport". The headless
  tests had not caught it because the plugin needs a renderer and was not added to the test shell.
- **Rule**: before adding a system to a set, read the set's `configure_sets` and quote its condition in a
  comment at the call; a system for a state the set excludes gets its own ordering (`after(...)`) and its
  own `run_if`. And a plugin for a window is still made to run headless with `Option` parameters and a
  `is_plugin_added` check for the parts that need the renderer, so its wiring has a test.

## 2026-10-24 — Interface strings use glyphs the interface font has
- **What happened**: "Level…" and "0‰" drew a box in the editor: the font lacks the ellipsis and the
  per-mille sign.
- **Rule**: interface strings are plain ASCII unless the glyph has been seen rendered; a unit is spelled
  out ("per mille", "eighths"), an ellipsis is three dots.

## 2026-10-24 — A field's text is not a word: a new value from the model needs a new field
- **What happened**: the RON editor's second file opened showed the first file's text (the owner's
  report); Revert file left the broken text in the field; a note's form replaced by another cell's kept
  the first cell's words. The kit rewrites a column's words in place when its shape stands, and a text
  field's content is deliberately not among them, since what is typed there is the model's already. A
  change that comes from the model (a file opened, a revert, a form replaced) left the field as it was,
  with the model holding other text: a keystroke would have carried the first file's text into the second.
- **Rule**: a field's text flows one way, from the field to the model, while the field stands. Whenever
  the model sets a field's text itself, it marks the thing `fresh` and the layer rebuilds that column
  whatever the shape. Any new path that sets a field's text in the model (open, revert, reset, a form
  replaced) sets `fresh`, and its test reads the drawn field's `value()`, not the model.

## 2026-10-24 — "Changed" is not "typed": a framework system marks a component changed every frame
- **What happened**: in the window, Bevy's text layout takes every `EditableText` mutably every frame, so
  every field reads as changed every frame. The editor's route read any changed field into the model when
  its text differed; in the frame a file was picked, the screen router had set the model to the second
  file and the route then typed the old field's text (the first file's) into it. The owner saw the first
  file's text under the second file's name, and Revert "revealed" the second file. Headless nothing touches
  a field without an edit, so the tests passed, and a first fix (rebuilding the column) did not help.
- **Rule**: a change filter on a component a framework system also writes is not a signal of an edit; the
  route decides from the model's state (here `fresh`: the drawn field is stale until the rebuild) and never
  from "it changed". A test of such a route runs under a churn system that marks the components changed
  every frame (`common::app::churn_fields`), so the headless test meets the window's condition.

## 2026-10-24 — A guard that dims a whole list reads as a broken list
- **What happened**: the first file picker dimmed every other file while the open one had unsaved text,
  the reason appended to each label. In the window that looked like a list that ignored clicks; the owner
  asked for the check at the Open file... button instead: is a file loaded, does it need saving, then clear
  the editor and list.
- **Rule**: a decision the owner must make belongs at the action that needs it, asked once with the ways
  out (Save and continue, Discard changes, Cancel), not spread over the choices as dim reasons. Dimming
  with a reason suits one control that cannot act now; it does not suit a whole list.

## 2026-10-25 — A mode is state the opening action must set, not state it inherits
- **What happened**: the editor's centre shows the plan or a file's text by a `mode` flag. `Editor::open`
  reset the cursor, the view, the brush, the stroke, the form and the message, and left the mode alone: a
  map opened from the tools while a file was showing stayed on the text ("I don't get the map editing view
  in the central column"). The field was added for the file view and the older action was never revisited.
- **Rule**: when a flag that chooses what a pane shows is added, every action that opens something into
  that pane is read again and sets the flag it needs; the one action that restores (reopen after a save or
  revert) saves and restores it with the rest of the carried state. A test opens the thing from the other
  mode, not only from the default one.

## 2026-10-25 — A layout exception for one row is written as an exception, not a new default
- **What happened**: the document's row needs `min_height: 0` to take the panel's remaining height and
  scroll inside it. The change set `min_height: 0` on every row, since one line was simpler than a
  branch. Every other row then shrank into its neighbours once a column had more rows than height
  ("button overlap ... in the left column"); the headless tests had no crowded column laid out.
- **Rule**: a property set for one kind of row stays under that kind's branch, the others keeping the
  framework's default (`Val::Auto`, the content's height). A change to shared layout code runs against
  a crowded panel under Bevy's layout (`ui_editor_app`: the tools column overflows at the test window's
  height), asserting every row is at least as tall as its tallest control.


## 2026-10-25 — A literal about a fixture the owner edits through the product is a test that breaks on use
- **What happened**: four tests pinned the test pack's next seal as the literal `"0.1.2"`, one expected an
  empty notes store. The owner saved the town through the editor and committed it (0.1.3, a note added):
  eight tests went red on a tree the owner had every right to produce, and the gate could not run green
  until M5.10 reconciled it.
- **Rule**: a fixture the product itself can change (a pack the editor saves, a store it writes) is read
  by the test, never pinned: the version expected after a save is the manifest's patch one higher
  (`seal::status`), a count is the shipped count plus the test's own, a key is asserted present, not the
  only one. A literal is for what only a commit can move (a schema, a protocol, a golden fingerprint), and
  those are rebaselined in the same commit as the change.

## 2026-10-25 — A tool over "what is under the pointer" enumerates what the plan draws there, not one list
- **What happened**: the Move tool took `here_at(cell)`, the inspector's placements, as the eraser does. The
  map's start is drawn on the plan and listed by the inspector, but it is a field of the map, not a
  placement, and so not in that list: a press on the start marker panned the plan ("I couldn't move the start
  marker"). The tests moved a portal and a site and never pressed the start.
- **Rule**: a tool that acts on what is under the pointer is written from what the plan draws on a cell
  (`plan::Source`: the start, the marks, the cursor, the edges) and what the inspector lists there, with one
  arm per kind, even the kind that has no key in the placements list; its test presses each drawn kind once.
  Where kinds share a cell, the tool takes them in the inspector's order, so the owner can predict it.

## 2026-10-25 — A window report is reproduced in the window's terms before the fix is called done
- **What happened**: "I couldn't move the start marker." The code showed a plain cause (Move took the
  placements list, the start is not in it), the fix was built with a red headless test and reported as done.
  The owner's second run: still not working, while the services move. The headless test, a test through
  Bevy's real UI path and the binary's build time all agree the fix is in; what the window does is still
  unknown, since the first report did not say whether the plan panned or nothing happened.
- **Rule**: a report from the window names an observation, not a cause. Before building, read back what the
  owner should see under the suspected cause (here: the plan pans) and, when the report does not say, ask
  that one question or read the running game over the dev socket (`editor_status`, `screen_text`) while the
  owner holds it open. A fix for a window report is "done" when the owner's run agrees, not when the test is
  green; say "built, please try" rather than "fixed".

## 2026-10-25 — An edit that changes a key is a removal and a placement, not a put
- **What happened**: a placement's form (M5.6) made its Apply as `Document::place` of the request, which
  puts by key. Renaming an encounter's id in its form left the old entry beside the new one; the loader
  then refused the two on one tile. Nothing had tested a rename for the count on the cell, only for the
  new id's presence. Found by the object stamp's test (step 3), which renamed an encounter to take an id.
- **Rule**: an edit of something keyed (an id, a tile) is made as `[Remove(old key), new]` in one step
  whenever the request's key differs from the one the form opened on (`Place::key`), and a test that
  renames or moves asserts the count where it was and where it is, then the loader's verdict.

## 2026-10-31 — A choice over pack content is a picker, not a stepper
- **What happened**: step 3 gave each stamp its detail as a left/right choice under the palette, with a
  modal picker noted as "later, when the list is long". The owner's first hand test rejected it: the ids
  overflowed the 330 px column ("ter < test:monster:bob_the_rat_king") and stepping one by one through
  monsters would not scale past placeholder content. The stepper was built knowing the lists grow with
  the packs, and the owner had already asked for pickers as modals (2026-10-24).
- **Rule**: a control whose options are the packs' content (monsters, items, maps, text keys, services)
  is a picker modal from the first commit (search, several at once where the data takes several, Finish,
  Cancel); a choice box is for a short set the code names (facings, dispositions, eighths). A row of the
  column that would show an id is read against the column's width before it is drawn: the id's last
  segment, or a summary line of its own.
- **Also**: two controls under one id break the layer's by-id word sync (`LabelOf`: the first control
  found gives its words to both); a row that wants two entry points for one action gets two acts.

## 2026-10-31 — A "you will see" in an acceptance doc needs a test behind it
- **What happened**: step 4's acceptance doc and its report promised the lasso's loop "outlined in yellow as
  you draw". Nothing drew it: `Editor::preview()` returned None unless a run tool was held, and the loop in
  progress (`Editor.lasso`) reached the plan only once it had become a selection. The tests pinned the
  selection's outline and a cell run's preview, not the loop's. The owner's hand test found it in a minute:
  "there is no indicator of the lasso shape you are drawing".
- **Rule**: a "You will see" line in an acceptance doc is written from a test that asserts it (the
  `preview()`, `Source` or screen text it names), or it is struck before the report. Every transient the
  model holds for the pointer (a loop, a run, a carried drag) has a test that reads what the plan paints
  while it is held, not only what it leaves behind. The report describes what was asserted, not what was
  meant.

## 2026-10-31 — The test pack is the owner's canvas too; pins name the owner's content
- **What happened**: the owner's hand tests of steps 4 and 5 saved onto `packs/test` itself, not the scratch
  copy, and the second time committed and pushed the pack. Eight tests leaned on the fixture as it was: the
  golden replays (the packs' fingerprint is in them), an encounter count, and five checks assuming every
  shipped map is flat and carries nothing (sim and client).
- **Rule**: a change to `packs/test` by the owner is content, kept as they left it; the tests follow in a
  commit of their own: `cargo test -p omnis-sim rebaseline -- --ignored` for the replays, and a pin that
  assumed a shape of the fixture (flat, empty, a count) is rewritten to name the owner's content or to run on
  the maps the assumption still holds for (`tests/common/flat_maps`), never loosened to nothing. A new pin on
  the fixture's shape is written against the content, not the assumption that the fixture stays still.

## 2026-11-01 — A button's dim reason is a phrase; the words live where text lives
- **What happened**: Save and Playtest dimmed with the loader's first error quoted in full ("1 error: data/maps/
  dungeon.ron: portal at (0, 5) has no way back: no portal behind its landing ..."). Step 1 of the entrances
  enhancement made the errors long, and the owner's hand test read the column as the Save button gone and the
  Playtest option lost: "I seem to have lost the 'test play' option", "the save file seems overlapped by the
  red error text". The same words stood three times in one column (both buttons and the red message), and a
  fourth in the inspector.
- **Rule**: a control's `why_not` is a phrase the column's width holds in a line or two ("1 error", "no
  changes", "open a map first"); it never quotes a message. The words of a verdict are shown once, where text
  lives: the inspector's rows for what stands, the message for what just happened. When a reason would quote
  something, write the count or the kind and point nowhere (the layout tells where the words are). A test that
  pins a reason compares it whole (`assert_eq!` on the phrase), not `contains("error")`: the loose pin let the
  playtest test pass on the wrong branch after step 1 added the start rule.

## 2026-11-01 — A check that stands on another's soundness runs only on a sound world
- **What happened**: reachability (every world map reached through portal pairs) ran whatever else the loader
  had found. With pairs derived from links, one link to nothing loaded broke its pair, and the loader listed every
  map beyond it as unreached: four rows for one fault, in the editor's status too. The corpus pack built for the
  link faults could then never show the reachability row, since its world was never sound.
- **Rule**: a check whose input is another check's result (reachability from pairs, a landing from a link) runs
  only when that input is sound: gate it on no earlier errors, in the loader and in `MapData::resolve` alike, and
  say so in its doc comment. A corpus pack holds one fault family; a fault that hides another gets a pack of its
  own (`bad-reach` beside `bad-ways`). The test that counts a report's rows is the one that catches the cascade.

## 2026-11-07 — A press on the frame a screen closes is that screen's; the harness's press carries no mouse
- **What happened**: "the left column palette based controls don't seem to actually link anything." The model
  was airtight headless: the picker's Finish held the choice and the next click placed the linked portal. In
  the window the Finish press closed the modal in the screens' router, and the editor's router, running later
  in the same frame, saw the plan uncovered with the left button just down and the pointer over it: the stamp
  dropped its linked portal under the Finish button, spending the choice, and the owner's click placed an
  unlinked one. Three sessions of diagnosis went to the form's text field first, because the harness's
  `press_ui` sets a control's `Interaction` and sends no mouse button, so the plan's frame never saw a press.
- **Rule**: a button that went down while a screen stood over the editor belongs to that screen, whatever the
  system order (`route` keeps whether a screen stood last frame; the frame the cover lifts takes no `Down`).
  When a window report resists a headless reproduction, reproduce the window's *input* too: a modal control is
  pressed with the mouse button down and the pointer somewhere real (`real_click` in `editor_link.rs`); the
  system order the window runs is the harness's as well, so the harness can show it once the input is honest.
  Two reports with one symptom ("the link is lost") get one cause looked for before two fixes.

## 2026-11-07 — A field's text set by the model needs the field drawn anew; a shape that happens to change is no mechanism
- **What happened**: step 2b's picker wrote the chosen id into the form's link field and it showed in the window,
  so the path was taken as sound. It showed only because Unlink's dim reason flipped with the field going empty or
  not, which changed the column's shape and rebuilt it. A link changed from one id to another changed no shape: the
  drawn field kept the old id and the window's frame pushed it back into the model (found while planning the list
  under the field; the switch test in `editor_link.rs` is red without the fix).
- **Rule**: the words' sync never rewrites a drawn text field. Every model-side write to a field goes through
  `Form::set_text`, which marks the form `refill`, and the column is drawn anew for it (`shell/editor.rs`
  `refresh`), the keyboard kept. When a behaviour is seen to work, name the mechanism that makes it work; "it
  rebuilt" is not one until the condition that rebuilt it is the one meant.
