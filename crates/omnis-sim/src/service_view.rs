//! The service the party is inside, as a client sees it (`service.get`): every thing on offer as
//! the command that asks for it, its price and, when the rules would refuse it, why. The one
//! model the service panel and agents both read. Each offer is quoted on its own copy of the
//! `town` stream, never stored, so the view changes nothing and each price is what that command
//! would cost if it were sent next.

use crate::combat::Roller;
use crate::command::Rejection;
use crate::service::{self, ServiceCommand};
use crate::world::{Mode, World};
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::{CharacterId, SpellId};
use omnis_data::{Data, ServiceDef, ServiceKind};
use omnis_rules::{SpellRefusal, may_learn};
use serde::{Deserialize, Serialize};

/// The service as a client sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceView {
    /// Service id such as `base:service:inn`.
    pub service: String,
    /// Text key of its name.
    pub name: String,
    /// What kind of service it is.
    pub kind: ServiceKind,
    /// The party's money, in copper.
    pub gold: u32,
    /// The party's money in the bank, in copper.
    pub bank: u32,
    /// The party's food.
    pub food: u32,
    /// Everything on offer, leaving last.
    pub offers: Vec<OfferView>,
}

/// One thing on offer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfferView {
    /// The command that asks for it, exactly as it is sent (counts are 1).
    pub command: ServiceCommand,
    /// The member it names, at the temple and the trainer and for a spell.
    pub member: Option<CharacterId>,
    /// Copper it costs (0 when free), unless it is a sale or refused before its price.
    pub price: Option<u32>,
    /// Copper a sale brings.
    pub pays: Option<u32>,
    /// Why the rules would refuse it, if they would.
    pub refusal: Option<Rejection>,
    /// The id of the item or spell the command names, when it names one (M8 step 8).
    #[serde(default)]
    pub subject: Option<String>,
}

/// The service the party is inside, or `None` outside one.
#[must_use]
pub fn service_view(world: &World, data: &Data) -> Option<ServiceView> {
    let Mode::Town(state) = world.mode else {
        return None;
    };
    let def = data.services.get(&state.service)?;
    let mut offers: Vec<OfferView> = commands(world, data, def)
        .into_iter()
        .map(|(command, member)| offer(world, data, def, command, member))
        .collect();
    offers.push(OfferView {
        command: ServiceCommand::Leave,
        member: None,
        price: None,
        pays: None,
        refusal: None,
        subject: None,
    });
    Some(ServiceView {
        service: def.id.clone(),
        name: def.name.clone(),
        kind: def.kind,
        gold: world.party.gold,
        bank: world.party.bank,
        food: world.party.food,
        offers,
    })
}

type Ask = (ServiceCommand, Option<CharacterId>);

/// What this kind of service offers the party now, but leaving.
fn commands(world: &World, data: &Data, def: &ServiceDef) -> Vec<Ask> {
    match def.kind {
        ServiceKind::Inn => alloc::vec![(ServiceCommand::Room, None)],
        ServiceKind::Tavern => alloc::vec![
            (ServiceCommand::Rumor, None),
            (ServiceCommand::BuyFood { count: 1 }, None),
        ],
        ServiceKind::Smith => {
            let mut asks: Vec<Ask> = Vec::new();
            for item in &def.items {
                let buy = ServiceCommand::Buy {
                    item: item.clone(),
                    count: 1,
                };
                if !asks.iter().any(|(c, _)| *c == buy) {
                    asks.push((buy, None));
                }
            }
            for (id, _) in &world.party.inventory {
                let Some(item) = data.registry.items.name(*id) else {
                    continue;
                };
                let sell = ServiceCommand::Sell {
                    item: String::from(item),
                    count: 1,
                };
                if !asks.iter().any(|(c, _)| *c == sell) {
                    asks.push((sell, None));
                }
            }
            asks
        }
        ServiceKind::Temple => {
            let mut asks: Vec<Ask> = world
                .party
                .members
                .iter()
                .map(|m| m.id)
                .flat_map(|member| {
                    [
                        ServiceCommand::Heal { member },
                        ServiceCommand::Cure { member },
                        ServiceCommand::Raise { member },
                    ]
                    .map(|command| (command, Some(member)))
                })
                .collect();
            asks.extend(purchases(world, data, def));
            asks
        }
        ServiceKind::Guild => purchases(world, data, def),
        ServiceKind::Trainer => {
            let mut asks: Vec<Ask> = world
                .party
                .members
                .iter()
                .map(|m| (ServiceCommand::Train { member: m.id }, Some(m.id)))
                .collect();
            asks.extend(picks(world, data));
            asks
        }
        ServiceKind::Bank => Vec::new(),
    }
}

/// Whether a spell belongs among a member's offers: one they may add now, or one their level
/// does not reach yet (shown with that refusal). Known spells, cantrips and spells off the
/// member's list are left out.
fn worth_offering(world: &World, data: &Data, member: usize, spell: Option<SpellId>) -> bool {
    let Some(spell) = spell else {
        return false;
    };
    matches!(
        may_learn(&world.party.members[member], data, spell),
        Ok(None | Some(SpellRefusal::TooHigh { .. }))
    )
}

/// The trainer's picks: for each member owed some, each spell of their class list worth
/// offering, in list order.
fn picks(world: &World, data: &Data) -> Vec<Ask> {
    let mut asks = Vec::new();
    for (index, who) in world.party.members.iter().enumerate() {
        let Some(list) = data
            .classes
            .get(&who.class)
            .and_then(|c| c.casting.as_ref())
            .filter(|_| who.spell_picks > 0)
            .map(|c| &c.list)
        else {
            continue;
        };
        let member = who.id;
        for spell in list {
            if worth_offering(world, data, index, data.registry.spells.get(spell)) {
                let spell = spell.clone();
                asks.push((ServiceCommand::Choose { member, spell }, Some(member)));
            }
        }
    }
    asks
}

/// A guild's or temple's spells: for each member, each stocked spell worth offering them.
fn purchases(world: &World, data: &Data, def: &ServiceDef) -> Vec<Ask> {
    let mut asks = Vec::new();
    for (index, who) in world.party.members.iter().enumerate() {
        let member = who.id;
        for spell in &def.spells {
            if worth_offering(world, data, index, data.registry.spells.get(spell)) {
                let spell = spell.clone();
                asks.push((ServiceCommand::Learn { member, spell }, Some(member)));
            }
        }
    }
    asks
}

/// One offer, quoted on a fresh copy of the stream.
fn offer(
    world: &World,
    data: &Data,
    def: &ServiceDef,
    command: ServiceCommand,
    member: Option<CharacterId>,
) -> OfferView {
    let mut roller = Roller::take_stream(world, "town");
    let (price, pays, refusal) = match service::quote(world, data, def, &command, &mut roller) {
        Ok(deal) => {
            let price = deal.paid().is_none().then_some(deal.cost());
            (price, deal.paid(), service::afford(world, &deal).err())
        }
        Err(refusal) => (None, None, Some(refusal)),
    };
    OfferView {
        subject: subject(&command),
        command,
        member,
        price,
        pays,
        refusal,
    }
}

/// The id of the item or spell an offer's command names.
fn subject(command: &ServiceCommand) -> Option<String> {
    match command {
        ServiceCommand::Buy { item, .. } | ServiceCommand::Sell { item, .. } => Some(item.clone()),
        ServiceCommand::Choose { spell, .. } | ServiceCommand::Learn { spell, .. } => {
            Some(spell.clone())
        }
        _ => None,
    }
}
