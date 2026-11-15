//! The minimap's pixels on the real test maps (presentation-ARCHITECTURE.md §8): only what the automap
//! knows is painted, the party is marked where the pose is, and walls appear once seen.

use crate::common;

use common::{data, drive, map, session};
use omnis_vector::minimap::{BACKGROUND, DOOR, MARKER, Raster, SIDE, WALL, Zoom, corner};
use omnis_vector::pose::Pose;

/// Eight pixels a cell: twelve cells each way from the party.
const S: u32 = 8;

/// The minimap at eight pixels a cell.
fn paint(
    map: &omnis_sim::omnis_data::MapData,
    level: omnis_sim::omnis_core::Level,
    known: Option<&std::collections::BTreeMap<(u16, u16), omnis_sim::Known>>,
    pose: Pose,
) -> Raster {
    omnis_vector::minimap::paint(map, level, known, pose, S)
}

/// The picture's pixel at the map's pixel `(x, y)`, with the party at `pose`; `None` outside
/// the picture.
fn spot(raster: &Raster, pose: Pose, x: u32, y: u32) -> Option<[u8; 4]> {
    let (cx, cy) = corner(raster.scale, pose);
    let px = u32::try_from(i64::from(x) - cx).ok()?;
    let py = u32::try_from(i64::from(y) - cy).ok()?;
    raster.pixel(px, py)
}

fn painted(raster: &Raster) -> usize {
    raster
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| **p != BACKGROUND)
        .count()
}

#[test]
fn the_zoom_steps_through_whole_pixel_scales_and_stops_at_its_ends() {
    assert_eq!(Zoom::default().scale(), 12);
    let mut zoom = Zoom::default();
    let mut closer = vec![zoom.scale()];
    while let Some(next) = zoom.step(true) {
        zoom = next;
        closer.push(zoom.scale());
    }
    assert_eq!(closer, [12, 16, 24], "closer, then no further");
    let mut farther = Vec::new();
    while let Some(next) = zoom.step(false) {
        zoom = next;
        farther.push(zoom.scale());
    }
    assert_eq!(farther, [16, 12, 8, 6, 4]);
    for scale in omnis_vector::minimap::ZOOMS {
        assert_eq!(SIDE % scale, 0, "whole cells across at {scale}");
    }
}

#[test]
fn a_fresh_party_sees_only_its_surroundings_and_is_marked() {
    let data = data();
    let (world, _) = session(&data);
    let p = world.position;
    let map = &data.maps[&p.map];
    let pose = Pose::at(p);
    let raster = paint(
        map,
        omnis_sim::omnis_core::Level::GROUND,
        world
            .automap
            .tiles(p.map, omnis_sim::omnis_core::Level::GROUND),
        pose,
    );
    let s = raster.scale;
    assert_eq!((raster.width, raster.height), (SIDE, SIDE));
    // The marker sits on the pose, at the picture's middle; the party's cell is known and
    // painted around it.
    let (cx, cy) = (u32::from(p.x) * s, u32::from(p.y) * s);
    assert_eq!(raster.pixel(SIDE / 2, SIDE / 2), Some(MARKER));
    assert_eq!(spot(&raster, pose, cx + s / 2, cy + s / 2), Some(MARKER));
    assert_ne!(spot(&raster, pose, cx, cy + s - 1), Some(BACKGROUND));
    // A corner far from the party is unknown.
    assert_eq!(raster.pixel(0, raster.height - 1), Some(BACKGROUND));
    assert!(
        painted(&raster) < raster.rgba.len() / 4,
        "not everything is known"
    );
}

#[test]
fn walking_reveals_more_and_the_dungeon_walls_show() {
    let data = data();
    let (mut world, mut binder) = session(&data);
    let start = world.position;
    let before = paint(
        &data.maps[&start.map],
        omnis_sim::omnis_core::Level::GROUND,
        world
            .automap
            .tiles(start.map, omnis_sim::omnis_core::Level::GROUND),
        Pose::at(start),
    );
    // North up the road and through the portal into the dungeon.
    let (pose, _, _) = drive(
        &mut binder,
        &mut world,
        &data,
        Pose::at(start),
        (0.0, -0.05),
        0.0,
        240,
    );
    let meadow = paint(
        &data.maps[&start.map],
        omnis_sim::omnis_core::Level::GROUND,
        world
            .automap
            .tiles(start.map, omnis_sim::omnis_core::Level::GROUND),
        Pose::at(start),
    );
    assert!(painted(&meadow) > painted(&before), "the road was seen");
    let here = world.position;
    assert_ne!(here.map, start.map, "in the dungeon");
    let dungeon = paint(
        &data.maps[&here.map],
        omnis_sim::omnis_core::Level::GROUND,
        world
            .automap
            .tiles(here.map, omnis_sim::omnis_core::Level::GROUND),
        pose,
    );
    assert!(
        dungeon.rgba.as_chunks::<4>().0.contains(&WALL),
        "the dungeon's walls are drawn once seen"
    );
}

#[test]
fn the_marker_points_along_the_yaw() {
    let data = data();
    let (world, _) = session(&data);
    let p = world.position;
    let map = &data.maps[&p.map];
    for facing in omnis_sim::omnis_core::Facing::ALL {
        let pose = Pose::at(omnis_sim::omnis_core::Position { facing, ..p });
        let raster = paint(map, omnis_sim::omnis_core::Level::GROUND, None, pose);
        let s = raster.scale as f32;
        let (cx, cy) = corner(raster.scale, pose);
        let (ox, oz) = (cx as f32 / s, cy as f32 / s);
        let (fx, fz) = omnis_vector::geom::forward(pose.yaw);
        // How far ahead of the pose, in cells, each marker pixel lies.
        let mut along = Vec::new();
        for y in 0..raster.height {
            for x in 0..raster.width {
                if raster.pixel(x, y) == Some(MARKER) {
                    let (dx, dz) = (
                        (x as f32 + 0.5) / s + ox - pose.x,
                        (y as f32 + 0.5) / s + oz - pose.z,
                    );
                    along.push(dx * fx + dz * fz);
                }
            }
        }
        let ahead = along.iter().copied().fold(f32::MIN, f32::max);
        let behind = along.iter().copied().fold(f32::MAX, f32::min);
        // The tip reaches well ahead of the pose; the base stays close behind it.
        assert!(ahead > 0.5, "the tip is ahead, {facing:?}: {ahead}");
        assert!(
            ahead > -behind + 0.2,
            "tip beyond base, {facing:?}: {ahead} {behind}"
        );
    }
}

#[test]
fn an_edge_recorded_on_one_side_is_drawn_once_known() {
    use omnis_sim::Known;
    use omnis_sim::omnis_core::{Edges, Facing};
    let data = data();
    let (world, _) = session(&data);
    let map = &data.maps[&world.position.map];
    let tile = |walls: Edges| Known {
        terrain: 0,
        walls,
        doors: Edges::NONE,
        layers: 0,
        seen_at: 0,
    };
    // Cell (5, 5) knows a wall on its east side; (6, 5) knows none on its west.
    let mut known = std::collections::BTreeMap::new();
    known.insert((5, 5), tile(Edges::NONE.with(Facing::East)));
    known.insert((6, 5), tile(Edges::NONE));
    // The party stands where the cells in question are in the picture and clear of its mark.
    let far = Pose {
        x: 10.5,
        z: 10.5,
        yaw: 0.0,
    };
    let raster = paint(map, omnis_sim::omnis_core::Level::GROUND, Some(&known), far);
    let s = raster.scale;
    for dy in 0..s {
        assert_eq!(spot(&raster, far, 6 * s, 5 * s + dy), Some(WALL));
    }
    // The west side of (5, 5) has no wall.
    assert_ne!(spot(&raster, far, 5 * s, 5 * s + 1), Some(WALL));
}

/// Every tile of `map` known and visited, as the automap would hold it.
fn all_known(
    map: &omnis_sim::omnis_data::MapData,
) -> std::collections::BTreeMap<(u16, u16), omnis_sim::Known> {
    let mut known = std::collections::BTreeMap::new();
    for y in 0..map.def.height {
        for x in 0..map.def.width {
            let cell = map
                .cell(omnis_sim::omnis_core::Level::GROUND, x, y)
                .expect("a cell");
            known.insert(
                (x, y),
                omnis_sim::Known {
                    terrain: cell.terrain,
                    walls: cell.walls,
                    doors: cell.doors,
                    layers: omnis_sim::world::layer::VISITED,
                    seen_at: 0,
                },
            );
        }
    }
    known
}

/// M8.5 step 4: a cliff is drawn along its edge and a ramp ticked across its middle.
#[test]
fn cliffs_and_ramps_show_on_the_elevation_fixture() {
    use omnis_vector::minimap::{CLIFF, RAMP};
    let data = common::heights_data();
    let map = &data.maps[&common::map(&data, "heights:map:hills")];
    let known = all_known(map);
    let pose = Pose {
        x: 8.5,
        z: 4.5,
        yaw: 0.0,
    };
    let raster = paint(
        map,
        omnis_sim::omnis_core::Level::GROUND,
        Some(&known),
        pose,
    );
    let s = raster.scale;
    // The cliff at x 3 on row 0, from the slope up to 12: its column of pixels (the first is
    // the map's north wall, which wins).
    for i in 1..s {
        assert_eq!(spot(&raster, pose, 3 * s, i), Some(CLIFF), "row {i}");
    }
    // The ramp at x 1 on row 2: a tick across the middle of its edge.
    assert_eq!(spot(&raster, pose, s, 2 * s + s / 2), Some(RAMP));
    assert_eq!(spot(&raster, pose, s - s / 4, 2 * s + s / 2), Some(RAMP));
    // The slope between x 0 and 1 on row 0 is neither.
    let mid = spot(&raster, pose, s, s / 2);
    assert!(mid != Some(CLIFF) && mid != Some(RAMP), "{mid:?}");
}

/// The cell's pixels of `colour` on `raster`, painted with the party at `pose`.
fn in_cell(raster: &Raster, pose: Pose, (x, y): (u32, u32), colour: [u8; 4]) -> usize {
    let s = raster.scale;
    (y * s..(y + 1) * s)
        .flat_map(|py| (x * s..(x + 1) * s).map(move |px| (px, py)))
        .filter(|&(px, py)| spot(raster, pose, px, py) == Some(colour))
        .count()
}

#[test]
fn the_minimap_marks_the_joins_the_party_has_seen_on_its_level() {
    use omnis_sim::omnis_core::{Facing, Level, Position, Rotation};
    use omnis_vector::minimap::{HOLE, JOIN};
    let data = common::levels_data();
    let id = common::map(&data, "levels:map:tower");
    let mut world =
        omnis_vector::sim::SimWorld::start(&data, common::SEED, omnis_sim::Settings::default())
            .expect("an entry map");
    world.position = Position {
        map: id,
        level: Level(1),
        x: 4,
        y: 1,
        facing: Facing::East,
    };
    world
        .apply(&data, omnis_sim::Command::Turn(Rotation::Right))
        .expect("a turn");
    let map = &data.maps[&id];
    let pose = Pose::at(world.position);
    let upper = paint(map, Level(1), world.automap(id, Level(1)), pose);
    assert!(
        in_cell(&upper, pose, (5, 2), HOLE) > 0,
        "the hole beside the bridge"
    );
    assert!(
        in_cell(&upper, pose, (4, 3), JOIN) > 0,
        "the stair at the bridge's end"
    );
    assert_eq!(
        in_cell(&upper, pose, (4, 2), JOIN),
        0,
        "the bridge itself is no join"
    );
    let ground = paint(map, Level::GROUND, world.automap(id, Level::GROUND), pose);
    assert_eq!(
        in_cell(&ground, pose, (5, 2), HOLE),
        0,
        "the road below is not the upper floor"
    );
}

#[test]
fn a_map_of_several_levels_names_the_party_s_level_and_a_map_of_one_does_not() {
    use omnis_sim::omnis_core::Level;
    let data = common::levels_data();
    let tower = &data.maps[&common::map(&data, "levels:map:tower")];
    assert_eq!(
        omnis_vector::labels::level(&data, tower, Level(2)).as_deref(),
        Some("Roof (2)")
    );
    assert_eq!(
        omnis_vector::labels::level(&data, tower, Level::GROUND).as_deref(),
        Some("Test Tower (0)")
    );
    let shipped = common::data();
    for map in shipped.maps.values() {
        assert_eq!(
            omnis_vector::labels::level(&shipped, map, Level::GROUND),
            None
        );
    }
}

#[test]
fn a_stair_s_mark_points_the_way_it_rises() {
    use omnis_sim::omnis_core::Level;
    use omnis_vector::minimap::JOIN;
    let data = common::levels_data();
    let map = &data.maps[&common::map(&data, "levels:map:tower")];
    let pose = Pose {
        x: 0.5,
        z: 4.5,
        yaw: 0.0,
    };
    let raster = paint(map, Level::GROUND, Some(&all_known(map)), pose);
    let s = raster.scale;
    // A point of cell (x, y) in eighths of the cell.
    let at = |(x, y): (u32, u32), (fx, fy): (u32, u32)| {
        spot(&raster, pose, x * s + s * fx / 8, y * s + s * fy / 8)
    };
    // The tower's stair rises west: a chevron pointing west, its arms ending at the east.
    assert_eq!(at((2, 0), (5, 6)), Some(JOIN));
    assert_ne!(
        at((2, 0), (6, 5)),
        Some(JOIN),
        "not a chevron pointing north"
    );
    // The courtyard's rises north: a chevron pointing north.
    assert_eq!(at((4, 3), (6, 5)), Some(JOIN));
    assert_ne!(at((4, 3), (5, 6)), Some(JOIN));
}

/// A portal on a tile the party has seen is marked with a rectangle and a circle in the door
/// colour (owner, 2026-10-24); an unseen portal and a tile without one get no mark.
#[test]
fn a_seen_portal_is_marked_with_a_rectangle_and_a_circle() {
    let data = data();
    let town = &data.maps[&map(&data, "test:map:town")];
    let known = all_known(town);
    let pose = Pose::at(omnis_sim::omnis_core::Position {
        map: map(&data, "test:map:town"),
        level: omnis_sim::omnis_core::Level::GROUND,
        x: 3,
        y: 3,
        facing: omnis_sim::omnis_core::Facing::North,
    });
    let raster = paint(
        town,
        omnis_sim::omnis_core::Level::GROUND,
        Some(&known),
        pose,
    );
    let marked = in_cell(&raster, pose, (11, 2), DOOR);
    assert!(marked > 0, "the portal's cell carries the mark");
    // The cell's middle is inside the circle, not on it; a corner is outside the rectangle.
    let s = raster.scale;
    assert_ne!(
        spot(&raster, pose, 11 * s + s / 2, 2 * s + s / 2),
        Some(DOOR)
    );
    assert_ne!(spot(&raster, pose, 11 * s, 2 * s), Some(DOOR));
    // A street cell with no portal and no door has none of the colour.
    assert_eq!(in_cell(&raster, pose, (6, 3), DOOR), 0);
    // Unseen, the portal's tile shows nothing.
    let mut unseen = known;
    unseen.remove(&(11, 2));
    let raster = paint(
        town,
        omnis_sim::omnis_core::Level::GROUND,
        Some(&unseen),
        pose,
    );
    assert_eq!(in_cell(&raster, pose, (11, 2), DOOR), 0);
}

/// The owner's rule (2026-11-15, after the grown yard "got squished" and a window that stopped
/// at the map's edges was "still unusable"): "a consistent square centered on the party", its
/// scale the zoom's and never the map's size.
#[test]
fn the_picture_is_one_square_with_the_party_at_its_middle_on_any_map_at_any_zoom() {
    use omnis_sim::Known;
    use omnis_sim::omnis_core::{Edges, Facing, Level};
    let data = common::objects_data();
    let map = &data.maps[&common::map(&data, "objects:map:yard")];
    assert_eq!((map.def.width, map.def.height), (37, 5), "wide and low");
    let at = |x: f32| Pose {
        x: x + 0.5,
        z: 2.5,
        yaw: 0.0,
    };
    // A wall on the east of (33, 2), far down the hall.
    let mut known = std::collections::BTreeMap::new();
    known.insert(
        (33, 2),
        Known {
            terrain: 0,
            walls: Edges::NONE.with(Facing::East),
            doors: Edges::NONE,
            layers: 0,
            seen_at: 0,
        },
    );
    let half = SIDE / 2;
    for scale in omnis_vector::minimap::ZOOMS {
        // At the west edge, in the middle and at the east edge: the same square, the mark at
        // its middle, though the map ends beside the party.
        for x in [0.0, 20.0, 36.0] {
            let raster = omnis_vector::minimap::paint(map, Level::GROUND, None, at(x), scale);
            assert_eq!(
                (raster.width, raster.height, raster.scale),
                (SIDE, SIDE, scale)
            );
            assert_eq!(raster.pixel(half, half), Some(MARKER), "at {x}, {scale}");
            let marks: Vec<(u32, u32)> = (0..SIDE)
                .flat_map(|y| (0..SIDE).map(move |x| (x, y)))
                .filter(|&(x, y)| raster.pixel(x, y) == Some(MARKER))
                .collect();
            assert!(
                marks
                    .iter()
                    .all(|&(x, y)| x.abs_diff(half) <= 40 && y.abs_diff(half) <= 40),
                "the mark is nowhere else"
            );
        }
        // The wall is drawn where it lies from the party, three cells and a half east of the
        // party's place at (30, 2): the zoom's pixels a cell, not the map's size, set the
        // distance.
        let pose = at(30.0);
        let raster = omnis_vector::minimap::paint(map, Level::GROUND, Some(&known), pose, scale);
        let east = half + scale * 7 / 2;
        for dy in 0..scale {
            assert_eq!(
                raster.pixel(east, half - scale / 2 + dy),
                Some(WALL),
                "at {scale}"
            );
        }
        assert_eq!(
            corner(scale, pose).0,
            i64::from(scale) * 61 / 2 - i64::from(half)
        );
    }
    // A known cell outside the square is not in it: nothing but the mark and the ground.
    let raster = omnis_vector::minimap::paint(map, Level::GROUND, Some(&known), at(2.0), 8);
    assert!(!raster.rgba.as_chunks::<4>().0.contains(&WALL));
}

/// The zoom is the buttons' on the minimap (owner, 2026-11-15: "A zoom control (+/-) button
/// controls the scale of the view"): each press is one step, the picture stays the same
/// square, and a button with no step left is dim, its tip saying why.
#[test]
fn the_buttons_on_the_minimap_step_the_zoom_and_the_last_step_dims_its_button() {
    use bevy::asset::AssetPlugin;
    use bevy::prelude::*;
    use common::app::{Pointing, press_ui, set};
    use omnis_vector::shell::controls::Control;
    use omnis_vector::shell::minimap::{Minimap, MinimapPlugin, ZoomButton, Zooming};
    use omnis_vector::shell::tips::Tip;
    let (closer, farther) = (ZoomButton { closer: true }, ZoomButton { closer: false });
    let mut app = common::app::app();
    app.add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .add_plugins(MinimapPlugin);
    app.update();
    let scale = |app: &App| app.world().resource::<Zooming>().0.scale();
    let picture = |app: &mut App| {
        let world = app.world_mut();
        let handle = world
            .query_filtered::<&ImageNode, With<Minimap>>()
            .single(world)
            .expect("the minimap")
            .image
            .clone();
        let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
        (image.width(), image.height())
    };
    let pixels = |app: &mut App| {
        let world = app.world_mut();
        let handle = world
            .query_filtered::<&ImageNode, With<Minimap>>()
            .single(world)
            .expect("the minimap")
            .image
            .clone();
        let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
        image.data.clone().expect("pixels")
    };
    let tip = |app: &mut App, button: ZoomButton| {
        let world = app.world_mut();
        let mut tips = world.query::<(&ZoomButton, &Tip, Has<Control>)>();
        let (_, tip, live) = tips.iter(world).find(|(b, ..)| **b == button).unwrap();
        (tip.text.clone(), live)
    };
    assert_eq!((scale(&app), picture(&mut app)), (12, (SIDE, SIDE)));
    assert_eq!(tip(&mut app, closer), ("Zoom the map in".into(), true));
    assert_eq!(tip(&mut app, farther), ("Zoom the map out".into(), true));
    let before = pixels(&mut app);
    press_ui(&mut app, closer);
    assert_eq!((scale(&app), picture(&mut app)), (16, (SIDE, SIDE)));
    assert_ne!(
        pixels(&mut app),
        before,
        "the picture is drawn at the new scale"
    );
    press_ui(&mut app, farther);
    press_ui(&mut app, farther);
    assert_eq!(scale(&app), 8, "one step a press, either way");
    press_ui(&mut app, farther);
    press_ui(&mut app, farther);
    assert_eq!((scale(&app), picture(&mut app)), (4, (SIDE, SIDE)));
    assert_eq!(
        tip(&mut app, farther),
        ("The map is as far out as it goes".into(), false)
    );
    // And back: the first step in lights it again.
    press_ui(&mut app, closer);
    assert_eq!(tip(&mut app, farther), ("Zoom the map out".into(), true));
    for _ in 0..4 {
        press_ui(&mut app, closer);
    }
    assert_eq!(scale(&app), 24);
    assert_eq!(
        tip(&mut app, closer),
        ("The map is as close as it goes".into(), false)
    );
    // Held down, a button is one press.
    set(&mut app, &farther, Pointing::Press);
    app.update();
    app.update();
    assert_eq!(scale(&app), 16);
}
