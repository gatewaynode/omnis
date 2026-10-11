//! `DevSocketPlugin` (feature `devtools`): the loopback listener the MCP bridge and scripts talk
//! to (ARCHITECTURE.md §9.1), the same address file, transport and ops `omnis-app` serves, so
//! `omnis-mcp` does not change. Newline-delimited JSON: one request per line,
//! `{"id": .., "op": "<name>", "args": {..}}`, one reply per line,
//! `{"id": .., "ok": true, "result": ..}` or `{"id": .., "ok": false, "error": {"kind": ..}}`.
//!
//! No async runtime: a system polls the non-blocking listener every frame, reads whole lines,
//! answers on the main thread with the session in hand (`shell/host.rs`), and queues replies.
//! `screenshot` waits for the live capture (`shell/capture.rs`) and `screen.text` for a system
//! that holds the whole world (`shell/screen_text.rs`). One client at a time, loopback only.
//! Every line is untrusted (§6.2): longer than `MAX_LINE` bytes and the client is told so and
//! dropped.
//!
//! The framing (`parse_request`, `encode`, the client buffers) is `omnis-app`'s, copied: that
//! copy goes when `omnis-app` is retired (tasks/TODO.md, parity P9).

use super::VectorSet;
use super::capture::{ShotRequest, ShotSaved};
use super::host;
use super::screen_text::screen_text;
use super::session::Session;
use bevy::prelude::*;
use omnis_sim::ops::client_path;
use omnis_sim::{Op, OpError, Reply};
use serde_json::{Value, json};
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};

/// Longest request line accepted, in bytes.
pub const MAX_LINE: usize = 1 << 20;
/// Where the bound address is written for the bridge to find.
pub const ADDR_FILE: &str = ".omnis/dev.addr";
/// The default bind address: loopback, any free port.
pub const DEFAULT_ADDR: &str = "127.0.0.1:0";
/// Where `screenshot` writes when no path is given.
pub const DEFAULT_SCREENSHOT: &str = ".omnis/screenshot.png";

/// The dev socket plugin.
#[derive(Debug, Clone)]
pub struct DevSocketPlugin {
    /// A loopback `ip:port` to bind; port 0 picks a free one.
    pub addr: String,
    /// The file the bound address is written to.
    pub addr_file: PathBuf,
    /// The directory a request's relative paths (saves, screenshots) are under: the working
    /// directory, where the bridge reads them.
    pub root: PathBuf,
}

impl Default for DevSocketPlugin {
    fn default() -> Self {
        DevSocketPlugin {
            addr: DEFAULT_ADDR.into(),
            addr_file: PathBuf::from(ADDR_FILE),
            root: PathBuf::from("."),
        }
    }
}

impl Plugin for DevSocketPlugin {
    fn build(&self, app: &mut App) {
        match bind(&self.addr, &self.addr_file, &self.root) {
            Ok(socket) => {
                info!(
                    "dev socket on {} (address in {})",
                    socket.local_addr,
                    self.addr_file.display()
                );
                app.insert_resource(socket);
            }
            Err(e) => error!("dev socket: {e}"),
        }
        app.add_systems(
            Update,
            // With the screens, so the socket answers at the title and under the pause.
            (serve, answer_screen_text)
                .chain()
                .in_set(VectorSet::Screens)
                .run_if(resource_exists::<DevSocket>),
        );
    }
}

/// The answer to every op but the screen's while no game runs.
pub const NO_GAME: &str = "no game is running; start one from the title";

/// The listener and its one client.
#[derive(Resource)]
pub struct DevSocket {
    listener: TcpListener,
    /// Where the listener is bound.
    pub local_addr: SocketAddr,
    root: PathBuf,
    client: Option<Client>,
    /// Screenshots awaiting the capture: request id, the path as asked, the file.
    pending: Vec<(Value, String, PathBuf)>,
    /// `screen.text` requests, answered by `answer_screen_text` with the whole world in hand.
    wants_text: Vec<Value>,
}

struct Client {
    stream: TcpStream,
    incoming: Vec<u8>,
    outgoing: Vec<u8>,
}

fn bind(addr: &str, addr_file: &Path, root: &Path) -> std::io::Result<DevSocket> {
    let wanted: SocketAddr = addr
        .parse()
        .map_err(|e| std::io::Error::new(ErrorKind::InvalidInput, format!("'{addr}': {e}")))?;
    if !wanted.ip().is_loopback() {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            format!("'{addr}' is not a loopback address"),
        ));
    }
    let listener = TcpListener::bind(wanted)?;
    listener.set_nonblocking(true)?;
    let local_addr = listener.local_addr()?;
    if let Some(parent) = addr_file.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(addr_file, format!("{local_addr}\n"))?;
    Ok(DevSocket {
        listener,
        local_addr,
        root: root.to_owned(),
        client: None,
        pending: Vec::new(),
        wants_text: Vec::new(),
    })
}

/// Split one request line into its id and op. The id comes back even when the op is bad. An
/// op whose arguments all have defaults may omit `args`, and an op without arguments may
/// carry `args: {}`; serde wants the opposite in each case, so a failed parse is retried once
/// with `args` toggled.
///
/// # Errors
/// The id (or null) and why the line is not a request.
pub fn parse_request(line: &[u8]) -> Result<(Value, Op), (Value, String)> {
    let mut value: Value =
        serde_json::from_slice(line).map_err(|e| (Value::Null, e.to_string()))?;
    let Some(object) = value.as_object_mut() else {
        return Err((Value::Null, "a request is a JSON object".into()));
    };
    let id = object.remove("id").unwrap_or(Value::Null);
    let first = match serde_json::from_value::<Op>(value.clone()) {
        Ok(op) => return Ok((id, op)),
        Err(e) => e.to_string(),
    };
    let Some(object) = value.as_object_mut() else {
        return Err((id, first));
    };
    match object.get("args") {
        None => {
            object.insert("args".into(), json!({}));
        }
        Some(args) if args.is_null() || args.as_object().is_some_and(serde_json::Map::is_empty) => {
            object.remove("args");
        }
        Some(_) => return Err((id, first)),
    }
    match serde_json::from_value::<Op>(value) {
        Ok(op) => Ok((id, op)),
        Err(_) => Err((id, first)),
    }
}

/// One reply line, newline included.
#[must_use]
pub fn encode(id: &Value, result: &Result<Reply, OpError>) -> String {
    let mut line = match result {
        Ok(reply) => json!({"id": id, "ok": true, "result": reply}),
        Err(error) => json!({"id": id, "ok": false, "error": error}),
    }
    .to_string();
    line.push('\n');
    line
}

impl DevSocket {
    /// Whether a client is connected.
    #[must_use]
    pub fn connected(&self) -> bool {
        self.client.is_some()
    }

    /// Take the next connection when there is no client; tell any extra caller to wait.
    fn accept(&mut self) {
        loop {
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    if self.client.is_some() {
                        let busy = encode(
                            &Value::Null,
                            &Err(OpError::bad_request("another client is connected")),
                        );
                        let _ = stream.write_all(busy.as_bytes());
                        continue;
                    }
                    if stream.set_nonblocking(true).is_ok() {
                        self.client = Some(Client {
                            stream,
                            incoming: Vec::new(),
                            outgoing: Vec::new(),
                        });
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => return,
                Err(e) => {
                    warn!("dev socket accept: {e}");
                    return;
                }
            }
        }
    }

    fn queue(&mut self, id: &Value, result: &Result<Reply, OpError>) {
        if let Some(client) = &mut self.client {
            client
                .outgoing
                .extend_from_slice(encode(id, result).as_bytes());
        }
    }

    /// The file a screenshot request writes, its directory made.
    fn shot_file(&self, path: Option<&str>) -> Result<(String, PathBuf), OpError> {
        let asked = client_path(path.unwrap_or(DEFAULT_SCREENSHOT), &["png"])?;
        let file = self.root.join(asked);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).map_err(OpError::failed)?;
        }
        Ok((asked.to_owned(), file))
    }
}

impl Client {
    /// Whole lines received, and the reason the connection must close, if it must.
    fn read_lines(&mut self) -> (Vec<Vec<u8>>, Option<String>) {
        let mut chunk = [0u8; 8192];
        let mut close = None;
        loop {
            match self.stream.read(&mut chunk) {
                Ok(0) => {
                    close = Some("closed by the client".into());
                    break;
                }
                Ok(n) => self.incoming.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => {
                    close = Some(e.to_string());
                    break;
                }
            }
        }
        let mut lines = Vec::new();
        while let Some(end) = self.incoming.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.incoming.drain(..=end).collect();
            lines.push(line);
        }
        if lines.iter().any(|l| l.len() > MAX_LINE) || self.incoming.len() > MAX_LINE {
            lines.clear();
            close = Some(format!("line longer than {MAX_LINE} bytes"));
        }
        (lines, close)
    }

    /// Send what the kernel will take.
    fn flush(&mut self) -> std::io::Result<()> {
        while !self.outgoing.is_empty() {
            match self.stream.write(&self.outgoing) {
                Ok(0) => return Err(ErrorKind::WriteZero.into()),
                Ok(n) => {
                    self.outgoing.drain(..n);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(()),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

fn serve(
    mut socket: ResMut<DevSocket>,
    mut session: Option<ResMut<Session>>,
    mut shots: MessageWriter<ShotRequest>,
    mut saved: MessageReader<ShotSaved>,
) {
    socket.accept();
    for shot in saved.read() {
        if let Some(i) = socket.pending.iter().position(|(_, _, f)| *f == shot.path) {
            let (id, asked, _) = socket.pending.remove(i);
            let result = match &shot.error {
                None => Ok(Reply::Written { path: asked }),
                Some(e) => Err(OpError::failed(e)),
            };
            socket.queue(&id, &result);
        }
    }
    let Some(client) = &mut socket.client else {
        return;
    };
    let (lines, close) = client.read_lines();
    for line in lines {
        let (id, op) = match parse_request(&line) {
            Ok(request) => request,
            Err((id, message)) => {
                socket.queue(&id, &Err(OpError::bad_request(message)));
                continue;
            }
        };
        match &op {
            Op::Screenshot { path, .. } => match socket.shot_file(path.as_deref()) {
                Ok((asked, file)) => {
                    shots.write(ShotRequest(file.clone()));
                    socket.pending.push((id, asked, file));
                }
                Err(e) => socket.queue(&id, &Err(e)),
            },
            Op::ScreenText => socket.wants_text.push(id),
            _ => {
                let root = socket.root.clone();
                let result = match session.as_deref_mut() {
                    Some(session) => host::handle(session, &root, &op),
                    None => Err(OpError::failed(NO_GAME)),
                };
                socket.queue(&id, &result);
            }
        }
    }
    if let Some(client) = &mut socket.client {
        if let Some(reason) = close {
            client.outgoing.extend_from_slice(
                encode(&Value::Null, &Err(OpError::bad_request(&reason))).as_bytes(),
            );
            let _ = client.flush();
            info!("dev socket client dropped: {reason}");
            socket.client = None;
        } else if let Err(e) = client.flush() {
            info!("dev socket client dropped: {e}");
            socket.client = None;
        }
    }
}

/// Answer `screen.text` requests with the whole world in hand; the reply goes out with the
/// next frame's flush.
fn answer_screen_text(world: &mut World) {
    world.resource_scope(|world, mut socket: Mut<DevSocket>| {
        if socket.wants_text.is_empty() {
            return;
        }
        let text = screen_text(world);
        for id in std::mem::take(&mut socket.wants_text) {
            socket.queue(&id, &Ok(Reply::Text { text: text.clone() }));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnis_sim::Command;
    use omnis_sim::ops::ShotTarget;

    #[test]
    fn requests_parse_with_their_id_and_replies_are_one_line() {
        let (id, op) =
            parse_request(br#"{"id": 7, "op": "sim.command", "args": {"command": "Interact"}}"#)
                .unwrap();
        assert_eq!(id, json!(7));
        assert_eq!(
            op,
            Op::SimCommand {
                command: Command::Interact
            }
        );
        let (id, op) = parse_request(br#"{"op": "game.status", "id": "a"}"#).unwrap();
        assert_eq!((id, op), (json!("a"), Op::GameStatus));
        assert_eq!(
            parse_request(br#"{"op": "events.tail"}"#).unwrap().1,
            Op::EventsTail { count: 32 },
            "defaults stand in for a missing args"
        );
        assert_eq!(
            parse_request(br#"{"op": "screenshot"}"#).unwrap().1,
            Op::Screenshot {
                path: None,
                target: ShotTarget::Canvas
            }
        );
        assert_eq!(
            parse_request(br#"{"op": "game.status", "args": {}}"#)
                .unwrap()
                .1,
            Op::GameStatus,
            "and an empty args is fine on an op without any"
        );
        let (id, message) = parse_request(br#"{"id": 3, "op": "fly"}"#).unwrap_err();
        assert_eq!(id, json!(3));
        assert!(message.contains("fly"), "{message}");
        assert_eq!(parse_request(b"[1]").unwrap_err().0, Value::Null);
        assert_eq!(parse_request(b"nope").unwrap_err().0, Value::Null);

        let line = encode(&json!(1), &Ok(Reply::Value { value: None }));
        assert_eq!(
            line, "{\"id\":1,\"ok\":true,\"result\":{\"reply\":\"value\",\"value\":null}}\n",
            "a reply names itself (ARCHITECTURE.md §4.9)"
        );
        let line = encode(&Value::Null, &Err(OpError::HostOnly));
        assert_eq!(
            line,
            "{\"error\":{\"kind\":\"HostOnly\"},\"id\":null,\"ok\":false}\n"
        );
        assert_eq!(line.matches('\n').count(), 1);
    }
}
