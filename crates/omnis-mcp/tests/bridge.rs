//! The bridge binary over its stdio, headless against the real test pack: both handshake
//! eras, the tool list, tool calls, and protocol errors. Game mode is exercised against a
//! fake dev socket that answers every op the same way.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Server {
    child: Child,
    reader: BufReader<std::process::ChildStdout>,
}

impl Server {
    fn start(args: &[&str]) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_omnis-mcp"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let reader = BufReader::new(child.stdout.take().unwrap());
        Server { child, reader }
    }

    fn headless() -> Server {
        let base = repo().join("packs/base");
        let test = repo().join("packs/test");
        Server::start(&[
            "--headless",
            "--pack",
            base.to_str().unwrap(),
            "--pack",
            test.to_str().unwrap(),
            "--seed",
            "1",
        ])
    }

    fn send(&mut self, line: &Value) {
        let stdin = self.child.stdin.as_mut().unwrap();
        stdin.write_all(line.to_string().as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
    }

    fn call(&mut self, line: &Value) -> Value {
        self.send(line);
        let mut reply = String::new();
        self.reader.read_line(&mut reply).unwrap();
        serde_json::from_str(&reply).unwrap_or_else(|e| panic!("{e}: {reply}"))
    }

    fn tool(&mut self, id: u64, name: &str, arguments: Value) -> Value {
        self.call(&json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": arguments}}))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        drop(self.child.stdin.take());
        let status = self.child.wait().unwrap();
        assert!(status.success(), "the bridge exits cleanly on EOF");
    }
}

fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name": "test", "version": "0"},
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

#[test]
fn legacy_handshake_lists_tools_and_drives_the_headless_game() {
    let mut server = Server::headless();
    let reply = server.call(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}}));
    assert_eq!(reply["result"]["protocolVersion"], json!("2025-06-18"));
    assert_eq!(reply["result"]["serverInfo"]["name"], json!("omnis-mcp"));
    assert!(reply["result"]["capabilities"]["tools"].is_object());
    assert!(reply["result"].get("resultType").is_none(), "legacy shape");
    server.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    let reply = server.call(&json!({"jsonrpc": "2.0", "id": 2, "method": "ping"}));
    assert_eq!(
        (reply["id"].clone(), reply["result"].clone()),
        (json!(2), json!({})),
        "the notification got no reply"
    );

    let reply = server.call(&json!({"jsonrpc": "2.0", "id": 3, "method": "tools/list"}));
    let tools = reply["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 18);
    assert!(
        tools
            .iter()
            .all(|t| t["inputSchema"]["type"] == json!("object"))
    );
    assert!(tools.iter().any(|t| t["name"] == json!("sim_command")));

    let reply = server.tool(4, "game_status", json!({}));
    assert_eq!(
        reply["result"]["structuredContent"]["map"],
        json!("test:map:meadow")
    );
    assert_eq!(reply["result"]["content"][0]["type"], json!("text"));
    assert_eq!(reply["result"]["isError"], json!(false));
    let reply = server.tool(5, "sim_command", json!({"command": {"Step": "Forward"}}));
    let events = reply["result"]["structuredContent"]["events"]
        .as_array()
        .unwrap();
    assert!(events[0]["Moved"].is_object(), "{reply}");
    let visible = events.iter().find(|e| e.get("Visible").is_some()).unwrap();
    assert!(
        visible["Visible"]["count"].as_u64().unwrap() > 10,
        "{visible}"
    );
    assert!(
        visible["Visible"].get("tiles").is_none(),
        "tiles compact to a count"
    );
    let text = reply["result"]["content"][0]["text"].as_str().unwrap();
    assert!(!text.contains("\"depth\""), "no tiles in the text either");
    let reply = server.tool(6, "world_query", json!({"path": "position.y"}));
    assert_eq!(reply["result"]["structuredContent"]["value"], json!("15"));
    let reply = server.tool(7, "map_text", json!({"map": "test:map:dungeon"}));
    assert!(
        reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("+-+")
    );
    let reply = server.tool(8, "screenshot", json!({}));
    assert_eq!(
        reply["result"]["isError"],
        json!(true),
        "headless has no canvas: {reply}"
    );
    let reply = server.tool(9, "save_write", json!({"path": "../x.ron"}));
    assert_eq!(reply["result"]["isError"], json!(true));
    assert!(
        reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("Failed: path"),
        "{reply}"
    );
}

#[test]
fn the_party_and_the_rules_go_through_the_same_pipe() {
    let mut server = Server::headless();
    server.call(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}}));
    let draft = json!({"name": "Ilvara", "race": "base:race:elf", "class": "base:class:wizard", "background": "base:background:acolyte", "alignment": "ChaoticGood", "scores": [8, 14, 13, 15, 12, 10], "skills": ["Arcana", "History"]});
    let reply = server.tool(10, "party_create", json!({"character": draft}));
    assert_eq!(reply["result"]["isError"], json!(false), "{reply}");
    let reply = server.tool(11, "party_get", json!({}));
    let member = &reply["result"]["structuredContent"]["party"]["members"][0];
    assert_eq!(member["name"], json!("Ilvara"));
    assert_eq!(member["spell_points_max"], json!(4), "{member}");
    let reply = server.tool(
        12,
        "rules_set",
        json!({"slot": "spell_points.pool", "source": "level * 10"}),
    );
    assert_eq!(
        reply["result"]["structuredContent"]["rule"]["source"],
        json!("level * 10"),
        "{reply}"
    );
    let reply = server.tool(13, "party_create", json!({"character": draft}));
    assert_eq!(reply["result"]["isError"], json!(false), "{reply}");
    let reply = server.tool(14, "party_get", json!({}));
    assert_eq!(
        reply["result"]["structuredContent"]["party"]["members"][1]["spell_points_max"],
        json!(10),
        "the new formula, no rebuild"
    );
    let reply = server.tool(
        15,
        "rules_set",
        json!({"slot": "spell_points.pool", "source": "level * wisdom"}),
    );
    assert_eq!(reply["result"]["isError"], json!(true));
    assert!(
        reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("unknown input 'wisdom'"),
        "{reply}"
    );
    let reply = server.tool(16, "party_create", json!({"character": {"name": "", "race": "base:race:elf", "class": "base:class:wizard", "background": "base:background:acolyte", "alignment": "ChaoticGood", "scores": [8, 14, 13, 15, 12, 10]}}));
    assert_eq!(
        reply["result"]["structuredContent"]["kind"],
        json!("Rejected"),
        "{reply}"
    );
}

#[test]
fn protocol_errors_have_the_right_codes() {
    let mut server = Server::headless();
    let reply = server.tool(1, "fly", json!({}));
    assert_eq!(reply["error"]["code"], json!(-32602));
    assert!(reply["error"]["message"].as_str().unwrap().contains("fly"));
    let reply = server.call(&json!({"jsonrpc": "2.0", "id": 2, "method": "resources/list"}));
    assert_eq!(reply["error"]["code"], json!(-32601));
    let reply = server.call(&json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "sim_command", "arguments": []}}));
    assert_eq!(reply["error"]["code"], json!(-32602));
    let reply = server.call(&json!({"jsonrpc": "2.0", "id": 4, "method": "server/discover"}));
    assert_eq!(
        reply["error"]["code"],
        json!(-32602),
        "no modern _meta: {reply}"
    );
    server.send(&json!("not an object"));
    let mut line = String::new();
    server.reader.read_line(&mut line).unwrap();
    let reply: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(
        (reply["id"].clone(), reply["error"]["code"].clone()),
        (Value::Null, json!(-32600))
    );
    let stdin = server.child.stdin.as_mut().unwrap();
    stdin.write_all(b"{{{\n").unwrap();
    let mut line = String::new();
    server.reader.read_line(&mut line).unwrap();
    let reply: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(reply["error"]["code"], json!(-32700));
}

#[test]
fn modern_requests_are_stateless_and_versioned() {
    let mut server = Server::headless();
    let reply = server.call(&json!({"jsonrpc": "2.0", "id": "d", "method": "server/discover", "params": {"_meta": meta()}}));
    let result = &reply["result"];
    assert_eq!(result["resultType"], json!("complete"), "{reply}");
    assert_eq!(result["supportedVersions"], json!(["2026-07-28"]));
    assert_eq!(
        result["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        json!("omnis-mcp")
    );
    assert!(result["capabilities"]["tools"].is_object());
    assert!(
        result.get("tools").is_none(),
        "discovery lists capabilities, not tools"
    );

    let reply = server.call(
        &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {"_meta": meta()}}),
    );
    assert_eq!(reply["result"]["resultType"], json!("complete"));
    assert_eq!(reply["result"]["tools"].as_array().unwrap().len(), 18);
    let reply = server.call(&json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "game_status", "arguments": {}, "_meta": meta()}}));
    assert_eq!(reply["result"]["resultType"], json!("complete"));
    assert_eq!(reply["result"]["structuredContent"]["turn"], json!(0));

    let mut old = meta();
    old["io.modelcontextprotocol/protocolVersion"] = json!("1900-01-01");
    let reply = server.call(
        &json!({"jsonrpc": "2.0", "id": 4, "method": "tools/list", "params": {"_meta": old}}),
    );
    assert_eq!(reply["error"]["code"], json!(-32022));
    assert_eq!(reply["error"]["data"]["supported"], json!(["2026-07-28"]));
    assert_eq!(reply["error"]["data"]["requested"], json!("1900-01-01"));
    let mut bare = meta();
    bare.as_object_mut()
        .unwrap()
        .remove("io.modelcontextprotocol/clientCapabilities");
    let reply = server.call(
        &json!({"jsonrpc": "2.0", "id": 5, "method": "tools/list", "params": {"_meta": bare}}),
    );
    assert_eq!(reply["error"]["code"], json!(-32602));

    // A legacy request on the same process still works: dual era, per request.
    let reply = server.call(&json!({"jsonrpc": "2.0", "id": 6, "method": "tools/list"}));
    assert!(reply["result"].get("resultType").is_none());
}

/// A stand-in for the game's dev socket: one client, every op answered with `reply`.
fn fake_game(reply: Value) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut stream = stream;
        let mut line = String::new();
        while reader.read_line(&mut line).is_ok_and(|n| n > 0) {
            let request: Value = serde_json::from_str(&line).unwrap();
            assert!(request["op"].is_string(), "{request}");
            let answer = json!({"id": request["id"], "ok": true, "result": reply});
            stream.write_all(answer.to_string().as_bytes()).unwrap();
            stream.write_all(b"\n").unwrap();
            line.clear();
        }
    });
    addr
}

#[test]
fn game_mode_talks_to_the_dev_socket_and_returns_screenshots_as_images() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("bridge-game");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("shot.png"), [0x89, b'P', b'N', b'G']).unwrap();
    let addr = fake_game(json!({"path": "shot.png", "turn": 5}));
    let mut server = Server::start(&[
        "--root",
        root.to_str().unwrap(),
        "--addr",
        &addr.to_string(),
    ]);
    server.call(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-11-25"}}));
    let reply = server.tool(2, "game_status", json!({}));
    assert_eq!(
        reply["result"]["structuredContent"]["turn"],
        json!(5),
        "{reply}"
    );
    let reply = server.tool(3, "screenshot", json!({}));
    let content = reply["result"]["content"].as_array().unwrap();
    assert_eq!(content[0]["type"], json!("image"));
    assert_eq!(content[0]["mimeType"], json!("image/png"));
    assert_eq!(content[0]["data"], json!("iVBORw=="));
    assert_eq!(content[1]["text"], json!("saved to shot.png"));

    let mut nobody = Server::start(&["--root", root.to_str().unwrap()]);
    let reply = nobody.tool(1, "game_status", json!({}));
    assert_eq!(reply["result"]["isError"], json!(true));
    assert!(
        reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("start the game"),
        "{reply}"
    );
}

#[test]
fn a_missing_pack_is_a_clean_failure() {
    let status = Command::new(env!("CARGO_BIN_EXE_omnis-mcp"))
        .args(["--headless", "--pack", "/nonexistent/pack"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(!status.success());
}
