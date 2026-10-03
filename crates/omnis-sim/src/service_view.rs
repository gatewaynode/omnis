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
use omnis_data::{Data, ServiceDef, ServiceKind};
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
    /// The row it names: the service's stock for a purchase, the stores for a sale.
    pub row: Option<u8>,
    /// The member it names, at the temple.
    pub member: Option<u8>,
    /// Copper it costs (0 when free), unless it is a sale or refused before its price.
    pub price: Option<u32>,
    /// Copper a sale brings.
    pub pays: Option<u32>,
    /// Why the rules would refuse it, if they would.
    pub refusal: Option<Rejection>,
}

/// The service the party is inside, or `None` outside one.
#[must_use]
pub fn service_view(world: &World, data: &Data) -> Option<ServiceView> {
    let Mode::Town(state) = world.mode else {
        return None;
    };
    let def = data.services.get(&state.service)?;
    let mut offers: Vec<OfferView> = commands(world, def)
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

type Ask = (ServiceCommand, Option<u8>, Option<u8>);

/// What this kind of service offers the party now, but leaving.
fn commands(world: &World, def: &ServiceDef) -> Vec<Ask> {
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
        ServiceKind::Temple => rows(world.party.members.len())
            .flat_map(|member| {
                [
                    ServiceCommand::Heal { member },
                    ServiceCommand::Cure { member },
                    ServiceCommand::Raise { member },
                ]
                .map(|command| (command, None, Some(member)))
            })
            .collect(),
        ServiceKind::Bank | ServiceKind::Trainer | ServiceKind::Guild => Vec::new(),
    }
}

/// One offer, quoted on a fresh copy of the stream.
fn offer(
    world: &World,
    data: &Data,
    def: &ServiceDef,
    command: ServiceCommand,
    row: Option<u8>,
    member: Option<u8>,
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
    }
}
