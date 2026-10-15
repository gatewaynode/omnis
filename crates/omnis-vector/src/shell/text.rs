//! HUD text for simulation events. Town lines are the retired client's (`service_text.rs`), in
//! their long form, with prices in every coin.

use crate::labels;
use crate::rolllog::Names;
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_data::EquipSlot;
use omnis_sim::omnis_rules::Gains;
use omnis_sim::{
    BlockReason, CombatOutcome, DevCommand, EffectEnd, EffectTarget, Event, ItemPlace,
};

/// A HUD line for the events worth showing; the per-step chatter is left out. Members are
/// named through `names`, so a line still reads after its member has left the party.
#[must_use]
pub fn describe(event: &Event, names: &Names, data: &Data) -> Option<String> {
    if let Some(line) = town(event, names, data)
        .or_else(|| kit_and_spells(event, names, data))
        .or_else(|| camp(event, names))
    {
        return Some(line);
    }
    if let Event::Dev { command } = event {
        return Some(dev(command, names, data));
    }
    Some(match event {
        Event::Moved { from, to } => {
            // A change of level is named (M8.5 step 1b): a stair, a hole, a ladder.
            let level = data
                .registry
                .maps
                .get(&to.map)
                .and_then(|id| data.maps.get(&id))
                .filter(|_| from.level != to.level)
                .and_then(|map| crate::labels::level(data, map, to.level))
                .map_or_else(String::new, |l| format!(" on {l}"));
            format!(
                "Moved to ({}, {}){level} facing {:?}",
                to.x, to.y, to.facing
            )
        }
        Event::Blocked { reason } => format!(
            "Blocked: {}",
            match reason {
                BlockReason::Wall => "a wall",
                BlockReason::ClosedDoor => "a closed door",
                BlockReason::Impassable => "impassable ground",
                BlockReason::MapEdge => "the map edge",
                BlockReason::Cliff => "too steep",
                BlockReason::NoGround => "no ground",
                BlockReason::Object => "something in the way",
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
            CombatOutcome::Victory => format!("Victory! {xp} XP each, {}", labels::gold(*gold)),
            CombatOutcome::Fled => "The party got away".to_owned(),
            CombatOutcome::Defeat => "The party has fallen".to_owned(),
        },
        Event::Message { key } => labels::message(data, *key).to_owned(),
        _ => return None,
    })
}

/// A line for an item or a spell event (the retired client's `item_text.rs` and `spell_text.rs`,
/// the short form). An effect lapsing at a turn or with a fight is the fight log's alone.
fn kit_and_spells(event: &Event, names: &Names, data: &Data) -> Option<String> {
    let who = |m: &CharacterId| names.member(*m);
    let item = |id: &str| labels::item(data, id).to_owned();
    let spell = |id: &str| labels::spell(data, id).to_owned();
    Some(match event {
        Event::Equipped {
            member,
            slot,
            item: it,
        } => {
            let verb = if *slot == EquipSlot::Body {
                "wears"
            } else {
                "wields"
            };
            format!("{} {verb} {}", who(member), item(it))
        }
        Event::Unequipped {
            member, item: it, ..
        } => {
            format!("{} takes off {}", who(member), item(it))
        }
        Event::ItemMoved {
            item: it,
            count,
            from,
            to,
        } => {
            let what = if *count == 1 {
                item(it)
            } else {
                format!("{count} {}", item(it))
            };
            match (from, to) {
                (ItemPlace::Member(f), ItemPlace::Member(t)) => {
                    format!("{} gives {what} to {}", who(f), who(t))
                }
                (ItemPlace::Member(f), ItemPlace::Stores) => format!("{} stows {what}", who(f)),
                (ItemPlace::Stores, ItemPlace::Member(t)) => format!("{} takes {what}", who(t)),
                (ItemPlace::Member(f), ItemPlace::Ground) => format!("{} drops {what}", who(f)),
                (ItemPlace::Ground, ItemPlace::Member(t)) => format!("{} picks up {what}", who(t)),
                (ItemPlace::Member(f), ItemPlace::Object(_)) => {
                    format!("{} puts {what} in", who(f))
                }
                (ItemPlace::Object(_), ItemPlace::Member(t)) => {
                    format!("{} takes {what} out", who(t))
                }
                (ItemPlace::Stores, ItemPlace::Stores) => format!("{what} stays in the stores"),
                // The engine moves nothing else between these.
                _ => format!("{what} is moved"),
            }
        }
        // A look has no receiver; its `Sensed` line is the outcome.
        Event::ItemUsed { receiver: None, .. } => return None,
        Event::ItemUsed {
            member,
            item: it,
            receiver: Some(r),
            ..
        } => {
            if r == member {
                format!("{} uses {}", who(member), item(it))
            } else {
                format!("{} uses {} on {}", who(member), item(it), who(r))
            }
        }
        Event::SpellCast {
            caster,
            spell: s,
            points,
            components_consumed,
        } => {
            let cost = if *points == 0 {
                "free".to_owned()
            } else {
                format!("{points} pt")
            };
            let parts = match components_consumed.len() {
                0 => String::new(),
                n => format!(", {n} component(s)"),
            };
            format!("{} casts {} ({cost}{parts})", who(caster), spell(s))
        }
        Event::Healed {
            target, amount, hp, ..
        } => {
            format!("{} regains {amount} hp, now {hp}", who(target))
        }
        Event::EffectApplied {
            target, spell: s, ..
        } => match target {
            EffectTarget::Member(m) => format!("{} settles on {}", spell(s), who(m)),
            EffectTarget::Party => format!("{} lights the party's way", spell(s)),
        },
        Event::EffectEnded {
            target,
            spell: s,
            why,
        } => {
            let whom = match target {
                EffectTarget::Member(m) => who(m).to_owned(),
                EffectTarget::Party => "the party".to_owned(),
            };
            match why {
                EffectEnd::Expired => format!("{} fades from {whom}", spell(s)),
                EffectEnd::Consumed => format!("{} is spent on {whom}", spell(s)),
                EffectEnd::Concentration => format!("{} leaves {whom}", spell(s)),
                EffectEnd::TurnBegan | EffectEnd::FightOver => return None,
            }
        }
        Event::Concentration {
            caster, spell: s, ..
        } => {
            format!("{} lets {} go", who(caster), spell(s))
        }
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

/// A line for a rest or a change of tactics (the retired client's `service_text.rs` and
/// `spell_text.rs`).
fn camp(event: &Event, names: &Names) -> Option<String> {
    Some(match event {
        Event::Rested {
            long: true, food, ..
        } => format!("The party rests for the night and eats {food} food"),
        Event::Rested { minutes, .. } => format!("The party rests for {}", span(*minutes)),
        Event::HitDiceSpent { member, dice } => format!(
            "{} spends {dice} hit {}",
            names.member(*member),
            if *dice == 1 { "die" } else { "dice" }
        ),
        Event::RestInterrupted { minutes } => format!("Ambushed after {}!", span(*minutes)),
        Event::ReactionsSwitched { member, on } => format!(
            "{}'s reactions are {}",
            names.member(*member),
            if *on { "on" } else { "off" }
        ),
        Event::TacticsChanged { member } => format!("{}'s tactics are set", names.member(*member)),
        _ => return None,
    })
}

/// A debugging edit, as the retired client's debug panel logged it, with names for numbers.
fn dev(command: &DevCommand, names: &Names, data: &Data) -> String {
    let who = |m: &CharacterId| names.member(*m);
    match command {
        DevCommand::GiveItem {
            member,
            item,
            count,
        } => {
            let what = labels::count_label(labels::item(data, item), *count);
            match member {
                Some(m) => format!("dev: {what} to {}", who(m)),
                None => format!("dev: {what} to the stores"),
            }
        }
        DevCommand::SetHp { member, hp } => format!("dev: {} hp {hp}", who(member)),
        DevCommand::SetSpellPoints { member, points } => {
            format!("dev: {} sp {points}", who(member))
        }
        DevCommand::SetXp { member, xp } => format!("dev: {} xp {xp}", who(member)),
        DevCommand::SetScore {
            member,
            ability,
            score,
        } => format!("dev: {} {} {score}", who(member), ability.short()),
        DevCommand::SetCondition {
            member,
            condition,
            applied,
        } => format!(
            "dev: {} {} {}",
            who(member),
            labels::condition(data, condition),
            if *applied { "on" } else { "off" }
        ),
        DevCommand::SetGold { gold } => format!("dev: gold {}", labels::coins(*gold)),
        DevCommand::SetFood { food } => format!("dev: food {food}"),
        DevCommand::SetFlag { flag, value } => format!("dev: {flag} = {value}"),
        DevCommand::Teleport {
            map,
            level,
            x,
            y,
            facing,
        } => format!(
            "dev: teleport to {}{} ({x}, {y}) facing {facing}",
            labels::map(data, map),
            if level.is_ground() {
                String::new()
            } else {
                format!(" level {level}")
            }
        ),
        DevCommand::SetMonsterHp { stack, index, hp } => format!(
            "dev: stack {} #{} hp {hp}",
            u16::from(*stack) + 1,
            u16::from(*index) + 1
        ),
        DevCommand::KillStack { stack } => format!("dev: stack {} slain", u16::from(*stack) + 1),
        DevCommand::Reconcile { region } => format!("dev: reconcile with {region}"),
    }
}

/// A stretch of party-clock time: "an hour", "3 hours", "40 minutes".
fn span(minutes: u32) -> String {
    match minutes {
        60 => "an hour".to_owned(),
        m if m % 60 == 0 => format!("{} hours", m / 60),
        m => format!("{m} minutes"),
    }
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
