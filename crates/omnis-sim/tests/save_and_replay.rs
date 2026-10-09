//! Saves round-trip through RON text with equal fingerprints, refuse the wrong packs or
//! schema, and replays reproduce the golden fingerprint under `tests/replays/`.

mod common;

use common::{data, interact, play, six, step, turn, walk_to_the_rats, world};
use omnis_core::{Direction, Facing, Position, Rotation};
use omnis_data::ron_io::{read_ron, write_ron};
use omnis_data::{Data, EquipSlot};
use omnis_sim::Event;
use omnis_sim::omnis_rules::DeathSaves;
use omnis_sim::{
    Command, EncounterChoice, LoadError, Mode, PartyCommand, Replay, ReplayError, SaveRule,
    Settings, World, apply, query,
};
use std::path::PathBuf;

/// The dungeon as the old fixtures number it. Map ids are interned in file order, so a map
/// added since (M7b's depths) renumbers it; a save names the id it was written with, and the
/// fixtures are loaded with `force`, past the pack fingerprint that refuses them in play.
const FIXTURE_DUNGEON: omnis_core::MapId = omnis_core::MapId(0);

fn replay_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/replays")
        .join(format!("{name}.ron"))
}

/// The command script behind `tests/replays/walk.ron`: down the road, into the dungeon,
/// through the first door, and a bump against a pillar.
/// A walk through the first row of rooms, for the determinism checks under any seed.
fn walk() -> Vec<Command> {
    let mut commands = vec![Command::Step(Direction::Forward); 14];
    commands.extend([
        Command::Turn(Rotation::Left),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Turn(Rotation::Around),
        Command::Interact,
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Step(Direction::Forward),
        Command::Turn(Rotation::Right),
        Command::Step(Direction::Back),
        Command::Interact,
    ]);
    commands
}

/// Out of town through the gate, which lands beside the road home at (16, 30), then up the road
/// to the meadow's start at (16, 16), where the older scripts begin.
fn out_of_town() -> Vec<Command> {
    let mut commands = vec![Command::Step(Direction::Back)];
    commands.extend((0..14).map(|_| Command::Step(Direction::Forward)));
    commands
}

/// The golden walk's seed: the smallest under which the walk meets the dungeon's random table
/// (the golden seed's stream stopped firing on the way once the gate landed beside the road
/// home, 14 road steps further; 37 of seeds 1 to 199 fire, TODO 3c).
const WALK_SEED: u64 = 2;

/// The same walk under `WALK_SEED` from a new game, out of town through the gate first.
/// Built by running it: where the dungeon's random table fires, an empty party meets rats,
/// fights, falls at once, and walks on (surprise is off, so the choice waits for a command).
fn golden_walk(data: &Data) -> Vec<Command> {
    let mut world = World::new(data, WALK_SEED, Settings::default()).unwrap();
    let mut script = out_of_town();
    script.extend(walk());
    let (commands, _) = play(&mut world, data, &script);
    assert!(
        commands.contains(&Command::Encounter(EncounterChoice::Attack)),
        "the random table fires on the way"
    );
    commands
}

#[test]
fn save_round_trip_keeps_the_fingerprint() {
    let data = data();
    let mut world = world(&data);
    for _ in 0..12 {
        step(&mut world, &data);
    }
    turn(&mut world, &data, Rotation::Left);
    interact(&mut world, &data);
    let text = world.to_ron().unwrap();
    let loaded = World::from_ron(&text, &data, false).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(loaded.fingerprint().unwrap(), world.fingerprint().unwrap());
    assert_eq!(loaded.position, world.position);
    assert_eq!(loaded.automap, world.automap);
    assert!(loaded.log.is_empty(), "the log starts empty after a load");
    let mut a = loaded.clone();
    let mut b = world.clone();
    let ea = step(&mut a, &data);
    let eb = step(&mut b, &data);
    assert_eq!(ea, eb, "a loaded world continues identically");
}

#[test]
fn loads_are_checked() {
    let data = data();
    let world = world(&data);
    let text = world.to_ron().unwrap();

    let other = text.replacen("schema: 7", "schema: 8", 1);
    assert_eq!(
        World::from_ron(&other, &data, false).unwrap_err(),
        LoadError::Schema(8)
    );

    let mut foreign = world.clone();
    foreign.packs[0].hash ^= 1;
    let foreign_text = foreign.to_ron().unwrap();
    assert_eq!(
        World::from_ron(&foreign_text, &data, false).unwrap_err(),
        LoadError::PackMismatch
    );
    assert!(
        World::from_ron(&foreign_text, &data, true).is_ok(),
        "force skips only the pack check"
    );

    let mut lost = world.clone();
    lost.position = Position {
        map: omnis_core::MapId(99),
        x: 0,
        y: 0,
        facing: Facing::North,
    };
    let lost_text = lost.to_ron().unwrap();
    assert_eq!(
        World::from_ron(&lost_text, &data, true).unwrap_err(),
        LoadError::BadPosition(lost.position)
    );

    assert!(matches!(
        World::from_ron("(", &data, false).unwrap_err(),
        LoadError::Parse(_)
    ));
}

#[test]
fn path_query_reads_the_world() {
    let data = data();
    let mut world = world(&data);
    step(&mut world, &data);
    assert_eq!(query::path(&world, "position.x").as_deref(), Some("16"));
    assert_eq!(query::path(&world, "position.y").as_deref(), Some("15"));
    assert_eq!(
        query::path(&world, "position.facing").as_deref(),
        Some("North")
    );
    assert_eq!(
        query::path(&world, "clocks.Party(0).elapsed").as_deref(),
        Some("1")
    );
    assert_eq!(query::path(&world, "turn").as_deref(), Some("1"));
    assert_eq!(query::path(&world, "mode").as_deref(), Some("Explore"));
    assert_eq!(query::path(&world, "packs.0.id").as_deref(), Some("base"));
    assert_eq!(query::path(&world, "packs.1.id").as_deref(), Some("test"));
    let map = world.position.map.0;
    assert_eq!(
        query::path(&world, &format!("automap.maps.{map}.16,15.layers")).as_deref(),
        Some("7")
    );
    assert_eq!(query::path(&world, "party.gold").as_deref(), Some("0"));
    assert_eq!(
        query::path(&world, "party.members.0.hp"),
        None,
        "no members yet"
    );
    assert_eq!(
        query::path(&world, "settings.save_rule").as_deref(),
        Some("Anywhere")
    );
    assert_eq!(
        query::path(&world, "settings.devtools").as_deref(),
        Some("false")
    );
}

/// A schema-1 save (captured from the M2 build's `schema dump`) loads through the migration.
#[test]
fn a_schema_1_save_migrates() {
    let data = data();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/saves/v1.ron");
    let text = omnis_data::ron_io::read_text(&path, &path).unwrap();
    assert!(text.contains("schema: 1") && text.contains("save_anywhere: true"));
    assert_eq!(
        World::from_ron(&text, &data, false).unwrap_err(),
        LoadError::PackMismatch,
        "the fixture names an example pack"
    );
    let world = World::from_ron(&text, &data, true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(world.schema, 7);
    assert!(world.party.members.is_empty());
    assert_eq!(world.settings, Settings::default());
    assert_eq!(world.to_ron().unwrap().matches("schema: 7").count(), 1);
    let inn_only = text.replace("save_anywhere: true", "save_anywhere: false");
    let world = World::from_ron(&inn_only, &data, true).unwrap();
    assert_eq!(world.settings.save_rule, SaveRule::InnOnly);
    assert!(!world.may_save());
}

/// A schema-2 save (captured from the M3 build, `capture_schema_2_fixture`) loads through the
/// migration: the mode, cleared encounters, and death saves default.
#[test]
fn a_schema_2_save_migrates() {
    let data = data();
    let path = save_path("v2");
    let text = omnis_data::ron_io::read_text(&path, &path).unwrap();
    assert!(text.contains("schema: 2") && !text.contains("death_saves"));
    assert_eq!(
        World::from_ron(&text, &data, false).unwrap_err(),
        LoadError::PackMismatch,
        "the fixture's base pack predates the combat rules"
    );
    let world = World::from_ron(&text, &data, true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(world.schema, 7);
    assert_eq!(world.mode, Mode::Explore);
    assert_eq!(world.party.members.len(), 1);
    assert_eq!(world.party.members[0].death_saves, DeathSaves::default());
    assert_eq!(world.party.gold, 1500, "the acolyte's 15 gp in copper");
    assert!(!world.party.members[0].is_down());
    assert!(
        world.party.members[0].equipped.len() >= 3,
        "the M3 kit is worn on load: {:?}",
        world.party.members[0].equipped
    );
    let dungeon = FIXTURE_DUNGEON;
    assert!(world.maps[&dungeon].door_open(5, 3, Facing::East));
    assert!(world.maps[&dungeon].cleared.is_empty());
    let text = world.to_ron().unwrap();
    assert_eq!(text.matches("schema: 7").count(), 1);
    assert_eq!(
        World::from_ron(&text, &data, true).unwrap(),
        world,
        "written back at schema 5, still on the fixture's pack hashes"
    );
}

/// A schema-3 save (captured from the M4 build, `capture_schema_3_fixture`) loads through the
/// migration: the member wears what the everything-counts rule counted, the effects and the
/// reaction list are empty, and the settings gain a devtools bit that is off.
#[test]
fn a_schema_3_save_migrates() {
    let data = data();
    let path = save_path("v3");
    let text = omnis_data::ron_io::read_text(&path, &path).unwrap();
    assert!(text.contains("schema: 3") && !text.contains("equipped") && !text.contains("devtools"));
    let world = World::from_ron(&text, &data, true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(world.schema, 7);
    assert!(!world.settings.devtools);
    let brenna = &world.party.members[0];
    let item = |name: &str| {
        data.registry
            .items
            .get(&format!("base:item:{name}"))
            .unwrap()
    };
    assert_eq!(brenna.equipped[&EquipSlot::Body], item("chain_mail"));
    assert_eq!(brenna.equipped[&EquipSlot::OffHand], item("shield"));
    assert_eq!(brenna.equipped[&EquipSlot::MainHand], item("longsword"));
    assert_eq!(brenna.equipped[&EquipSlot::Ranged], item("light_crossbow"));
    assert_eq!(
        omnis_rules::armor_class(brenna, &data),
        18,
        "as before the slots"
    );
    assert!(brenna.effects.is_empty() && brenna.tactics.reactions().is_empty());
    assert!(world.party.effects.is_empty());
    assert_eq!(world.party.gold, 1500, "the acolyte's 15 gp in copper");
    let dungeon = FIXTURE_DUNGEON;
    assert!(world.maps[&dungeon].door_open(5, 3, Facing::East));
    let text = world.to_ron().unwrap();
    assert_eq!(text.matches("schema: 7").count(), 1);
    assert_eq!(World::from_ron(&text, &data, true).unwrap(), world);

    let mut torn = world.clone();
    torn.party.members[0].equipment.clear();
    let torn_text = torn.to_ron().unwrap();
    assert_eq!(
        World::from_ron(&torn_text, &data, true).unwrap_err(),
        LoadError::BadParty("an equipped item is not carried")
    );
}

/// A schema-4 save (captured from the M7 step 3b build, `capture_schema_4_fixture`) loads
/// through the migration: the purse counted whole gold and now counts copper; spent hit dice,
/// the bank, the last long rest and the spell picks start empty.
#[test]
fn a_schema_4_save_migrates() {
    let data = data();
    let path = save_path("v4");
    let text = omnis_data::ron_io::read_text(&path, &path).unwrap();
    assert!(text.contains("schema: 4") && text.contains("gold: 15,") && !text.contains("bank"));
    let world = World::from_ron(&text, &data, true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(world.schema, 7);
    assert_eq!(
        (
            world.party.gold,
            world.party.bank,
            world.party.last_long_rest
        ),
        (1500, 0, None)
    );
    assert_eq!(world.party.members[0].hit_dice_spent, 0);
    // M7b's spell picks default to none, so the schema stayed 5 for them.
    assert!(!text.contains("spell_picks"));
    assert!(world.party.members.iter().all(|m| m.spell_picks == 0));
    let dungeon = FIXTURE_DUNGEON;
    assert!(world.maps[&dungeon].door_open(5, 3, Facing::East));
    let text = world.to_ron().unwrap();
    assert_eq!(text.matches("schema: 7").count(), 1);
    assert_eq!(
        World::from_ron(&text, &data, true).unwrap(),
        world,
        "a schema-6 save is read as written: no second multiplication"
    );
}

/// A schema-5 save (captured on the M7b build, `capture_schema_5_fixture`) loads through the
/// migration: Ilvara's auto-cast shield becomes a declared reaction, `Attacked` when it would
/// turn the hit into a miss, in her default runbook; the fight in round one gets the budget and
/// the reactions the turn slots give, and goes on.
#[test]
fn a_schema_5_save_migrates() {
    let data = data();
    let path = save_path("v5");
    let text = omnis_data::ron_io::read_text(&path, &path).unwrap();
    assert!(text.contains("schema: 5") && text.contains("auto_cast: ["));
    assert!(!text.contains("tactics") && !text.contains("budget"));
    let world = World::from_ron(&text, &data, true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(world.schema, 7);
    let shield = data.registry.spells.get("base:spell:shield").unwrap();
    let ilvara = &world.party.members[2];
    assert!(ilvara.legacy_auto_cast.is_empty());
    assert_eq!(
        ilvara.tactics.reactions(),
        [&omnis_sim::omnis_rules::CriteriaSet {
            name: "Shield".to_owned(),
            action: omnis_sim::omnis_rules::ActionRef::Spell(shield),
            trigger: omnis_sim::omnis_rules::Trigger::Attacked,
            when: omnis_sim::omnis_rules::Criteria::Is(
                omnis_sim::omnis_rules::Predicate::WouldChangeOutcome
            ),
        }]
    );
    assert!(ilvara.tactics.reactions_on);
    for member in &world.party.members[..2] {
        assert!(member.tactics.reactions().is_empty(), "{}", member.name);
    }
    let Mode::Combat(fight) = &world.mode else {
        panic!("still fighting");
    };
    for entry in &fight.order {
        assert_eq!(fight.reactions_left(entry.actor), 1, "{:?}", entry.actor);
    }
    if let Some(omnis_sim::ActorRef::Member(_)) = fight.current_actor() {
        assert_eq!(
            fight.budget,
            omnis_sim::Budget {
                actions: 1,
                bonus_actions: 1
            },
            "the member the fight waits on can act"
        );
    }
    let written = world.to_ron().unwrap();
    assert_eq!(written.matches("schema: 7").count(), 1);
    assert!(!written.contains("auto_cast"), "never written");
    assert_eq!(World::from_ron(&written, &data, true).unwrap(), world);
    let mut world = world;
    let events = apply(
        &mut world,
        &data,
        Command::Combat(omnis_sim::CombatCommand::Dodge),
    )
    .unwrap();
    assert!(!events.is_empty(), "the migrated fight goes on");
}

/// Loot not yet paid out is money too: a schema-4 save made mid-fight converts it with the
/// purse, so the victory pays copper.
#[test]
fn a_schema_4_fight_converts_its_loot() {
    let data = data();
    let mut world = world(&data);
    for draft in six() {
        apply(
            &mut world,
            &data,
            Command::Party(PartyCommand::Create(draft)),
        )
        .unwrap();
    }
    for command in walk_to_the_rats() {
        if world.mode != Mode::Explore {
            break;
        }
        apply(&mut world, &data, command).unwrap();
    }
    apply(
        &mut world,
        &data,
        Command::Encounter(EncounterChoice::Attack),
    )
    .unwrap();
    let Mode::Combat(fight) = &mut world.mode else {
        panic!("the rats fight: {:?}", world.mode);
    };
    fight.gold = 7;
    world.party.gold = 90;
    let old = world
        .to_ron()
        .unwrap()
        .replacen("schema: 7", "schema: 4", 1);
    let loaded = World::from_ron(&old, &data, false).unwrap_or_else(|e| panic!("{e}"));
    let Mode::Combat(fight) = &loaded.mode else {
        panic!("still fighting");
    };
    assert_eq!((fight.gold, loaded.party.gold), (700, 9000));
}

#[test]
fn replays_are_deterministic() {
    let data = data();
    let commands = walk();
    let settings = Settings::default();
    let a = omnis_sim::replay::run(&data, 1, settings, &commands).unwrap();
    let b = omnis_sim::replay::run(&data, 1, settings, &commands).unwrap();
    let c = omnis_sim::replay::run(&data, 2, settings, &commands).unwrap();
    let hard = Settings {
        save_rule: SaveRule::InnOnly,
        permadeath: true,
        devtools: false,
    };
    let d = omnis_sim::replay::run(&data, 1, hard, &commands).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c, "the seed is part of the world");
    assert_ne!(a, d, "the settings are part of the world");
    let recorded = Replay::record(&data, 1, settings, commands).unwrap();
    assert_eq!(recorded.check(&data), Ok(()));
    let mut wrong = recorded.clone();
    wrong.fingerprint ^= 1;
    assert!(matches!(
        wrong.check(&data),
        Err(ReplayError::Diverged { .. })
    ));
    let mut other_packs = recorded;
    other_packs.packs.clear();
    assert_eq!(other_packs.check(&data), Err(ReplayError::PackMismatch));
}

/// The golden replay. When `packs/test` or the simulation changes on purpose, re-baseline in
/// the same commit with `cargo test -p omnis-sim rebaseline -- --ignored`.
#[test]
fn golden_walk_replay_reproduces() {
    let data = data();
    let replay: Replay =
        read_ron(&replay_path("walk"), &replay_path("walk")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        replay.commands,
        golden_walk(&data),
        "the script in this file is the recorded one"
    );
    replay.check(&data).unwrap_or_else(|e| panic!("{e}"));
}

/// A declared reaction is part of the command log: the golden fight's walk with Ilvara's shield
/// declared for every hit replays to the fingerprint the play ended on.
#[test]
fn a_replay_reproduces_declared_reactions() {
    use omnis_sim::omnis_rules::{ActionRef, Criteria, CriteriaSet, Named, Trigger};
    use omnis_sim::tactics::TacticsCommand;
    let data = data();
    let seed = 0x0123_4567_89ab_cdef;
    let mut world = World::new(&data, seed, Settings::default()).unwrap();
    let mut commands: Vec<Command> = six()
        .into_iter()
        .map(|d| Command::Party(PartyCommand::Create(d)))
        .collect();
    for command in &commands {
        apply(&mut world, &data, command.clone()).unwrap();
    }
    let put = Command::Party(PartyCommand::Tactics(TacticsCommand::PutReaction {
        member: world.party.members[2].id,
        entry: None,
        set: CriteriaSet::<Named> {
            name: "Shield".to_owned(),
            action: ActionRef::Spell("base:spell:shield".to_owned()),
            trigger: Trigger::Attacked,
            when: Criteria::Always,
        },
    }));
    apply(&mut world, &data, put.clone()).unwrap();
    commands.push(put);
    let mut walk = out_of_town();
    walk.extend(walk_to_the_rats());
    let (taken, events) = play(&mut world, &data, &walk);
    commands.extend(taken);
    let fired = events.iter().any(|e| matches!(e, Event::Reaction { .. }));
    let recorded = Replay::record(&data, seed, Settings::default(), commands.clone()).unwrap();
    assert_eq!(recorded.check(&data), Ok(()));
    assert_eq!(
        Replay::record(&data, seed, Settings::default(), commands)
            .unwrap()
            .fingerprint,
        recorded.fingerprint
    );
    assert_eq!(
        recorded.fingerprint,
        world.fingerprint().unwrap(),
        "the replay ends where the play did (shield fired: {fired})"
    );
}

/// The command script behind `tests/replays/fight.ron`: six members, out of town through the
/// gate, the walk to the rats at (3, 8) of the dungeon, and the fight to victory. Built by running it, so whatever the
/// dice bring on the way (a random encounter, a member down) is part of the record.
fn fight(data: &Data) -> Vec<Command> {
    let mut world = World::new(data, 0x0123_4567_89ab_cdef, Settings::default()).unwrap();
    let mut commands = Vec::new();
    for draft in six() {
        let command = Command::Party(PartyCommand::Create(draft));
        apply(&mut world, data, command.clone()).unwrap();
        commands.push(command);
    }
    let mut walk = out_of_town();
    walk.extend(walk_to_the_rats());
    let (taken, events) = play(&mut world, data, &walk);
    commands.extend(taken);
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::CombatEnded {
                outcome: omnis_sim::CombatOutcome::Victory,
                ..
            }
        )),
        "the rats are cleared"
    );
    assert_eq!(world.mode, Mode::Explore);
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    assert!(world.maps[&dungeon].cleared.contains(&0));
    commands
}

/// The golden fight; re-baseline with `cargo test -p omnis-sim rebaseline -- --ignored` when
/// the packs or the simulation change on purpose.
#[test]
fn golden_fight_replay_reproduces() {
    let data = data();
    let replay: Replay =
        read_ron(&replay_path("fight"), &replay_path("fight")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        replay.commands,
        fight(&data),
        "the script in this file is the recorded one"
    );
    replay.check(&data).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
#[ignore = "writes the golden file; run deliberately"]
fn rebaseline_fight_replay() {
    let data = data();
    let commands = fight(&data);
    let replay =
        Replay::record(&data, 0x0123_4567_89ab_cdef, Settings::default(), commands).unwrap();
    write_ron(&replay_path("fight"), &replay).unwrap();
}

#[test]
#[ignore = "writes the golden file; run deliberately"]
fn rebaseline_walk_replay() {
    let data = data();
    let replay = Replay::record(&data, WALK_SEED, Settings::default(), golden_walk(&data)).unwrap();
    write_ron(&replay_path("walk"), &replay).unwrap();
}

fn save_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/saves")
        .join(format!("{name}.ron"))
}

/// Captures `tests/saves/v2.ron` from a schema-2 build: one member and one open door, so the
/// migration test exercises every field a party and a map state carry. Run once, deliberately,
/// before the schema moves on.
#[test]
#[ignore = "writes the fixture; run deliberately on a schema-2 build"]
fn capture_schema_2_fixture() {
    let data = data();
    let mut world = world(&data);
    let draft = omnis_rules::Draft {
        name: "Brenna".to_owned(),
        race: "base:race:human".to_owned(),
        class: "base:class:fighter".to_owned(),
        background: "base:background:acolyte".to_owned(),
        alignment: omnis_data::Alignment::LawfulGood,
        scores: [15, 14, 13, 12, 10, 8],
        skills: vec![omnis_data::Skill::Athletics, omnis_data::Skill::Perception],
    };
    apply(
        &mut world,
        &data,
        Command::Party(PartyCommand::Create(draft)),
    )
    .unwrap();
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    world.position = Position {
        map: dungeon,
        x: 5,
        y: 3,
        facing: Facing::East,
    };
    interact(&mut world, &data);
    assert!(world.maps[&dungeon].door_open(5, 3, Facing::East));
    assert_eq!(world.schema, 2);
    write_ron(&save_path("v2"), &world).unwrap();
}

/// Captures `tests/saves/v3.ron` from a schema-3 build: one member with the kit rule that
/// counted everything carried as worn, and one open door. Run once, deliberately, before the
/// schema moves on.
#[test]
#[ignore = "writes the fixture; run deliberately on a schema-3 build"]
fn capture_schema_3_fixture() {
    let data = data();
    let mut world = world(&data);
    let draft = omnis_rules::Draft {
        name: "Brenna".to_owned(),
        race: "base:race:human".to_owned(),
        class: "base:class:fighter".to_owned(),
        background: "base:background:acolyte".to_owned(),
        alignment: omnis_data::Alignment::LawfulGood,
        scores: [15, 14, 13, 12, 10, 8],
        skills: vec![omnis_data::Skill::Athletics, omnis_data::Skill::Perception],
    };
    apply(
        &mut world,
        &data,
        Command::Party(PartyCommand::Create(draft)),
    )
    .unwrap();
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    world.position = Position {
        map: dungeon,
        x: 5,
        y: 3,
        facing: Facing::East,
    };
    interact(&mut world, &data);
    assert!(world.maps[&dungeon].door_open(5, 3, Facing::East));
    assert_eq!(world.schema, 3);
    write_ron(&save_path("v3"), &world).unwrap();
}

/// Captures `tests/saves/v4.ron` from a schema-4 build: one acolyte fighter carrying 15 whole
/// gold pieces, in the dungeon behind an open door, so the copper migration has a purse to
/// convert. Run once, deliberately, before the schema moves on.
#[test]
#[ignore = "writes the fixture; run deliberately on a schema-4 build"]
fn capture_schema_4_fixture() {
    let data = data();
    let mut world = world(&data);
    let draft = omnis_rules::Draft {
        name: "Brenna".to_owned(),
        race: "base:race:human".to_owned(),
        class: "base:class:fighter".to_owned(),
        background: "base:background:acolyte".to_owned(),
        alignment: omnis_data::Alignment::LawfulGood,
        scores: [15, 14, 13, 12, 10, 8],
        skills: vec![omnis_data::Skill::Athletics, omnis_data::Skill::Perception],
    };
    apply(
        &mut world,
        &data,
        Command::Party(PartyCommand::Create(draft)),
    )
    .unwrap();
    let dungeon = data.registry.maps.get("test:map:dungeon").unwrap();
    world.position = Position {
        map: dungeon,
        x: 5,
        y: 3,
        facing: Facing::East,
    };
    interact(&mut world, &data);
    assert!(world.maps[&dungeon].door_open(5, 3, Facing::East));
    assert_eq!((world.schema, world.party.gold), (4, 15));
    write_ron(&save_path("v4"), &world).unwrap();
}

/// A schema-6 save (captured from the M8 step 2 build, `capture_schema_6_fixture`) loads through
/// the migration: the party's past counts in full as shared time and date, the regions subscribe
/// to reconciliation, the fight saved mid-way subscribes its reactions, Ilvara's declared Shield
/// stays, and the fight goes on.
#[test]
fn a_schema_6_save_migrates() {
    use omnis_sim::bus::{Subscriber, Topic};
    let data = data();
    let path = save_path("v6");
    let text = omnis_data::ron_io::read_text(&path, &path).unwrap();
    assert!(text.contains("schema: 6") && !text.contains("party_time") && !text.contains("bus"));
    let mut world = World::from_ron(&text, &data, true).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(world.schema, 7);
    let age = world.party_clock().elapsed;
    assert!(age > 0, "two steps lived");
    assert_eq!(
        (world.party_time.shared_milli, world.party_time.date),
        (age * 1000, age)
    );
    assert_eq!(
        world.bus.subscribers(Topic::Battle),
        [Subscriber::Reactions]
    );
    assert_eq!(world.party.members[2].tactics.reactions().len(), 1);
    assert!(matches!(world.mode, Mode::Combat(_)));
    let text = world.to_ron().unwrap();
    assert_eq!(World::from_ron(&text, &data, true).unwrap(), world);
    for _ in 0..40 {
        if !matches!(world.mode, Mode::Combat(_)) {
            break;
        }
        omnis_sim::apply(
            &mut world,
            &data,
            omnis_sim::Command::Combat(omnis_sim::CombatCommand::Dodge),
        )
        .unwrap_or_else(|r| panic!("{r}"));
    }
}

/// Captures `tests/saves/v5.ron` from a schema-5 build: Brenna, Durin and Ilvara, Ilvara with
/// shield in `auto_cast`, in round one of a fight with two goblins, so the schema-6 migration has
/// an auto-cast spell to turn into a declared reaction and a fight state to carry. Run once,
/// deliberately, before the schema moves on.
#[test]
#[ignore = "writes the fixture; run deliberately on a schema-5 build"]
fn capture_schema_5_fixture() {
    use omnis_sim::{Surprise, combat};
    let data = data();
    let mut world = world(&data);
    common::party_of(&mut world, &data, 3);
    let shield = data.registry.spells.get("base:spell:shield").unwrap();
    // On the schema-5 build this was `PartyCommand::AutoCast { member: 2, spell, on: true }`;
    // the command is gone since schema 6, and the field it set is read only from old saves.
    world.party.members[2].legacy_auto_cast = vec![shield];
    let here = world.position;
    let goblins = common::encounter(
        &data,
        &[("goblin", 2)],
        omnis_data::Disposition::Hostile,
        here,
    );
    let mut events = Vec::new();
    combat::start(&mut world, &data, goblins, Surprise::None, &mut events).unwrap();
    assert!(matches!(world.mode, Mode::Combat(_)));
    assert_eq!(world.schema, 5);
    write_ron(&save_path("v5"), &world).unwrap();
}

/// Captures `tests/saves/v6.ron` from a schema-6 build (M8 step 2, `0bcb02f`): Brenna, Durin and
/// Ilvara, Ilvara with Shield declared, two steps lived, in round one of a fight with two goblins,
/// so the schema-7 migration has a past to count as shared time and a fight to subscribe to the
/// bus. Run once, deliberately, before the schema moved on (captured late, in M8 step 8c, from a
/// worktree of that commit).
#[test]
#[ignore = "writes the fixture; run deliberately on a schema-6 build"]
fn capture_schema_6_fixture() {
    use omnis_sim::omnis_rules::{ActionRef, Criteria, CriteriaSet, Named, Predicate, Trigger};
    use omnis_sim::tactics::TacticsCommand;
    use omnis_sim::{PartyCommand, Surprise, apply, combat};
    let data = data();
    let mut world = world(&data);
    common::party_of(&mut world, &data, 3);
    let set = CriteriaSet::<Named> {
        name: "Shield".to_owned(),
        action: ActionRef::Spell("base:spell:shield".to_owned()),
        trigger: Trigger::Attacked,
        when: Criteria::Is(Predicate::WouldChangeOutcome),
    };
    let put = PartyCommand::Tactics(TacticsCommand::PutReaction {
        member: world.party.members[2].id,
        entry: None,
        set,
    });
    apply(&mut world, &data, omnis_sim::Command::Party(put)).unwrap();
    step(&mut world, &data);
    step(&mut world, &data);
    let here = world.position;
    let goblins = common::encounter(
        &data,
        &[("goblin", 2)],
        omnis_data::Disposition::Hostile,
        here,
    );
    let mut events = Vec::new();
    combat::start(&mut world, &data, goblins, Surprise::None, &mut events).unwrap();
    assert!(matches!(world.mode, Mode::Combat(_)));
    assert_eq!(world.schema, 6);
    write_ron(&save_path("v6"), &world).unwrap();
}
