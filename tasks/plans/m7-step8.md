# M7 step 8, split: 8a the tool bar on Feathers, 8b the camp panel

## Context
M7a steps 0–7 are done (HEAD `afe3f09`, gate 444/8). Rest exists in the sim since step 5
(`Command::Rest(RestCommand::{Short { dice }, Long})`, `Mode::Explore` only) but the window cannot
reach it. Step 8 of `tasks/plans/m7-town.md` asked for a camp panel and a door measured on the
canvas tool pad. The canvas captures (3×3 at 24 px, 4×2 at 70 px) showed the pad has no room left
in its pixel grid. **Owner decision (this session):** follow D26 ("Feathers for … all user
interface moving forward, retrofit as convenient") and split the step:
- **8a** — the canvas tool pad becomes a Feathers bar, with CAMP as its seventh button (dim until 8b).
- **8b** — the camp panel, R, and hit dice on the sheet.

The movement pad (arrows and USE) stays on the canvas; moving it goes to `horizons.md`.

Toolchain: inside the sandbox `xcodebuild` cannot read the license plist, so a fresh link fails;
every cargo and gate run keeps `DEVELOPER_DIR=/Library/Developer/CommandLineTools`.

## 8a. The tool bar on Feathers (commit 1)

**What exists (measured):** `TOOLS` (968,300,304,88) holds six 96×40 buttons (`layout.rs:188–211`).
They are painted by `panels::tools` from `screen.rs:322` and hit-tested by `ui::hit`, which only
matches enabled widgets; that disabled-skip is the **only** gate, since `map_tools` (`input.rs:209`)
and `shell` (`sim.rs:280`) check nothing. States come from `ui::tool_states` (`ui.rs:394`). Nothing
paints over the right column. `ui_kit::place` puts every `PanelRoot` on the viewport. Many tests
assume a single `PanelRoot` (`layout_faults`, `text_tree`, `escape_abandons`, root counts,
`NO_PANEL`). Click tests in `pointer.rs`, `inventory.rs`, `look.rs` and `sheet.rs` run under
`MinimalPlugins` (no `bevy_ui`); only `service.rs`'s runs under `feathers_app`.

**Design.**
- **Model** (Bevy-free, `tool_bar.rs`): `ToolButton` gains `Camp`, which is dim in 8a. `ToolButton`
  and `ToolStates` move out of `widget.rs` (`[PadState; 7]`). `tool_for` stays in `input.rs`.
- **One gate in logic, not in the widget:**
  - A `ToolStates` resource is computed each frame by a system from `tool_states`.
  - A new message `ToolPressed(ToolButton)` replaces `UiClick`/`WidgetId::Tool`.
  - `map_tools` reads `ToolPressed` and sends `tool_for(b)` only when the resource says Enabled.
  - So a dim button is refused by the rules, whatever the widget does (defensive; today's
    canvas-only gate disappears with the canvas buttons).
- **Scenes** (`feathers_tools.rs`):
  - Root: a `ToolBar` marker of its own, **not** a `PanelRoot`. Modal-panel code (Escape,
    `screen_text`, single-root helpers) is untouched by it.
  - Layout: two rows, ITEMS SPELLS SHEET CAMP / LOOK MAP MENU. Each row is a flex row of
    `ui_kit::button`s with `Control(UiId::Tool(b))`, so the kit's observers publish `UiReport`;
    `reports` turns them into `ToolPressed`.
  - Lifecycle: spawned while a world exists in `AppState::Playing`, despawned otherwise (Hidden
    today).
  - `sync` puts `InteractionDisabled` on each button per `ToolStates`.
  - Labels are the same words in title case (Items, Spells, …).
- **Placement:** `ui_kit::place` becomes target-aware:
  - `PanelRoot` keeps the viewport;
  - `ToolBar` goes to `TOOLS` shifted by `layout.core`, so on the wide canvas it lands at
    (1768,300).
  - The same `canvas_rect_to_window` / `UiScale` path keeps it in step with the letterbox and
    integer scale.
- **Fonts:** `feathers_fonts::wear` also dresses text under `ToolBar`.
- **Pointer:** the bar's nodes are hovered nodes, so the existing `capture_pointer` already keeps
  clicks off the canvas beneath it.
- **Canvas removed:**
  - `panels::tools` and its call in `screen.rs`, `TOOL_BUTTONS`, `tool_button`, `WidgetId::Tool`,
    and the `pressed` handling for tools in `ui::hit`.
  - `TOOLS` stays as the reserved region (the layout tests keep checking it is disjoint).
  - `View.tools` and `dump_screens`' tool states go too; the dumps show an empty strip where the
    bar sits.
- **Size, measured first:**
  - The box is ≈276×80 UI px at 1280×720 (UiScale 1.10) and ≈405×117 at 5120×1440.
  - Four buttons a row leave ≈66 UI px per button at 1×.
  - `layout_faults` (made to check one root's descendants, with the bar as a second root it can
    be pointed at) runs over 1280×720 and 5120×1440, each at the fitted scale and at `scale_cap`.
  - If a label faults at the cap, the bar's text steps down a size, as the measurement says, and
    no lower than legible. The numbers go in the commit.
- **Tests:**
  - New `tests/tool_bar.rs` (`feathers_app`):
    - every enabled button by pointer opens its screen;
    - a dim button by pointer and a forged `ToolPressed` both do nothing;
    - states follow the screen (explore, service, fight, creation);
    - no faults at both sizes;
    - the bar sits on `TOOLS` at both sizes;
    - text tree dumped (`tool_bar.txt`).
  - The `MinimalPlugins` tests keep their meaning: `widget(..).enabled` becomes the `ToolStates`
    resource; `click(.., WidgetId::Tool(b))` becomes a `common::tool(app, b)` helper that writes
    `ToolPressed`. Real pointer clicks are covered in `tool_bar.rs`.
  - Unit tests: `the_tool_pad_follows_the_screen` (`ui.rs`) and `tool_buttons_send_what_their_keys_send`
    (`input.rs`) stay; the canvas paint and fit tests in `panels.rs` and `widget.rs` go with the
    painter.
  - Rule breaks:
    - drop the state gate in `map_tools`;
    - leave Camp enabled;
    - place the bar on the viewport.

  Each must fail a named test.

## 8b. The camp panel (commits 2 and 3)

**Commit 2, the sim's camp view** (`omnis-sim`). No rest preview exists today.
- `rest.rs` splits `food_needed` into `food_need` (a count) and its check. There is no behaviour
  change.
- New `rest_view.rs`:
  - `RestView { allowed: Option<Rejection>, members: Vec<CampMember>, long_food, food, long: Option<Rejection> }`;
  - `CampMember { index, hp, hp_max, dice_left, die, spendable }`, where `spendable` is 0 if the
    member is dead or at full HP;
  - `long` is exactly `too_soon`, then the food check, so the view and the command agree.
- Tests in `tests/rest.rs`:
  - each refusal the view names equals the command's;
  - `spendable` is accepted and `spendable + 1` refused;
  - the fingerprint is unchanged.
- No MCP tool; the counts, replays, tuple and `SAVE_SCHEMA` are unmoved.

**Commit 3, the app.**
- **The door:**
  - CAMP on the bar is live only in `Active::None` with members;
  - `ShellCommand::Camp`, its `tool_for` arm, and R in `shell_for` (map only; fights keep r for
    Run);
  - the help line names R.
- **The screen:**
  - `PlayState::Camp` and `Active::Camp`, with arms in `menus.rs` (:131, :207–219, :350–358) and
    `ui.rs` (:417–428, :490–520);
  - opened by `shell()`, closed to `for_mode`;
  - `combat.rs::follow_mode` gains `Camp`, so an ambush moves into the fight at once.
- **Model** `camp_panel.rs`:
  - ids `CampPanelId { Dice(usize), Short, Long, Close }` and `CampLabelId { Member(usize), Dice(usize), Food, Reason, Message }`;
  - `CampForm { dice, message }`;
  - `apply` returns `CampAsk::{Send(RestCommand), Close}`; slides are clamped to `0..=spendable`,
    and an all-zero Short gives `None`;
  - text: "Brenna  HP 5/12 · Hit dice 2/3 d10" and "Long rest: eats 2 food (stores 7)";
  - reasons come through `service_panel::reason`; plus `shape`.
- **Scenes** `feathers_camp.rs`:
  - one `FeathersSlider` per member (precedent `feathers_creation.rs:84`), disabled at 0;
  - Short rest is dim while every count is 0; Long rest is dim with its reason;
  - a message line, and Close / Escape;
  - `look`, `refusals`, `reconcile` and `sync` run in `UiSet::Model`, `reports` in
    `UiSet::Dispatch`;
  - `UiId`, `UiLabel` and `UiScreen::Camp` are new;
  - the panel stays open after a rest.
- **Sheet:** `SheetView.hit_dice: (left, total, die)` on row 4 of the stats page ("Hit dice 2/3
  d10"); `widest()` covers it.
- **Tests** `tests/camp.rs` (`feathers_app`):
  - the bar and R open it, in Explore only;
  - slide, then Short rest, spends dice and logs;
  - a second Long rest is dim with "rested too recently";
  - with no food the reason shows;
  - an ambush leaves the panel for the fight;
  - Escape and Close return to the map;
  - layout at both sizes, and a text tree (`camp.txt`);
  - plus unit tests in `camp_panel.rs`.

  Rule breaks each fail a named test.

## Verification (every commit)
- Run `DEVELOPER_DIR=/Library/Developer/CommandLineTools scripts/verify.sh > log 2>&1` and read
  the status; it must say `VERIFY-GREEN`. Run Sentrux `rescan` and `check_rules` before each
  commit, and again after one that adds files. Watch `ui.rs` (702 lines), `screen.rs` (985) and
  `menus.rs`.
- Replays, tuple, `SAVE_SCHEMA 5`, and MCP 20 / `oneOf` 11 / proof 77/111 stay unmoved.
- Docs go in each commit: `TODO.md` (step 8 is split into 8a and 8b, each with its review),
  `code-map.md` and `verification.md`. The linker note says the sandbox still needs
  `DEVELOPER_DIR` and why. `horizons.md` gets the movement pad on Feathers. `CONTINUITY.md` is
  rewritten.
- A "what you will see" line for each manual test:
  - 8a: the same seven-button bar in modern text, CAMP dim, everything else as before.
  - 8b: CAMP and R open the camp; sliders; both rests; the reasons; an ambush in the dungeon.
- The owner checks the bar on the ultrawide; agent windows draw no frames.

## Risks
- Four labels a row at the scale cap may not fit at 1×. Measured first; the fallback is a
  smaller text size for the bar.
- A Feathers slider with max 0: such rows show "no hit dice to spend" instead.
- Removing `WidgetId::Tool` touches the five integration test files listed. Their meaning is kept
  through the resource and the `tool` helper; real pointer coverage moves to `tool_bar.rs`.
