//! Inside a town service (parity P4): the party's money, the service's lists of offers by kind
//! as the other client shows them, and Leave. Each offer is a row: what it names, a button with
//! its caption, dim with the rules' reason when they would refuse it, and its price or what it
//! pays. The bank has no list; it takes an amount of whole gold to pay in or draw out. Every
//! offer, price and refusal is the engine's (`service_view`); this module only words and
//! groups them.

use super::{Control, ControlId, Effect, Game, Kind, Panel, Payload, Refused, Row, Screen};
use super::{ScreenId, Span};
use crate::labels;
use crate::style::{Role, Size};
use omnis_sim::omnis_data::{Data, ServiceKind};
use omnis_sim::{Command, ServiceCommand, ServiceView};

/// The largest amount the bank's field takes, in whole gold: its copper fits a `u32`.
pub const AMOUNT_MAX_GP: u32 = u32::MAX / 100;

/// The most digits the bank's field takes: [`AMOUNT_MAX_GP`] has eight.
pub const AMOUNT_DIGITS: usize = 8;

/// A list's heading: the title's colour at the body's size.
fn heading(text: &str) -> Row {
    Row::text(Span {
        text: text.to_owned(),
        role: Role::Title,
        size: Size::Body,
    })
}

/// Dim words: an empty list, or a note beside a control.
fn dim(text: impl Into<String>, size: Size) -> Span {
    Span {
        text: text.into(),
        role: Role::Dim,
        size,
    }
}

/// The lists a kind of service shows, each a heading and its offers by index into the view's
/// offers (the other client's `feathers_service::lists`). Leaving is never in a list.
fn lists(view: &ServiceView) -> Vec<(&'static str, Vec<usize>)> {
    let pick = |wanted: fn(&ServiceCommand) -> bool| -> Vec<usize> {
        view.offers
            .iter()
            .enumerate()
            .filter(|(_, o)| o.command != ServiceCommand::Leave && wanted(&o.command))
            .map(|(i, _)| i)
            .collect()
    };
    let buy = |c: &ServiceCommand| matches!(c, ServiceCommand::Buy { .. });
    let sell = |c: &ServiceCommand| matches!(c, ServiceCommand::Sell { .. });
    let learn = |c: &ServiceCommand| matches!(c, ServiceCommand::Learn { .. });
    let train = |c: &ServiceCommand| matches!(c, ServiceCommand::Train { .. });
    let choose = |c: &ServiceCommand| matches!(c, ServiceCommand::Choose { .. });
    match view.kind {
        ServiceKind::Smith => vec![("For sale", pick(buy)), ("Your stores", pick(sell))],
        ServiceKind::Inn | ServiceKind::Tavern => vec![("On offer", pick(|_| true))],
        ServiceKind::Temple => vec![
            (
                "On offer",
                pick(|c| !matches!(c, ServiceCommand::Learn { .. })),
            ),
            ("Spells", pick(learn)),
        ],
        ServiceKind::Trainer => vec![("Levels", pick(train)), ("Spell picks", pick(choose))],
        ServiceKind::Guild => vec![("Spells", pick(learn))],
        ServiceKind::Bank => Vec::new(),
    }
}

/// One offer's row: its label, its button, its price or what it pays.
fn offer_row(data: &Data, game: Game, view: &ServiceView, index: usize) -> Option<Row> {
    let offer = view.offers.get(index)?;
    let note = labels::offer_note(offer);
    Some(Row {
        spans: vec![Span::body(labels::offer_label(
            data,
            &game.views.party,
            offer,
        ))],
        controls: vec![Control {
            id: ControlId::Offer(u16::try_from(index).ok()?),
            label: labels::offer_caption(&offer.command).to_owned(),
            kind: Kind::Button,
            why_not: offer.refusal.as_ref().map(labels::service_reason),
        }],
        after: if note.is_empty() {
            Vec::new()
        } else {
            vec![dim(note, Size::Small)]
        },
    })
}

/// The bank's amount in copper, when one is typed and it is not nothing.
fn copper(bank: &str) -> Option<u32> {
    let gold: u32 = bank.parse().ok()?;
    (gold > 0).then(|| gold.saturating_mul(100))
}

/// The bank's command for `amount` copper, paid in or drawn out.
fn banking(deposit: bool, amount: u32) -> ServiceCommand {
    if deposit {
        ServiceCommand::Deposit { amount }
    } else {
        ServiceCommand::Withdraw { amount }
    }
}

/// The bank's row: the amount, then Deposit and Withdraw, dim until an amount is typed and then
/// with the rules' reason, tried on a copy of the world.
fn bank_row(data: &Data, game: Game, bank: &str) -> Row {
    let button = |id, label: &str, deposit| Control {
        why_not: match copper(bank) {
            None => Some("Type an amount".to_owned()),
            Some(amount) => game
                .world
                .trial(data, &Command::Service(banking(deposit, amount)))
                .err()
                .map(|r| labels::service_reason(&r)),
        },
        ..Control::button(id, label)
    };
    Row::controls(vec![
        Control {
            id: ControlId::Amount,
            label: "Gold pieces".to_owned(),
            kind: Kind::Text {
                value: bank.to_owned(),
                max: AMOUNT_DIGITS,
            },
            why_not: None,
        },
        button(ControlId::Deposit, "Deposit", true),
        button(ControlId::Withdraw, "Withdraw", false),
    ])
}

/// The screen: the service's name; the money and Leave, then the lists side by side, or the
/// bank's amount. `bank` is the amount typed. Outside a service (the shell shows it only
/// inside) it is empty.
#[must_use]
pub fn content(data: &Data, game: Game, bank: &str) -> Screen {
    let Some(view) = game.views.service.as_ref() else {
        return super::empty(ScreenId::Service);
    };
    // Two spaces apart, as the HUD's figures are: the font has no middle dot.
    let money = format!(
        "Gold {}  Bank {}  Food {}",
        labels::coins(view.gold),
        labels::coins(view.bank),
        view.food
    );
    let top = Row {
        spans: vec![Span::body(money)],
        controls: vec![Control::button(ControlId::Leave, "Leave")],
        after: Vec::new(),
    };
    let mut panels: Vec<Panel> = lists(view)
        .into_iter()
        .map(|(title, offers)| {
            let mut rows = vec![heading(title)];
            rows.extend(
                offers
                    .iter()
                    .filter_map(|&i| offer_row(data, game, view, i)),
            );
            if offers.is_empty() {
                rows.push(Row::text(dim("Nothing to offer", Size::Body)));
            }
            Panel { rows }
        })
        .collect();
    if view.kind == ServiceKind::Bank {
        panels.push(Panel {
            rows: vec![bank_row(data, game, bank)],
        });
    }
    match panels.first_mut() {
        Some(first) => first.rows.insert(0, top),
        None => panels.push(Panel { rows: vec![top] }),
    }
    Screen {
        id: ScreenId::Service,
        title: Span::title(labels::service(data, &view.service)),
        panels,
        keys: Vec::new(),
        message: None,
    }
}

/// An offer sends its command; Leave leaves; the amount takes digits up to [`AMOUNT_MAX_GP`];
/// Deposit and Withdraw send the amount in copper. The control was found live on the screen
/// as it stands, so a refused offer never gets here.
///
/// # Errors
/// [`Refused::NotDigits`] for an amount that is not all digits, [`Refused::OutOfRange`] for
/// one over [`AMOUNT_MAX_GP`]; the amount is kept.
pub fn apply(
    game: Game,
    bank: &mut String,
    control: ControlId,
    payload: &Payload,
) -> Result<Effect, Refused> {
    let send = |command| Effect::Command {
        command: Command::Service(command),
        close: false,
    };
    Ok(match (control, payload) {
        (ControlId::Offer(i), _) => game
            .views
            .service
            .as_ref()
            .and_then(|v| v.offers.get(usize::from(i)))
            .map_or(Effect::None, |o| send(o.command.clone())),
        (ControlId::Leave, _) => send(ServiceCommand::Leave),
        (ControlId::Amount, Payload::Text(text)) => {
            if !text.bytes().all(|b| b.is_ascii_digit()) {
                return Err(Refused::NotDigits);
            }
            if !text.is_empty() && text.parse::<u32>().map_or(true, |g| g > AMOUNT_MAX_GP) {
                return Err(Refused::OutOfRange);
            }
            bank.clone_from(text);
            Effect::None
        }
        (ControlId::Deposit | ControlId::Withdraw, _) => copper(bank).map_or(Effect::None, |c| {
            send(banking(control == ControlId::Deposit, c))
        }),
        _ => Effect::None,
    })
}
