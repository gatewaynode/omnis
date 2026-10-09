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
    /// The row it names: the service's stock for a purchase, the stores for a sale, the
    /// service's spells for a spell bought, the member's class list for a pick.
    pub row: Option<u8>,
    /// The member it names, at the temple and the trainer and for a spell.
    pub member: Option<CharacterId>,
    /// Copper it costs (0 when free), unless it is a sale or refused before its price.
    pub price: Option<u32>,
    /// Copper a sale brings.
    pub pays: Option<u32>,
    /// Why the rules would refuse it, if they would.
    pub refusal: Option<Rejection>,
    /// The id of the item or spell the row names, when it names one (M8 step 8).
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
        .map(|(command, row, member)| offer(world, data, def, command, row, member))
        .collect();
    offers.push(OfferView {
        command: ServiceCommand::Leave,
        row: None,
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

type Ask = (ServiceCommand, Option<u8>, Option<CharacterId>);

/// What this kind of service offers the party now, but leaving.
fn commands(world: &World, data: &Data, def: &ServiceDef) -> Vec<Ask> {
    let rows = |len: usize| (0..len).filter_map(|i| u8::try_from(i).ok());
    match def.kind {
        ServiceKind::Inn => alloc::vec![(ServiceCommand::Room, None, None)],
        ServiceKind::Tavern => alloc::vec![
            (ServiceCommand::Rumor, None, None),
            (ServiceCommand::BuyFood { count: 1 }, None, None),
        ],
        ServiceKind::Smith => {
            let buy = rows(def.items.len())
                .map(|item| (ServiceCommand::Buy { item, count: 1 }, Some(item), None));
            let sell = rows(world.party.inventory.len())
                .map(|item| (ServiceCommand::Sell { item, count: 1 }, Some(item), None));
            buy.chain(sell).collect()
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
                    .map(|command| (command, None, Some(member)))
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
                .map(|m| (ServiceCommand::Train { member: m.id }, None, Some(m.id)))
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

/// The trainer's picks: for each member owed some, each row of their class list worth offering.
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
        for (row, id) in list.iter().enumerate() {
            let Ok(spell) = u8::try_from(row) else {
                break;
            };
            if worth_offering(world, data, index, data.registry.spells.get(id)) {
                let command = ServiceCommand::Choose { member, spell };
                asks.push((command, Some(spell), Some(member)));
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
        for (row, id) in def.spells.iter().enumerate() {
            let Ok(spell) = u8::try_from(row) else {
                break;
            };
            if worth_offering(world, data, index, data.registry.spells.get(id)) {
                let command = ServiceCommand::Learn { member, spell };
                asks.push((command, Some(spell), Some(member)));
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
    row: Option<u8>,
    member: Option<CharacterId>,
) -> OfferView {
    let mut roller = Roller::take_stream(world, "town");
    let (price, pays, refusal) = match service::quote(world, data, def, command, &mut roller) {
        Ok(deal) => {
            let price = deal.paid().is_none().then_some(deal.cost());
            (price, deal.paid(), service::afford(world, &deal).err())
        }
        Err(refusal) => (None, None, Some(refusal)),
    };
    OfferView {
        command,
        row,
        member,
        price,
        pays,
        refusal,
        subject: subject(world, data, def, command),
    }
}

/// The id of the item or spell an offer's row names.
fn subject(
    world: &World,
    data: &Data,
    def: &ServiceDef,
    command: ServiceCommand,
) -> Option<String> {
    let row = |list: &[String], row: u8| list.get(usize::from(row)).cloned();
    match command {
        ServiceCommand::Buy { item, .. } => row(&def.items, item),
        ServiceCommand::Learn { spell, .. } => row(&def.spells, spell),
        ServiceCommand::Sell { item, .. } => world
            .party
            .inventory
            .get(usize::from(item))
            .and_then(|(id, _)| data.registry.items.name(*id))
            .map(String::from),
        ServiceCommand::Choose { member, spell } => world
            .party
            .slot_of(member)
            .ok()
            .and_then(|slot| world.party.members.get(usize::from(slot)))
            .and_then(|m| data.classes.get(&m.class))
            .and_then(|c| c.casting.as_ref())
            .and_then(|c| row(&c.list, spell)),
        _ => None,
    }
}
