//! A pile that is easy to get to (owner, 2026-11-13): the floor of a pile's cell is its click
//! target, not the heap alone; the floor's outline lights under the pointer; and a "Pick up..."
//! button stands on the view while anything on the ground is in reach, the party's own cell
//! included, where the heap is below the view. On the yard fixture: the arrows at (2, 2).

use crate::common;
use crate::loot::{cursor, hover, yard, yard_app};

use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};
use common::app::press_ui;
use common::app::{click, config, shell, top};
use common::objects_data;
use omnis_sim::omnis_core::Facing;
use omnis_vector::aim::{self, Eye};
use omnis_vector::ground::{Ground, eye};
use omnis_vector::screen::ControlId;
use omnis_vector::screen::ScreenId;
use omnis_vector::screen::loot::LootAct;
use omnis_vector::shapes::{Body, PILE_LOOK};
use omnis_vector::shell::aim::{Aimed, PickUp};
use omnis_vector::shell::controls::Control;
use omnis_vector::shell::lines::{ATTRIBUTE_COLOUR, ATTRIBUTE_STYLE};
use omnis_vector::shell::movement::Intent;
use omnis_vector::shell::render::{RenderPlugin, ViewAim};
use omnis_vector::shell::screen::{Pressable, Screens};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::{Config, Session};
use omnis_vector::strokes::{PILE, PLAIN};

const SIZE: (f32, f32) = (1600.0, 900.0);

/// The eye of a session's pose at a pitch, its floor, and the arrow pile's body.
fn seen(app: &App, pitch: f32) -> (Eye, Ground, Body) {
    let s = app.world().resource::<Session>();
    let pos = s.world.exploring_at(&s.data).expect("exploring");
    let ground = Ground::from_map(s.data.maps.get(&pos.map).unwrap(), pos.level);
    let (_, piles) = s.world.objects_on(&s.data, pos.map, pos.level);
    let pile = &piles[0];
    assert_eq!((pile.x, pile.y), (2, 2));
    let body = Body::of(pile.x, pile.y, pile.pos, Facing::North, &PILE_LOOK, &ground);
    let view = Eye {
        at: [s.pose.x, eye(&ground, s.pose), s.pose.z],
        yaw: s.pose.yaw,
        pitch,
        fov: 0.96,
    };
    (view, ground, body)
}

/// The far edge of a floor's quad on the screen, left end then right end.
fn far_edge(quad: &[(f32, f32); 4]) -> ((f32, f32), (f32, f32)) {
    let mut corners = *quad;
    corners.sort_by(|a, b| a.1.total_cmp(&b.1));
    let (a, b) = (corners[0], corners[1]);
    if a.0 < b.0 { (a, b) } else { (b, a) }
}

#[test]
fn the_floor_of_the_faced_pile_s_cell_is_under_the_pointer_and_the_cells_around_are_not() {
    let app = yard_app(1, 2, Facing::East);
    let (view, ground, body) = seen(&app, 0.0);
    let s = app.world().resource::<Session>();
    let piles = s.world.piles_in_reach(&s.data);
    assert_eq!(piles.len(), 1);
    let quad = aim::floor(&view, SIZE, &ground, 2, 2).expect("the faced cell is ahead");
    let (left, right) = far_edge(&quad);
    assert!(right.0 - left.0 > 200.0, "a cell wide: {left:?} {right:?}");
    assert!(
        (left.1 - right.1).abs() < 1.0 && left.1 > 450.0,
        "level, under the horizon"
    );
    let on = |p: (f32, f32)| aim::pile_under(p, SIZE, &view, &piles, &ground);

    // Just inside the far left corner: the floor, well away from the heap.
    let corner = (left.0 + 40.0, left.1 + 8.0);
    assert!(!aim::on_heap(corner, SIZE, &view, &body), "not the heap");
    assert!(aim::inside(corner, &quad));
    assert_eq!(on(corner), Some(0), "the floor is the pile's");
    assert_eq!(on((right.0 - 40.0, right.1 + 8.0)), Some(0));
    // Beyond the far edge is the next cell; outside the corners, the cells beside.
    assert_eq!(on((left.0 + 40.0, left.1 - 30.0)), None, "the cell beyond");
    assert_eq!(
        on((left.0 - 60.0, left.1 + 8.0)),
        None,
        "the cell to the left"
    );
    assert_eq!(on((right.0 + 60.0, right.1 + 8.0)), None, "to the right");
    assert_eq!(on((800.0, 100.0)), None, "the sky");

    // The party's own cell has corners behind the eye: no floor to point at.
    assert_eq!(aim::floor(&view, SIZE, &ground, 1, 2), None);
    assert_eq!(
        aim::floor(&view, SIZE, &ground, 200, 200),
        None,
        "off the map"
    );
}

#[test]
fn a_pile_underfoot_is_pointed_at_only_on_its_heap_looking_down() {
    let data = objects_data();
    let world = yard(&data, 2, 2, Facing::East);
    let pos = world.exploring_at(&data).unwrap();
    let ground = Ground::from_map(data.maps.get(&pos.map).unwrap(), pos.level);
    let piles = world.piles_in_reach(&data);
    assert_eq!((piles[0].x, piles[0].y), (2, 2), "underfoot");
    let pose = omnis_vector::pose::Pose::at(pos);
    let level = Eye {
        at: [pose.x, eye(&ground, pose), pose.z],
        yaw: pose.yaw,
        pitch: 0.0,
        fov: 0.96,
    };
    assert_eq!(aim::floor(&level, SIZE, &ground, 2, 2), None);
    for x in [100.0, 800.0, 1500.0] {
        for y in [500.0, 700.0, 899.0] {
            assert_eq!(
                aim::pile_under((x, y), SIZE, &level, &piles, &ground),
                None,
                "nothing of it shows at ({x}, {y})"
            );
        }
    }
    let down = Eye {
        pitch: -1.2,
        ..level
    };
    let body = Body::of(2, 2, piles[0].pos, Facing::North, &PILE_LOOK, &ground);
    let [l, t, r, b] = aim::rect(&down, SIZE, &body).expect("the heap, looking down");
    let middle = ((l + r) / 2.0, (t + b) / 2.0);
    assert!(middle.0 > 0.0 && middle.0 < 1600.0 && middle.1 > 0.0 && middle.1 < 900.0);
    assert_eq!(
        aim::pile_under(middle, SIZE, &down, &piles, &ground),
        Some(0)
    );
}

#[test]
fn a_point_is_inside_a_quad_whichever_way_round_and_not_outside_it() {
    let quad = [(10.0, 10.0), (30.0, 10.0), (40.0, 30.0), (0.0, 30.0)];
    let mut back = quad;
    back.reverse();
    for q in [quad, back] {
        assert!(aim::inside((20.0, 20.0), &q));
        assert!(aim::inside((3.0, 28.0), &q), "under the slanted side");
        assert!(!aim::inside((3.0, 12.0), &q), "beside the slanted side");
        assert!(!aim::inside((20.0, 5.0), &q));
        assert!(!aim::inside((20.0, 35.0), &q));
        assert!(!aim::inside((45.0, 20.0), &q));
    }
}

#[test]
fn a_click_on_the_floor_opens_the_window_and_one_beyond_the_cell_is_the_view_s() {
    let mut app = yard_app(1, 2, Facing::East);
    app.world_mut().spawn((
        Window {
            resolution: bevy::window::WindowResolution::new(1600, 900),
            ..default()
        },
        PrimaryWindow,
    ));
    app.update();
    let (view, ground, body) = seen(&app, 0.0);
    let quad = aim::floor(&view, SIZE, &ground, 2, 2).unwrap();
    let (left, _) = far_edge(&quad);
    let floor = (left.0 + 40.0, left.1 + 8.0);
    assert!(!aim::on_heap(floor, SIZE, &view, &body));
    let looking = |app: &App| app.world().resource::<Intent>().looking;

    hover(&mut app, floor);
    assert_eq!(app.world().resource::<Aimed>().0, Some((2, 2)));
    assert_eq!(
        cursor(&mut app),
        Some(CursorIcon::System(SystemCursorIcon::Pointer))
    );
    click(&mut app, floor);
    assert_eq!(top(&app), Some(ScreenId::Loot));
    assert!(!looking(&app), "the click was the pile's");

    common::app::tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), None);
    let beyond = (left.0 + 40.0, left.1 - 30.0);
    hover(&mut app, beyond);
    assert_eq!(app.world().resource::<Aimed>().0, None);
    click(&mut app, beyond);
    assert_eq!(top(&app), None);
    assert!(
        looking(&app),
        "a click beyond the cell takes the mouse look"
    );
}

/// The yard in a 1600 by 900 window with the view's renderer, the party at (1, 2) facing the
/// arrows, and a floor point of their cell away from the heap.
fn drawn_yard() -> (App, (f32, f32)) {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut packs = config().packs;
    packs.push(root.join("crates/omnis-sim/tests/packs/objects"));
    let mut app = shell(Config { packs, ..config() }, true);
    // The renderer before the first update: its entities are spawned at startup.
    app.add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .add_plugins(RenderPlugin);
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let session = &mut *session;
        let map = common::map(&session.data, "objects:map:yard");
        session.pose = common::place(&mut session.world, map, 1, 2, Facing::East);
        session.refresh();
    }
    app.world_mut().spawn((
        Window {
            resolution: bevy::window::WindowResolution::new(1600, 900),
            ..default()
        },
        PrimaryWindow,
    ));
    app.update();
    app.update();
    let (view, ground, _) = seen(&app, 0.0);
    let (left, _) = far_edge(&aim::floor(&view, SIZE, &ground, 2, 2).unwrap());
    (app, (left.0 + 40.0, left.1 + 8.0))
}

/// The first ends of the aim outline's lines, when it is drawn.
fn outline(app: &mut App) -> Option<Vec<[f32; 3]>> {
    let world = app.world_mut();
    let mut q = world.query_filtered::<Option<&Mesh3d>, With<ViewAim>>();
    let handle = q.single(world).expect("one aim entity")?.0.clone();
    let mesh = world.resource::<Assets<Mesh>>().get(&handle).expect("mesh");
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(a)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("positions");
    };
    let Some(bevy::mesh::VertexAttributeValues::Float32x4(styles)) =
        mesh.attribute(ATTRIBUTE_STYLE)
    else {
        panic!("styles");
    };
    assert!(styles.iter().all(|s| *s == PLAIN), "a plain solid line");
    let Some(bevy::mesh::VertexAttributeValues::Float32x4(colours)) =
        mesh.attribute(ATTRIBUTE_COLOUR)
    else {
        panic!("colours");
    };
    let (r, g, b) = PILE;
    assert!(
        colours.iter().all(|c| *c == [r, g, b, 1.0]),
        "the pile's colour"
    );
    Some(a.chunks(4).map(|quad| quad[0]).collect())
}

#[test]
fn the_floor_s_outline_is_drawn_under_the_pointer_and_gone_when_it_leaves_or_a_screen_opens() {
    let (mut app, floor) = drawn_yard();
    assert_eq!(outline(&mut app), None, "nothing aimed: no mesh");

    hover(&mut app, floor);
    assert_eq!(app.world().resource::<Aimed>().0, Some((2, 2)));
    let corners = outline(&mut app).expect("the outline");
    // Four lines round the pile's cell, an eighth inside it, on the yard's flat ground.
    let h = corners[0][1];
    assert_eq!(
        corners,
        vec![
            [2.125, h, 2.125],
            [2.875, h, 2.125],
            [2.875, h, 2.875],
            [2.125, h, 2.875]
        ]
    );
    let (_, ground, _) = seen(&app, 0.0);
    assert_eq!(h, ground.height(2.5, 2.5), "lying on the floor");

    hover(&mut app, (800.0, 100.0));
    assert_eq!(outline(&mut app), None, "the pointer left: taken away");

    hover(&mut app, floor);
    assert!(outline(&mut app).is_some());
    click(&mut app, floor);
    assert_eq!(top(&app), Some(ScreenId::Loot));
    app.update();
    assert_eq!(outline(&mut app), None, "a screen is up: nothing is aimed");
}

/// Whether the "Pick up..." button is shown, and whether it takes a press.
fn button(app: &mut App) -> (bool, bool) {
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&Visibility, Has<Control>), With<PickUp>>();
    let (visibility, live) = q.single(world).expect("one Pick up button");
    (*visibility != Visibility::Hidden, live)
}

/// Stand the party somewhere on the yard and let the view follow.
fn stand(app: &mut App, x: u16, y: u16, facing: Facing) {
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        let session = &mut *session;
        let map = common::map(&session.data, "objects:map:yard");
        session.pose = common::place(&mut session.world, map, x, y, facing);
        session.refresh();
    }
    app.update();
    app.update();
}

#[test]
fn pick_up_stands_on_the_view_while_a_pile_is_in_reach_and_opens_the_window() {
    // On the pile's own cell, where the heap is below the view.
    let mut app = yard_app(2, 2, Facing::West);
    app.update();
    assert_eq!(button(&mut app), (true, true), "underfoot");
    assert!(screen_text(app.world()).contains("Pick up..."));
    press_ui(&mut app, PickUp);
    assert_eq!(top(&app), Some(ScreenId::Loot));
    assert_eq!(
        app.world().resource::<Screens>().forms.loot,
        omnis_vector::screen::loot::Loot::default()
    );
    app.update();
    assert_eq!(button(&mut app), (false, false), "a screen is up");
    common::app::tap(&mut app, KeyCode::Escape);
    app.update();
    assert_eq!(top(&app), None);
    assert_eq!(button(&mut app), (true, true), "back on the view");

    // Facing it from the next cell; then turned away, and a hidden button takes no press.
    stand(&mut app, 1, 2, Facing::East);
    assert_eq!(button(&mut app), (true, true), "ahead");
    stand(&mut app, 1, 2, Facing::West);
    assert_eq!(button(&mut app), (false, false), "nothing in reach");
    // Even a press forced onto it opens nothing: it is no control.
    let hidden = {
        let world = app.world_mut();
        let mut q = world.query_filtered::<Entity, With<PickUp>>();
        q.single(world).unwrap()
    };
    app.world_mut().entity_mut(hidden).insert(bevy::ui::Pressed);
    app.update();
    assert_eq!(top(&app), None);
    app.world_mut()
        .entity_mut(hidden)
        .remove::<bevy::ui::Pressed>();

    // Not while the party is doing anything but exploring (here the view's mode alone is
    // moved: no fixture has a pile where a fight starts).
    stand(&mut app, 1, 2, Facing::East);
    app.world_mut().resource_mut::<Session>().views.here.mode = omnis_sim::api::ModeKind::Combat;
    app.update();
    assert_eq!(button(&mut app), (false, false), "not exploring");
    stand(&mut app, 1, 2, Facing::East);
    assert_eq!(button(&mut app), (true, true));

    // Everything taken: nothing to offer.
    press_ui(&mut app, PickUp);
    assert_eq!(top(&app), Some(ScreenId::Loot));
    press_ui(&mut app, Pressable(ControlId::Loot(LootAct::Everything)));
    app.update();
    app.update();
    assert_eq!(top(&app), None, "the window closed itself");
    assert_eq!(button(&mut app), (false, false), "the pile is gone");
}
