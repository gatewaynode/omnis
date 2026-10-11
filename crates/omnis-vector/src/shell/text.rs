//! HUD text for simulation events. Town lines are the other client's (`service_text.rs`), in
//! their long form, with prices in every coin.

use crate::labels;
use crate::rolllog::Names;
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_rules::Gains;
use omnis_sim::{BlockReason, CombatOutcome, Event};

/// A HUD line for the events worth showing; the per-step chatter is left out. Members are
/// named through `names`, so a line still reads after its member has left the party.
#[must_use]
pub fn describe(event: &Event, names: &Names, data: &Data) -> Option<String> {
    if let Some(line) = town(event, names, data) {
        return Some(line);
    }
    Some(match event {
        Event::Moved { to, .. } => format!("Moved to ({}, {}) facing {:?}", to.x, to.y, to.facing),
        Event::Blocked { reason } => format!(
            "Blocked: {}",
            match reason {
                BlockReason::Wall => "a wall",
                BlockReason::ClosedDoor => "a closed door",
                BlockReason::Impassable => "impassable ground",
                BlockReason::MapEdge => "the map edge",
            }
        ),
        Event::Door { open, .. } => if *open {
            "The door opens"
        } else {
            "The door closes"
        }
        .to_owned(),
        Event::EncounterStarted { .. } => "Monsters ahead!".to_owned(),
        Event::Bribed { cost } => format!("Paid {}; the monsters leave", labels::gold(*cost)),
        Event::CombatStarted { .. } => "The fight begins".to_owned(),
        Event::CombatEnded {
            outcome, xp, gold, ..
        } => match outcome {
            CombatOutcome::Victory => format!("Victory: {xp} xp each, {}", labels::gold(*gold)),
            CombatOutcome::Fled => "The party got away".to_owned(),
            CombatOutcome::Defeat => "The party has fallen".to_owned(),
        },
        Event::Message { key } => labels::message(data, *key).to_owned(),
        _ => return None,
    })
}

/// A line for a town event: going in and out of a service and what the party did inside.
fn town(event: &Event, names: &Names, data: &Data) -> Option<String> {
    let coins = |cp: &u32| labels::coins(*cp);
    Some(match event {
        Event::ServiceEntered { service } => {
            format!("The party enters the {}", labels::service(data, service))
        }
        Event::ServiceLeft { service } => {
            format!("The party leaves the {}", labels::service(data, service))
        }
        Event::RoomTaken { cost } => format!("A night's rest for {}", coins(cost)),
        Event::FoodBought { count, cost } => format!("Bought {count} food for {}", coins(cost)),
        Event::Rumor {
            service,
            rumor,
            ago,
        } => format!("\"{}\"", labels::rumor(data, service, *rumor, *ago)),
        Event::Treated { member, cost } => {
            format!("{} is treated for {}", names.member(*member), coins(cost))
        }
        Event::Raised { member, cost } => {
            format!("{} is raised for {}", names.member(*member), coins(cost))
        }
        Event::Bought { item, count, cost } => format!(
            "Bought {count} {} for {}",
            labels::item(data, item),
            coins(cost)
        ),
        Event::Sold { item, count, price } => format!(
            "Sold {count} {} for {}",
            labels::item(data, item),
            coins(price)
        ),
        Event::Banked {
            amount,
            deposit: true,
        } => format!("Deposited {}", coins(amount)),
        Event::Banked {
            amount,
            deposit: false,
        } => format!("Withdrew {}", coins(amount)),
        Event::LevelUp {
            member,
            level,
            gains,
            ..
        } => level_line(names, data, *member, *level, gains),
        Event::SpellLearned {
            member,
            spell,
            cost: 0,
        } => format!(
            "{} chooses {}",
            names.member(*member),
            labels::spell(data, spell)
        ),
        Event::SpellLearned {
            member,
            spell,
            cost,
        } => format!(
            "{} learns {} for {}",
            names.member(*member),
            labels::spell(data, spell),
            coins(cost)
        ),
        _ => return None,
    })
}

/// A level gained and what it brought: "Brenna reaches level 2: +8 HP, Action Surge". The
/// price shows on the money line.
fn level_line(names: &Names, data: &Data, member: CharacterId, level: u8, gains: &Gains) -> String {
    let mut brought = vec![format!("+{} HP", gains.hp)];
    if gains.spell_points > 0 {
        brought.push(format!("+{} SP", gains.spell_points));
    }
    match gains.picks {
        0 => {}
        1 => brought.push("1 pick".to_owned()),
        n => brought.push(format!("{n} picks")),
    }
    brought.extend(
        gains
            .features
            .iter()
            .map(|f| labels::text(data, f).to_owned()),
    );
    format!(
        "{} reaches level {level}: {}",
        names.member(member),
        brought.join(", ")
    )
}
