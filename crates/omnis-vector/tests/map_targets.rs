//! Things of the map in the road's target step (owner, 2026-11-14; `tasks/plans/targeting.md`
//! step 6): a hand's targets are the doors and objects the engine lists, each named with the way
//! it lies and how far, dim with the engine's reason; the one chosen is what the cast names; and
//! the cell of the row under the pointer is marked in the 3D view.

use crate::common;
use crate::common::app::Pointing;
use common::ctx;

use bevy::prelude::*;
use omnis_sim::omnis_core::{Facing, Level, Position};
use omnis_sim::{Command, Target};
use omnis_vector::labels;
use omnis_vector::pose::Pose;
use omnis_vector::screen::roster::Tool;
use omnis_vector::screen::{self, ControlId, Effect, Forms, Payload, Refused, ScreenId};
use omnis_vector::shell::aim::Marked;
use omnis_vector::shell::render::{RenderPlugin, ViewAim};
use omnis_vector::shell::screen::{Pressable, Screens, Side};
use omnis_vector::shell::session::Session;
use omnis_vector::targeting::Aim;

const HAND: &str = "base:spell:mage_hand";

/// South of the yard's hedge, at the gate, facing east: the gate a tile north (shut fast), the
/// chest two east, the fountain two west, and the door behind which the torch stands.
#[test]
fn a_hand_s_targets_are_named_with_the_way_they_lie_and_how_far() {
    let data = common::objects_data();
    let world = crate::loot::yard(&data, 3, 3, Facing::East);
    let ilvara = common::id(&world, 2);
    let views = common::views(&world, &data);
    let context = ctx(&data, &world, &views);
    let game = context.game.unwrap();
    let mut forms = Forms::default();
    forms.owner = Some(ilvara);
    forms.aim = Some(Aim::spell(ilvara, HAND));
    let step = screen::content(ScreenId::Cast, &context, &forms);
    assert_eq!(step.title.text, "Choose a target");
    let rows: Vec<_> = step.panels[0]
        .rows
        .iter()
        .filter(|r| r.controls.first().is_some_and(|c| c.label == "Choose"))
        .collect();
    let said: Vec<(String, String)> = rows
        .iter()
        .map(|r| (r.spans[0].text.clone(), r.after[0].text.clone()))
        .collect();
    let name = |key: &str| labels::text(&data, key).to_owned();
    assert_eq!(
        said,
        [
            (
                format!("{}, north", name("objects:text:gate.name")),
                "1 tile".to_owned()
            ),
            (
                format!("{}, east", name("objects:text:chest.name")),
                "2 tiles".to_owned()
            ),
            (
                format!("{}, west", name("objects:text:fountain.name")),
                "2 tiles".to_owned()
            ),
            ("Door, east".to_owned(), "3 tiles".to_owned()),
        ]
    );
    // The gate is listed dim, with the engine's reason, and pressing it sends nothing.
    let why = rows[0].controls[0].why_not.clone().expect("shut fast");
    assert!(!why.is_empty());
    let press = |forms: &mut Forms, at: u8| {
        let id = ControlId::Target(at);
        screen::apply(ScreenId::Cast, &context, forms, id, &Payload::Press, 0)
    };
    assert_eq!(press(&mut forms.clone(), 0), Err(Refused::Dim(why)));
    // The door is free: the cast names it, and the engine takes it.
    let mut chosen = forms.clone();
    let Ok(Effect::Command { command, .. }) = press(&mut chosen, 3) else {
        panic!("the door's row sends the cast");
    };
    let door = Target::Door {
        x: 6,
        y: 3,
        facing: Facing::South,
    };
    assert_eq!(
        command,
        Command::Cast {
            caster: ilvara,
            spell: HAND.to_owned(),
            target: door,
        }
    );
    assert!(world.trial(&data, &command).is_ok());
    assert_eq!(chosen.aim, None, "the step is over");
    // What the 3D view marks for each row: the thing's cell; a member has none.
    let aim = forms.aim.clone().unwrap();
    let marks: Vec<_> = (0..5).map(|at| aim.marked(&data, game, at)).collect();
    assert_eq!(
        marks,
        [Some((3, 2)), Some((5, 3)), Some((1, 3)), Some((6, 3)), None]
    );
    let heal = Aim::spell(common::id(&world, 1), "base:spell:cure_wounds");
    assert_eq!(heal.marked(&data, game, 0), None);
}

/// The ends of the view's outline, when one is drawn.
fn outline(app: &mut App) -> Option<Vec<[f32; 3]>> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<Option<&Mesh3d>, With<ViewAim>>();
    let handle = q.single(world).expect("one aim entity")?.0.clone();
    let mesh = world.resource::<Assets<Mesh>>().get(&handle).expect("mesh");
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(bevy::mesh::VertexAttributeValues::Float32x3(ends)) => Some(ends.clone()),
        _ => panic!("positions"),
    }
}

/// In the dungeon: the pointer on a door's row marks the door's cell in the 3D view, off it the
/// mark goes, and it goes when the step ends.
#[test]
fn the_row_under_the_pointer_is_marked_in_the_view() {
    let mut app = common::app::app_with(".omnis/vector-session.ron".into(), 1);
    app.add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_plugins(RenderPlugin);
    app.update();
    let ilvara = {
        let mut session = app.world_mut().resource_mut::<Session>();
        let map = common::map(&session.data, "test:map:dungeon");
        session.world.position = Position {
            map,
            level: Level::GROUND,
            x: 9,
            y: 2,
            facing: Facing::South,
        };
        session.pose = Pose::at(session.world.position);
        session.refresh();
        session.views.party.members[2].member
    };
    app.update();
    common::app::press_ui(&mut app, Side(ControlId::Member(ilvara)));
    common::app::press_ui(&mut app, Side(ControlId::Open(Tool::Spells)));
    let hand = {
        let casts = &app.world().resource::<Session>().views.casts;
        let at = casts
            .iter()
            .position(|c| c.caster == ilvara && c.spell == HAND);
        ControlId::Cast(u16::try_from(at.expect("Ilvara's mage hand")).unwrap())
    };
    common::app::press_ui(&mut app, Pressable(hand));
    let marked = |app: &App| app.world().resource::<Marked>().0;
    assert!(app.world().resource::<Screens>().forms.aim.is_some());
    assert_eq!(marked(&app), None, "the pointer is on no row");
    let expected = {
        let session = app.world().resource::<Session>();
        let rows = session.world.targets(
            &session.data,
            ilvara,
            &app.world()
                .resource::<Screens>()
                .forms
                .aim
                .clone()
                .unwrap()
                .option,
        );
        let Target::Door { x, y, .. } = rows[1].target else {
            panic!("two doors are in sight: {rows:?}");
        };
        (x, y)
    };
    let row = Pressable(ControlId::Target(1));
    common::app::set(&mut app, &row, Pointing::Hover);
    app.update();
    assert_eq!(marked(&app), Some(expected));
    app.update();
    let drawn = outline(&mut app).expect("the cell's floor is outlined");
    let (west, north) = (f32::from(expected.0), f32::from(expected.1));
    assert!(
        drawn
            .iter()
            .all(|[x, _, z]| (west..=west + 1.0).contains(x) && (north..=north + 1.0).contains(z)),
        "{drawn:?} lies in {expected:?}"
    );
    common::app::let_go(&mut app, &row);
    app.update();
    assert_eq!(marked(&app), None);
    app.update();
    assert_eq!(outline(&mut app), None, "and the outline goes with it");
    common::app::set(&mut app, &row, Pointing::Hover);
    app.update();
    assert!(marked(&app).is_some());
    common::app::press_ui(&mut app, Pressable(ControlId::Back));
    app.update();
    assert_eq!(marked(&app), None, "the step is over");
}
