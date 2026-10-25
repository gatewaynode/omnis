//! The headless driver answers host ops on disk and the schema dump parses with the real types.
//! One process: `save.write` paths are relative, so the test changes the working directory.

use omnis_cli::{Headless, schema};
use omnis_data::ron_io::parse;
use omnis_data::{Alignment, Skill};
use omnis_data::{
    Background, Class, Condition, Item, MapDef, Monster, PackManifest, Race, RegionDef, RulesFile,
    Spell, Tileset,
};
use omnis_sim::omnis_core::Direction;
use omnis_sim::omnis_rules::Draft;
use omnis_sim::save::{CharacterSave, Manifest};
use omnis_sim::{Command, Op, OpError, Replay, Reply};
use std::path::{Path, PathBuf};

fn test_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/test")
}

fn base_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/base")
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
    let mut game = Headless::new(vec![base_pack(), test_pack()], 1).unwrap();
    step(&mut game);
    let mut saved = game.world.clone();
    saved.log.retain(|l| omnis_sim::told(&l.event));
    let path = "saves/one.ron";
    assert_eq!(
        game.handle(&Op::SaveWrite { path: path.into() }).unwrap(),
        Reply::Written {
            path: path.into(),
            version: None,
        }
    );
    assert!(dir.join(path).is_file());
    step(&mut game);
    assert_ne!(game.world.position, saved.position);
    let Reply::Status(status) = game
        .handle(&Op::SaveRead {
            path: path.into(),
            force: false,
            migrate: false,
        })
        .unwrap()
    else {
        panic!("not a status")
    };
    assert_eq!(status.turn, 1);
    assert_eq!(game.world, saved, "restored, with the party's log as saved");

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
            force: false,
            migrate: false,
        }),
        Err(OpError::Failed { .. })
    ));

    let before = game.world.clone();
    let Reply::Done { check } = game.handle(&Op::PackReload { migrate: false }).unwrap() else {
        panic!("a reload answers done");
    };
    assert_eq!(
        check.verdict,
        omnis_sim::save::Verdict::Same,
        "the same packs"
    );
    assert!(
        !check.packs.is_empty() && check.packs.iter().all(|p| p.saved == p.loaded),
        "the report of the packs: {check:?}"
    );
    assert_eq!(game.world, before, "a reload keeps the world");
    assert!(matches!(
        game.handle(&Op::Screenshot {
            path: None,
            target: omnis_sim::ops::ShotTarget::Canvas,
        }),
        Err(OpError::Failed { message }) if message.contains("headless")
    ));
    assert!(
        Headless::new(vec![dir.join("no-such-pack")], 1).is_err(),
        "missing packs are an error, not a panic"
    );
    // M8.5 7h6: a save the packs' migrations change loads only with `migrate`.
    let fixture = |name: &str| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../omnis-sim/tests/packs")
            .join(name)
    };
    let chain = |v: &str| {
        vec![
            base_pack(),
            test_pack(),
            fixture("objects"),
            fixture(&format!("chain/{v}")),
        ]
    };
    let mut old = Headless::new(chain("v1"), 1).unwrap();
    let sword = old.data.registry.items.get("chain:item:old_sword").unwrap();
    old.world.party.inventory.push((sword, 1));
    old.handle(&Op::SaveWrite {
        path: "saves/chain.ron".into(),
    })
    .unwrap();
    let mut new = Headless::new(chain("v3"), 1).unwrap();
    let read = |migrate| Op::SaveRead {
        path: "saves/chain.ron".into(),
        force: false,
        migrate,
    };
    assert!(
        matches!(new.handle(&read(false)), Err(OpError::Failed { message }) if message.contains("migrate")),
        "asks first"
    );
    assert!(matches!(new.handle(&read(true)), Ok(Reply::Status(_))));
    let names: Vec<&str> = new
        .world
        .party
        .inventory
        .iter()
        .filter_map(|(i, _)| new.data.registry.items.name(*i))
        .collect();
    assert_eq!(names, ["chain:item:sword"]);
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
    let region = parse::<RegionDef>(&body("# data/regions/<name>.ron\n")).unwrap();
    assert_eq!(
        (region.company, region.rule.as_str()),
        (900, "time.settled")
    );
    let save = parse::<CharacterSave>(&body("# save (schema 9)\n")).unwrap();
    assert_eq!(
        save.journal.entries()[0].changes.len(),
        1,
        "one door opened"
    );
    parse::<Manifest>(&body("# .omnis/worlds/<id>/manifest.ron\n")).unwrap();
    parse::<Replay>(&body("# replay\n")).unwrap();
    let ops =
        parse::<Vec<Op>>(&body("# protocol ops (JSON on the dev socket; RON here)\n")).unwrap();
    assert_eq!(ops.len(), 44);
}

/// The M3 "done when": the spell point formula changes through `rules.set` without a rebuild.
#[test]
fn rules_set_changes_the_pool_without_a_rebuild() {
    let mut game = Headless::new(vec![base_pack(), test_pack()], 1).unwrap();
    let wizard = Draft {
        name: "Ilvara".into(),
        race: "base:race:elf".into(),
        class: "base:class:wizard".into(),
        background: "base:background:acolyte".into(),
        alignment: Alignment::ChaoticGood,
        scores: [8, 14, 13, 15, 12, 10],
        skills: vec![Skill::Arcana, Skill::History],
    };
    let pool = |game: &mut Headless, index: usize| match game.handle(&Op::PartyGet) {
        Ok(Reply::Party { party }) => party.members[index].spell_points_max,
        other => panic!("{other:?}"),
    };
    game.handle(&Op::PartyCreate {
        character: wizard.clone(),
    })
    .unwrap();
    assert_eq!(pool(&mut game, 0), 4);
    let reply = game
        .handle(&Op::RulesSet {
            slot: "spell_points.pool".into(),
            source: "level * 10".into(),
        })
        .unwrap();
    assert!(matches!(reply, Reply::Rule { rule } if rule.source == "level * 10"));
    game.handle(&Op::PartyCreate { character: wizard }).unwrap();
    assert_eq!(
        pool(&mut game, 1),
        10,
        "the new formula applies to the next creation"
    );
    assert_eq!(pool(&mut game, 0), 4, "an existing sheet keeps its pool");
    let error = game
        .handle(&Op::RulesSet {
            slot: "spell_points.pool".into(),
            source: "level * wisdom".into(),
        })
        .unwrap_err();
    assert!(
        matches!(&error, OpError::BadRequest { message } if message.contains("1:9: unknown input 'wisdom'")),
        "{error}"
    );
    let error = game
        .handle(&Op::RulesSet {
            slot: "nope".into(),
            source: "1".into(),
        })
        .unwrap_err();
    assert_eq!(
        error,
        OpError::UnknownSlot {
            slot: "nope".into()
        },
        "as rules.get answers it (B5)"
    );
    assert!(matches!(
        game.handle(&Op::RulesGet { slot: "spell_points.pool".into() }).unwrap(),
        Reply::Rule { rule } if rule.source == "level * 10" && rule.inputs.len() == 4
    ));
    assert_eq!(
        game.handle(&Op::RulesGet {
            slot: "nope".into()
        })
        .unwrap_err(),
        OpError::UnknownSlot {
            slot: "nope".into()
        }
    );
}

/// A copy of the test pack the editor may write to: `pack.ron` and the directories the loader
/// reads, under the target directory.
fn scratch_pack(name: &str) -> PathBuf {
    fn copy_dir(src: &Path, dst: &Path) {
        let Ok(entries) = std::fs::read_dir(src) else {
            return;
        };
        std::fs::create_dir_all(dst).unwrap();
        for entry in entries.map(Result::unwrap) {
            let to = dst.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &to);
            } else {
                std::fs::copy(entry.path(), to).unwrap();
            }
        }
    }
    let dst = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dst);
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::copy(test_pack().join("pack.ron"), dst.join("pack.ron")).unwrap();
    for dir in ["data", "text", "notes", "migrations"] {
        copy_dir(&test_pack().join(dir), &dst.join(dir));
    }
    dst
}

/// The version the scratch pack names, and the patch one higher its next save seals at: read
/// from the pack, not pinned, since the owner's saves through the editor move it (M5.10).
fn versions(root: &Path) -> (String, String) {
    let version = omnis_data::seal::status(root).unwrap().version;
    let (rest, patch) = version.rsplit_once('.').unwrap();
    let next = format!("{rest}.{}", patch.parse::<u32>().unwrap() + 1);
    (version, next)
}

/// The editor's ops through `Headless` (M5.3): open, paint, status, save, reload; a bad text
/// refused in the loader's words and nothing written; notes written at once; `dispatch` refuses
/// every editor op as host-only.
#[test]
fn headless_edits_a_pack_and_reloads_it() {
    let scratch = scratch_pack("headless-editor");
    let (shipped, next) = versions(&scratch);
    let mut game = Headless::new(vec![base_pack(), scratch.clone()], 1).unwrap();
    let status = |game: &mut Headless| match game.handle(&Op::EditorStatus).unwrap() {
        Reply::Editor { editor } => editor,
        other => panic!("{other:?}"),
    };
    assert_eq!(status(&mut game).map, None);
    match game
        .handle(&Op::EditorOpen {
            map: "test:map:town".into(),
        })
        .unwrap()
    {
        Reply::Editor { editor } => {
            assert_eq!(editor.map.as_deref(), Some("test:map:town"));
            assert_eq!((editor.width, editor.height), (12, 5));
            assert!(!editor.dirty);
            assert_eq!(editor.errors, Vec::<String>::new());
        }
        other => panic!("{other:?}"),
    }
    game.handle(&Op::EditorPaint {
        level: omnis_sim::omnis_core::Level::GROUND,
        x: 1,
        y: 1,
        brush: omnis_data::edit::Brush::Edge {
            side: omnis_sim::omnis_core::Facing::East,
            kind: omnis_data::edit::EdgeKind::Door,
        },
        stroke: false,
    })
    .unwrap();
    let after = status(&mut game);
    assert!(after.dirty);
    assert_eq!(after.undo, 1);
    match game.handle(&Op::EditorSave).unwrap() {
        Reply::Written { path, version } => {
            assert_eq!(path, "data/maps/town.ron");
            assert_eq!(version.as_deref(), Some(next.as_str()));
        }
        other => panic!("{other:?}"),
    }
    assert!(!status(&mut game).dirty);
    // The world runs on the packs as loaded until they are reloaded.
    assert_eq!(game.data.fingerprints[1].version, shipped);
    game.handle(&Op::PackReload { migrate: false }).unwrap();
    assert_eq!(game.data.fingerprints[1].version, next);
    let town = game.data.registry.maps.get("test:map:town").unwrap();
    assert!(
        game.data.maps[&town].def.layout[3].contains(':'),
        "the door is in the loaded map: {}",
        game.data.maps[&town].def.layout[3]
    );
    // A text the loader would refuse: its words, and nothing written.
    let text = match game
        .handle(&Op::EditorRead {
            path: "data/maps/meadow.ron".into(),
        })
        .unwrap()
    {
        Reply::File { text, .. } => text,
        other => panic!("{other:?}"),
    };
    let broken = text.replacen("width: ", "widt: ", 1);
    match game
        .handle(&Op::EditorCheck {
            path: "data/maps/meadow.ron".into(),
            text: broken.clone(),
        })
        .unwrap()
    {
        Reply::Checked { errors } => {
            assert_eq!(errors.len(), 1);
            assert!(errors[0].contains("parse error"), "{errors:?}");
        }
        other => panic!("{other:?}"),
    }
    match game.handle(&Op::EditorWrite {
        path: "data/maps/meadow.ron".into(),
        text: broken,
    }) {
        Err(OpError::Invalid { errors }) => assert_eq!(errors.len(), 1),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        std::fs::read_to_string(scratch.join("data/maps/meadow.ron")).unwrap(),
        text
    );
    // Notes: written at once, no reseal.
    let sealed = std::fs::read_to_string(scratch.join("pack.ron")).unwrap();
    game.handle(&Op::EditorNoteSet {
        key: "test:map:town/0/1,1".into(),
        text: Some("The inn's front room.".into()),
    })
    .unwrap();
    match game.handle(&Op::EditorNotes { prefix: None }).unwrap() {
        Reply::Notes { keys, orphans } => {
            assert!(keys.contains(&"test:map:town/0/1,1".to_owned()), "{keys:?}");
            assert!(orphans.is_empty());
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        std::fs::read_to_string(scratch.join("pack.ron")).unwrap(),
        sealed
    );
    // Bounds and the simulation's refusal.
    assert!(matches!(
        game.handle(&Op::EditorRead {
            path: "../pack.ron".into()
        }),
        Err(OpError::Failed { .. })
    ));
    assert!(matches!(
        game.handle(&Op::EditorNote {
            key: "k".repeat(4097)
        }),
        Err(OpError::TooLong { .. })
    ));
    assert_eq!(
        omnis_sim::dispatch(&mut game.world, &game.data, &Op::EditorStatus),
        Err(OpError::HostOnly)
    );
}
