# M7 step 7: the service panel

## Context
M7 steps 0–6 are done (HEAD `17573cc`, gate 434/8). Inside a service the game window can only
leave: `PlayState::for_mode` maps `Mode::Town` to `Explore`, and the town commands are reachable
only through MCP and scripts. Step 7 of `tasks/plans/m7-town.md` gives every service a `bevy_ui`
panel on the kit, built from `omnis_sim::service_view` (step 6). Each `OfferView` already holds:
- the exact `ServiceCommand` to send;
- its price, or what a sale pays;
- the refusal the rules would give.

So the panel adds no rules. Log lines for every town and rest event already exist in
`service_text.rs` (4b, 5); this step only checks that they reach the log from the panel.

## Choices (each follows a standing rule or an existing pattern; listed for approval)
1. **Leaving asks.** The owner's rule: "entering or leaving a service from the game: always a log
   line and a confirmation". The panel's Leave button and Escape raise the existing confirmation
   ("Leave the Inn?"). Go sends `Service(Leave)`, which leaves in place; Stay returns to the panel.
2. **No walking inside the panel.**
   - The movement keys and the pad are inert in `PlayState::Service`. The pad is drawn dim, as in
     the other overlays.
   - Leave is the way out. A step out still leaves through MCP and scripts (4b's rule stands).
   - Why: the panel covers the viewport, and letters typed into the bank's amount must not walk
     or open screens.
3. **A refused offer is a dim button, with the reason on its row.**
   - It uses Feathers' `InteractionDisabled`; the reason is short and app-side, e.g. "not enough
     money" or "nothing to treat".
   - The message line shows the last `CommandRefused` in full: the refusals the view cannot
     foresee, such as a deposit larger than the purse.
   - A successful command clears the message.
4. **Bank amounts are typed in whole gold.** A number input sends `amount × 100` copper, because
   money is shown in whole gold. Any loose silver and copper stays in the purse (named as a limit).
5. **Counts are 1 per press**, as the view offers them: food, buying, selling. A count input is
   a later polish if wanted.
6. **The tool pad stays live where it makes sense.** ITEMS, SPELLS, SHEET and MENU work inside a
   service (the owner confirmed Party, Cast and Item inside a service); MAP and LOOK are dim.
   - Closing one of those overlays goes back to the panel, through `for_mode`.
   - Pause and load also come back through `for_mode`; saving at the inn already works.
7. **File name:** `feathers_service.rs` beside `feathers_creation.rs` and `feathers_confirm.rs`.
   The plan said `service_scenes.rs`; the code's own convention wins.

## Design

### Bevy-free model: new `crates/omnis-app/src/service_panel.rs`
**Ids.**
- `ServicePanelId { Offer(usize) /* index into view.offers */, Amount, Deposit, Withdraw, Leave }`.
- `ServiceLabelId { Money, Price(usize), Reason(usize), Message, Note }`.

**State.** `ServiceForm { amount_gp: u32, message: String }`, a resource reset on entering a
service.

**`apply(id, payload, view, form) -> Option<ServiceAsk>`.** `ServiceAsk` is
`Send(ServiceCommand)` or `Leave`. Widgets are not trusted:
- an `Offer(i)` out of range, or one with a refusal, gives `None`;
- `Amount` clamps to `0..=u32::MAX / 100`;
- Deposit and Withdraw at 0 give `None`.

**Text.**
- `rows(view, world, data) -> Vec<RowText>`: the label, the price (`text::coins`) or what a sale
  pays, and the button caption.
  - Smith: item names from the stock and the stores.
  - Temple: one row per member (name and HP), with Heal, Cure and Raise.
- `money_line(view)`: "Gold 16 gp 4 sp 5 cp · Bank 2 gp · Food 10".
- `reason(&Rejection) -> String`: the short reasons.
- Trainer and Guild: a note that they open in M7b.

**`shape(view, names) -> u64`:**
- what it covers: the kind, each offer's command, row and member, and the row names;
- what it leaves out: gold, prices and refusals, which are synced in place;
- why: a buy or a sale that changes the stores respawns the panel, while a price change only
  rewrites text, so focus survives.

### Scenes and systems: new `crates/omnis-app/src/feathers_service.rs`
The pattern of `feathers_creation.rs`.

**The panel**, `panel()` over the viewport:
- the title (the service's name) and the money line;
- a body by kind:
  - Inn and Tavern: the offer rows;
  - Smith: two columns, "For sale" and "Your stores", each a `ScrollArea` (22 stock rows);
  - Temple: member rows with three buttons each;
  - Bank: a number input with Deposit and Withdraw;
  - Trainer and Guild: the note;
- the message line, then Leave.

**Systems.**
- `reconcile`: spawn or despawn by `Active::Service` and `shape`; `UiScreen::Service`.
- `sync`:
  - when it runs: when `SimWorld` or the form changed, or after a spawn;
  - what it writes: money, prices, reasons and the message, plus `InteractionDisabled` on the
    refused offers;
  - the amount input is written through `UpdateNumberInput`, but only when it is not focused.
- `reports` (`UiSet::Dispatch`): `apply`, then
  - `Send` goes out as `PlayerCommand(Command::Service(..))`;
  - `Leave` goes through the input gate, so it asks.
- `refusals`: `CommandRefused` while in Service sets the message; a `SimEvent` clears it.

**Kit, `ui_kit.rs`:**
- `UiScreen::Service`, `UiId::Service`, `UiLabel::Service`, with their `From` and `name` arms;
- a `button_shown(id, caption: UiLabel, variant)` scene for captions with prices.

### Wiring
- **`sim.rs`:** `PlayState::Service`; `for_mode(Mode::Town(_))` is `Service`.
- **`menus.rs`:** `Active::Service`, plus its arms in `screen()`, `click_keys` and the key
  routing (the panel takes its own).
- **`combat.rs::follow_mode`:** `Service` joins the states that follow the mode. Explore becomes
  Service after a step in, and Service becomes Explore after Leave.
- **`input.rs`:**
  - `confirm_panel::ask` also covers `Command::Service(Leave)` while in `Mode::Town`;
  - `Gate` becomes `pub(crate)`;
  - `settle_answers` returns to `for_mode(world.mode)` instead of `Explore`, so Stay on a leave
    goes back to the panel;
  - a `service_keys` system: Escape sends Leave through the gate.
- **`ui.rs`:**
  - `menu_for`: `(Active::Service, _) => (Menu::None, HELP_SERVICE)`;
  - `tool_states` and the `has_casts`/`has_look` computations treat Service as in choice 6.
- **`feathers_ui.rs`:** register `reconcile` and `sync` (`UiSet::Model`), and `reports` and
  `refusals` (`UiSet::Dispatch`).

## Tests (each checked to fail when its rule is broken)
- **Unit tests in `service_panel.rs`:**
  - each control maps to its offer's command;
  - a refused or out-of-range offer sends nothing;
  - the amount clamps, and a deposit is `gp × 100`;
  - `reason` wording;
  - the shape moves with the stores, but not with gold.
- **New `tests/service.rs`**, on `feathers_app`, as `confirm.rs` does:
  - **Census per kind:** at each of the seven services the state is `Service`, the title and
    money line show, there is one control per offer, and the trainer and guild note shows.
  - **By event:** Room, Rumor, food, Buy, Sell, Heal, Deposit and Withdraw each change the world
    as the view promised, and a log line appears.
  - **By pointer:**
    - a Buy click lowers gold by its shown price and adds a stores row (a respawn);
    - a dim, refused button click does nothing;
    - a Withdraw beyond the bank shows the `BankShort` message, and the next success clears it.
  - **Leave:**
    - by button and by Escape, the confirmation shows "Leave the Smith?";
    - Stay returns to `Service`;
    - Go gives `Explore` on the site tile, with "The party leaves the Smith" in the log.
  - **Overlays:** the Items tool opens the inventory, and closing it returns to `Service`; MENU
    pauses, and resuming returns to `Service`.
  - **Layout:** smith, temple and bank at 1280×720 and 5120×1440, with no `layout_faults`.
  - **Text tree:** `screen_text` is "Service panel\n…"; trees dumped under `OMNIS_DUMP_SCREENS`
    as `service_<kind>.txt`.
- **Existing `tests/confirm.rs` changes.** After Go into the inn the state is `Service`, not
  `Explore`. The step-out test leaves by Escape and the Leave button, because arrows are inert
  inside. Named as an intended change.
- **Pins do not move:** replays, tuple, `SAVE_SCHEMA`, MCP counts, proof (no sim, pack or `World`
  change). If one moves, stop and report.

## Files
- **New:** `src/service_panel.rs`, `src/feathers_service.rs`, `tests/service.rs`.
- **Edited:** `src/{ui_kit, sim, menus, combat, input, confirm_panel, ui, feathers_ui, lib}.rs`,
  and `tests/confirm.rs`.
- **Docs:** `tasks/TODO.md` step 7; `code-map.md`; `verification.md` (count and caps);
  `tasks/plans/m7-step7.md` (this plan); CONTINUITY.

## Verification
- `scripts/verify.sh > log 2>&1`, unpiped, then read the status.
- Sentrux `scan` and `check_rules` before the commit, then `rescan` and `check_rules` after it
  (new files).
- Watch the caps: `menus.rs` is 496, `ui.rs` 680, `input.rs` 230; the new files stay under about
  500 each.
- Report what you will see:
  - Walk into any shop and confirm: a panel replaces the view.
  - Buy, sell, eat, get treated, bank, take a room; the log shows each line.
  - Leave asks first.
- Report what does not work yet:
  - trainer and guild (M7b);
  - counts above 1;
  - camp (step 8).
- No pictures from an agent-launched window: the text trees stand in.
