//! `omnis-cli schema dump`: the data files by example. There is no reflection over the schema
//! structs, so each section is a minimal instance of the real type, written by the same RON
//! writer the game reads with, so it always parses.

use omnis_data::ron_io::{parse, to_string};
use omnis_data::{DataError, MapDef, PackFingerprint, PackManifest, SCHEMA, Tileset};
use omnis_sim::omnis_core::{Clock, EraId, Facing, MapId, Position};
use omnis_sim::world::SAVE_SCHEMA;
use omnis_sim::{Command, Op, PARTY, Replay, World};
use std::collections::BTreeMap;

const MANIFEST: &str = r#"(
    schema: 1,
    id: "example",
    version: "0.1.0",
    name: "Example pack",
    license: "CC0-1.0",
    attribution: [("Tiles by someone", "CC0-1.0", "assets/LICENSE.txt")],
    depends: [],
    entry: Some("example:map:start"),
)"#;

const TILESET: &str = r#"(
    schema: 1,
    id: "example:tileset:stone",
    detail_depth: 1,
    width: 0,
    viewport: (240, 135),
    surfaces: {
        "floor": (kind: Floor, slots: [(depth: 0, offset: 0, path: "assets/tilesets/stone/floor_d0_o0.png", x: 0, y: 68)]),
        "wall": (kind: WallFront, slots: []),
        "wall.left": (kind: WallLeft, slots: []),
        "wall.right": (kind: WallRight, slots: []),
        "door": (kind: Door, slots: []),
        "door.open": (kind: DoorFrame, slots: []),
        "rock": (kind: Block, slots: []),
    },
)"#;

const MAP: &str = r#"(
    schema: 1,
    id: "example:map:start",
    name: "example:text:map.start.name",
    kind: Dungeon,
    tileset: "example:tileset:stone",
    width: 2,
    height: 1,
    start: (0, 0, East),
    wall: (front: "wall", left: "wall.left", right: "wall.right"),
    door: "door",
    door_open: Some("door.open"),
    terrains: [
        (glyph: '.', name: "floor", floor: "floor", ceiling: None, passable: true, opaque: false, color: (110, 90, 70), visibility_depth: 4, step_minutes: 1),
        (glyph: '#', name: "rock", floor: "floor", block: Some("rock"), passable: false, opaque: true, color: (60, 50, 45), visibility_depth: 4, step_minutes: 1),
    ],
    layout: [
        "+-+-+",
        "|.:#|",
        "+-+-+",
    ],
    portals: [(x: 1, y: 0, to_map: "example:map:start", to_x: 0, to_y: 0, to_facing: East)],
)"#;

/// Every data file type as a parsed-and-rewritten example, then a save, a replay, and the
/// protocol ops.
pub fn dump() -> Result<String, DataError> {
    let fingerprint = PackFingerprint {
        id: "example".into(),
        version: "0.1.0".into(),
        hash: 0x0123_4567_89ab_cdef,
    };
    let world = World {
        schema: SAVE_SCHEMA,
        seed: 1,
        packs: vec![fingerprint.clone()],
        rngs: BTreeMap::new(),
        clocks: BTreeMap::from([(PARTY, Clock::new(EraId(0)))]),
        position: Position {
            map: MapId(0),
            x: 0,
            y: 0,
            facing: Facing::East,
        },
        maps: BTreeMap::new(),
        automap: Default::default(),
        mode: omnis_sim::Mode::Explore,
        flags: BTreeMap::new(),
        settings: Default::default(),
        turn: 0,
        log: Vec::new(),
    };
    let replay = Replay {
        seed: 1,
        packs: vec![fingerprint],
        commands: vec![Command::Step(omnis_sim::omnis_core::Direction::Forward)],
        fingerprint: 0,
    };
    let ops = vec![
        Op::GameStatus,
        Op::WorldQuery {
            path: "position.x".into(),
        },
        Op::SimCommand {
            command: Command::Interact,
        },
        Op::SimScript {
            commands: vec![Command::Step(omnis_sim::omnis_core::Direction::Forward)],
        },
        Op::EventsTail { count: 32 },
        Op::ViewportGet,
        Op::MapText { map: None },
        Op::AutomapGet { map: None },
        Op::SaveWrite {
            path: ".omnis/quick.ron".into(),
        },
        Op::SaveRead {
            path: ".omnis/quick.ron".into(),
            force: false,
        },
        Op::PackReload,
        Op::Screenshot { path: None },
    ];
    let mut out = String::new();
    section(
        &mut out,
        &format!("pack.ron (schema {SCHEMA})"),
        &parse::<PackManifest>(MANIFEST)?,
    )?;
    section(
        &mut out,
        &format!("data/tiles/<name>.ron (schema {SCHEMA})"),
        &parse::<Tileset>(TILESET)?,
    )?;
    section(
        &mut out,
        &format!("data/maps/<name>.ron (schema {SCHEMA})"),
        &parse::<MapDef>(MAP)?,
    )?;
    section(&mut out, &format!("save (schema {SAVE_SCHEMA})"), &world)?;
    section(&mut out, "replay", &replay)?;
    section(
        &mut out,
        "protocol ops (JSON on the dev socket; RON here)",
        &ops,
    )?;
    Ok(out)
}

fn section<T: serde::Serialize>(out: &mut String, title: &str, value: &T) -> Result<(), DataError> {
    out.push_str("# ");
    out.push_str(title);
    out.push('\n');
    out.push_str(&to_string(value)?);
    out.push_str("\n\n");
    Ok(())
}
