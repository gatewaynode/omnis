# M7 step 6: ops, MCP and CLI for the town and rest

## Context
M7 steps 0–5 are done (HEAD `8fbcaa5`, gate 426/8). The owner confirmed the four 4b departures
(2026-09-27; recorded in TODO/CONTINUITY, uncommitted, goes in with this step). Step 6 of the
approved plan `tasks/plans/m7-town.md` gives agents and the coming panels one read model of a
service and of rest, and lets an agent read a `bevy_ui` panel without a PNG:
- `ops::service_view`: the one model the step-7 panel and agents both read.
- MCP tool 19 `service_get`.
- `party.get` gains bank, hit dice and the long-rest wait.
- `game.status` names the service.
- Script words for every service and rest command.
- Tool 20 `screen_text` over the `bevy_ui` text tree.

The `Rest` and `Service` schema branches and proof instances already exist (4b, 5), so the
schema proof does not move.

No owner questions: every choice below follows an existing pattern.

## Design

### 1. Service view (sim)
**Split the checks in `service.rs`, no behaviour change.**
- `validate` becomes `quote` (every check but money; returns the `Deal`), then `afford` (the
  `CannotAfford` check).
- `apply` calls both, as it does now.
- `Deal` gains `fn cost(&self)` (money out) and `fn paid(&self)` (money in, Sell).
- `quote`, `Deal` and these helpers become `pub(crate)`.

**New `crates/omnis-sim/src/service_view.rs`** (`ops.rs` is at 765 lines).
`pub fn service_view(world, data) -> Option<ServiceView>`; `None` outside `Mode::Town`, like
`combat_view`.

`ServiceView`:
- `service` (id), `name` (text key), `kind`, `gold`, `bank`, `food`;
- `offers: Vec<OfferView>`.

`OfferView`:
- `command: ServiceCommand`: exactly what to send;
- `row: Option<u8>` and `member: Option<u8>`, for the panel's labels;
- `price: Option<u32>` (what it costs) and `pays: Option<u32>` (what a sale brings);
- `refusal: Option<Rejection>`: why not, which `Display` already words.

**Offers by kind**, all at count 1:

| Kind | Offers |
|---|---|
| Inn | Room |
| Tavern | Rumor; BuyFood 1 |
| Smith | Buy per stock row; Sell per stores row |
| Temple | Heal, Cure, Raise per member |
| Bank | none (amounts are typed); gold and bank are shown |
| Trainer, Guild | none until M7b |
| Every kind | Leave |

**How an offer is priced.**
- Each offer is quoted on a fresh copy of the "town" stream that is never stored. The world is
  unchanged, streams included.
- Each price is therefore what that command would cost if sent next.
- An unaffordable offer still shows its price, with `CannotAfford` as the refusal.

**Wire.**
- `Op::ServiceGet` (`service.get`).
- `Reply::Service { service }`, placed before `Value`/`Done`.
- New `OpError::NoService`, like `NoEncounter`.

### 2. `party.get` and `game.status` (sim, `ops.rs`)
All new fields are `#[serde(default)]`, so older clients still parse.
- `MemberView`: `hit_dice: u8` (= level) and `hit_dice_left: u8` (level − spent, as `rest.rs`
  computes it).
- `PartyView`:
  - `bank: u32`;
  - `last_long_rest: Option<i64>`;
  - `long_rest_wait: i64`: the minutes until a long rest is allowed, 0 when ready, from
    `rest::too_soon`. The step-8 camp panel shows it.
- `Status`: `service: Option<String>` (the service id while in `Mode::Town`). `save.read` returns
  `Status` too and gains it for free.

### 3. Script words (sim, `command.rs`)
`from_word` parses these new words, following the `attack-N` style:

| Word | Command |
|---|---|
| `food-N` | BuyFood N |
| `heal-M`, `cure-M`, `raise-M` | Heal, Cure, Raise member M |
| `buy-R`, `buy-R-N` | Buy row R (count N) |
| `sell-R`, `sell-R-N` | Sell row R (count N) |
| `deposit-N`, `withdraw-N` | Deposit, Withdraw N copper |
| `short-rest-A-B-…` | Short rest; dice per member |

- `word()` stays `&'static str` and logs the bare verb (`food`, `heal`, `buy`, …) instead of
  `service`, as combat logs `attack`.
- The parsing goes in a helper `parse_town(word)`, to keep `from_word` under the complexity cap.

### 4. `screen_text` (app, tool 20)
**One walker, in the library.**
- New `crates/omnis-app/src/ui_text.rs`: `pub fn text_tree(world: &World) -> String`, moved from
  `tests/common/feathers.rs` (`tree_line`, same line format).
- It covers every `PanelRoot`, not exactly one. Each root is headed by its `UiScreen`.
- With no panel open it returns `no panel is open` (the canvas screens are not `bevy_ui`).
- The test helper becomes a thin call to it.
- `creation_feathers.txt` must stay byte-identical below its heading line. This proves the move
  changed nothing.

**Socket path.**
- `serve` holds no `&World`, so `screen.text` is answered one step late, like `screenshot`.
- `serve` queues the request id; an exclusive system `answer_screen_text` after `serve` builds the
  text and writes the reply.
- `Op::ScreenText` (`screen.text`) is a host op.
- Headless refuses it: "screen text needs the game window; this is headless", as `screenshot`
  does.
- The bridge returns it as plain text, like `map.text`.

### 5. MCP and lists
- `tools.rs`:
  - `service_get` goes in `party_tools()`, `screen_text` beside `screenshot_tool()`.
  - The examples array gets one entry per new tool, and the count goes from 18 to 20.
- Every place that lists ops or tools gets the two new ones:
  - `tests/bridge.rs` (the 18 twice);
  - `omnis-sim/tests/ops.rs` (the round-trip list);
  - `omnis-cli/src/schema.rs` (the ops list);
  - `README.md` ("eighteen tools");
  - ARCH §9.3 table.
- The bridge `INSTRUCTIONS` gain one line: `service_get` in a shop, `screen_text` for a panel.

## Files
- **Sim:** `crates/omnis-sim/src/{service.rs, service_view.rs (new), ops.rs, command.rs, lib.rs}`.
- **App:** `crates/omnis-app/src/{ui_text.rs (new), socket.rs, lib.rs}`, plus
  `tests/common/feathers.rs`.
- **Headless:** `crates/omnis-cli/src/{headless.rs, schema.rs}`.
- **MCP:** `crates/omnis-mcp/src/{tools.rs, bridge.rs}`.
- **Docs:** README, ARCH §9.3, `tasks/TODO.md`, `verification.md` (tool count), `code-map.md`,
  `tasks/plans/m7-step6.md` (this plan, saved).

## Tests (each checked to fail when its rule is broken)
- **New `crates/omnis-sim/tests/service_view.rs`.**
  - At the smith, every Buy offer's price equals the gold actually spent by sending its
    `command`. A Sell offer's `pays` equals the gold gained.
  - An unaffordable row shows its price and `CannotAfford`.
  - At the temple: `NothingToTreat` for a member at full HP, `NotDead` for Raise on a living one,
    and a price on a wounded one.
  - The view leaves the world byte-identical (serialized, streams included).
  - Outside a service it returns `None`, and the op gives `NoService`.
  - After the `quote`/`afford` split, the town tests pass unedited.
- **`party_view`:**
  - `hit_dice_left` drops after a short rest with dice;
  - `bank` after a deposit;
  - `last_long_rest` and `long_rest_wait` after a room (1440 → 0 as the clock passes);
  - `status.service` inside and outside.
- **Script words:** each new word parses to its command, and bad forms (`buy-`, `buy-x`,
  `short-rest-`) return `None`. A town script run end to end: `forward…, buy-0, leave`.
- **App `tests/socket.rs`:** `screen.text` with the creation panel open returns its `[Name]`
  line; with no panel open it returns the message.
- **MCP `tests/bridge.rs`, headless:**
  - `service_get` after a script walks into a service;
  - `screen_text` is refused;
  - there are 20 tools.
- **Pins:** replays, tuple, `SAVE_SCHEMA` and schema-proof numbers do not move (no pack or
  `World` change). If a pin moves, stop and report.

## Verification
- `scripts/verify.sh > log 2>&1`, unpiped, status read.
- Sentrux `rescan` + `check_rules` before the commit, again after it (new files).
- Watch the caps: `socket.rs` 531, `ops.rs` 765, `command.rs` 495, `service.rs` 543.
- Report "what you will see":
  - Nothing new in the game window.
  - Through MCP, `service_get` in a shop and `screen_text` on the creation or confirm panel.
  - The service panel itself is step 7.
