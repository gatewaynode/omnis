//! Every `Reply` names itself on the wire (ARCHITECTURE.md §4.9): a headless world on the real
//! packs answers an op of each kind, and each reply goes through JSON, carries its `reply` tag,
//! and reads back unchanged without knowing the op that asked for it. The tag list is an
//! exhaustive `match`, so a new variant fails the build until it is named here and covered.

use omnis_cli::Headless;
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
        Reply::Value { .. } => "value",
        Reply::Done {} => "done",
    }
}

const EVERY: usize = 17;

#[test]
fn every_reply_names_itself_and_reads_back_without_its_op() {
    let packs = vec![repo().join("packs/base"), repo().join("packs/test")];
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
        ask(&mut game, Op::AutomapGet { map: None }),
        ask(&mut game, Op::MapText { map: None }),
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
        ask(
            &mut game,
            Op::WorldQuery {
                path: "position.x".into(),
            },
        ),
        // The host ops need files; their replies are built as the hosts build them.
        Reply::Written {
            path: client_path("a.ron", &["ron"]).unwrap().into(),
        },
        Reply::Done {},
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
