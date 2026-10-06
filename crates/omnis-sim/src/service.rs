//! Town services (PRD §7.4, M7): going into the service on the party's tile, and what the party
//! does inside one. Every command is validated in full before anything changes, so a refusal
//! leaves the world as it was. Every price is a slot in `services.ron`, in copper; a rumor's
//! die rolls on a copy of the `town` stream, written back only on success; every transaction
//! but leaving and a night's room takes `service_minutes`.

use crate::apply::advance;
use crate::combat::Roller;
use crate::combat::state::is_dead;
use crate::command::Rejection;
use crate::event::Event;
use crate::items::{add_to, count_of, take_from};
use crate::party;
use crate::rest;
use crate::service_level;
use crate::world::{Mode, World};
use alloc::boxed::Box;
use alloc::vec::Vec;
use omnis_core::{ItemId, ServiceId, SpellId};
use omnis_data::omnis_expr::Value;
use omnis_data::{Data, ServiceDef, ServiceKind};
use omnis_rules::{Character, Gains, RuleError};
use serde::{Deserialize, Serialize};

/// Party-clock minutes a transaction takes when the rules do not say.
const DEFAULT_SERVICE_MINUTES: u32 = 10;

/// The service the party is inside. The kind is kept so the save rule needs no pack data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceState {
    /// Which service.
    pub service: ServiceId,
    /// What it does.
    pub kind: ServiceKind,
}

/// What the party asks of the service it is in. A `member` is a party slot; `Buy`'s `item` is
/// a row of the service's stock as its pack lists it, `Sell`'s a row of the party's stores.
/// Amounts are copper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ServiceCommand {
    /// Go back out onto the tile; any service.
    Leave,
    /// A night's room: a long rest with no food and no ambush; an inn.
    Room,
    /// Hear one of the rumors; free; a tavern.
    Rumor,
    /// Food into the larder; a tavern.
    BuyFood {
        /// Units.
        count: u16,
    },
    /// A living member back to full hit points; a temple.
    Heal {
        /// The member's slot.
        member: u8,
    },
    /// Every condition but death and unconsciousness removed; a temple.
    Cure {
        /// The member's slot.
        member: u8,
    },
    /// A dead member back at one hit point; a temple.
    Raise {
        /// The member's slot.
        member: u8,
    },
    /// Items from the stock into the party's stores; a smith.
    Buy {
        /// The row of the service's stock.
        item: u8,
        /// How many.
        count: u16,
    },
    /// Items from the party's stores, at the sell price; a smith.
    Sell {
        /// The row of the stores.
        item: u8,
        /// How many.
        count: u16,
    },
    /// Copper from the purse into the bank; a bank.
    Deposit {
        /// Copper.
        amount: u32,
    },
    /// Copper from the bank into the purse; a bank.
    Withdraw {
        /// Copper.
        amount: u32,
    },
    /// The member's next level, for a fee; a trainer.
    Train {
        /// The member's slot.
        member: u8,
    },
    /// A spell owed by a level onto the member's list, free; a trainer.
    Choose {
        /// The member's slot.
        member: u8,
        /// The row of the member's class list.
        spell: u8,
    },
    /// A spell bought onto the member's list; a guild or a temple.
    Learn {
        /// The member's slot.
        member: u8,
        /// The row of the service's spells.
        spell: u8,
    },
}

impl ServiceCommand {
    /// Whether a service of `kind` does this; leaving, any does.
    #[must_use]
    pub const fn offered_in(self, kind: ServiceKind) -> bool {
        match self {
            ServiceCommand::Leave => true,
            ServiceCommand::Room => matches!(kind, ServiceKind::Inn),
            ServiceCommand::Rumor | ServiceCommand::BuyFood { .. } => {
                matches!(kind, ServiceKind::Tavern)
            }
            ServiceCommand::Heal { .. }
            | ServiceCommand::Cure { .. }
            | ServiceCommand::Raise { .. } => matches!(kind, ServiceKind::Temple),
            ServiceCommand::Buy { .. } | ServiceCommand::Sell { .. } => {
                matches!(kind, ServiceKind::Smith)
            }
            ServiceCommand::Deposit { .. } | ServiceCommand::Withdraw { .. } => {
                matches!(kind, ServiceKind::Bank)
            }
            ServiceCommand::Train { .. } | ServiceCommand::Choose { .. } => {
                matches!(kind, ServiceKind::Trainer)
            }
            ServiceCommand::Learn { .. } => {
                matches!(kind, ServiceKind::Guild | ServiceKind::Temple)
            }
        }
    }
}

/// A command that passed every check, with its price.
pub(crate) enum Deal {
    Room {
        cost: u32,
        minutes: u32,
    },
    Rumor {
        index: u16,
        ago: i64,
    },
    Food {
        count: u16,
        cost: u32,
    },
    Heal {
        index: usize,
        missing: i32,
        cost: u32,
    },
    Cure {
        index: usize,
        cost: u32,
    },
    Raise {
        index: usize,
        cost: u32,
    },
    Buy {
        item: ItemId,
        count: u16,
        cost: u32,
    },
    Sell {
        item: ItemId,
        count: u16,
        price: u32,
    },
    Deposit {
        amount: u32,
    },
    Withdraw {
        amount: u32,
    },
    Train {
        index: usize,
        cost: u32,
        /// The member as the level leaves them, worked out on a copy.
        after: Box<Character>,
        gains: Gains,
    },
    Spell {
        index: usize,
        spell: SpellId,
        cost: u32,
        /// A pick owed by a level (a trainer's, free) rather than a purchase.
        pick: bool,
    },
}

impl Deal {
    /// Copper the party pays: the price, or the amount put in the bank.
    pub(crate) const fn cost(&self) -> u32 {
        match *self {
            Deal::Room { cost, .. }
            | Deal::Food { cost, .. }
            | Deal::Heal { cost, .. }
            | Deal::Cure { cost, .. }
            | Deal::Raise { cost, .. }
            | Deal::Buy { cost, .. }
            | Deal::Train { cost, .. }
            | Deal::Spell { cost, .. } => cost,
            Deal::Deposit { amount } => amount,
            Deal::Rumor { .. } | Deal::Sell { .. } | Deal::Withdraw { .. } => 0,
        }
    }

    /// Copper the party is paid for a sale.
    pub(crate) const fn paid(&self) -> Option<u32> {
        match *self {
            Deal::Sell { price, .. } => Some(price),
            _ => None,
        }
    }
}

/// Go into the service on the party's tile, if there is one. Whether the party went in.
pub(crate) fn enter_here(world: &mut World, data: &Data, events: &mut Vec<Event>) -> bool {
    let p = world.position;
    let Some(service) = data.maps.get(&p.map).and_then(|m| m.site_at(p.x, p.y)) else {
        return false;
    };
    let Some(def) = data.services.get(&service) else {
        return false;
    };
    world.mode = Mode::Town(ServiceState {
        service,
        kind: def.kind,
    });
    events.push(Event::ServiceEntered { service });
    true
}

/// Apply a command inside `state`'s service.
pub(crate) fn apply(
    world: &mut World,
    data: &Data,
    state: ServiceState,
    command: ServiceCommand,
    events: &mut Vec<Event>,
) -> Result<(), Rejection> {
    if command == ServiceCommand::Leave {
        world.mode = Mode::Explore;
        events.push(Event::ServiceLeft {
            service: state.service,
        });
        return Ok(());
    }
    let def = data
        .services
        .get(&state.service)
        .filter(|_| command.offered_in(state.kind))
        .ok_or(Rejection::NotOffered)?;
    let mut roller = Roller::take_stream(world, "town");
    let deal = quote(world, data, def, command, &mut roller)?;
    afford(world, &deal)?;
    roller.store(world);
    settle(world, data, state.service, deal, events);
    Ok(())
}

/// Every check for `command` but the money, and its price; nothing changes. [`afford`] is the
/// last check; the service view shows a price the party cannot pay with that refusal.
pub(crate) fn quote(
    world: &World,
    data: &Data,
    def: &ServiceDef,
    command: ServiceCommand,
    roller: &mut Roller,
) -> Result<Deal, Rejection> {
    let deal = match command {
        ServiceCommand::Leave => return Err(Rejection::NotOffered),
        ServiceCommand::Room => room(world, data, roller)?,
        ServiceCommand::Rumor => {
            // Only what has happened by the region's clock is told (M8).
            let now = crate::time::region_clock(world, data);
            let told: Vec<u16> = (0..def.rumors.len())
                .filter(|i| def.rumors[*i].at <= now)
                .filter_map(|i| u16::try_from(i).ok())
                .collect();
            let rows = u32::try_from(told.len()).unwrap_or(u32::MAX);
            if rows == 0 {
                return Err(Rejection::NotOffered);
            }
            let pick = usize::try_from(roller.rng.below(rows)).unwrap_or(0);
            let index = told[pick];
            let ago = now - def.rumors[usize::from(index)].at;
            Deal::Rumor { index, ago }
        }
        ServiceCommand::BuyFood { count } => {
            nonzero(u32::from(count))?;
            world
                .party
                .food
                .checked_add(u32::from(count))
                .ok_or(Rejection::OutOfRange)?;
            let cost = price(data, "tavern.food_cost", &[("count", count.into())], roller)?;
            Deal::Food { count, cost }
        }
        ServiceCommand::Heal { member } | ServiceCommand::Cure { member } => {
            temple(world, data, command, member, roller)?
        }
        ServiceCommand::Raise { member } => {
            let index = living_or_dead(world, member)?;
            let who = &world.party.members[index];
            if !is_dead(who, data) {
                return Err(Rejection::NotDead { index: member });
            }
            let level = [("level", who.level.into())];
            let cost = price(data, "temple.raise_cost", &level, roller)?;
            Deal::Raise { index, cost }
        }
        ServiceCommand::Buy { item, count } => buy(world, data, def, item, count, roller)?,
        ServiceCommand::Sell { item, count } => sell(world, data, item, count, roller)?,
        ServiceCommand::Deposit { amount } | ServiceCommand::Withdraw { amount } => {
            bank(world, command, amount)?
        }
        ServiceCommand::Train { member } => {
            let index = alive(world, data, member)?;
            service_level::train(world, data, index, roller)?
        }
        ServiceCommand::Choose { member, spell } => {
            let index = alive(world, data, member)?;
            service_level::choose(world, data, index, spell)?
        }
        ServiceCommand::Learn { member, spell } => {
            let index = alive(world, data, member)?;
            service_level::learn(world, data, def, index, spell, roller)?
        }
    };
    Ok(deal)
}

/// Refused when the party cannot pay what `deal` costs.
pub(crate) fn afford(world: &World, deal: &Deal) -> Result<(), Rejection> {
    let cost = deal.cost();
    if cost > world.party.gold {
        return Err(Rejection::CannotAfford {
            cost,
            gold: world.party.gold,
        });
    }
    Ok(())
}

/// Copper into or out of the bank: the purse pays a deposit (checked with every other price),
/// the balance a withdrawal, and neither side may overflow.
fn bank(world: &World, command: ServiceCommand, amount: u32) -> Result<Deal, Rejection> {
    nonzero(amount)?;
    let party = &world.party;
    if let ServiceCommand::Deposit { .. } = command {
        party
            .bank
            .checked_add(amount)
            .ok_or(Rejection::OutOfRange)?;
        return Ok(Deal::Deposit { amount });
    }
    if amount > party.bank {
        return Err(Rejection::BankShort {
            amount,
            bank: party.bank,
        });
    }
    party
        .gold
        .checked_add(amount)
        .ok_or(Rejection::OutOfRange)?;
    Ok(Deal::Withdraw { amount })
}

/// A room, unless the last long rest ended less than `long_rest_every_minutes` before this
/// one would end (SRD: one long rest in 24 hours).
fn room(world: &World, data: &Data, roller: &mut Roller) -> Result<Deal, Rejection> {
    let minutes = rest::long_rest_minutes(data);
    rest::too_soon(world, data, minutes)?;
    let members = i64::try_from(world.party.members.len()).unwrap_or(i64::MAX);
    let cost = price(data, "inn.room_cost", &[("members", members)], roller)?;
    Ok(Deal::Room { cost, minutes })
}

/// Healing or curing a member who is alive and has something to treat.
fn temple(
    world: &World,
    data: &Data,
    command: ServiceCommand,
    member: u8,
    roller: &mut Roller,
) -> Result<Deal, Rejection> {
    let index = alive(world, data, member)?;
    let who = &world.party.members[index];
    if let ServiceCommand::Heal { .. } = command {
        let missing = who.hp_max - who.hp;
        if missing <= 0 {
            return Err(Rejection::NothingToTreat { index: member });
        }
        let cost = price(
            data,
            "temple.heal_cost",
            &[("missing", missing.into())],
            roller,
        )?;
        return Ok(Deal::Heal {
            index,
            missing,
            cost,
        });
    }
    if curable(who, data).next().is_none() {
        return Err(Rejection::NothingToTreat { index: member });
    }
    let cost = price(data, "temple.cure_cost", &[], roller)?;
    Ok(Deal::Cure { index, cost })
}

/// A member's conditions a temple cures: all but death and unconsciousness.
fn curable<'a>(
    who: &'a omnis_rules::Character,
    data: &Data,
) -> impl Iterator<Item = omnis_core::ConditionId> + 'a {
    let kept = [
        omnis_rules::condition_id(data, "dead"),
        omnis_rules::condition_id(data, "unconscious"),
    ];
    who.conditions
        .iter()
        .copied()
        .filter(move |c| !kept.contains(&Some(*c)))
}

/// Items from the smith's stock.
fn buy(
    world: &World,
    data: &Data,
    def: &ServiceDef,
    row: u8,
    count: u16,
    roller: &mut Roller,
) -> Result<Deal, Rejection> {
    let item = def
        .items
        .get(usize::from(row))
        .and_then(|name| data.registry.items.get(name))
        .ok_or(Rejection::NotOffered)?;
    nonzero(u32::from(count))?;
    count_of(&world.party.inventory, item)
        .checked_add(count)
        .ok_or(Rejection::OutOfRange)?;
    let each = price(
        data,
        "shop.buy_price",
        &[("cost_cp", list_price(data, item))],
        roller,
    )?;
    let cost = each
        .checked_mul(u32::from(count))
        .ok_or(Rejection::OutOfRange)?;
    Ok(Deal::Buy { item, count, cost })
}

/// Items from the party's stores.
fn sell(
    world: &World,
    data: &Data,
    row: u8,
    count: u16,
    roller: &mut Roller,
) -> Result<Deal, Rejection> {
    let (item, have) = *world
        .party
        .inventory
        .get(usize::from(row))
        .ok_or(Rejection::NotInStores { item: row })?;
    nonzero(u32::from(count))?;
    if have < count {
        return Err(Rejection::NotEnough { item, have });
    }
    let each = price(
        data,
        "shop.sell_price",
        &[("cost_cp", list_price(data, item))],
        roller,
    )?;
    let price = each
        .checked_mul(u32::from(count))
        .ok_or(Rejection::OutOfRange)?;
    world
        .party
        .gold
        .checked_add(price)
        .ok_or(Rejection::OutOfRange)?;
    Ok(Deal::Sell { item, count, price })
}

/// Carry out a validated deal.
fn settle(world: &mut World, data: &Data, service: ServiceId, deal: Deal, events: &mut Vec<Event>) {
    let party = &mut world.party;
    match deal {
        Deal::Room { cost, minutes } => {
            party.gold -= cost;
            events.push(Event::RoomTaken { cost });
            advance(world, data, minutes, events);
            rest::long_rest_restore(world, data, events);
            world.party.last_long_rest = Some(world.party_clock().elapsed);
            return;
        }
        Deal::Rumor { index, ago } => events.push(Event::Rumor {
            service,
            index,
            ago,
        }),
        Deal::Food { count, cost } => {
            party.gold -= cost;
            party.food += u32::from(count);
            events.push(Event::FoodBought { count, cost });
        }
        Deal::Heal {
            index,
            missing,
            cost,
        } => {
            party.gold -= cost;
            let member = party.members[index].id;
            events.push(Event::Treated { member, cost });
            party::heal(world, data, index, Vec::new(), missing.into(), events);
        }
        Deal::Cure { index, cost } => {
            party.gold -= cost;
            let member = &mut party.members[index];
            events.push(Event::Treated {
                member: member.id,
                cost,
            });
            let cured: Vec<_> = curable(member, data).collect();
            for condition in cured {
                party::set_condition_id(member, condition, false, events);
            }
        }
        Deal::Raise { index, cost } => {
            party.gold -= cost;
            let member = &mut party.members[index];
            events.push(Event::Raised {
                member: member.id,
                cost,
            });
            party::set_condition(member, data, "dead", false, events);
            let gain = 1 - member.hp;
            party::heal(world, data, index, Vec::new(), gain.into(), events);
        }
        Deal::Buy { item, count, cost } => {
            party.gold -= cost;
            add_to(&mut party.inventory, item, count);
            events.push(Event::Bought { item, count, cost });
        }
        Deal::Sell { item, count, price } => {
            // Validated: the stores hold `count`.
            let _ = take_from(&mut party.inventory, item, count);
            party.gold += price;
            events.push(Event::Sold { item, count, price });
        }
        Deal::Deposit { amount } => {
            party.gold -= amount;
            party.bank += amount;
            events.push(Event::Banked {
                amount,
                deposit: true,
            });
        }
        Deal::Withdraw { amount } => {
            party.bank -= amount;
            party.gold += amount;
            events.push(Event::Banked {
                amount,
                deposit: false,
            });
        }
        Deal::Train {
            index,
            cost,
            after,
            gains,
        } => {
            party.gold -= cost;
            party.members[index] = *after;
            let member = &party.members[index];
            events.push(Event::LevelUp {
                member: member.id,
                level: member.level,
                cost,
                gains,
            });
        }
        Deal::Spell {
            index,
            spell,
            cost,
            pick,
        } => {
            party.gold -= cost;
            let member = &mut party.members[index];
            member.known_spells.push(spell);
            events.push(Event::SpellLearned {
                member: member.id,
                spell,
                cost,
            });
            if pick {
                // Validated: at least one pick is left. A pick takes no time.
                member.spell_picks -= 1;
                return;
            }
        }
    }
    let minutes = rest::rule_minutes(data, "service_minutes", DEFAULT_SERVICE_MINUTES);
    advance(world, data, minutes, events);
}

/// A price from its rule slot, in copper.
pub(crate) fn price(
    data: &Data,
    slot: &str,
    inputs: &[(&str, i64)],
    roller: &mut Roller,
) -> Result<u32, Rejection> {
    let inputs: Vec<(&str, Value)> = inputs.iter().map(|(n, v)| (*n, Value::Int(*v))).collect();
    let outcome = data
        .rules
        .eval(slot, &inputs, &mut roller.rng, &roller.stream)
        .map_err(Rejection::Rule)?;
    match outcome.value {
        Value::Int(cp) => u32::try_from(cp).map_err(|_| {
            Rejection::Rule(RuleError::new(slot, "the price is not 0..=u32::MAX copper"))
        }),
        _ => Err(Rejection::Rule(RuleError::new(
            slot,
            "the price is not an integer",
        ))),
    }
}

/// An item's list price in copper, as the slot's `cost_cp` input.
fn list_price(data: &Data, item: ItemId) -> i64 {
    data.items.get(&item).map_or(0, |i| i64::from(i.cost_cp))
}

/// A member's index by slot, refused when they are dead.
fn alive(world: &World, data: &Data, member: u8) -> Result<usize, Rejection> {
    let index = living_or_dead(world, member)?;
    if is_dead(&world.party.members[index], data) {
        return Err(Rejection::MemberDead { index: member });
    }
    Ok(index)
}

/// A member's index by slot, alive or dead.
fn living_or_dead(world: &World, member: u8) -> Result<usize, Rejection> {
    let index = usize::from(member);
    if index < world.party.members.len() {
        Ok(index)
    } else {
        Err(Rejection::NoSuchMember { index: member })
    }
}

/// A count or amount of zero moves nothing.
const fn nonzero(n: u32) -> Result<(), Rejection> {
    if n == 0 {
        Err(Rejection::ZeroCount)
    } else {
        Ok(())
    }
}
