//! The HUD's lines for objects (M8.5 step 2b item 5): an object's check, a failed use, a new
//! state, a removal, what was found or spent, a search and each way of noticing a hidden thing,
//! each object by its name in the pack. Real commands on the yard fixture, the fixed party.

use crate::common;

use common::{objects_data, place, town_session};
use omnis_sim::omnis_core::Facing;
use omnis_sim::omnis_data::{Data, Verb};
use omnis_sim::{Command, Event, ItemPlace, Notice};
use omnis_vector::labels;
use omnis_vector::rolllog::Names;
use omnis_vector::shell::session::{Config, Session};
use omnis_vector::shell::text;
use omnis_vector::sim::SimWorld;
use std::path::PathBuf;

fn yard(data: &Data, x: u16, y: u16, facing: Facing) -> SimWorld {
    let (mut world, _) = town_session(data);
    let map = common::map(data, "objects:map:yard");
    place(&mut world, map, x, y, facing);
    world
}

/// The names as the session keeps them: the party, and the map it stands on.
fn names(world: &SimWorld, data: &Data) -> Names {
    let views = common::views(world, data);
    let mut names = Names::default();
    names.observe(&views.party);
    names.locate(views.position.map);
    names
}

/// The HUD lines of `events`.
fn said(world: &SimWorld, data: &Data, events: &[Event]) -> Vec<String> {
    let names = names(world, data);
    events
        .iter()
        .filter_map(|e| text::describe(e, &names, data))
        .collect()
}

/// Apply `command` and answer its lines.
fn lines(world: &mut SimWorld, data: &Data, command: Command) -> Vec<String> {
    let events = world.apply(data, command).expect("accepted");
    said(world, data, &events)
}

fn act(object: &str, verb: Verb) -> Command {
    Command::Act {
        object: object.to_owned(),
        verb,
        option: 0,
    }
}

#[test]
fn a_found_gem_goes_to_the_stores_and_the_wish_spends_it_with_its_check() {
    let data = objects_data();
    let mut world = yard(&data, 0, 2, Facing::East);
    let gem = labels::item(&data, "base:item:gem");
    assert_eq!(
        lines(&mut world, &data, act("stone_1", Verb::Take)),
        [
            format!("Found {gem}; into the stores"),
            "Under the stone lies a gem.".to_owned(),
            "Loose stone is gone".to_owned(),
        ]
    );
    // The fountain, from the cell south of it (the hedge runs north of it). A failed wish keeps
    // the gem; try until one holds.
    let map = common::map(&data, "objects:map:yard");
    place(&mut world, map, 1, 4, Facing::North);
    for _ in 0..50 {
        let got = lines(&mut world, &data, act("fountain_1", Verb::Use));
        let check = &got[0];
        assert!(check.contains(" Wisdom: "), "{got:?}");
        assert!(check.ends_with(" against 12, success") || check.ends_with(" against 12, failure"));
        if check.ends_with("failure") {
            assert_eq!(
                got[1..],
                [
                    "Fountain: Use failed".to_owned(),
                    "The water stays still. You fish the gem back out.".to_owned(),
                ]
            );
            continue;
        }
        assert_eq!(
            got[1..],
            [
                format!("{gem} from the stores is used up"),
                format!("Found {}", labels::coins(100)),
                "Silver coins bob up where the gem sank.".to_owned(),
            ]
        );
        return;
    }
    panic!("no wish held");
}

#[test]
fn a_new_state_is_said_by_the_object_s_name() {
    let data = objects_data();
    let mut world = yard(&data, 6, 3, Facing::South);
    world.apply(&data, Command::Door).expect("the door opens");
    assert_eq!(
        lines(&mut world, &data, act("torch_1", Verb::Extinguish)),
        ["Torch is now out"]
    );
}

#[test]
fn a_search_says_who_searched_and_what_they_rolled() {
    let data = objects_data();
    // Far from the lever and the rune, so nothing is spotted as the search's minutes pass.
    let mut world = yard(&data, 5, 1, Facing::North);
    let got = lines(&mut world, &data, Command::Search);
    let party = common::views(&world, &data).party;
    assert_eq!(got.len(), 1, "nothing hidden here: {got:?}");
    let (who, rolled) = got[0]
        .split_once(" searches: Investigation ")
        .expect(&got[0]);
    assert!(party.members.iter().any(|m| m.name == who), "{who}");
    assert!(rolled.parse::<i64>().is_ok(), "{rolled}");
}

#[test]
fn each_way_of_noticing_names_the_object() {
    let data = objects_data();
    let world = yard(&data, 2, 3, Facing::South);
    let noticed = |how| Event::Noticed {
        object: "rune_1".to_owned(),
        how,
    };
    assert_eq!(
        said(
            &world,
            &data,
            &[
                noticed(Notice::Passive),
                noticed(Notice::Search),
                noticed(Notice::Spell),
            ]
        ),
        ["Spotted: Rune", "Found: Rune", "Sensed by magic: Rune"]
    );
    // A member's own item used up, and an object the map does not hold.
    let first = common::views(&world, &data).party.members[0].clone();
    let tools = labels::item(&data, "base:item:thieves_tools");
    assert_eq!(
        said(
            &world,
            &data,
            &[
                Event::ItemSpent {
                    item: "base:item:thieves_tools".to_owned(),
                    count: 1,
                    from: ItemPlace::Member(first.member),
                },
                Event::ObjectRemoved {
                    object: "no_such".to_owned(),
                },
            ]
        ),
        [
            format!("{} uses up {tools}", first.name),
            "Something is gone".to_owned(),
        ]
    );
}

#[test]
fn a_placement_s_own_name_wins_over_its_kind_s() {
    // The statue's placement names it "The knight".
    let data = objects_data();
    let world = yard(&data, 2, 3, Facing::South);
    let got = said(
        &world,
        &data,
        &[Event::ObjectState {
            object: statue(&data),
            state: "toppled".to_owned(),
        }],
    );
    assert_eq!(got, ["The knight is now toppled"]);
}

/// The yard's statue's placement id.
fn statue(data: &Data) -> String {
    let map = common::map(data, "objects:map:yard");
    data.maps[&map]
        .objects
        .iter()
        .find(|p| p.name.is_some())
        .map(|p| p.id.clone())
        .expect("a placement with its own name")
}

#[test]
fn in_the_window_a_search_s_line_stands_in_the_log_and_the_notice() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut packs = common::app::config().packs;
    packs.push(root.join("crates/omnis-sim/tests/packs/objects"));
    let mut app = common::app::shell(
        Config {
            packs,
            ..common::app::config()
        },
        true,
    );
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let session = &mut *session;
        let map = common::map(&session.data, "objects:map:yard");
        session.pose = place(&mut session.world, map, 1, 2, Facing::North);
        session.refresh();
    }
    app.update();
    let _ = app
        .world_mut()
        .resource_mut::<Session>()
        .ordered(Command::Search);
    app.update();
    let s = app.world().resource::<Session>();
    assert!(
        s.lines
            .iter()
            .any(|l| l.contains(" searches: Investigation ")),
        "{:?}",
        s.lines
    );
    let notice = s.notice.as_ref().expect("the search's notice");
    assert!(
        notice.lines[0].contains(" searches: "),
        "{:?}",
        notice.lines
    );
    // The session names objects on the map the party stands on.
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let session = &mut *session;
        let map = common::map(&session.data, "objects:map:yard");
        session.pose = place(&mut session.world, map, 0, 2, Facing::East);
        session.refresh();
    }
    let _ = app
        .world_mut()
        .resource_mut::<Session>()
        .ordered(act("stone_1", Verb::Take));
    app.update();
    let s = app.world().resource::<Session>();
    assert!(
        s.lines.iter().any(|l| l == "Loose stone is gone"),
        "{:?}",
        s.lines
    );
}
