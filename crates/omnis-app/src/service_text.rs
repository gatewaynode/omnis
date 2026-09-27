//! Town events as text: going in and out of a service, and what the party did inside. Sits
//! beside `item_text.rs`; the restoration a room or a temple brings reads through the healing
//! and condition lines that follow. Prices show every coin (`text::coins`). Bevy-free.

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
        _ => return None,
    })
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
}
