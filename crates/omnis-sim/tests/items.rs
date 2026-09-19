//! Item stock: counts in a member's kit and the party's stores, the components a cast
//! consumes, the gem item by name, and healing shared by potions and spells.

mod common;

use common::{data, party_of, world};
use omnis_sim::items::{consume, count_of, has_all, item_id};
use omnis_sim::omnis_rules::DeathSaves;
use omnis_sim::party::heal;
use omnis_sim::{Event, Party};

#[test]
fn stores_are_counted_and_consumed_exactly_or_not_at_all() {
    let data = data();
    let gem = item_id(&data, "gem").unwrap();
    assert_eq!(data.registry.items.name(gem), Some("base:item:gem"));
    assert_eq!(item_id(&data, "philosopher_stone"), None);
    let potion = item_id(&data, "potion_of_healing").unwrap();
    let mut party = Party {
        inventory: vec![(gem, 3), (potion, 1)],
        ..Party::default()
    };
    assert_eq!(count_of(&party.inventory, gem), 3);
    assert_eq!(
        count_of(&party.inventory, item_id(&data, "dagger").unwrap()),
        0
    );
    assert!(has_all(&party, &[(gem, 3), (potion, 1)]));
    assert!(!has_all(&party, &[(gem, 4)]));
    let before = party.clone();
    let refused = consume(&mut party, &[(gem, 2), (potion, 2)]).unwrap_err();
    assert_eq!(
        refused,
        omnis_sim::Rejection::NotEnough {
            item: potion,
            have: 1
        }
    );
    assert_eq!(party, before, "a refusal takes nothing, not even the gems");
    consume(&mut party, &[(gem, 2)]).unwrap();
    assert_eq!(party.inventory, [(gem, 1), (potion, 1)]);
    consume(&mut party, &[(gem, 1), (potion, 1)]).unwrap();
    assert!(party.inventory.is_empty(), "empty entries are dropped");
    assert!(has_all(&party, &[]));
}

#[test]
fn healing_caps_at_the_maximum_and_gets_a_downed_member_up() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 1);
    let unconscious = data
        .registry
        .conditions
        .get("base:condition:unconscious")
        .unwrap();
    let max = world.party.members[0].hp_max;
    world.party.members[0].hp = max - 3;
    let mut events = Vec::new();
    heal(&mut world, &data, 0, Vec::new(), 10, &mut events);
    assert_eq!(world.party.members[0].hp, max);
    assert_eq!(
        events,
        [Event::Healed {
            target: world.party.members[0].id,
            rolls: Vec::new(),
            amount: 10,
            hp: max
        }]
    );
    let member = &mut world.party.members[0];
    member.hp = 0;
    member.conditions.push(unconscious);
    member.death_saves.failures = 2;
    let mut events = Vec::new();
    heal(&mut world, &data, 0, Vec::new(), 4, &mut events);
    let member = &world.party.members[0];
    assert_eq!(member.hp, 4);
    assert!(!member.conditions.contains(&unconscious));
    assert_eq!(member.death_saves, DeathSaves::default());
    assert!(matches!(events[1], Event::Condition { applied: false, .. }));
    let mut events = Vec::new();
    heal(&mut world, &data, 0, Vec::new(), -5, &mut events);
    assert_eq!(world.party.members[0].hp, 4, "healing never hurts");
}
