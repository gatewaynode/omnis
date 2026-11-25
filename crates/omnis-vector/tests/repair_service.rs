//! The smith's repair service as the player meets it (`tasks/plans/repair-service.md`, step
//! 11b): a "Repairs" list on the smith's screen only when something of its region is harmed,
//! each row naming the thing, its harm and its map with the price or the reason; the button
//! sends the command, and the work is told. The fixture's forge and yard are one region.

use crate::common;
use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::Data;
use omnis_sim::world::DoorWear;
use omnis_sim::{Command, Event, Job, ServiceCommand};
use omnis_vector::labels;
use omnis_vector::rolllog::Names;
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, Screen, ScreenId};
use omnis_vector::shell::text;
use omnis_vector::sim::SimWorld;
use omnis_vector::style::Role;

const FORGE: &str = "objects:map:forge";
const YARD: &str = "objects:map:yard";

/// The party inside the forge's smith with 50 gp, the forge's crate broken, the yard's crate
/// dented and the forge's door fallen in.
fn at_smith(data: &Data) -> SimWorld {
    let mut world = crate::loot::yard(data, 0, 2, Facing::East);
    let (forge, yard) = (common::map(data, FORGE), common::map(data, YARD));
    world.party.gold = 5000;
    let state = world.map_state(forge);
    let broken = state.objects.entry("crate_f".into()).or_default();
    (broken.damage, broken.broken) = (18, true);
    state.wear_door(Level::GROUND, (1, 0, Facing::East), |w| {
        *w = DoorWear {
            damage: 18,
            broken: true,
            collapsed: true,
        };
    });
    let dented = world.map_state(yard).objects.entry("crate_1".into());
    dented.or_default().damage = 3;
    common::place(&mut world, forge, 0, 0, Facing::North);
    world
        .apply(data, Command::Interact)
        .expect("a site tile opens its service");
    world
}

fn shown(data: &Data, world: &SimWorld) -> Screen {
    let views = common::views(world, data);
    screen::content(
        ScreenId::Service,
        &common::ctx(data, world, &views),
        &Forms::default(),
    )
}

/// Every heading on the screen: the spans drawn in the title role.
fn headings(shown: &Screen) -> Vec<String> {
    let rows = shown.panels.iter().flat_map(|p| &p.rows);
    let spans = rows.flat_map(|r| &r.spans);
    let titles = spans.filter(|s| s.role == Role::Title);
    titles.map(|s| s.text.clone()).collect()
}

/// Each repair on offer: its row's words, its price or what a sale pays, and why not.
fn repairs(data: &Data, world: &SimWorld) -> Vec<(String, &'static str, String, Option<String>)> {
    let views = common::views(world, data);
    let service = views.service.as_ref().expect("inside a service");
    let offers = service.offers.iter();
    offers
        .filter(|o| matches!(o.command, ServiceCommand::Repair { .. }))
        .map(|o| {
            (
                labels::offer_label(data, &views.party, o),
                labels::offer_caption(&o.command),
                labels::offer_note(o),
                o.refusal.as_ref().map(labels::service_reason),
            )
        })
        .collect()
}

#[test]
fn the_smith_lists_repairs_only_when_something_is_harmed() {
    let data = common::objects_data();
    let world = at_smith(&data);
    assert_eq!(
        headings(&shown(&data, &world)),
        ["For sale", "Your stores", "Repairs"]
    );
    assert_eq!(
        repairs(&data, &world),
        [
            (
                "Crate (broken), Test Forge".to_owned(),
                "Repair",
                labels::coins(1000),
                None
            ),
            (
                "Door (broken), Test Forge".to_owned(),
                "Repair",
                String::new(),
                Some("the rubble must be cleared first".to_owned())
            ),
            (
                "Crate (damaged), Test Yard".to_owned(),
                "Repair",
                labels::coins(500),
                None
            ),
        ]
    );

    // With nothing harmed the smith's screen is as it was.
    let mut whole = crate::loot::yard(&data, 0, 2, Facing::East);
    common::place(&mut whole, common::map(&data, FORGE), 0, 0, Facing::North);
    whole.apply(&data, Command::Interact).expect("the smith");
    assert_eq!(headings(&shown(&data, &whole)), ["For sale", "Your stores"]);
    assert_eq!(repairs(&data, &whole), []);
}

#[test]
fn a_door_above_the_ground_is_named_with_its_level() {
    let data = common::objects_data();
    let mut world = at_smith(&data);
    let yard = common::map(&data, YARD);
    let loft = Level(1);
    world
        .map_state(yard)
        .wear_door(loft, (5, 0, Facing::East), |w| w.damage = 2);
    let rows = repairs(&data, &world);
    let row = rows.iter().find(|r| r.0.starts_with("Door (damaged)"));
    let row = row.expect("the loft's door is listed, whatever the smith then says of it");
    assert_eq!(row.0, "Door (damaged), Test Yard, Loft (1)");
}

#[test]
fn the_repair_button_sends_the_job_and_the_work_is_told() {
    let data = common::objects_data();
    let mut world = at_smith(&data);
    let views = common::views(&world, &data);
    let offers = &views.service.as_ref().expect("inside").offers;
    let job = Job::Object {
        map: FORGE.into(),
        object: "crate_f".into(),
    };
    let command = ServiceCommand::Repair { job };
    let index = offers.iter().position(|o| o.command == command);
    let id = ControlId::Offer(u16::try_from(index.expect("the crate's offer")).unwrap());
    let ctx = common::ctx(&data, &world, &views);
    let screen = screen::content(ScreenId::Service, &ctx, &Forms::default());
    assert_eq!(screen.control(id).expect("its button").label, "Repair");
    let used = screen::apply(
        ScreenId::Service,
        &ctx,
        &mut Forms::default(),
        id,
        &Payload::Press,
        0,
    );
    let command = Command::Service(command);
    assert_eq!(
        used,
        Ok(Effect::Command {
            command: command.clone(),
            close: false
        })
    );

    let names = Names::default();
    let events = world.apply(&data, command).expect("paid and done");
    let told: Vec<String> = events
        .iter()
        .filter(|e| matches!(e, Event::RepairBought { .. } | Event::ObjectRepaired { .. }))
        .map(|e| text::describe(e, &names, &data).expect("told"))
        .collect();
    assert_eq!(
        told,
        [
            format!("A repair for {}", labels::coins(1000)),
            "The smith repairs Crate".to_owned()
        ]
    );
    let door = Event::DoorRepaired {
        map: FORGE.into(),
        level: Level::GROUND,
        x: 1,
        y: 0,
        facing: Facing::East,
        by: None,
        mended: false,
    };
    assert_eq!(
        text::describe(&door, &names, &data).as_deref(),
        Some("The smith repairs the door")
    );
    assert_eq!(repairs(&data, &world).len(), 2, "the crate is off the list");
}
