//! Loot on the yard fixture (M8.5 step 2a, item 12): the pack rat's hoard drawn by weight and
//! chance, what the dead drop laid on the fight's cell on victory, and left behind on a flight.

use crate::common::{settle, six};
use crate::objects::{in_yard, objects, yard};
use omnis_core::{Facing, ItemId, Level, Pcg32, StreamName};
use omnis_data::{Data, Disposition};
use omnis_sim::combat::{self, loot};
use omnis_sim::ground::pile_at;
use omnis_sim::{
    CombatCommand, CombatOutcome, Command, EncounterSource, EncounterState, Event, Loot, Mode,
    PartyCommand, Stack, Surprise, World, apply,
};

fn item(data: &Data, id: &str) -> ItemId {
    data.registry.items.get(id).expect(id)
}

/// Four members in the yard at (4, 3), the party seeded `seed`.
fn party(data: &Data, seed: u64) -> World {
    let mut world = in_yard(data, 0, 4, 3, Facing::North);
    world.seed = seed;
    for draft in six().into_iter().skip(1).take(3) {
        apply(
            &mut world,
            data,
            Command::Party(PartyCommand::Create(draft)),
        )
        .expect("a draft");
    }
    world
}

/// Pack rats of one hit point each, `count` of them, retreating to (4, 4).
fn rats(data: &Data, world: &World, count: u8, disposition: Disposition) -> EncounterState {
    let monster = data
        .registry
        .monsters
        .get("objects:monster:pack_rat")
        .expect("the pack rat");
    let mut retreat = world.position;
    retreat.y = 4;
    EncounterState {
        source: EncounterSource::Random,
        stacks: vec![Stack {
            monster,
            initial: count,
            hp: vec![1; usize::from(count)],
            spent: Vec::new(),
        }],
        disposition,
        retreat,
    }
}

/// Every item named in the events' deaths, summed by kind.
fn dropped(events: &[Event]) -> Vec<Loot> {
    let mut all: Vec<Loot> = Vec::new();
    for e in events {
        let Event::Death { loot, .. } = e else {
            continue;
        };
        for l in loot {
            match all.iter_mut().find(|a| a.item == l.item) {
                Some(a) => a.count += l.count,
                None => all.push(l.clone()),
            }
        }
    }
    all
}

#[test]
fn the_hoard_drops_a_gem_three_times_in_four_and_a_dagger_one_in_eight() {
    let data = objects();
    let id = data.registry.loot.get("objects:loot:hoard").expect("hoard");
    let table = &data.loot[&id];
    let (gem, dagger) = (
        item(&data, "base:item:gem"),
        item(&data, "base:item:dagger"),
    );
    let stream = StreamName::new("combat");
    let (mut gems, mut daggers, mut none, mut two_gems) = (0, 0, 0, 0);
    let draws = 4000;
    for seed in 0..draws {
        let mut rng = Pcg32::for_stream(seed, &stream);
        let drop = loot::draw(&data, table, &mut rng, &stream).expect("rolls");
        match drop.as_slice() {
            [] => none += 1,
            [(i, n)] if *i == gem => {
                gems += 1;
                assert!((1..=2).contains(n), "1d2 gems");
                two_gems += usize::from(*n == 2);
            }
            [(i, 1)] if *i == dagger => daggers += 1,
            other => panic!("one pick draws one entry: {other:?}"),
        }
    }
    // Expected: gems 3000, daggers 500, nothing 500, two gems half the gems.
    assert!((2880..=3120).contains(&gems), "gems {gems}");
    assert!((420..=580).contains(&daggers), "daggers {daggers}");
    assert!((420..=580).contains(&none), "nothing {none}");
    assert!((1400..=1600).contains(&two_gems), "two gems {two_gems}");
}

#[test]
fn no_base_monster_carries_loot() {
    let data = objects();
    let base: Vec<&str> = data
        .registry
        .monsters
        .names()
        .filter(|n| {
            n.starts_with("base:") && {
                let id = data.registry.monsters.get(n).expect("named");
                data.monsters[&id].loot.is_some()
            }
        })
        .collect();
    assert_eq!(base, Vec::<&str>::new(), "the fight replay stays as it was");
}

#[test]
fn victory_lays_what_the_dead_dropped_on_the_fight_s_cell() {
    let data = objects();
    let mut fights_with_loot = 0;
    for seed in 1..=12 {
        let mut world = party(&data, seed);
        world.mode = Mode::Encounter(rats(&data, &world, 2, Disposition::Hostile));
        let events = settle(&mut world, &data, &mut Vec::new());
        let ended = events.iter().find_map(|e| match e {
            Event::CombatEnded { outcome, loot, .. } => Some((*outcome, loot.clone())),
            _ => None,
        });
        let (outcome, laid) = ended.expect("the fight ended");
        assert_eq!(outcome, CombatOutcome::Victory, "seed {seed}");
        assert_eq!(
            laid,
            dropped(&events),
            "seed {seed}: the victory lays what fell"
        );
        let here = (Level(0), world.position.x, world.position.y);
        let pile = pile_at(&world, &data, yard(&data), here);
        let on_ground: Vec<Loot> = pile.map(|p| Loot::of(&data, &p.items)).unwrap_or_default();
        assert_eq!(
            on_ground, laid,
            "seed {seed}: on the ground where they fought"
        );
        fights_with_loot += usize::from(!laid.is_empty());
    }
    assert!(
        fights_with_loot >= 10,
        "two rats drop nothing one time in 64"
    );
}

#[test]
fn a_flight_leaves_the_loot_behind() {
    let data = objects();
    let mut world = party(&data, 3);
    let friendly = rats(&data, &world, 1, Disposition::Friendly);
    let mut events = Vec::new();
    combat::start(&mut world, &data, friendly, Surprise::None, &mut events).expect("a fight");
    let Mode::Combat(state) = &mut world.mode else {
        panic!("fighting")
    };
    state.loot = vec![(item(&data, "base:item:gem"), 1)];
    let fight = (Level(0), world.position.x, world.position.y);
    let events = apply(&mut world, &data, Command::Combat(CombatCommand::Run)).expect("run");
    let ended = events.iter().find_map(|e| match e {
        Event::CombatEnded { outcome, loot, .. } => Some((*outcome, loot.clone())),
        _ => None,
    });
    assert_eq!(ended, Some((CombatOutcome::Fled, Vec::new())));
    assert_eq!(pile_at(&world, &data, yard(&data), fight), None);
    assert!(
        world
            .maps
            .get(&yard(&data))
            .is_none_or(|s| s.ground.is_empty()),
        "nothing laid anywhere"
    );
}

#[test]
fn loot_joins_a_pile_already_on_the_cell() {
    let data = objects();
    let mut world = party(&data, 1);
    world.position.x = 2;
    world.position.y = 2;
    world.mode = Mode::Encounter(rats(&data, &world, 2, Disposition::Hostile));
    let events = settle(&mut world, &data, &mut Vec::new());
    let mut expected = vec![(item(&data, "base:item:arrows"), 10)];
    for l in dropped(&events) {
        expected.push((item(&data, &l.item), l.count));
    }
    assert!(expected.len() > 1, "seed 1 drops something");
    let pile = pile_at(&world, &data, yard(&data), (Level(0), 2, 2)).expect("a pile");
    assert_eq!(pile.items, expected, "the arrows, then what fell");
    assert_eq!(pile.pos, (1, 1), "where the arrows lie");
}

#[test]
fn a_one_in_a_hundred_chance_drops_about_once_in_a_hundred_and_every_pick_draws() {
    let data = objects();
    let gem = item(&data, "base:item:gem");
    let table = |picks, chance_percent| omnis_data::LootTable {
        schema: 1,
        id: "objects:loot:trial".into(),
        picks,
        entries: vec![omnis_data::LootEntry {
            item: "base:item:gem".into(),
            count: omnis_core::Dice::new(1, 1),
            weight: 1,
            chance_percent,
        }],
    };
    let stream = StreamName::new("combat");
    let rare = table(1, 1);
    let drops = (0..4000)
        .filter(|seed| {
            let mut rng = Pcg32::for_stream(*seed, &stream);
            !loot::draw(&data, &rare, &mut rng, &stream)
                .expect("rolls")
                .is_empty()
        })
        .count();
    assert!((20..=60).contains(&drops), "1% of 4000 is 40: {drops}");
    let mut rng = Pcg32::for_stream(0, &stream);
    let three = loot::draw(&data, &table(3, 100), &mut rng, &stream).expect("rolls");
    assert_eq!(three, vec![(gem, 3)], "three picks, each a sure gem");
}
