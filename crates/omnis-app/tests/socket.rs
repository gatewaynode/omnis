//! The dev socket over a real loopback connection, headless: boot, a client steps the game
//! and reads it back, bad lines are refused, saves round-trip, oversize lines drop the client.
#![cfg(feature = "devtools")]

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use omnis_app::AppConfig;
use omnis_app::sim::{SimEvent, SimPlugin, SimWorld, WorldReplaced};
use omnis_app::socket::{DevSocketPlugin, MAX_LINE};
use omnis_sim::Event;
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
        // Loopback delivery is asynchronous: the game polls once per frame, so the peer keeps
        // running frames until the reply shows up, like a client of the real game loop.
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

fn messages<M: Message + Clone>(app: &App) -> Vec<M> {
    let messages = app.world().resource::<Messages<M>>();
    let mut cursor = messages.get_cursor();
    cursor.read(messages).cloned().collect()
}

/// A headless app with the socket bound, and the address it wrote.
fn boot(dir: &Path) -> (App, SocketAddr) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let addr_file = dir.join("dev.addr");
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_resource(AppConfig {
            packs: vec![repo.join("packs/base"), repo.join("packs/test")],
            seed: 3,
            save_path: dir.join("quick.ron"),
        })
        .add_plugins((
            SimPlugin,
            DevSocketPlugin {
                addr: "127.0.0.1:0".into(),
                addr_file: addr_file.clone(),
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

fn status_commands_and_queries(app: &mut App, peer: &mut Peer) {
    let reply = peer.send(app, r#"{"id": 1, "op": "game.status"}"#);
    assert_eq!(reply["ok"], json!(true), "{reply}");
    assert_eq!(reply["id"], json!(1));
    assert_eq!(reply["result"]["map"], json!("test:map:meadow"));
    assert_eq!(reply["result"]["turn"], json!(0));

    let reply = peer.send(
        app,
        r#"{"id": 2, "op": "sim.command", "args": {"command": {"Step": "Forward"}}}"#,
    );
    assert_eq!(reply["ok"], json!(true), "{reply}");
    assert!(reply["result"]["events"][0]["Moved"].is_object(), "{reply}");
    let position = app.world().resource::<SimWorld>().0.position;
    assert_eq!((position.x, position.y), (16, 15));
    assert!(
        messages::<SimEvent>(app)
            .iter()
            .any(|e| matches!(e.0, Event::Visible { .. })),
        "the socket's events reach presentation"
    );

    let reply = peer.send(
        app,
        r#"{"id": "x", "op": "map.text", "args": {"map": "test:map:dungeon"}}"#,
    );
    assert_eq!(reply["id"], json!("x"));
    assert!(
        reply["result"]["text"].as_str().unwrap().starts_with("+-+"),
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
    let message = reply["error"]["message"].as_str().unwrap();
    assert!(message.starts_with("path '"), "{reply}");
    let reply = peer.send(app, r#"{"id": 7, "op": "screenshot"}"#);
    let message = reply["error"]["message"].as_str().unwrap();
    assert!(message.contains("no canvas"), "headless: {reply}");
}

fn saves_and_reload(app: &mut App, peer: &mut Peer, dir: &Path) {
    let reply = peer.send(
        app,
        r#"{"id": 8, "op": "save.write", "args": {"path": "saves/a.ron"}}"#,
    );
    assert_eq!(reply["result"]["path"], json!("saves/a.ron"), "{reply}");
    assert!(dir.join("saves/a.ron").is_file());
    peer.send(
        app,
        r#"{"id": 9, "op": "sim.command", "args": {"command": {"Turn": "Left"}}}"#,
    );
    let reply = peer.send(
        app,
        r#"{"id": 10, "op": "save.read", "args": {"path": "saves/a.ron"}}"#,
    );
    assert_eq!(reply["result"]["turn"], json!(1), "{reply}");
    assert_eq!(app.world().resource::<SimWorld>().0.turn, 1);
    assert_eq!(
        messages::<WorldReplaced>(app).len(),
        1,
        "presentation redraws"
    );
    let reply = peer.send(app, r#"{"id": 11, "op": "pack.reload"}"#);
    assert_eq!(reply["ok"], json!(true), "{reply}");
}

/// A second caller is told to wait and dropped; an oversize line ends the first connection
/// with a reason, and the slot frees up.
/// The party and the rules through the same socket, after the save has been reloaded.
fn party_and_rules(app: &mut App, peer: &mut Peer) {
    let reply = peer.send(
        app,
        r#"{"id": 3, "op": "rules.set", "args": {"slot": "spell_points.pool", "source": "level * 10"}}"#,
    );
    assert_eq!(reply["ok"], json!(true), "{reply}");
    assert_eq!(reply["result"]["rule"]["source"], json!("level * 10"));
    let reply = peer.send(
        app,
        r#"{"id": 4, "op": "party.create", "args": {"character": {"name": "Ilvara", "race": "base:race:elf", "class": "base:class:wizard", "background": "base:background:acolyte", "alignment": "ChaoticGood", "scores": [8, 14, 13, 15, 12, 10], "skills": ["Arcana", "History"]}}}"#,
    );
    assert_eq!(reply["ok"], json!(true), "{reply}");
    let reply = peer.send(app, r#"{"id": 5, "op": "party.get"}"#);
    assert_eq!(
        reply["result"]["party"]["members"][0]["spell_points_max"],
        json!(10),
        "{reply}"
    );
    assert!(
        messages::<SimEvent>(app)
            .iter()
            .any(|e| e.0 == Event::PartyChanged),
        "party changes reach presentation"
    );
}

fn second_client_and_flood(app: &mut App, peer: &mut Peer, addr: SocketAddr) {
    let mut other = Peer::connect(addr);
    let busy = other.read(app);
    let message = busy["error"]["message"].as_str().unwrap();
    assert!(message.contains("another client"), "{busy}");
    other.expect_closed(app);
    assert!(
        app.world()
            .resource::<omnis_app::socket::DevSocket>()
            .connected(),
        "the first client stays"
    );

    // The game reads only between frames, so a megabyte is written from another thread
    // while this one keeps the frames coming; otherwise the kernel buffers fill and both
    // sides wait.
    let mut writer = peer.stream.try_clone().unwrap();
    let flood = std::thread::spawn(move || writer.write_all(&vec![b'x'; MAX_LINE + 1]));
    let reply = peer.read(app);
    let _ = flood.join().unwrap();
    let message = reply["error"]["message"].as_str().unwrap();
    assert!(message.contains("longer than"), "{reply}");
    peer.expect_closed(app);
    let mut again = Peer::connect(addr);
    let reply = again.send(app, r#"{"id": 12, "op": "game.status"}"#);
    assert_eq!(reply["ok"], json!(true), "{reply}");
}

/// One test, one process: `save.write` paths are relative, so the working directory is set
/// once for the whole run.
#[test]
fn a_client_drives_the_game_over_loopback() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("socket");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_current_dir(&dir).unwrap();
    let (mut app, addr) = boot(&dir);
    let mut peer = Peer::connect(addr);
    app.update();
    status_commands_and_queries(&mut app, &mut peer);
    refusals(&mut app, &mut peer);
    saves_and_reload(&mut app, &mut peer, &dir);
    party_and_rules(&mut app, &mut peer);
    second_client_and_flood(&mut app, &mut peer, addr);
}
