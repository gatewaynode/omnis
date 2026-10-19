//! Saves loaded by name (M8.5 7h3). A save is rebuilt in the numbering of its world folder and
//! proved against its fingerprint, then renumbered by name for the loaded packs. The proof: the
//! `ahead` fixture pack adds one definition of every kind a save numbers, each sorting first, so
//! loaded beside base, test and objects it moves every number they had. A save made without it
//! and loaded with it reads the same through every view: exploring, in town, mid-fight and among
//! the yard's objects.

use crate::common::{base_pack, inside, party_of, play, test_pack, walk_to_the_rats, world};
use omnis_core::{Facing, HolderId, Level, MapId, Pcg32, StreamName};
use omnis_data::{Data, load_packs};
use omnis_sim::bus::Topic;
use omnis_sim::objects::ObjectState;
use omnis_sim::omnis_rules::tactics::{ActionRef, Criteria, CriteriaSet, Predicate, Trigger, Who};
use omnis_sim::omnis_rules::{ActiveEffect, EffectKind, Expiry};
use omnis_sim::save::{
    Archived, Manifest, NameKind, PackFit, Restored, load_save, save_files, tidy_list, world_id,
};
use omnis_sim::{
    CombatCommand, Command, EncounterChoice, LoadError, Mode, ModeKind, Op, World, apply, dispatch,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Turn about, so the automap has the cells around.
fn look(world: &mut World, data: &Data) {
    for _ in 0..4 {
        crate::common::turn(world, data, omnis_core::Rotation::Left);
    }
}

const SAVE: &str = ".omnis/saves/by-name.ron";

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/packs")
        .join(name)
}

/// Base, test and the objects fixture: the packs a save is made on.
fn before() -> Data {
    load_packs(&[&base_pack(), &test_pack(), &fixture("objects")]).unwrap_or_else(|r| panic!("{r}"))
}

/// The same with `ahead`, whose every definition sorts first.
fn after() -> Data {
    load_packs(&[
        &base_pack(),
        &test_pack(),
        &fixture("objects"),
        &fixture("ahead"),
    ])
    .unwrap_or_else(|r| panic!("{r}"))
}

type Disk = BTreeMap<String, String>;

fn write(world: &World, data: &Data, disk: &mut Disk) {
    for file in save_files(world, data, SAVE).unwrap_or_else(|e| panic!("{e}")) {
        if !(file.keep && disk.contains_key(&file.path)) {
            disk.insert(file.path, file.text);
        }
    }
}

fn read(disk: &Disk, data: &Data) -> Result<(World, Restored), LoadError> {
    load_save(&disk[SAVE], data, false, &mut |p| {
        disk.get(p).cloned().ok_or_else(|| format!("no file {p}"))
    })
}

/// Every view a client reads, as the protocol answers it: string ids only, so equal across a
/// renumbering when the remap is right.
fn views(world: &World, data: &Data) -> Vec<String> {
    let ops = [
        Op::PartyGet,
        Op::ViewportGet,
        Op::CombatGet,
        Op::ServiceGet,
        Op::RestGet,
        Op::CastGet,
        Op::ObjectsGet,
        Op::TimeClocks,
        Op::AutomapGet {
            map: None,
            level: None,
        },
        Op::EventsTail { count: 64 },
    ];
    let mut out: Vec<String> = ops
        .iter()
        .map(|op| format!("{:?}", dispatch(&mut world.clone(), data, op)))
        .collect();
    let set: Vec<_> = omnis_sim::api::flags(world, data)
        .into_iter()
        .filter(|(_, v)| *v != 0)
        .collect();
    out.push(format!("{set:?}"));
    out.push(format!("{:?}", omnis_sim::api::here(world, data)));
    out
}

/// What no view shows, by name: the `time:` streams' holders, the bus's region topics, each
/// map's cleared encounters and object states, and the party's knowledge.
fn named(world: &World, data: &Data) -> Vec<String> {
    let region = |h: &HolderId| match h {
        HolderId::Region(r) => format!("region {}", data.registry.regions.name(*r).unwrap()),
        other => other.to_string(),
    };
    let map = |m: &MapId| data.registry.maps.name(*m).unwrap().to_owned();
    let mut out = Vec::new();
    for (name, stream) in &world.rngs {
        let name = data
            .registry
            .regions
            .names()
            .enumerate()
            .fold(name.0.clone(), |n, (i, r)| {
                n.replace(&format!("region:{i}"), r)
            });
        out.push(format!("{name} {stream:?}"));
    }
    out.sort();
    for (holder, clock) in &world.clocks {
        out.push(format!("{} {clock:?}", region(holder)));
    }
    for ((a, b), contact) in &world.contacts {
        out.push(format!(
            "{} {} {}",
            region(a),
            region(b),
            region(&contact.other)
        ));
    }
    let mut topics: Vec<String> = data
        .regions
        .keys()
        .map(|r| {
            let subs = world.bus.subscribers(Topic::Region(*r));
            format!("{} {subs:?}", data.registry.regions.name(*r).unwrap())
        })
        .collect();
    topics.sort();
    // A region the loaded packs add is subscribed too; `proves` checks it.
    topics.retain(|t| !t.starts_with("ahead:"));
    out.extend(topics);
    out.push(format!("{:?}", world.bus.subscribers(Topic::Battle)));
    let mut maps: Vec<String> = world
        .maps
        .iter()
        .map(|(m, s)| {
            let used: Vec<_> = s
                .objects
                .iter()
                .map(|(id, o)| (id, &o.used, &o.state))
                .collect();
            format!("{} {:?} {used:?}", map(m), s.cleared)
        })
        .collect();
    maps.sort();
    out.extend(maps);
    let terrain = |m: &MapId, row: u8| data.maps[m].def.terrains[usize::from(row)].name.clone();
    let mut tiles: Vec<String> = world
        .automap
        .maps
        .iter()
        .map(|(m, t)| (m, Level::GROUND, t))
        .chain(world.automap.levels.iter().map(|((m, l), t)| (m, *l, t)))
        .flat_map(|(m, level, t)| {
            t.iter().map(move |(cell, k)| {
                let known = (k.walls, k.doors, k.layers, k.seen_at);
                format!(
                    "{} {level:?} {cell:?} {} {known:?}",
                    map(m),
                    terrain(m, k.terrain)
                )
            })
        })
        .collect();
    tiles.sort();
    out.extend(tiles);
    let encounter = match &world.mode {
        Mode::Encounter(e) => Some((e, &[][..])),
        Mode::Combat(c) => Some((&c.encounter, &c.loot[..])),
        Mode::Explore | Mode::Town(_) => None,
    };
    if let Some((e, loot)) = encounter {
        let item = |i: &omnis_core::ItemId| data.registry.items.name(*i).unwrap().to_owned();
        let loot: Vec<_> = loot.iter().map(|(i, n)| (item(i), n)).collect();
        out.push(format!("retreat {} {loot:?}", map(&e.retreat.map)));
    }
    let said: Vec<_> = world
        .log
        .iter()
        .map(|l| data.registry.maps.name(l.map).unwrap_or("?"))
        .collect();
    out.push(format!("log {said:?}"));
    for member in &world.party.members {
        let named = |a: &ActionRef| match a {
            ActionRef::Spell(s) => data.registry.spells.name(*s).unwrap().to_owned(),
            ActionRef::Item(i) => data.registry.items.name(*i).unwrap().to_owned(),
            other => format!("{other:?}"),
        };
        for book in &member.tactics.runbooks {
            let entries: Vec<_> = book.entries.iter().map(|(a, n)| (named(a), n)).collect();
            out.push(format!("{} {entries:?}", book.name));
        }
    }
    let known = &world.known;
    for set in [&known.noticed, &known.told, &known.lore] {
        let mut set: Vec<_> = set.iter().map(|(m, id)| (map(m), id)).collect();
        set.sort();
        out.push(format!("{set:?}"));
    }
    out
}

/// Load `world`, saved on `before`, on `after`: by name, every view and every named thing the
/// same, the world folded, its journal's segment kept under the old folder.
fn proves(world: &World, what: &str) -> World {
    let (old, new) = (before(), after());
    let mut disk = Disk::new();
    write(world, &old, &mut disk);
    let (back, report) = read(&disk, &new).unwrap_or_else(|e| panic!("{what}: {e}"));
    // A save keeps the log lines the party's log retells.
    let mut kept = world.clone();
    kept.log.retain(|l| omnis_sim::told(&l.event));
    let world = &kept;
    assert_eq!(report.fit, PackFit::Renumbered, "{what}");
    assert_eq!(back.packs, new.fingerprints, "{what}");
    for (i, (a, b)) in views(&back, &new)
        .iter()
        .zip(views(world, &old))
        .enumerate()
    {
        if *a != b {
            let at = a
                .bytes()
                .zip(b.bytes())
                .position(|(x, y)| x != y)
                .unwrap_or(0);
            let from = at.saturating_sub(300);
            panic!(
                "{what}: view {i} differs:\n now {}\n was {}",
                &a[from..(at + 200).min(a.len())],
                &b[from..(at + 200).min(b.len())]
            );
        }
    }
    assert_eq!(named(&back, &new), named(world, &old), "{what}: by name");
    assert!(back.journal.is_empty(), "{what}: folded");
    let hills = new.registry.regions.get("ahead:region:hills").unwrap();
    assert_eq!(
        back.bus.subscribers(Topic::Region(hills)),
        [omnis_sim::bus::Subscriber::Reconcile],
        "{what}: a new region is reconciled"
    );
    if !world.journal.is_empty() {
        let Some(Archived::Held {
            from: Some(from), ..
        }) = back.archive.last()
        else {
            panic!(
                "{what}: the journal held for its old folder: {:?}",
                back.archive
            );
        };
        assert_eq!(from.packs, old.fingerprints, "{what}");
    }
    // Saved on the new packs, it writes a new folder, its segment in the old one, and loads
    // back as the same world.
    let mut again = disk.clone();
    write(&back, &new, &mut again);
    let (twice, report) = read(&again, &new).unwrap_or_else(|e| panic!("{what}: {e}"));
    assert_eq!(report.fit, PackFit::Same, "{what}");
    assert_eq!(twice.fingerprint(), back.fingerprint(), "{what}");
    let all: Vec<String> = again.keys().cloned().collect();
    let listed = tidy_list(&all, &mut |p| {
        again.get(p).cloned().ok_or_else(|| format!("no {p}"))
    })
    .unwrap();
    let old_folder = world_id(&Manifest::of(world, &old)).unwrap();
    assert!(
        !world.journal.is_empty() || listed.iter().all(|p| !p.contains("/archive/")),
        "{what}"
    );
    assert!(
        listed
            .iter()
            .all(|p| !p.contains(&format!("{old_folder}/archive"))),
        "{what}: the old folder's segment is named: {listed:?}"
    );
    back
}

/// Every kind the save numbers moves under `ahead`.
#[test]
fn ahead_moves_every_number() {
    let (old, new) = (before(), after());
    let (a, b) = (
        omnis_sim::save::NameTable::of(&old),
        omnis_sim::save::NameTable::of(&new),
    );
    for (kind, was, now) in [
        ("maps", &a.maps, &b.maps),
        ("flags", &a.flags, &b.flags),
        ("regions", &a.regions, &b.regions),
        ("items", &a.items, &b.items),
        ("spells", &a.spells, &b.spells),
        ("monsters", &a.monsters, &b.monsters),
        ("conditions", &a.conditions, &b.conditions),
        ("races", &a.races, &b.races),
        ("classes", &a.classes, &b.classes),
        ("backgrounds", &a.backgrounds, &b.backgrounds),
        ("services", &a.services, &b.services),
    ] {
        assert!(!was.is_empty(), "{kind}");
        for (i, name) in was.iter().enumerate() {
            let moved = now.iter().position(|n| n == name);
            assert!(
                moved.is_some_and(|j| j != i),
                "{kind}: {name} at {i}, now {moved:?}"
            );
        }
    }
}

/// The dagger, a spell and a monster and a condition the members' tactics, effects and
/// conditions name, so each id a member holds is in the save.
fn armed(world: &mut World, data: &Data) {
    let id = |kind: &str, name: &str| format!("base:{kind}:{name}");
    let spell = data
        .registry
        .spells
        .get(&id("spell", "magic_missile"))
        .unwrap();
    let monster = data
        .registry
        .monsters
        .get(&id("monster", "goblin"))
        .unwrap();
    let condition = data
        .registry
        .conditions
        .get(&id("condition", "prone"))
        .unwrap();
    let potion = data
        .registry
        .items
        .get(&id("item", "potion_of_healing"))
        .unwrap();
    let caster = world.party.members[0].id;
    let effect = ActiveEffect {
        source: spell,
        caster,
        concentration: false,
        kind: EffectKind::ArmorBonus(1),
        until: Expiry::Minute(100_000),
    };
    world.party.effects.push(effect.clone());
    world.party.inventory.push((potion, 2));
    let member = &mut world.party.members[0];
    member.effects.push(effect);
    member.conditions.push(condition);
    member.known_spells.push(spell);
    member.tactics.library.push(CriteriaSet {
        name: "missiles".into(),
        action: ActionRef::Spell(spell),
        trigger: Trigger::SpellCast,
        when: Criteria::All(vec![
            Criteria::Is(Predicate::MonsterCount {
                monster,
                cmp: omnis_sim::omnis_rules::tactics::Cmp::Ge,
                n: 1,
            }),
            Criteria::Is(Predicate::HasCondition {
                who: Who::Me,
                condition,
            }),
        ]),
    });
    member.tactics.runbooks[0]
        .entries
        .push((ActionRef::Item(potion), 0));
}

#[test]
fn exploring_after_a_fight_reads_the_same() {
    let data = before();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    play(&mut world, &data, &walk_to_the_rats());
    assert_eq!(world.mode.kind(), ModeKind::Explore);
    armed(&mut world, &data);
    assert!(!world.journal.is_empty(), "the fight's interactions");
    assert!(
        world.rngs.keys().any(|n| n.0.starts_with("time:")),
        "a region met: {:?}",
        world.rngs.keys().collect::<Vec<_>>()
    );
    proves(&world, "exploring");
}

#[test]
fn mid_fight_reads_the_same() {
    let data = before();
    let mut world = world(&data);
    party_of(&mut world, &data, 6);
    // Up to the first monsters met, the rats or wanderers on the way.
    for command in walk_to_the_rats() {
        if world.mode.kind() != ModeKind::Explore {
            break;
        }
        apply(&mut world, &data, command).unwrap_or_else(|r| panic!("{r}"));
    }
    if world.mode.kind() == ModeKind::Encounter {
        apply(
            &mut world,
            &data,
            Command::Encounter(EncounterChoice::Attack),
        )
        .unwrap();
    }
    armed(&mut world, &data);
    let Mode::Combat(fight) = &mut world.mode else {
        panic!("a fight: {:?}", world.mode.kind());
    };
    let potion = data
        .registry
        .items
        .get("base:item:potion_of_healing")
        .unwrap();
    fight.loot.push((potion, 1));
    let back = proves(&world, "mid-fight");
    let stack = crate::common::reachable_stack(&back, &after()).unwrap();
    apply(
        &mut back.clone(),
        &after(),
        Command::Combat(CombatCommand::Attack { stack }),
    )
    .expect("the fight goes on");
}

#[test]
fn in_town_reads_the_same() {
    let data = before();
    for service in ["smith", "bank", "inn"] {
        let mut world = inside(&data, service);
        armed(&mut world, &data);
        proves(&world, service);
    }
}

/// The yard's objects: a chest searched once and refilled, a pile on the ground, a lever found,
/// lore told, the party's and the world's flags.
#[test]
fn the_yard_reads_the_same() {
    let data = before();
    let yard = data.registry.maps.get("objects:map:yard").unwrap();
    let mut world = crate::objects::in_yard(&data, 0, 4, 3, Facing::East);
    armed(&mut world, &data);
    let chest = data.registry.objects.get("objects:object:chest").unwrap();
    let search = data.objects[&chest]
        .interactions
        .iter()
        .position(|i| i.once)
        .unwrap();
    let dagger = data.registry.items.get("base:item:dagger").unwrap();
    let state = world.map_state(yard);
    state.objects.insert(
        "chest_1".into(),
        ObjectState {
            state: Some("open".into()),
            removed: false,
            used: [u8::try_from(search).unwrap()].into(),
            contents: Some(vec![(dagger, 3)]),
        },
    );
    state.ground.insert(
        (Level::GROUND, 4, 3),
        omnis_data::placement::Pile {
            pos: (0, 0),
            items: vec![(dagger, 1)],
        },
    );
    world.known.notice(yard, "lever_1");
    world.known.told.insert((yard, "statue_1".into()));
    world.known.lore.insert((yard, "carving".into()));
    look(&mut world, &data);
    let seen = *world.automap.maps[&yard].values().next().unwrap();
    world.automap.record(yard, Level(1), 5, 0, seen);
    for flag in ["objects:flag:debt", "objects:flag:lever"] {
        world
            .flags
            .insert(data.registry.flags.get(flag).unwrap(), 2);
    }
    look(&mut world, &data);
    // Set by hand, not played: a fold makes it the bases the save names.
    omnis_sim::save::fold(&mut world, &data);
    proves(&world, "the yard");
}

/// A save naming what the loaded packs do not have is refused, each name listed with where.
#[test]
fn a_name_the_packs_lack_refuses_the_load() {
    let (old, new) = (before(), after());
    let mut world = inside(&new, "smith");
    let dagger = new.registry.items.get("ahead:item:dagger").unwrap();
    world.party.members[0].equipment.push((dagger, 1));
    world.party.inventory.push((dagger, 1));
    let mut disk = Disk::new();
    write(&world, &new, &mut disk);
    let Err(LoadError::Unresolved(missing)) = read(&disk, &old) else {
        panic!("refused");
    };
    let named: Vec<(NameKind, &str, &str)> = missing
        .iter()
        .map(|m| (m.kind, m.name.as_str(), m.at.as_str()))
        .collect();
    assert!(
        named.contains(&(NameKind::Item, "ahead:item:dagger", "party.inventory")),
        "{named:?}"
    );
    assert!(
        named.contains(&(
            NameKind::Item,
            "ahead:item:dagger",
            "party.members[0].equipment"
        )),
        "{named:?}"
    );
    let shown = LoadError::Unresolved(missing).to_string();
    assert!(
        shown.contains("item 'ahead:item:dagger' in party.inventory"),
        "{shown}"
    );
}

/// The rows a save keeps by position are found again by name: with the save's table as other
/// packs would leave it (the yard's terrains reversed, the chest's `once` interaction at another
/// row), each saved row maps to the row the loaded packs give that name.
#[test]
fn terrain_and_interaction_rows_are_found_by_name() {
    let data = before();
    let yard = data.registry.maps.get("objects:map:yard").unwrap();
    let mut world = crate::objects::in_yard(&data, 0, 1, 2, Facing::East);
    look(&mut world, &data);
    let chest = data.registry.objects.get("objects:object:chest").unwrap();
    let search = data.objects[&chest]
        .interactions
        .iter()
        .position(|i| i.once)
        .unwrap();
    // Row 9 is what the save's table will say the search was.
    world.map_state(yard).objects.insert(
        "chest_1".into(),
        ObjectState {
            used: [9].into(),
            ..ObjectState::default()
        },
    );
    omnis_sim::save::fold(&mut world, &data);
    let tiles = world.automap.maps[&yard].clone();
    assert!(tiles.len() > 3, "the yard seen");
    let mut disk = Disk::new();
    write(&world, &data, &mut disk);
    let path = disk
        .keys()
        .find(|p| p.ends_with("manifest.ron"))
        .unwrap()
        .clone();
    let mut manifest: Manifest = omnis_data::ron_io::parse(&disk[&path]).unwrap();
    let was = world_id(&manifest).unwrap();
    let terrains = manifest.names.terrains.get_mut("objects:map:yard").unwrap();
    terrains.reverse();
    let reversed = terrains.clone();
    let rows = manifest
        .names
        .interactions
        .get_mut("objects:object:chest")
        .unwrap();
    let signature = rows[&u8::try_from(search).unwrap()].clone();
    *rows = [(9, signature)].into();
    let id = world_id(&manifest).unwrap();
    let mut moved = Disk::new();
    for (p, text) in &disk {
        let text = if *p == path {
            omnis_data::ron_io::to_string(&manifest).unwrap()
        } else {
            text.replace(&was, &id)
        };
        moved.insert(p.replace(&was, &id), text);
    }
    // Read on `ahead`, so the names differ and the rows go through the table.
    let new = after();
    let (back, report) = read(&moved, &new).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(report.fit, PackFit::Renumbered);
    let yard = new.registry.maps.get("objects:map:yard").unwrap();
    let names = &new.maps[&yard].def.terrains;
    for (cell, known) in &back.automap.maps[&yard] {
        let saved = tiles[cell].terrain;
        let expect = names
            .iter()
            .position(|t| t.name == reversed[usize::from(saved)])
            .unwrap();
        assert_eq!(usize::from(known.terrain), expect, "{cell:?}");
    }
    assert!(
        reversed.len() > 1
            && tiles.values().any(|k| {
                let expect = names
                    .iter()
                    .position(|t| t.name == reversed[usize::from(k.terrain)]);
                expect != Some(usize::from(k.terrain))
            }),
        "some tile's row moves"
    );
    let used = &back.maps[&yard].objects["chest_1"].used;
    assert_eq!(
        used.iter().copied().collect::<Vec<_>>(),
        [u8::try_from(search).unwrap()],
        "found by its signature"
    );
}

/// On other packs the save is still held to its fingerprint: rebuilt in its own numbering, the
/// world must be the one it recorded.
#[test]
fn a_save_altered_on_other_packs_is_refused() {
    let (old, new) = (before(), after());
    let mut world = inside(&old, "smith");
    armed(&mut world, &old);
    let mut disk = Disk::new();
    write(&world, &old, &mut disk);
    let text = disk[SAVE].clone();
    let altered = text.replacen("turn:", "turn:1", 1);
    assert_ne!(altered, text);
    disk.insert(SAVE.into(), altered);
    assert!(
        matches!(read(&disk, &new), Err(LoadError::Diverged { .. })),
        "{:?}",
        read(&disk, &new).map(|(_, r)| r)
    );
}

/// A `time:` stream between two regions keeps its pair in order when renumbering swaps them, so
/// both sides of the contact still draw from one stream.
#[test]
fn a_stream_between_regions_is_put_back_in_order() {
    let data = before();
    let mut world = inside(&data, "smith");
    let region = |name: &str| data.registry.regions.get(name).unwrap();
    let (a, b) = (
        region("test:region:crossroads"),
        region("test:region:dungeon"),
    );
    let stream = StreamName::time(HolderId::Region(a), HolderId::Region(b));
    *world.stream(&stream) = Pcg32::for_stream(7, &stream);
    omnis_sim::save::fold(&mut world, &data);
    let mut disk = Disk::new();
    write(&world, &data, &mut disk);
    // The save's table as other packs would leave it: the two regions' numbers traded.
    let path = disk
        .keys()
        .find(|p| p.ends_with("manifest.ron"))
        .unwrap()
        .clone();
    let mut manifest: Manifest = omnis_data::ron_io::parse(&disk[&path]).unwrap();
    let was = world_id(&manifest).unwrap();
    let (i, j) = (usize::try_from(a.0).unwrap(), usize::try_from(b.0).unwrap());
    manifest.names.regions.swap(i, j);
    let id = world_id(&manifest).unwrap();
    let mut moved = Disk::new();
    for (p, text) in &disk {
        let text = if *p == path {
            omnis_data::ron_io::to_string(&manifest).unwrap()
        } else {
            text.replace(&was, &id)
        };
        moved.insert(p.replace(&was, &id), text);
    }
    let new = after();
    let (back, _) = read(&moved, &new).unwrap_or_else(|e| panic!("{e}"));
    let now = |name: &str| HolderId::Region(new.registry.regions.get(name).unwrap());
    // Saved as (crossroads, dungeon), read through the traded table as (dungeon, crossroads).
    let expect = StreamName::time(now("test:region:dungeon"), now("test:region:crossroads"));
    assert_eq!(
        back.rngs.get(&expect),
        world.rngs.get(&stream),
        "{:?}",
        back.rngs.keys()
    );
    for name in back.rngs.keys().filter(|n| n.0.starts_with("time:region:")) {
        let parts: Vec<&str> = name.0.split(':').collect();
        assert!(
            parts[2].parse::<u32>().unwrap() <= parts[4].parse::<u32>().unwrap(),
            "{name}"
        );
    }
}

/// A pack the save's world never used may go: a save made with `ahead` and nothing of it loads
/// without it, its bus holding no topic of a region the packs no longer have.
#[test]
fn a_pack_the_save_never_used_may_go() {
    let (old, new) = (before(), after());
    let world = inside(&new, "smith");
    let mut disk = Disk::new();
    write(&world, &new, &mut disk);
    let (back, report) = read(&disk, &old).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(report.fit, PackFit::Renumbered);
    let regions = u32::try_from(old.regions.len()).unwrap();
    for n in 0..regions + 4 {
        let topic = Topic::Region(omnis_core::RegionId(n));
        assert_eq!(
            !back.bus.subscribers(topic).is_empty(),
            n < regions,
            "region {n}"
        );
    }
}

/// A save whose region base names a region the loaded packs lack is refused by name, with the
/// maps it holds there: the base is read through the save's own table, not the loaded registry.
#[test]
fn a_base_of_a_region_the_packs_lack_is_refused_by_name() {
    let (old, new) = (before(), after());
    let mut world = inside(&new, "smith");
    let hills = new.registry.maps.get("ahead:map:hills").unwrap();
    world.map_state(hills).cleared.insert("gone".into());
    omnis_sim::save::fold(&mut world, &new);
    let mut disk = Disk::new();
    write(&world, &new, &mut disk);
    assert!(
        disk.keys().any(|p| p.contains("/regions/ahead/hills/")),
        "{:?}",
        disk.keys()
    );
    let Err(LoadError::Unresolved(missing)) = read(&disk, &old) else {
        panic!("refused by name: {:?}", read(&disk, &old).map(|(_, r)| r));
    };
    assert!(
        missing
            .iter()
            .any(|m| m.kind == NameKind::Map && m.name == "ahead:map:hills" && m.at == "maps"),
        "{missing:?}"
    );
}
