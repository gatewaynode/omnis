//! One meaning per field name (protocol 2, ARCHITECTURE.md §4.9). Every JSON key the protocol
//! carries is collected from four sources: every `Command` instance of the schema proof, applied
//! in turn to a headless world (so their rejections are read too); a visit to a town service;
//! and the golden walk and fight, replayed command by command with each one's events. Every view
//! op is asked after each of those commands. The test fails when one key carries two JSON types
//! (a `spell` that is a string here and a registry number there), when a definition key is not a
//! string id, when a key that names a member holds a number no member has had, or when a key that
//! names a position in no particular list appears.

mod common;

use common::commands::instances;
use omnis_cli::Headless;
use omnis_cli::omnis_sim::omnis_core::Facing;
use omnis_cli::omnis_sim::omnis_data::ron_io::read_ron;
use omnis_cli::omnis_sim::{Command, DevCommand, Op, Replay, ServiceCommand, World};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Keys that name a party member, by its `CharacterId` wherever they appear.
const MEMBER_KEYS: [&str; 8] = [
    "member", "caster", "with", "giver", "receiver", "target", "order", "Member",
];

/// Keys that name a definition, by its string id wherever they appear.
const DEFINITION_KEYS: [&str; 8] = [
    "spell",
    "item",
    "monster",
    "map",
    "service",
    "condition",
    "class",
    "feature",
];

/// Keys protocol 1 used for a position in some list; protocol 2 names the list instead. A `row`
/// is banned only as a number: as a name it is the front or back row.
const BANNED_KEYS: [&str; 2] = ["caster_id", "at"];

/// The view ops asked after every command.
const VIEWS: [Op; 8] = [
    Op::GameStatus,
    Op::ViewportGet,
    Op::PartyGet,
    Op::CombatGet,
    Op::ServiceGet,
    Op::TimeClocks,
    Op::RestGet,
    Op::CastGet,
];

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A JSON value's type, as the vocabulary compares it. Serde tags an enum's unit variant as a
/// string and any other as an object, so the two are one type here; the definition keys are
/// held to strings alone by the test.
fn kind(value: &Value) -> Option<&'static str> {
    match value {
        Value::Null => None,
        Value::Bool(_) => Some("bool"),
        Value::Number(_) => Some("number"),
        Value::String(_) | Value::Object(_) => Some("string or object"),
        Value::Array(_) => Some("array"),
    }
}

/// Every key seen, the types it carried and where each type was first seen, and what broke
/// the rule.
#[derive(Default)]
struct Vocabulary {
    kinds: BTreeMap<String, BTreeMap<&'static str, String>>,
    /// Each rule broken, by shape, with the first place it was seen.
    problems: BTreeMap<String, String>,
}

impl Vocabulary {
    /// Read one wire value, seen at `place`, with `members` the identities that exist.
    fn read(&mut self, place: &str, value: &Value, members: &BTreeSet<u64>) {
        match value {
            Value::Object(fields) => {
                for (key, inner) in fields {
                    // A capitalized key is a variant's tag: its meaning is its enum's.
                    let field = key.starts_with(|c: char| c.is_ascii_lowercase());
                    if let Some(k) = kind(inner).filter(|_| field) {
                        self.kinds
                            .entry(key.clone())
                            .or_default()
                            .entry(k)
                            .or_insert_with(|| format!("{place}: {value}"));
                    }
                    if BANNED_KEYS.contains(&key.as_str())
                        || (key == "row" && inner.is_number())
                        || (key == "index" && !fields.contains_key("stack"))
                    {
                        let shape: Vec<&String> = fields.keys().collect();
                        self.problems
                            .entry(format!("banned key `{key}` among {shape:?}"))
                            .or_insert_with(|| format!("{place}: {value}"));
                    }
                    if DEFINITION_KEYS.contains(&key.as_str())
                        && !(inner.is_string() || inner.is_null())
                    {
                        self.problems
                            .entry(format!("`{key}` is not a string id"))
                            .or_insert_with(|| format!("{place}: {value}"));
                    }
                    if MEMBER_KEYS.contains(&key.as_str()) {
                        self.members(key, inner, place, members);
                    }
                    self.read(place, inner, members);
                }
            }
            Value::Array(items) => {
                for item in items {
                    self.read(place, item, members);
                }
            }
            _ => {}
        }
    }

    /// A member key's numbers must be identities some member has had.
    fn members(&mut self, key: &str, value: &Value, place: &str, members: &BTreeSet<u64>) {
        let numbers: Vec<u64> = match value {
            Value::Number(n) => n.as_u64().into_iter().collect(),
            Value::Array(items) => items.iter().filter_map(Value::as_u64).collect(),
            _ => Vec::new(),
        };
        for number in numbers {
            if !members.contains(&number) {
                self.problems
                    .entry(format!("`{key}` {number} is no member's identity"))
                    .or_insert_with(|| place.to_string());
            }
        }
    }

    /// Every rule broken: a key with two types, then the problems found while reading.
    fn broken(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .kinds
            .iter()
            .filter(|(_, kinds)| kinds.len() > 1)
            .map(|(key, kinds)| {
                let seen: Vec<String> = kinds.iter().map(|(k, at)| format!("{k} ({at})")).collect();
                format!("`{key}` carries {}", seen.join(" and "))
            })
            .collect();
        out.extend(
            self.problems
                .iter()
                .map(|(what, at)| format!("{what} (first at {at})")),
        );
        out
    }
}

/// The identities of the members the world has now.
fn identities(game: &Headless) -> BTreeSet<u64> {
    game.world
        .party
        .ids()
        .into_iter()
        .map(|id| u64::from(id.0))
        .collect()
}

/// Apply one command and read what came back, then every view.
fn play(game: &mut Headless, place: &str, command: Command, seen: &mut Seen) {
    seen.members.extend(identities(game));
    let wire = serde_json::to_value(&command).unwrap();
    let reply = answer(game, &Op::SimCommand { command });
    seen.members.extend(identities(game));
    seen.wire.push((format!("{place} command"), wire));
    seen.wire.push((format!("{place} answer"), reply));
    for op in &VIEWS {
        let view = answer(game, op);
        seen.wire.push((format!("{place} {op:?}"), view));
    }
}

/// A reply or an error, as JSON.
fn answer(game: &mut Headless, op: &Op) -> Value {
    match game.handle(op) {
        Ok(reply) => serde_json::to_value(reply).unwrap(),
        Err(error) => serde_json::to_value(error).unwrap(),
    }
}

/// What the sources put on the wire, with the identities members have had meanwhile.
#[derive(Default)]
struct Seen {
    wire: Vec<(String, Value)>,
    members: BTreeSet<u64>,
}

fn headless() -> Headless {
    Headless::new(
        vec![repo().join("packs/base"), repo().join("packs/test")],
        1,
    )
    .unwrap()
}

/// Every source, on the real packs.
fn everything() -> Seen {
    let mut seen = Seen::default();
    let mut game = headless();
    for (n, command) in instances().into_iter().enumerate() {
        play(&mut game, &format!("instance {n}"), command, &mut seen);
    }
    // Into the test town's smith, where the status and the service name it.
    let mut game = headless();
    let smith = [
        Command::Dev(DevCommand::Teleport {
            map: "test:map:town".into(),
            x: 1,
            y: 1,
            facing: Facing::North,
        }),
        Command::Interact,
        Command::Service(ServiceCommand::Leave),
    ];
    for (n, command) in smith.into_iter().enumerate() {
        play(&mut game, &format!("town {n}"), command, &mut seen);
    }
    for name in ["walk", "fight"] {
        let path = repo().join(format!("crates/omnis-sim/tests/replays/{name}.ron"));
        let replay: Replay = read_ron(&path, &path).unwrap();
        let mut game = headless();
        game.world = World::new(&game.data, replay.seed, replay.settings).unwrap();
        for (n, command) in replay.commands.into_iter().enumerate() {
            play(&mut game, &format!("{name} {n}"), command, &mut seen);
        }
    }
    seen
}

fn vocabulary(seen: &Seen) -> Vocabulary {
    let mut words = Vocabulary::default();
    for (place, value) in &seen.wire {
        words.read(place, value, &seen.members);
    }
    words
}

#[test]
fn every_key_on_the_wire_has_one_meaning() {
    let seen = everything();
    let words = vocabulary(&seen);
    let broken = words.broken();
    assert!(broken.is_empty(), "{}", broken.join("\n"));
    for key in DEFINITION_KEYS {
        assert!(
            words.kinds.contains_key(key),
            "`{key}` is on the wire somewhere"
        );
    }
    assert!(
        seen.wire
            .iter()
            .any(|(place, _)| place.starts_with("fight")),
        "the fight was replayed"
    );
}

#[test]
fn the_vocabulary_catches_a_name_that_drifted() {
    let members = BTreeSet::from([0, 1]);
    let cast = json!({"SpellCast": {"caster": 1, "spell": "base:spell:bless"}});
    let mut words = Vocabulary::default();
    words.read("cast", &cast, &members);
    assert!(words.broken().is_empty(), "{:?}", words.broken());

    // Protocol 1's event: the spell by its registry number.
    let mut drifted = Vocabulary::default();
    drifted.read("cast", &cast, &members);
    drifted.read(
        "event",
        &json!({"SpellCast": {"caster": 1, "spell": 3}}),
        &members,
    );
    assert_eq!(
        drifted.broken(),
        vec![
            "`spell` carries number (event: {\"caster\":1,\"spell\":3}) and string or object \
             (cast: {\"caster\":1,\"spell\":\"base:spell:bless\"})"
                .to_string(),
            "`spell` is not a string id (first at event: {\"caster\":1,\"spell\":3})".to_string(),
        ]
    );

    // A member by marching-order slot where no member has that identity.
    let mut slot = Vocabulary::default();
    slot.read("command", &json!({"Cast": {"caster": 2}}), &members);
    assert_eq!(
        slot.broken(),
        vec!["`caster` 2 is no member's identity (first at command)".to_string()]
    );

    // A bare row, and an index that is not an individual in a stack.
    let mut rows = Vocabulary::default();
    rows.read("view", &json!({"row": 1, "index": 0}), &members);
    rows.read(
        "event",
        &json!({"Monster": {"stack": 0, "index": 1}}),
        &members,
    );
    rows.read(
        "criteria",
        &json!({"Row": {"who": "Me", "row": "Front"}}),
        &members,
    );
    assert_eq!(rows.broken().len(), 2, "{:?}", rows.broken());
}
