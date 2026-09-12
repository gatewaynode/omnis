//! The headless driver answers host ops on disk and the schema dump parses with the real types.
//! One process: `save.write` paths are relative, so the test changes the working directory.

use omnis_cli::{Headless, schema};
use omnis_data::ron_io::parse;
use omnis_data::{
    Background, Class, Condition, Item, MapDef, Monster, PackManifest, Race, RulesFile, Spell,
    Tileset,
};
use omnis_sim::omnis_core::Direction;
use omnis_sim::{Command, Op, OpError, Replay, Reply, World};
use std::path::{Path, PathBuf};

fn test_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/test")
}

fn step(game: &mut Headless) {
    game.handle(&Op::SimCommand {
        command: Command::Step(Direction::Forward),
    })
    .unwrap();
}

#[test]
fn headless_writes_reads_and_reloads() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("headless");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_current_dir(&dir).unwrap();
    let mut game = Headless::new(vec![test_pack()], 1).unwrap();
    step(&mut game);
    let mut saved = game.world.clone();
    saved.log.clear();
    let path = "saves/one.ron";
    assert_eq!(
        game.handle(&Op::SaveWrite { path: path.into() }).unwrap(),
        Reply::Written { path: path.into() }
    );
    assert!(dir.join(path).is_file());
    step(&mut game);
    assert_ne!(game.world.position, saved.position);
    let Reply::Status(status) = game
        .handle(&Op::SaveRead {
            path: path.into(),
            force: false,
        })
        .unwrap()
    else {
        panic!("not a status")
    };
    assert_eq!(status.turn, 1);
    assert_eq!(game.world, saved, "restored, with an empty log");

    for bad in ["../one.ron", "/tmp/one.ron", "saves/one.txt", ""] {
        let error = game
            .handle(&Op::SaveWrite { path: bad.into() })
            .unwrap_err();
        assert!(
            matches!(&error, OpError::Failed { message } if message.starts_with("path '")),
            "{bad}: {error}"
        );
    }
    assert!(matches!(
        game.handle(&Op::SaveRead {
            path: "saves/none.ron".into(),
            force: false
        }),
        Err(OpError::Failed { .. })
    ));

    let before = game.world.clone();
    assert_eq!(game.handle(&Op::PackReload).unwrap(), Reply::Done {});
    assert_eq!(game.world, before, "a reload keeps the world");
    assert!(matches!(
        game.handle(&Op::Screenshot { path: None }),
        Err(OpError::Failed { message }) if message.contains("headless")
    ));
    assert!(
        Headless::new(vec![dir.join("no-such-pack")], 1).is_err(),
        "missing packs are an error, not a panic"
    );
}

#[test]
fn schema_dump_sections_parse_with_the_real_types() {
    let dump = schema::dump().unwrap();
    let body = |header: &str| -> String {
        let start = dump
            .find(header)
            .unwrap_or_else(|| panic!("{header} missing"))
            + header.len();
        let rest = &dump[start..];
        rest[..rest.find("\n\n").unwrap()].to_owned()
    };
    parse::<PackManifest>(&body("# pack.ron (schema 1)\n")).unwrap();
    let tileset = parse::<Tileset>(&body("# data/tiles/<name>.ron (schema 1)\n")).unwrap();
    assert!(tileset.surfaces.contains_key("door.open"));
    let map = parse::<MapDef>(&body("# data/maps/<name>.ron (schema 1)\n")).unwrap();
    assert_eq!(map.door_open.as_deref(), Some("door.open"));
    let mut errors = Vec::new();
    map.validate(Path::new("example"), &mut errors);
    assert!(
        errors.is_empty(),
        "the example map passes its own checks: {errors:?}"
    );
    parse::<Race>(&body("# data/races/<name>.ron\n")).unwrap();
    let class = parse::<Class>(&body("# data/classes/<name>.ron\n")).unwrap();
    assert_eq!(class.hit_die, 10);
    parse::<Background>(&body("# data/backgrounds/<name>.ron\n")).unwrap();
    parse::<Item>(&body("# data/items/<name>.ron\n")).unwrap();
    parse::<Condition>(&body("# data/conditions/<name>.ron\n")).unwrap();
    let spell = parse::<Spell>(&body("# data/spells/<name>.ron\n")).unwrap();
    assert_eq!(spell.point_cost(), 1);
    parse::<Monster>(&body("# data/monsters/<name>.ron\n")).unwrap();
    let rules = parse::<RulesFile>(&body("# data/rules/<name>.ron\n")).unwrap();
    assert!(rules.slots.contains_key("spell_points.pool"));
    parse::<World>(&body("# save (schema 1)\n")).unwrap();
    parse::<Replay>(&body("# replay\n")).unwrap();
    let ops =
        parse::<Vec<Op>>(&body("# protocol ops (JSON on the dev socket; RON here)\n")).unwrap();
    assert_eq!(ops.len(), 12);
}
