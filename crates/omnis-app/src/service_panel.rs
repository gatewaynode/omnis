//! The service panel, Bevy-free: which control is which (`ServicePanelId`), the texts it
//! rewrites (`ServiceLabelId`), and `apply`, which turns a control's report into the command
//! to send. Everything on offer comes from `omnis_sim::service_view`: each offer is the exact
//! command, its price or what a sale pays, and the refusal the rules would give, so the panel
//! adds no rule of its own. The widgets are not trusted: an offer out of range or one the view
//! refuses sends nothing, and the bank's amount is clamped. `feathers_service.rs` draws it.

use crate::defs;
use crate::text::coins;
use crate::ui_model::Payload;
use omnis_sim::api::{MemberView, PartyView};
use omnis_sim::omnis_core::{CharacterId, fnv1a64};
use omnis_sim::omnis_data::Data;
use omnis_sim::{OfferView, Rejection, ServiceCommand, ServiceView};

/// One control of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ServicePanelId {
    /// The button of an offer, by index into the view's offers.
    Offer(usize),
    /// The bank's amount, in whole gold.
    Amount,
    /// Put the amount in the bank.
    Deposit,
    /// Take the amount out of the bank.
    Withdraw,
    /// Leave the service (asks first).
    #[default]
    Leave,
}

/// A text the panel rewrites from the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServiceLabelId {
    /// "Gold 16 gp 4 sp 5 cp · Bank 2 gp 0 sp 0 cp · Food 10".
    #[default]
    Money,
    /// An offer's row label, by index into the view's offers.
    Row(usize),
    /// An offer's price, what it pays, or why not.
    Note(usize),
    /// The last refusal.
    Message,
}

/// What the panel keeps between frames: the bank's amount and the last refusal.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServiceForm {
    /// The amount typed for the bank, in whole gold.
    pub amount_gp: u32,
    /// The last refusal, or empty.
    pub message: String,
}

/// What a control asks of the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceAsk {
    /// Send this command.
    Send(ServiceCommand),
    /// Leave, through the question every leaving asks.
    Leave,
}

/// The largest amount the bank's input holds, in whole gold: its copper fits a `u32`.
pub const AMOUNT_MAX_GP: u32 = u32::MAX / 100;

/// Apply one control's report. `view` is the service as it is now.
pub fn apply(
    id: ServicePanelId,
    payload: &Payload,
    view: &ServiceView,
    form: &mut ServiceForm,
) -> Option<ServiceAsk> {
    let copper = form.amount_gp.saturating_mul(100);
    match (id, payload) {
        (ServicePanelId::Offer(index), Payload::Activate) => {
            let offer = view.offers.get(index)?;
            if offer.refusal.is_some() || offer.command == ServiceCommand::Leave {
                return None;
            }
            Some(ServiceAsk::Send(offer.command.clone()))
        }
        (ServicePanelId::Amount, Payload::Number(value)) => {
            form.amount_gp = u32::try_from((*value).clamp(0, i64::from(AMOUNT_MAX_GP))).ok()?;
            None
        }
        (ServicePanelId::Deposit, Payload::Activate) if copper > 0 => {
            Some(ServiceAsk::Send(ServiceCommand::Deposit { amount: copper }))
        }
        (ServicePanelId::Withdraw, Payload::Activate) if copper > 0 => {
            Some(ServiceAsk::Send(ServiceCommand::Withdraw {
                amount: copper,
            }))
        }
        (ServicePanelId::Leave, Payload::Activate) => Some(ServiceAsk::Leave),
        _ => None,
    }
}

/// One offer as the panel shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferRow {
    /// What the row names, unchanging while the panel stands: an item or a member.
    pub key: String,
    /// The row's label: "Longsword", "Dagger ×2", "Brenna, 5 of 12 HP".
    pub label: String,
    /// The button's caption.
    pub caption: &'static str,
    /// Its price, what it pays, or why not.
    pub note: String,
    /// Whether the rules would refuse it.
    pub refused: bool,
}

/// The rows of every offer but leaving, in the view's order.
#[must_use]
pub fn offer_rows(view: &ServiceView, party: &PartyView, data: &Data) -> Vec<OfferRow> {
    view.offers
        .iter()
        .filter(|o| o.command != ServiceCommand::Leave)
        .map(|offer| {
            let (key, label) = row_label(offer, party, data);
            OfferRow {
                key,
                label,
                caption: caption(&offer.command),
                note: note(offer),
                refused: offer.refusal.is_some(),
            }
        })
        .collect()
}

fn row_label(offer: &OfferView, party: &PartyView, data: &Data) -> (String, String) {
    let member = |id: CharacterId| party.members.iter().find(|m| m.member == id);
    match offer.command {
        ServiceCommand::Room => (String::new(), "A night's rest".to_owned()),
        ServiceCommand::Rumor => (String::new(), "A rumor".to_owned()),
        ServiceCommand::BuyFood { count } => (String::new(), format!("Food for {count} day")),
        ServiceCommand::Buy { .. } => {
            let name = offer
                .subject
                .as_deref()
                .and_then(|id| defs::item(data, id))
                .map_or("?", |i| data.label("en", &i.name))
                .to_owned();
            (name.clone(), name)
        }
        ServiceCommand::Sell { ref item, .. } => {
            let (name, count) = party
                .inventory
                .iter()
                .find(|i| i.item == *item)
                .map_or(("?", 0), |i| (data.label("en", &i.name), i.count));
            (name.to_owned(), format!("{name} ×{count}"))
        }
        ServiceCommand::Heal { member: id }
        | ServiceCommand::Cure { member: id }
        | ServiceCommand::Raise { member: id } => member(id).map_or_else(
            || ("?".to_owned(), "?".to_owned()),
            |m| (m.name.clone(), member_label(offer, m, data)),
        ),
        ServiceCommand::Train { member: id } => member(id).map_or_else(
            || ("?".to_owned(), "?".to_owned()),
            |m| {
                let label = format!("{}, level {} to {}", m.name, m.level, m.level + 1);
                (m.name.clone(), label)
            },
        ),
        ServiceCommand::Choose { member: id, .. } | ServiceCommand::Learn { member: id, .. } => {
            member(id).map_or_else(
                || ("?".to_owned(), "?".to_owned()),
                |m| {
                    let name = offer
                        .subject
                        .as_deref()
                        .and_then(|id| defs::spell(data, id))
                        .map_or("?", |s| data.label("en", &s.name));
                    (format!("{}: {name}", m.name), format!("{}: {name}", m.name))
                },
            )
        }
        ServiceCommand::Leave
        | ServiceCommand::Deposit { .. }
        | ServiceCommand::Withdraw { .. } => (String::new(), String::new()),
    }
}

/// A temple row names what it treats: hit points, conditions, or death.
fn member_label(offer: &OfferView, member: &MemberView, data: &Data) -> String {
    let name = &member.name;
    match offer.command {
        ServiceCommand::Cure { .. } if member.conditions.is_empty() => {
            format!("{name}, no conditions")
        }
        ServiceCommand::Cure { .. } => {
            let names: Vec<&str> = member
                .conditions
                .iter()
                .map(|id| defs::condition(data, id).map_or("?", |c| data.label("en", &c.name)))
                .collect();
            format!("{name}: {}", names.join(", "))
        }
        ServiceCommand::Raise { .. }
            if matches!(offer.refusal, Some(Rejection::NotDead { .. })) =>
        {
            format!("{name}, alive")
        }
        ServiceCommand::Raise { .. } => format!("{name}, dead"),
        _ => format!("{name}, {} of {} HP", member.hp.max(0), member.hp_max),
    }
}

/// The button's caption for an offer.
#[must_use]
pub const fn caption(command: &ServiceCommand) -> &'static str {
    match command {
        ServiceCommand::Room => "Take a room",
        ServiceCommand::Rumor => "Listen",
        ServiceCommand::BuyFood { .. } | ServiceCommand::Buy { .. } => "Buy",
        ServiceCommand::Sell { .. } => "Sell",
        ServiceCommand::Heal { .. } => "Heal",
        ServiceCommand::Cure { .. } => "Cure",
        ServiceCommand::Raise { .. } => "Raise",
        ServiceCommand::Deposit { .. } => "Deposit",
        ServiceCommand::Withdraw { .. } => "Withdraw",
        ServiceCommand::Train { .. } => "Train",
        ServiceCommand::Choose { .. } => "Choose",
        ServiceCommand::Learn { .. } => "Learn",
        ServiceCommand::Leave => "Leave",
    }
}

/// An offer's price ("free" at no cost), what a sale pays, or why the rules would refuse it;
/// a price the party cannot pay keeps its price beside the reason.
#[must_use]
pub fn note(offer: &OfferView) -> String {
    match (&offer.refusal, offer.price, offer.pays) {
        (Some(refusal @ Rejection::CannotAfford { .. }), Some(price), _) => {
            format!("{}, {}", coins(price), reason(refusal))
        }
        (Some(refusal), _, _) => reason(refusal),
        (None, Some(0), _) => "free".to_owned(),
        (None, Some(price), _) => coins(price),
        (None, None, Some(pays)) => format!("pays {}", coins(pays)),
        (None, None, None) => String::new(),
    }
}

/// A refusal in a few words, for an offer's row; the message line keeps the full wording.
#[must_use]
pub fn reason(refusal: &Rejection) -> String {
    match refusal {
        Rejection::CannotAfford { .. } => "not enough money".to_owned(),
        Rejection::NothingToTreat { .. } => "nothing to treat".to_owned(),
        Rejection::NotDead { .. } => "not dead".to_owned(),
        Rejection::RestTooSoon { minutes } => {
            format!(
                "rested too recently ({}h {:02}m)",
                minutes / 60,
                minutes % 60
            )
        }
        other => other.to_string(),
    }
}

/// The money line.
#[must_use]
pub fn money_line(view: &ServiceView) -> String {
    format!(
        "Gold {} · Bank {} · Food {}",
        coins(view.gold),
        coins(view.bank),
        view.food
    )
}

/// What the panel's entity tree depends on: the service, each offer's command and what its
/// row names. Prices, refusals and counts are rewritten in place; a sale that empties a row
/// or a purchase that adds one builds the panel again.
#[must_use]
pub fn shape(view: &ServiceView, rows: &[OfferRow]) -> u64 {
    let mut bytes = view.service.as_bytes().to_vec();
    bytes.extend_from_slice(&rows.len().to_le_bytes());
    for (offer, row) in view.offers.iter().zip(rows) {
        bytes.push(0);
        bytes.extend_from_slice(format!("{:?}", offer.command).as_bytes());
        bytes.push(1);
        bytes.extend_from_slice(row.key.as_bytes());
    }
    fnv1a64(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnis_sim::omnis_data::ServiceKind;

    const DAGGER: &str = "base:item:dagger";

    fn buy(item: &str) -> ServiceCommand {
        ServiceCommand::Buy {
            item: item.to_owned(),
            count: 1,
        }
    }

    fn sell(item: &str) -> ServiceCommand {
        ServiceCommand::Sell {
            item: item.to_owned(),
            count: 1,
        }
    }

    fn offer(command: ServiceCommand, price: Option<u32>, refusal: Option<Rejection>) -> OfferView {
        OfferView {
            command,
            member: None,
            price,
            pays: None,
            refusal,
            subject: None,
        }
    }

    fn smith() -> ServiceView {
        ServiceView {
            service: "base:service:smith".into(),
            name: "base:text:service.smith.name".into(),
            kind: ServiceKind::Smith,
            gold: 1650,
            bank: 200,
            food: 10,
            offers: vec![
                offer(buy(DAGGER), Some(200), None),
                offer(
                    buy("base:item:chain_mail"),
                    Some(7500),
                    Some(Rejection::CannotAfford {
                        cost: 7500,
                        gold: 1650,
                    }),
                ),
                OfferView {
                    pays: Some(250),
                    ..offer(sell(DAGGER), None, None)
                },
                offer(ServiceCommand::Leave, None, None),
            ],
        }
    }

    #[test]
    fn a_button_sends_its_offer_and_nothing_the_view_refuses() {
        let view = smith();
        let mut form = ServiceForm::default();
        let mut press = |id| apply(id, &Payload::Activate, &view, &mut form);
        assert_eq!(
            press(ServicePanelId::Offer(0)),
            Some(ServiceAsk::Send(buy(DAGGER)))
        );
        assert_eq!(press(ServicePanelId::Offer(1)), None, "refused");
        assert_eq!(
            press(ServicePanelId::Offer(2)),
            Some(ServiceAsk::Send(sell(DAGGER)))
        );
        assert_eq!(
            press(ServicePanelId::Offer(3)),
            None,
            "leaving is its own button"
        );
        assert_eq!(press(ServicePanelId::Offer(9)), None, "out of range");
        assert_eq!(press(ServicePanelId::Leave), Some(ServiceAsk::Leave));
        assert_eq!(
            apply(
                ServicePanelId::Offer(0),
                &Payload::Flag(true),
                &view,
                &mut form
            ),
            None,
            "only a press"
        );
    }

    #[test]
    fn the_amount_is_whole_gold_clamped_and_never_zero() {
        let view = smith();
        let mut form = ServiceForm::default();
        assert_eq!(
            apply(
                ServicePanelId::Deposit,
                &Payload::Activate,
                &view,
                &mut form
            ),
            None,
            "nothing typed"
        );
        let mut typed = |value| {
            apply(
                ServicePanelId::Amount,
                &Payload::Number(value),
                &view,
                &mut form,
            );
        };
        typed(12);
        assert_eq!(form.amount_gp, 12);
        assert_eq!(
            apply(
                ServicePanelId::Deposit,
                &Payload::Activate,
                &view,
                &mut form
            ),
            Some(ServiceAsk::Send(ServiceCommand::Deposit { amount: 1200 }))
        );
        assert_eq!(
            apply(
                ServicePanelId::Withdraw,
                &Payload::Activate,
                &view,
                &mut form
            ),
            Some(ServiceAsk::Send(ServiceCommand::Withdraw { amount: 1200 }))
        );
        apply(
            ServicePanelId::Amount,
            &Payload::Number(-5),
            &view,
            &mut form,
        );
        assert_eq!(form.amount_gp, 0);
        apply(
            ServicePanelId::Amount,
            &Payload::Number(i64::MAX),
            &view,
            &mut form,
        );
        assert_eq!(form.amount_gp, AMOUNT_MAX_GP);
        assert_eq!(
            apply(
                ServicePanelId::Withdraw,
                &Payload::Activate,
                &view,
                &mut form
            ),
            Some(ServiceAsk::Send(ServiceCommand::Withdraw {
                amount: AMOUNT_MAX_GP * 100
            })),
            "the largest amount's copper fits"
        );
    }

    #[test]
    fn a_row_says_its_price_what_it_pays_or_why_not() {
        let view = smith();
        let notes: Vec<String> = view.offers.iter().map(note).collect();
        assert_eq!(
            notes,
            [
                "2 gp 0 sp 0 cp",
                "75 gp 0 sp 0 cp, not enough money",
                "pays 2 gp 5 sp 0 cp",
                ""
            ]
        );
        assert_eq!(note(&offer(ServiceCommand::Rumor, Some(0), None)), "free");
        assert_eq!(
            note(&offer(
                ServiceCommand::Room,
                None,
                Some(Rejection::RestTooSoon { minutes: 605 })
            )),
            "rested too recently (10h 05m)"
        );
        assert_eq!(
            reason(&Rejection::NothingToTreat {
                member: CharacterId(1)
            }),
            "nothing to treat"
        );
        assert_eq!(
            reason(&Rejection::NotOffered),
            Rejection::NotOffered.to_string()
        );
        assert_eq!(
            money_line(&view),
            "Gold 16 gp 5 sp 0 cp · Bank 2 gp 0 sp 0 cp · Food 10"
        );
        assert_eq!(caption(&ServiceCommand::Room), "Take a room");
        assert_eq!(
            caption(&ServiceCommand::Train {
                member: CharacterId(0)
            }),
            "Train"
        );
    }

    #[test]
    fn the_shape_moves_with_the_rows_and_not_with_money() {
        let view = smith();
        let rows = |view: &ServiceView| -> Vec<OfferRow> {
            view.offers
                .iter()
                .filter(|o| o.command != ServiceCommand::Leave)
                .map(|o| OfferRow {
                    key: format!("{:?}", o.subject),
                    label: String::new(),
                    caption: caption(&o.command),
                    note: note(o),
                    refused: o.refusal.is_some(),
                })
                .collect()
        };
        let base = shape(&view, &rows(&view));
        let mut richer = smith();
        richer.gold = 99_999;
        richer.offers[1].refusal = None;
        assert_eq!(
            shape(&richer, &rows(&richer)),
            base,
            "money and refusals are synced"
        );
        let mut sold_out = smith();
        sold_out.offers.remove(2);
        assert_ne!(
            shape(&sold_out, &rows(&sold_out)),
            base,
            "a row gone is a new tree"
        );
        let mut renamed = rows(&view);
        renamed[0].key = "Mace".into();
        assert_ne!(shape(&view, &renamed), base, "another item in the row");
    }
}
