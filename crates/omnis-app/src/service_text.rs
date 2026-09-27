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
            Line::same(format!("The party enters the {}", names.service(*service)))
        }
        Event::ServiceLeft { service } => {
            Line::same(format!("The party leaves the {}", names.service(*service)))
        }
        Event::RoomTaken { cost } => Line::new(
            format!("A night's rest for {}", coins(*cost)),
            "A night's rest".to_owned(),
        ),
        Event::FoodBought { count, cost } => {
            Line::same(format!("Bought {count} food for {}", coins(*cost)))
        }
        Event::Rumor { service, index } => {
            Line::same(format!("\"{}\"", names.rumor(*service, *index)))
        }
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
            names.item(*item),
            coins(*cost)
        )),
        Event::Sold { item, count, price } => Line::same(format!(
            "Sold {count} {} for {}",
            names.item(*item),
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
        Event::RestEvent { map, index } => Line::same(names.rest_event(*map, *index).to_owned()),
        _ => return None,
    })
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
        let names = Names::new(&world, &data);
        let lines: Vec<String> = batch_lines(&events, &names)
            .into_iter()
            .map(|l| l.long)
            .collect();
        assert_eq!(lines[0], "The party enters the Tavern");
        assert_eq!(lines[1], "Bought 3 food for 1 gp 5 sp 0 cp");
        assert!(
            [
                "\"They say the rats below grow bolder every week.\"",
                "\"Old bones walk where the lamps have gone out.\"",
                "\"Goblins carry their coin with them. Take it back.\""
            ]
            .contains(&lines[2].as_str()),
            "{lines:?}"
        );
        assert_eq!(lines[3], "The party leaves the Tavern");
        assert_eq!(lines.len(), 4, "{lines:?}");
    }

    #[test]
    fn every_town_line_fits_and_other_events_are_not_town_lines() {
        let data = packs();
        let world = World::new(&data, 3, Settings::default()).unwrap();
        let names = Names::new(&world, &data);
        let guild = data.registry.services.get("base:service:guild").unwrap();
        let item = data.registry.items.get("base:item:map_making_kit").unwrap();
        let most = u32::MAX;
        let events = [
            Event::ServiceEntered { service: guild },
            Event::ServiceLeft { service: guild },
            Event::RoomTaken { cost: most },
            Event::FoodBought {
                count: u16::MAX,
                cost: most,
            },
            Event::Bought {
                item,
                count: u16::MAX,
                cost: most,
            },
            Event::Sold {
                item,
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
    fn rest_lines_read_and_fit() {
        let data = packs();
        let world = World::new(&data, 3, Settings::default()).unwrap();
        let names = Names::new(&world, &data);
        let meadow = data.registry.maps.get("test:map:meadow").unwrap();
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
                    map: meadow,
                    index: 0,
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
