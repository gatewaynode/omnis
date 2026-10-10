//! Town and rest events as text: going in and out of a service, what the party did inside, and
//! resting outside. Sits beside `item_text.rs`; the restoration a room, a temple or a rest
//! brings reads through the healing and condition lines that follow. Prices show every coin
//! (`text::coins`). Bevy-free.

use crate::text::{Line, Names, coins};
use omnis_sim::Event;

/// One line for a town event, or `None` for events this file does not render.
#[must_use]
pub fn service_line(event: &Event, names: &Names) -> Option<Line> {
    Some(match event {
        Event::ServiceEntered { service } => {
            Line::same(format!("The party enters the {}", names.service(service)))
        }
        Event::ServiceLeft { service } => {
            Line::same(format!("The party leaves the {}", names.service(service)))
        }
        Event::RoomTaken { cost } => Line::new(
            format!("A night's rest for {}", coins(*cost)),
            "A night's rest".to_owned(),
        ),
        Event::FoodBought { count, cost } => {
            Line::same(format!("Bought {count} food for {}", coins(*cost)))
        }
        Event::Rumor {
            service,
            rumor,
            ago,
        } => Line::same(format!("\"{}\"", names.rumor(service, *rumor, *ago))),
        Event::Treated { member, cost } => Line::same(format!(
            "{} is treated for {}",
            names.member(*member),
            coins(*cost)
        )),
        Event::Raised { member, cost } => Line::same(format!(
            "{} is raised for {}",
            names.member(*member),
            coins(*cost)
        )),
        Event::Bought { item, count, cost } => Line::same(format!(
            "Bought {count} {} for {}",
            names.item(item),
            coins(*cost)
        )),
        Event::Sold { item, count, price } => Line::same(format!(
            "Sold {count} {} for {}",
            names.item(item),
            coins(*price)
        )),
        Event::Banked {
            amount,
            deposit: true,
        } => Line::same(format!("Deposited {}", coins(*amount))),
        Event::Banked {
            amount,
            deposit: false,
        } => Line::same(format!("Withdrew {}", coins(*amount))),
        Event::Rested {
            long: true, food, ..
        } => Line::new(
            format!("The party rests for the night and eats {food} food"),
            "The party rests the night".to_owned(),
        ),
        Event::Rested { minutes, .. } => {
            Line::same(format!("The party rests for {}", span(*minutes)))
        }
        Event::HitDiceSpent { member, dice } => Line::same(format!(
            "{} spends {dice} hit {}",
            names.member(*member),
            if *dice == 1 { "die" } else { "dice" }
        )),
        Event::RestInterrupted { minutes } => Line::new(
            format!("Ambushed after {}!", span(*minutes)),
            "Ambushed!".to_owned(),
        ),
        Event::RestEvent { map, entry } => Line::same(names.rest_event(map, *entry).to_owned()),
        Event::LevelUp {
            member,
            level,
            gains,
            ..
        } => level_line(names, *member, *level, gains),
        Event::SpellLearned {
            member,
            spell,
            cost: 0,
        } => Line::same(format!(
            "{} chooses {}",
            names.member(*member),
            names.spell(spell)
        )),
        Event::SpellLearned {
            member,
            spell,
            cost,
        } => Line::new(
            format!(
                "{} learns {} for {}",
                names.member(*member),
                names.spell(spell),
                coins(*cost)
            ),
            format!("{} learns {}", names.member(*member), names.spell(spell)),
        ),
        _ => return None,
    })
}

/// A level gained and what it brought in the long form; the short form names the level only.
/// The price shows on the money line (a full purse, a long name and a feature would not fit).
fn level_line(
    names: &Names,
    member: omnis_sim::omnis_core::CharacterId,
    level: u8,
    gains: &omnis_sim::omnis_rules::Gains,
) -> Line {
    let who = names.member(member);
    let mut brought = vec![format!("+{} HP", gains.hp)];
    if gains.spell_points > 0 {
        brought.push(format!("+{} SP", gains.spell_points));
    }
    match gains.picks {
        0 => {}
        1 => brought.push("1 pick".to_owned()),
        n => brought.push(format!("{n} picks")),
    }
    brought.extend(gains.features.iter().map(|f| names.feature(f).to_owned()));
    Line::new(
        format!("{who} reaches level {level}: {}", brought.join(", ")),
        format!("{who} reaches level {level}"),
    )
}

/// Party-clock minutes as words: "an hour", "3 hours", "40 minutes".
fn span(minutes: u32) -> String {
    match (minutes / 60, minutes % 60) {
        (1, 0) => "an hour".to_owned(),
        (hours, 0) => format!("{hours} hours"),
        _ => format!("{minutes} minutes"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_text::batch_lines;
    use crate::text::{LONG_CELLS, SHORT_CELLS};
    use omnis_sim::omnis_data::load_packs;
    use omnis_sim::{Command, ServiceCommand, Settings, World, apply};
    use std::path::PathBuf;

    fn packs() -> omnis_sim::omnis_data::Data {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        load_packs(&[&repo.join("packs/base"), &repo.join("packs/test")]).unwrap()
    }

    #[test]
    fn a_night_at_the_inn_reads_from_the_door_to_the_price() {
        let data = packs();
        let mut world = World::new(&data, 3, Settings::default()).unwrap();
        world.party.gold = 1537;
        let map = data.registry.maps.get("test:map:town").unwrap();
        world.position.map = map;
        (world.position.x, world.position.y) = (4, 3);
        let mut events = apply(&mut world, &data, Command::Interact).unwrap();
        events.extend(
            apply(
                &mut world,
                &data,
                Command::Service(ServiceCommand::BuyFood { count: 3 }),
            )
            .unwrap(),
        );
        events.extend(apply(&mut world, &data, Command::Service(ServiceCommand::Rumor)).unwrap());
        events.extend(apply(&mut world, &data, Command::Service(ServiceCommand::Leave)).unwrap());
        let names = Names::of_world(&world, &data);
        let lines: Vec<String> = batch_lines(&events, &names)
            .into_iter()
            .map(|l| l.long)
            .collect();
        assert_eq!(lines[0], "The party enters the Tavern");
        assert_eq!(lines[1], "Bought 3 food for 1 gp 5 sp 0 cp");
        assert_eq!(
            lines[2], "\"Talk from today: the rats below grow bolder every week.\"",
            "at the town's origin only the rats are talked of"
        );
        assert_eq!(lines[3], "The party leaves the Tavern");
        assert_eq!(lines.len(), 4, "{lines:?}");
    }

    #[test]
    fn every_town_line_fits_and_other_events_are_not_town_lines() {
        let data = packs();
        let world = World::new(&data, 3, Settings::default()).unwrap();
        let names = Names::of_world(&world, &data);
        let guild = "base:service:guild";
        let item = "base:item:map_making_kit";
        let most = u32::MAX;
        let events = [
            Event::ServiceEntered {
                service: guild.to_owned(),
            },
            Event::ServiceLeft {
                service: guild.to_owned(),
            },
            Event::RoomTaken { cost: most },
            Event::FoodBought {
                count: u16::MAX,
                cost: most,
            },
            Event::Bought {
                item: item.to_owned(),
                count: u16::MAX,
                cost: most,
            },
            Event::Sold {
                item: item.to_owned(),
                count: u16::MAX,
                price: most,
            },
            Event::Banked {
                amount: most,
                deposit: false,
            },
        ];
        for event in &events {
            let line = service_line(event, &names).expect("a town line");
            assert!(line.long.chars().count() <= LONG_CELLS);
            assert!(line.short.chars().count() <= SHORT_CELLS);
        }
        assert_eq!(
            service_line(&events[4], &names).unwrap().long,
            "Bought 65535 Map-making kit for 42949672 gp 9 sp 5 cp"
        );
        assert!(service_line(&Event::PartyChanged, &names).is_none());
    }

    #[test]
    fn a_level_and_a_spell_read_with_what_they_brought() {
        use omnis_sim::omnis_rules::Gains;
        let data = packs();
        let mut world = World::new(&data, 3, Settings::default()).unwrap();
        let mut draft = omnis_sim::omnis_rules::Draft {
            name: "W".repeat(32),
            race: "base:race:human".to_owned(),
            class: "base:class:wizard".to_owned(),
            background: "base:background:acolyte".to_owned(),
            alignment: omnis_sim::omnis_data::Alignment::NeutralGood,
            scores: [8, 14, 13, 15, 12, 10],
            skills: vec![
                omnis_sim::omnis_data::Skill::Arcana,
                omnis_sim::omnis_data::Skill::History,
            ],
        };
        apply(
            &mut world,
            &data,
            Command::Party(omnis_sim::party::PartyCommand::Create(draft.clone())),
        )
        .unwrap();
        draft.name = "Ilvara".to_owned();
        apply(
            &mut world,
            &data,
            Command::Party(omnis_sim::party::PartyCommand::Create(draft)),
        )
        .unwrap();
        let names = Names::of_world(&world, &data);
        let member = world.party.members[0].id;
        let shatter = "base:spell:shatter";
        // The longest a level reads: a 32-byte name, a full caster's gains and the longest
        // feature a class lists.
        let worst = Event::LevelUp {
            member,
            level: 20,
            cost: u32::MAX,
            gains: Gains {
                hp: 12,
                spell_points: 9,
                picks: 2,
                proficiency: 6,
                features: vec!["base:text:class.wizard.ability_score_improvement".to_owned()],
            },
        };
        let line = service_line(&worst, &names).unwrap();
        assert!(line.long.chars().count() <= LONG_CELLS, "{}", line.long);
        assert!(line.short.chars().count() <= SHORT_CELLS, "{}", line.short);
        assert_eq!(
            line.long,
            format!(
                "{} reaches level 20: +12 HP, +9 SP, 2 picks, Ability Score Improvement",
                "W".repeat(32)
            )
        );
        let fighter = Event::LevelUp {
            member,
            level: 2,
            cost: 2000,
            gains: Gains {
                hp: 8,
                spell_points: 0,
                picks: 0,
                proficiency: 2,
                features: vec!["base:text:class.fighter.action_surge".to_owned()],
            },
        };
        assert!(
            service_line(&fighter, &names)
                .unwrap()
                .long
                .ends_with("reaches level 2: +8 HP, Action Surge")
        );
        let ilvara = world.party.members[1].id;
        let pick = Event::SpellLearned {
            member: ilvara,
            spell: shatter.to_owned(),
            cost: 0,
        };
        assert_eq!(
            service_line(&pick, &names).unwrap().long,
            "Ilvara chooses Shatter"
        );
        let bought = Event::SpellLearned {
            member: ilvara,
            spell: shatter.to_owned(),
            cost: 10_000,
        };
        let line = service_line(&bought, &names).unwrap();
        assert_eq!(line.long, "Ilvara learns Shatter for 100 gp 0 sp 0 cp");
        assert_eq!(line.short, "Ilvara learns Shatter");
    }

    #[test]
    fn rest_lines_read_and_fit() {
        let data = packs();
        let world = World::new(&data, 3, Settings::default()).unwrap();
        let names = Names::of_world(&world, &data);
        let meadow = "test:map:meadow";
        let lines = [
            (
                Event::Rested {
                    long: true,
                    minutes: 480,
                    food: 4,
                },
                "The party rests for the night and eats 4 food",
            ),
            (
                Event::Rested {
                    long: false,
                    minutes: 60,
                    food: 0,
                },
                "The party rests for an hour",
            ),
            (
                Event::RestInterrupted { minutes: 180 },
                "Ambushed after 3 hours!",
            ),
            (
                Event::RestInterrupted { minutes: 40 },
                "Ambushed after 40 minutes!",
            ),
            (
                Event::RestEvent {
                    map: meadow.to_owned(),
                    entry: 0,
                },
                "A cart rattles past on the road",
            ),
        ];
        for (event, long) in &lines {
            let line = service_line(event, &names).expect("a rest line");
            assert_eq!(line.long, *long);
            assert!(line.long.chars().count() <= LONG_CELLS);
            assert!(line.short.chars().count() <= SHORT_CELLS, "{}", line.short);
        }
    }
}
