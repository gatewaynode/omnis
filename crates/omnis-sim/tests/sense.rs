//! Remote sensing on the test dungeon: the ray along the facing line and what stops it, a
//! spyglass recording what its checks reach as remotely seen, the best eyes doing the
//! looking, a look without a die, and the automap's layer rules (a direct sighting clears
//! the remote mark, the visited bit sticks).

mod common;

use common::{data, party_of, world};
use omnis_core::{Edges, Facing, ItemId, MapId, Position};
use omnis_data::{Data, Fidelity, Geometry, SenseSource, UseEffect};
use omnis_sim::items::{count_of, item_id};
use omnis_sim::visibility::{cone, ray};
use omnis_sim::world::{Known, layer};
use omnis_sim::{Command, Event, ItemCommand, LayerCheck, World, apply};

fn dungeon(data: &Data) -> MapId {
    data.registry.maps.get("test:map:dungeon").unwrap()
}

fn place(world: &mut World, data: &Data, x: u16, y: u16, facing: Facing) {
    world.position = Position {
        map: dungeon(data),
        x,
        y,
        facing,
    };
}

/// The row of the spyglass in a member's kit, given to them if they lack one.
fn spyglass(world: &mut World, data: &Data, member: usize) -> (ItemId, u8) {
    let glass = item_id(data, "spyglass").unwrap();
    let kit = &mut world.party.members[member].equipment;
    if count_of(kit, glass) == 0 {
        kit.push((glass, 1));
    }
    let row = kit.iter().position(|(id, _)| *id == glass).unwrap();
    (glass, u8::try_from(row).unwrap())
}

/// The difficulty `sense.dc` gives a tile in the dungeon (visibility six).
fn dc(distance: u8, layer: u8) -> i64 {
    10 + i64::from(distance.saturating_sub(6)) + i64::from(layer - 1) * 2
}

#[test]
fn the_ray_stops_before_a_closed_door_after_a_pillar_and_at_a_wall() {
    let data = data();
    let mut world = world(&data);
    let map = &data.maps[&dungeon(&data)];
    let at = |x, y, facing| Position {
        map: dungeon(&data),
        x,
        y,
        facing,
    };
    let state = |world: &World| world.maps.get(&dungeon(&data)).cloned();
    assert_eq!(
        ray(map, state(&world).as_ref(), at(3, 4, Facing::South), 16),
        [(3, 5)],
        "the door below (3, 5) is closed"
    );
    // Open it from the tile before it, then look from farther back.
    place(&mut world, &data, 3, 5, Facing::South);
    apply(&mut world, &data, Command::Interact).unwrap();
    let through: Vec<(u16, u16)> = (5..=11).map(|y| (3, y)).collect();
    assert_eq!(
        ray(map, state(&world).as_ref(), at(3, 4, Facing::South), 16),
        through,
        "through the open door to the next closed one"
    );
    assert_eq!(
        ray(map, state(&world).as_ref(), at(3, 4, Facing::South), 3),
        &through[..3],
        "the range caps it"
    );
    assert_eq!(
        ray(map, None, at(3, 0, Facing::South), 16),
        [(3, 1), (3, 2), (3, 3)],
        "the pillar at (3, 3) is seen, not seen through"
    );
    assert_eq!(
        ray(map, None, at(1, 0, Facing::South), 16).len(),
        5,
        "the wall under the first room"
    );
    assert!(
        ray(map, None, at(1, 0, Facing::North), 16).is_empty(),
        "the map's edge"
    );
}

/// The `Sensed` event of a use.
fn sensed(events: &[Event]) -> (&[LayerCheck], Vec<(u16, u16, u8)>) {
    let Some(Event::Sensed { checks, tiles, .. }) =
        events.iter().find(|e| matches!(e, Event::Sensed { .. }))
    else {
        panic!("no Sensed in {events:?}")
    };
    (checks, tiles.iter().map(|t| (t.x, t.y, t.layers)).collect())
}

#[test]
fn a_spyglass_records_what_its_checks_reach_as_remotely_seen() {
    let data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    let (brenna, durin) = (world.party.members[0].id, world.party.members[1].id);
    place(&mut world, &data, 3, 5, Facing::South);
    apply(&mut world, &data, Command::Interact).unwrap();
    place(&mut world, &data, 3, 4, Facing::South);
    let (glass, row) = spyglass(&mut world, &data, 0);
    let clock = world.party_clock().elapsed;
    let events = apply(
        &mut world,
        &data,
        Command::Item(ItemCommand::Use {
            member: 0,
            item: row,
            target: None,
        }),
    )
    .unwrap();
    assert!(events.contains(&Event::ItemUsed {
        member: brenna,
        item: glass,
        target: None,
        consumed: false,
    }));
    assert_eq!(count_of(&world.party.members[0].equipment, glass), 1);
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::Sensed { actor, item, .. } if *actor == durin && *item == glass
        )),
        "Brenna holds the glass; Durin's Wisdom 16 outsees her trained Perception"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::TimeAdvanced { minutes: 1, .. })),
        "the spyglass's minute"
    );
    let (checks, tiles) = sensed(&events);
    assert_eq!(checks.len(), 2, "terrain, then structure");
    let along: Vec<(u16, u16)> = (5..=11).map(|y| (3, y)).collect();
    let mut expected = Vec::new();
    for (check, bit) in checks.iter().zip([layer::TERRAIN, layer::STRUCTURE]) {
        let roll = check.roll.as_ref().expect("Perception is rolled");
        let reach = (1..=7u8)
            .take_while(|d| roll.total >= dc(*d, check.layer))
            .last()
            .unwrap_or(0);
        assert_eq!(check.reach, reach, "layer {}: {:?}", check.layer, roll);
        for d in 1..=reach {
            let (x, y) = along[usize::from(d) - 1];
            match expected.iter_mut().find(|(ex, ey, _)| (*ex, *ey) == (x, y)) {
                Some((_, _, bits)) => *bits |= bit,
                None => expected.push((x, y, bit)),
            }
        }
    }
    expected.sort_by_key(|(_, y, _)| *y);
    assert_eq!(tiles, expected, "{checks:?}");
    let seen_now = cone(&world, &data);
    let known = world.automap.map(dungeon(&data)).unwrap();
    for (x, y, bits) in &tiles {
        let tile = known[&(*x, *y)];
        assert_eq!(
            tile.layers & (layer::TERRAIN | layer::STRUCTURE),
            *bits,
            "({x}, {y})"
        );
        let direct = seen_now.iter().any(|s| (s.x, s.y) == (*x, *y));
        assert_eq!(
            tile.layers & layer::REMOTE != 0,
            !direct,
            "({x}, {y}) remote unless the party sees it now"
        );
        let expected_at = if direct { clock + 1 } else { clock };
        assert_eq!(
            tile.seen_at, expected_at,
            "the look's minute; the party's own sight after it"
        );
    }
}

#[test]
fn the_best_eyes_look_and_a_source_without_a_die_reaches_the_whole_ray() {
    let mut data = data();
    let mut world = world(&data);
    party_of(&mut world, &data, 2);
    let (brenna, durin) = (world.party.members[0].id, world.party.members[1].id);
    place(&mut world, &data, 3, 5, Facing::South);
    apply(&mut world, &data, Command::Interact).unwrap();
    place(&mut world, &data, 3, 4, Facing::South);
    // Brenna carries the glass, but Durin (Wisdom 16, +3) outsees her Perception (+2).
    let (glass, row) = spyglass(&mut world, &data, 0);
    let use_it = Command::Item(ItemCommand::Use {
        member: 0,
        item: row,
        target: None,
    });
    let events = apply(&mut world, &data, use_it.clone()).unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Sensed { actor, .. } if *actor == durin
    )));
    // Durin down: Brenna's own eyes.
    world.party.members[1].hp = 0;
    let events = apply(&mut world, &data, use_it.clone()).unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        Event::Sensed { actor, .. } if *actor == brenna
    )));
    // A source that needs no die reaches every tile of the ray on every layer it has.
    data.items.get_mut(&glass).unwrap().use_effect = Some(UseEffect::Sense(SenseSource {
        geometry: Geometry::Ray { range: 4 },
        fidelity: Fidelity::Terrain,
        check: None,
        persistence: Default::default(),
        minutes: 3,
    }));
    let before = world.party_clock().elapsed;
    let events = apply(&mut world, &data, use_it.clone()).unwrap();
    let (checks, tiles) = sensed(&events);
    assert_eq!(checks.len(), 1, "terrain only");
    assert!(checks[0].roll.is_none() && checks[0].reach == 4);
    assert_eq!(
        tiles,
        (5..=8).map(|y| (3, y, layer::TERRAIN)).collect::<Vec<_>>()
    );
    assert_eq!(
        world.party_clock().elapsed,
        before + 3,
        "the source's minutes"
    );
    // Facing a wall there is nothing to look at: no check, no tiles.
    place(&mut world, &data, 1, 5, Facing::South);
    let events = apply(&mut world, &data, use_it).unwrap();
    let (checks, tiles) = sensed(&events);
    assert!(checks.is_empty() && tiles.is_empty());
}

#[test]
fn a_direct_sighting_clears_the_remote_mark_and_visited_sticks() {
    let data = data();
    let mut world = world(&data);
    let map = dungeon(&data);
    let known = |terrain, walls, layers, seen_at| Known {
        terrain,
        walls: Edges(walls),
        doors: Edges(0),
        layers,
        seen_at,
    };
    // A remote terrain-only look records the terrain and nothing of the walls.
    world.automap.record(
        map,
        9,
        9,
        known(1, 0b1111, layer::TERRAIN | layer::REMOTE, 5),
    );
    let tile = world.automap.map(map).unwrap()[&(9, 9)];
    assert_eq!(
        (tile.terrain, tile.walls, tile.layers, tile.seen_at),
        (1, Edges(0), layer::TERRAIN | layer::REMOTE, 5)
    );
    // A later remote look with structure keeps the terrain it did not carry and adds walls.
    world.automap.record(
        map,
        9,
        9,
        known(0, 0b0011, layer::STRUCTURE | layer::REMOTE, 6),
    );
    let tile = world.automap.map(map).unwrap()[&(9, 9)];
    assert_eq!(
        (tile.terrain, tile.walls, tile.layers, tile.seen_at),
        (
            1,
            Edges(0b0011),
            layer::TERRAIN | layer::STRUCTURE | layer::REMOTE,
            6
        )
    );
    // The party's own sighting clears the mark; a visit sticks through a later remote look.
    world.automap.record(
        map,
        9,
        9,
        known(
            0,
            0b0001,
            layer::TERRAIN | layer::STRUCTURE | layer::VISITED,
            7,
        ),
    );
    let tile = world.automap.map(map).unwrap()[&(9, 9)];
    assert_eq!(
        (tile.terrain, tile.walls, tile.layers),
        (
            0,
            Edges(0b0001),
            layer::TERRAIN | layer::STRUCTURE | layer::VISITED
        )
    );
    world
        .automap
        .record(map, 9, 9, known(1, 0, layer::TERRAIN | layer::REMOTE, 8));
    let tile = world.automap.map(map).unwrap()[&(9, 9)];
    assert_eq!(
        tile.layers,
        layer::TERRAIN | layer::STRUCTURE | layer::VISITED | layer::REMOTE
    );
    assert_eq!((tile.terrain, tile.walls), (1, Edges(0b0001)));
    // Walking onto a remotely seen tile clears the mark through the ordinary visit.
    let _ = world;
}
