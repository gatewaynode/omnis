//! Every `Reply` names itself on the wire (ARCHITECTURE.md §4.9): a headless world on the real
//! packs answers an op of each kind, and each reply goes through JSON, carries its `reply` tag,
//! and reads back unchanged without knowing the op that asked for it. The tag list is an
//! exhaustive `match`, so a new variant fails the build until it is named here and covered.

use omnis_cli::Headless;
use omnis_cli::omnis_sim::api::{examine, save_files};
use omnis_cli::omnis_sim::omnis_core::{Facing, Position};
use omnis_cli::omnis_sim::omnis_data::Disposition;
use omnis_cli::omnis_sim::ops::{PROTOCOL, SlotView};
use omnis_cli::omnis_sim::{
    Command, EncounterSource, EncounterState, Mode, Op, Reply, Stack, ops::client_path,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `save.check`'s reply as the hosts build it: the game saved in memory, then examined.
fn check(game: &Headless) -> Reply {
    let files = save_files(&game.world, &game.data, ".omnis/a.ron").unwrap();
    let disk: std::collections::BTreeMap<String, String> =
        files.into_iter().map(|f| (f.path, f.text)).collect();
    let examined = examine(&disk[".omnis/a.ron"], &game.data, &mut |p| {
        disk.get(p).cloned().ok_or_else(|| format!("no file {p}"))
    })
    .unwrap();
    let check = examined.into_report();
    assert!(!check.packs.is_empty());
    Reply::Check { check }
}

/// The wire name of a reply.
fn tag(reply: &Reply) -> &'static str {
    match reply {
        Reply::Status(_) => "status",
        Reply::Viewport(_) => "viewport",
        Reply::Script { .. } => "script",
        Reply::Events { .. } => "events",
        Reply::Automap { .. } => "automap",
        Reply::Text { .. } => "text",
        Reply::Written { .. } => "written",
        Reply::Party { .. } => "party",
        Reply::Rules { .. } => "rules",
        Reply::Rule { .. } => "rule",
        Reply::Combat { .. } => "combat",
        Reply::Service { .. } => "service",
        Reply::Time { .. } => "time",
        Reply::Rest { .. } => "rest",
        Reply::Casts { .. } => "casts",
        Reply::Objects { .. } => "objects",
        Reply::Creation { .. } => "creation",
        Reply::Value { .. } => "value",
        Reply::Done { .. } => "done",
        Reply::Check { .. } => "check",
        Reply::Editor { .. } => "editor",
        Reply::Maps { .. } => "maps",
        Reply::Files { .. } => "files",
        Reply::File { .. } => "file",
        Reply::Checked { .. } => "checked",
        Reply::Note { .. } => "note",
        Reply::Notes { .. } => "notes",
        Reply::Kinds { .. } => "kinds",
        Reply::Definitions { .. } => "definitions",
        Reply::Definition { .. } => "definition",
        Reply::Words { .. } => "words",
    }
}

const EVERY: usize = 31;

#[test]
fn every_reply_names_itself_and_reads_back_without_its_op() {
    let packs = vec![
        repo().join("packs/base"),
        repo().join("crates/omnis-sim/tests/packs/test"),
    ];
    let mut game = Headless::new(packs, 1).unwrap();
    let ask = |game: &mut Headless, op: Op| game.handle(&op).unwrap();
    let mut replies = vec![
        ask(&mut game, Op::GameStatus),
        ask(&mut game, Op::ViewportGet),
        ask(
            &mut game,
            Op::SimScript {
                commands: vec![Command::Interact],
            },
        ),
        ask(&mut game, Op::EventsTail { count: 4 }),
        ask(
            &mut game,
            Op::AutomapGet {
                map: None,
                level: None,
            },
        ),
        ask(
            &mut game,
            Op::MapText {
                map: None,
                level: None,
            },
        ),
        ask(&mut game, Op::PartyGet),
        ask(&mut game, Op::RulesList),
        ask(
            &mut game,
            Op::RulesGet {
                slot: "spell_points.pool".into(),
            },
        ),
        ask(&mut game, Op::TimeClocks),
        ask(&mut game, Op::RestGet),
        ask(&mut game, Op::CastGet),
        ask(&mut game, Op::ObjectsGet),
        ask(&mut game, Op::CreationGet),
        ask(
            &mut game,
            Op::WorldQuery {
                path: "position.x".into(),
            },
        ),
        ask(&mut game, Op::PackReload { migrate: false }),
        // The host ops need files; their replies are built as the hosts build them.
        Reply::Written {
            path: client_path("a.ron", &["ron"]).unwrap().into(),
            version: None,
        },
        // The editor's, read-only on the shipped pack (M5.3): nothing is written.
        ask(
            &mut game,
            Op::EditorOpen {
                map: "test:map:town".into(),
            },
        ),
        ask(&mut game, Op::EditorMaps),
        ask(&mut game, Op::EditorFiles),
        ask(
            &mut game,
            Op::EditorRead {
                path: "data/maps/town.ron".into(),
            },
        ),
        ask(
            &mut game,
            Op::EditorCheck {
                path: "data/maps/town.ron".into(),
                text: "(schema: 1)".into(),
            },
        ),
        ask(
            &mut game,
            Op::EditorNote {
                key: "test:map:town".into(),
            },
        ),
        ask(&mut game, Op::EditorNotes { prefix: None }),
        check(&game),
        Reply::Rule {
            rule: SlotView {
                name: "x".into(),
                inputs: vec!["level".into()],
                source: "level".into(),
            },
        },
    ];
    let town = game.data.registry.maps.get("test:map:town").unwrap();
    game.world.position = Position {
        map: town,
        level: omnis_cli::omnis_sim::omnis_core::Level::GROUND,
        x: 1,
        y: 1,
        facing: Facing::North,
    };
    ask(
        &mut game,
        Op::SimCommand {
            command: Command::Interact,
        },
    );
    replies.push(ask(&mut game, Op::ServiceGet));
    let goblin = game
        .data
        .registry
        .monsters
        .get("base:monster:goblin")
        .unwrap();
    game.world.mode = Mode::Encounter(EncounterState {
        source: EncounterSource::Random,
        stacks: vec![Stack {
            monster: goblin,
            initial: 2,
            hp: vec![7, 7],
            spent: Vec::new(),
        }],
        disposition: Disposition::Hostile,
        retreat: game.world.position,
    });
    replies.push(ask(&mut game, Op::CombatGet));
    replies.push(ask(&mut game, Op::DataKinds));
    replies.push(ask(
        &mut game,
        Op::DataList {
            kind: Some("monster".into()),
            prefix: None,
            matching: None,
            refers_to: None,
            lang: None,
            cursor: None,
            limit: Some(2),
        },
    ));
    // A map is the largest definition and a region the one written again from the loaded one.
    for id in ["test:map:town", "test:region:dungeon", "base:class:wizard"] {
        replies.push(ask(
            &mut game,
            Op::DataGet {
                id: id.into(),
                lang: None,
            },
        ));
    }
    replies.push(ask(
        &mut game,
        Op::DataText {
            keys: vec![
                "base:text:item.mace.name".into(),
                "base:text:no.such".into(),
            ],
            lang: None,
        },
    ));

    let mut seen = BTreeSet::new();
    for reply in &replies {
        let json = serde_json::to_value(reply).unwrap();
        assert_eq!(json["reply"], tag(reply), "{json}");
        let back: Reply = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(&back, reply, "{json}");
        seen.insert(tag(reply));
    }
    assert_eq!(seen.len(), EVERY, "every variant once at least: {seen:?}");
    let Reply::Status(status) = &replies[0] else {
        panic!("game.status answers with the status");
    };
    assert_eq!(status.protocol, PROTOCOL);
}

/// `done` gained its `check` in protocol 2 (M8.5 7h5), an addition: a `done` without it still
/// reads, as the report of the same packs.
#[test]
fn a_done_without_its_check_still_reads() {
    let back: Reply = serde_json::from_value(serde_json::json!({"reply": "done"})).unwrap();
    assert_eq!(
        back,
        Reply::Done {
            check: omnis_cli::omnis_sim::save::SaveCheck::default()
        }
    );
}

/// `save.read` and `pack.reload` gained `migrate` in protocol 2 (M8.5 7h6), an addition: an op
/// without it reads as before (a bare `pack.reload`, with no `args`, is the socket's to fill:
/// `omnis-vector`'s `parse_request`).
#[test]
fn migrate_is_optional_on_the_wire() {
    let read: Op = serde_json::from_str(r#"{"op":"save.read","args":{"path":"a.ron"}}"#).unwrap();
    assert_eq!(
        read,
        Op::SaveRead {
            path: "a.ron".into(),
            force: false,
            migrate: false
        }
    );
    let reload: Op = serde_json::from_str(r#"{"op":"pack.reload","args":{}}"#).unwrap();
    assert_eq!(reload, Op::PackReload { migrate: false });
    let applied: Op =
        serde_json::from_str(r#"{"op":"pack.reload","args":{"migrate":true}}"#).unwrap();
    assert_eq!(applied, Op::PackReload { migrate: true });
}

/// `data.get` on the wire (W1): the envelope's fields beside `def`, the definition as its pack
/// wrote it, and `kind` the word the id carries. A client reads it back as the same reply.
#[test]
fn a_definition_is_an_envelope_around_the_definition_as_written() {
    let packs = vec![
        repo().join("packs/base"),
        repo().join("crates/omnis-sim/tests/packs/test"),
    ];
    let mut game = Headless::new(packs, 1).unwrap();
    let reply = game
        .handle(&Op::DataGet {
            id: "base:item:mace".into(),
            lang: None,
        })
        .unwrap();
    let json = serde_json::to_value(&reply).unwrap();
    assert_eq!(json["reply"], "definition");
    let view = &json["definition"];
    let fields: Vec<&str> = view
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        fields,
        [
            "def", "id", "kind", "name", "pack", "refers", "replaces", "words"
        ]
    );
    assert_eq!(view["id"], "base:item:mace");
    assert_eq!(view["kind"], "item");
    assert_eq!(view["name"], "base:text:item.mace.name");
    assert_eq!(view["words"]["base:text:item.mace.name"], "Mace");
    assert_eq!(view["pack"], "base");
    assert_eq!(view["refers"], serde_json::json!([]));
    assert_eq!(view["replaces"], serde_json::json!([]));
    // `def` is the file's own shape: its id, its schema, its fields as the pack format has them.
    assert_eq!(view["def"]["id"], "base:item:mace");
    assert_eq!(view["def"]["schema"], 1);
    assert_eq!(view["def"]["cost_cp"], 500);
    assert_eq!(serde_json::from_value::<Reply>(json).unwrap(), reply);
    // A map's `def` carries its terrains, the rows a viewport tile's `terrain` counts.
    let town = game
        .handle(&Op::DataGet {
            id: "test:map:town".into(),
            lang: None,
        })
        .unwrap();
    let json = serde_json::to_value(&town).unwrap();
    assert_eq!(json["definition"]["kind"], "map");
    assert!(
        !json["definition"]["def"]["terrains"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(serde_json::from_value::<Reply>(json).unwrap(), town);
    // A row of a listing is the same envelope without the definition.
    let page = game
        .handle(&Op::DataList {
            kind: Some("item".into()),
            prefix: Some("base:item:mace".into()),
            matching: None,
            refers_to: None,
            lang: None,
            cursor: None,
            limit: None,
        })
        .unwrap();
    let json = serde_json::to_value(&page).unwrap();
    assert!(json["cursor"].is_null());
    let row = &json["definitions"][0];
    let fields: Vec<&str> = row
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(fields, ["id", "kind", "name", "pack", "replaces", "words"]);
    assert_eq!(row["words"]["base:text:item.mace.name"], "Mace");
}
