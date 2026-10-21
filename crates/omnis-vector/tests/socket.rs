//! The dev socket over a real loopback connection, headless (ARCHITECTURE.md §9.1): a client
//! steps the game and the presentation follows, the commands land in the binder's log and the
//! log still replays, bad lines are refused, saves round-trip and stop the log replaying, the
//! interface reads back as text, and a second client or an oversize line is turned away.
#![cfg(feature = "devtools")]

use crate::common::app::app_logging_to;
use bevy::prelude::*;
use omnis_vector::shell::capture::CapturePlugin;
use omnis_vector::shell::session::Session;
use omnis_vector::shell::socket::{DevSocket, DevSocketPlugin, MAX_LINE};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

struct Peer {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
    last: String,
}

/// Frames the game gets to answer before a reply is declared missing.
const PATIENCE: usize = 200;

impl Peer {
    fn connect(addr: SocketAddr) -> Peer {
        let stream = TcpStream::connect(addr).unwrap();
        // The game polls once per frame, so the peer keeps running frames until the reply
        // shows up, like a client of the real game loop.
        stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let reader = BufReader::new(stream.try_clone().unwrap());
        Peer {
            stream,
            reader,
            last: String::new(),
        }
    }

    fn send(&mut self, app: &mut App, line: &str) -> Value {
        self.last = line.to_owned();
        self.stream.write_all(line.as_bytes()).unwrap();
        self.stream.write_all(b"\n").unwrap();
        self.read(app)
    }

    /// The next reply line, running frames until it arrives.
    fn read(&mut self, app: &mut App) -> Value {
        for _ in 0..PATIENCE {
            app.update();
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => panic!("closed instead of replying to {}", self.last),
                Ok(_) => {
                    return serde_json::from_str(&line).unwrap_or_else(|e| panic!("{e}: {line}"));
                }
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(e) => panic!("{e} after {}", self.last),
            }
        }
        panic!("no reply after {}", self.last)
    }

    /// The connection is closed by the game, running frames until it is.
    fn expect_closed(&mut self, app: &mut App) {
        for _ in 0..PATIENCE {
            app.update();
            match self.reader.read(&mut [0; 8]) {
                Ok(0) => return,
                Ok(n) => panic!("{n} unexpected bytes"),
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(e) => panic!("{e}"),
            }
        }
        panic!("still open");
    }
}

fn session(app: &App) -> &Session {
    app.world().resource::<Session>()
}

/// The headless shell at the game's start with the socket bound under `dir`, and its address.
fn boot(dir: &Path) -> (App, SocketAddr) {
    let addr_file = dir.join("dev.addr");
    let mut app = app_logging_to(dir.join("session.ron"));
    app.add_plugins((
        CapturePlugin,
        DevSocketPlugin {
            addr: "127.0.0.1:0".into(),
            addr_file: addr_file.clone(),
            root: dir.to_owned(),
        },
    ));
    app.update();
    let addr: SocketAddr = std::fs::read_to_string(&addr_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(addr.ip().is_loopback());
    (app, addr)
}

fn commands_reach_the_presentation_and_the_log(app: &mut App, peer: &mut Peer) {
    let reply = peer.send(app, r#"{"id": 1, "op": "game.status"}"#);
    assert_eq!(reply["ok"], json!(true), "{reply}");
    assert_eq!(reply["id"], json!(1));
    assert_eq!(reply["result"]["position"]["map"], json!("test:map:town"));

    let logged = session(app).binder.log.len();
    let reply = peer.send(
        app,
        r#"{"id": 2, "op": "sim.command", "args": {"command": {"Step": "Forward"}}}"#,
    );
    assert_eq!(reply["ok"], json!(true), "{reply}");
    assert!(reply["result"]["events"][0]["Moved"].is_object(), "{reply}");
    let s = session(app);
    let p = s.views.position;
    assert_eq!((p.x, p.y), (9, 2), "the views follow the socket's step");
    assert_eq!(s.binder.log.len(), logged + 1, "the step is in the log");
    let pose = omnis_vector::grid::cell_of(s.pose.x, s.pose.z);
    assert_eq!(pose, (9, 2), "the pose follows");
    assert!(
        s.lines.iter().any(|l| l.starts_with("Moved")),
        "the HUD tells it: {:?}",
        s.lines
    );

    // A script stops at its first refusal; the log keeps exactly what was applied.
    let logged = session(app).binder.log.len();
    let reply = peer.send(
        app,
        r#"{"id": 3, "op": "sim.script", "args": {"commands": [{"Turn": "Left"}, {"Combat": "Dodge"}, {"Turn": "Right"}]}}"#,
    );
    assert_eq!(reply["result"]["applied"], json!(1), "{reply}");
    assert_eq!(session(app).binder.log.len(), logged + 1);
    app.update();
    assert_eq!(
        session(app).views.position.facing,
        omnis_sim::omnis_core::Facing::South,
        "the turn holds: the pose took the new facing, so the binder does not turn back"
    );
    // A refused command is not the client's refusal.
    let reply = peer.send(
        app,
        r#"{"id": 4, "op": "sim.command", "args": {"command": {"Combat": "Dodge"}}}"#,
    );
    assert_eq!(reply["error"]["kind"], json!("Rejected"), "{reply}");
    assert_eq!(session(app).binder.refusals, 0);

    // The log, socket commands and all, replays to the world as it stands.
    let s = session(app);
    let replay = s
        .binder
        .replay(&s.data, s.seed, s.settings)
        .expect("the log replays");
    let status = s.world.status(&s.data).unwrap();
    assert_eq!(format!("{:016x}", replay.fingerprint), status.fingerprint);

    let reply = peer.send(
        app,
        r#"{"id": "x", "op": "map.text", "args": {"map": "test:map:dungeon"}}"#,
    );
    assert_eq!(reply["id"], json!("x"));
    assert!(
        reply["result"]["text"].as_str().unwrap().starts_with("+-+"),
        "{reply}"
    );
    let reply = peer.send(app, r#"{"id": 5, "op": "party.get"}"#);
    assert_eq!(
        reply["result"]["party"]["members"][0]["can_fight"],
        json!(true),
        "{reply}"
    );
}

fn refusals(app: &mut App, peer: &mut Peer) {
    let reply = peer.send(app, "not json");
    assert_eq!(reply["ok"], json!(false));
    assert_eq!(reply["id"], Value::Null);
    assert_eq!(reply["error"]["kind"], json!("BadRequest"));
    let reply = peer.send(app, r#"{"id": 5, "op": "fly"}"#);
    assert_eq!(
        (reply["id"].clone(), reply["error"]["kind"].clone()),
        (json!(5), json!("BadRequest"))
    );
    let reply = peer.send(
        app,
        r#"{"id": 6, "op": "save.write", "args": {"path": "../x.ron"}}"#,
    );
    assert_eq!(reply["error"]["kind"], json!("Failed"), "{reply}");
    assert!(
        reply["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("path '"),
        "{reply}"
    );
    // Headless: both targets are what the window shows, and there is no window.
    for target in ["canvas", "window"] {
        let reply = peer.send(
            app,
            &format!(r#"{{"id": 7, "op": "screenshot", "args": {{"target": "{target}"}}}}"#),
        );
        assert_eq!(reply["id"], json!(7));
        let message = reply["error"]["message"].as_str().unwrap();
        assert!(message.contains("no window"), "headless: {reply}");
    }
    let reply = peer.send(
        app,
        r#"{"id": 8, "op": "screenshot", "args": {"path": "../x.png"}}"#,
    );
    assert_eq!(reply["error"]["kind"], json!("Failed"), "{reply}");
    let reply = peer.send(
        app,
        r#"{"id": 9, "op": "screenshot", "args": {"target": "desktop"}}"#,
    );
    assert_eq!(reply["error"]["kind"], json!("BadRequest"), "{reply}");
}

fn the_interface_reads_back(app: &mut App, peer: &mut Peer) {
    let reply = peer.send(app, r#"{"id": 10, "op": "screen.text"}"#);
    assert_eq!(reply["id"], json!(10), "{reply}");
    let text = reply["result"]["text"].as_str().unwrap();
    for label in ["Forward  W", "Actions  Space", "Menu  Esc"] {
        assert!(
            text.lines()
                .any(|l| l.contains("[button]") && l.contains(label)),
            "{label} is a button: {text}"
        );
    }
    assert!(
        !text.contains("Close"),
        "the closed action menu is not on screen: {text}"
    );
}

/// The rules and the party through the same socket: a rule set changes what the views read and
/// ends the log's replay, and a created member joins the party and the log.
fn party_and_rules(app: &mut App, peer: &mut Peer) {
    let before = session(app).views.party.members.len();
    let reply = peer.send(
        app,
        r#"{"id": 7, "op": "rules.set", "args": {"slot": "spell_points.pool", "source": "level * 10"}}"#,
    );
    assert_eq!(
        reply["result"]["rule"]["source"],
        json!("level * 10"),
        "{reply}"
    );
    let reply = peer.send(
        app,
        r#"{"id": 8, "op": "party.create", "args": {"character": {"name": "Wren", "race": "base:race:elf", "class": "base:class:wizard", "background": "base:background:acolyte", "alignment": "ChaoticGood", "scores": [8, 14, 13, 15, 12, 10], "skills": ["Arcana", "History"]}}}"#,
    );
    assert_eq!(reply["ok"], json!(true), "{reply}");
    let s = session(app);
    let wren = s.views.party.members.last().expect("a member");
    assert_eq!(s.views.party.members.len(), before + 1, "the member joins");
    assert_eq!(
        (wren.name.as_str(), wren.spell_points_max),
        ("Wren", 10),
        "the rule set reads in the views"
    );
    let why = s.save_log().expect_err("a rule set is not in the log");
    assert!(why.contains("spell_points.pool"), "{why}");
}

fn saves_stop_the_log_replaying(app: &mut App, peer: &mut Peer, dir: &Path) {
    let reply = peer.send(
        app,
        r#"{"id": 11, "op": "save.write", "args": {"path": "saves/a.ron"}}"#,
    );
    assert_eq!(reply["result"]["path"], json!("saves/a.ron"), "{reply}");
    assert!(dir.join("saves/a.ron").is_file());
    let turn = session(app).views.here.turn;
    peer.send(
        app,
        r#"{"id": 12, "op": "sim.command", "args": {"command": {"Turn": "Left"}}}"#,
    );
    assert!(session(app).save_log().is_ok(), "still replays");
    let reply = peer.send(
        app,
        r#"{"id": 16, "op": "save.check", "args": {"path": "saves/a.ron"}}"#,
    );
    assert_eq!(reply["result"]["reply"], json!("check"), "{reply}");
    assert_eq!(
        reply["result"]["check"]["verdict"],
        json!("same"),
        "{reply}"
    );
    assert_eq!(
        session(app).views.here.turn,
        turn + 1,
        "a check changes nothing"
    );
    let reply = peer.send(
        app,
        r#"{"id": 13, "op": "save.read", "args": {"path": "saves/a.ron"}}"#,
    );
    assert_eq!(reply["result"]["turn"], json!(turn), "{reply}");
    let s = session(app);
    assert_eq!(s.views.here.turn, turn, "the views are the loaded world's");
    assert!(s.reshape, "the lines are redrawn");
    let why = s.save_log().expect_err("a loaded world is not the log's");
    assert!(why.contains("saves/a.ron"), "{why}");
    let reply = peer.send(app, r#"{"id": 14, "op": "pack.reload"}"#);
    assert_eq!(reply["ok"], json!(true), "{reply}");
    assert_eq!(reply["result"]["reply"], json!("done"), "{reply}");
    assert_eq!(
        reply["result"]["check"]["verdict"],
        json!("same"),
        "the same packs: {reply}"
    );
    assert!(
        session(app)
            .lines
            .iter()
            .any(|l| l == "Packs reloaded: the same packs"),
        "the log says it"
    );
}

fn second_client_and_flood(app: &mut App, peer: &mut Peer, addr: SocketAddr) {
    let mut other = Peer::connect(addr);
    let busy = other.read(app);
    let message = busy["error"]["message"].as_str().unwrap();
    assert!(message.contains("another client"), "{busy}");
    other.expect_closed(app);
    assert!(
        app.world().resource::<DevSocket>().connected(),
        "the first client stays"
    );
    // The game reads only between frames, so a megabyte is written from another thread
    // while this one keeps the frames coming.
    let mut writer = peer.stream.try_clone().unwrap();
    let flood = std::thread::spawn(move || writer.write_all(&vec![b'x'; MAX_LINE + 1]));
    let reply = peer.read(app);
    let _ = flood.join().unwrap();
    let message = reply["error"]["message"].as_str().unwrap();
    assert!(message.contains("longer than"), "{reply}");
    peer.expect_closed(app);
    let mut again = Peer::connect(addr);
    let reply = again.send(app, r#"{"id": 15, "op": "game.status"}"#);
    assert_eq!(reply["ok"], json!(true), "{reply}");
}

/// One test, one connection, in order: each phase leaves the world the next one starts from.
#[test]
fn a_client_drives_the_vector_client_over_loopback() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("vector-socket");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (mut app, addr) = boot(&dir);
    let mut peer = Peer::connect(addr);
    app.update();
    commands_reach_the_presentation_and_the_log(&mut app, &mut peer);
    refusals(&mut app, &mut peer);
    the_interface_reads_back(&mut app, &mut peer);
    saves_stop_the_log_replaying(&mut app, &mut peer, &dir);
    party_and_rules(&mut app, &mut peer);
    second_client_and_flood(&mut app, &mut peer, addr);
}

/// At the title there is no game: every op but the screen's says so, and the screen reads back.
#[test]
fn at_the_title_the_socket_says_no_game_runs_and_reads_the_screen() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("vector-socket-title");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let addr_file = dir.join("dev.addr");
    let mut app = crate::common::app::app_at_title(".omnis/test-saves/none.ron");
    app.add_plugins((
        CapturePlugin,
        DevSocketPlugin {
            addr: "127.0.0.1:0".into(),
            addr_file: addr_file.clone(),
            root: dir.clone(),
        },
    ));
    app.update();
    let addr: SocketAddr = std::fs::read_to_string(&addr_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let mut peer = Peer::connect(addr);
    let reply = peer.send(&mut app, r#"{"id": 1, "op": "game.status"}"#);
    assert_eq!(reply["ok"], json!(false), "{reply}");
    assert_eq!(
        reply["error"]["message"],
        json!(omnis_vector::shell::socket::NO_GAME)
    );
    let reply = peer.send(&mut app, r#"{"id": 2, "op": "screen.text"}"#);
    let text = reply["result"]["text"].as_str().unwrap();
    assert!(text.contains("\"OMNIS\""), "{text}");
    assert!(text.contains("[button] \"New game\""), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}
