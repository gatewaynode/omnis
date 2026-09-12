//! `DevSocketPlugin` (feature `devtools`): the loopback listener the MCP bridge and scripts
//! talk to (ARCHITECTURE.md §9.1). Newline-delimited JSON: one request per line,
//! `{"id": .., "op": "<name>", "args": {..}}`, one reply per line,
//! `{"id": .., "ok": true, "result": ..}` or `{"id": .., "ok": false, "error": {"kind": ..}}`.
//! No async runtime: a system polls the non-blocking listener every frame, reads whole lines,
//! answers on the main thread with the world in hand, and queues replies. One client at a
//! time, loopback only. Every line is untrusted (§6.2): longer than `MAX_LINE` bytes and the
//! client is told so and dropped.

use crate::AppConfig;
use crate::pixel::CanvasImage;
use crate::sim::{PackData, SimEvent, SimSet, SimWorld, WorldReplaced, load, save};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use omnis_sim::omnis_data::load_packs;
use omnis_sim::ops::{bounded, client_path, slot_view, status};
use omnis_sim::{Op, OpError, Reply, dispatch};
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
}

impl Default for DevSocketPlugin {
    fn default() -> Self {
        DevSocketPlugin {
            addr: DEFAULT_ADDR.into(),
            addr_file: PathBuf::from(ADDR_FILE),
        }
    }
}

impl Plugin for DevSocketPlugin {
    fn build(&self, app: &mut App) {
        match bind(&self.addr, &self.addr_file) {
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
        app.add_message::<ScreenshotSaved>()
            .add_systems(Update, serve.in_set(SimSet::Collect));
    }
}

/// A screenshot request finished.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ScreenshotSaved {
    /// The file.
    pub path: PathBuf,
    /// Why it was not written, if it was not.
    pub error: Option<String>,
}

/// The listener and its one client.
#[derive(Resource)]
pub struct DevSocket {
    listener: TcpListener,
    /// Where the listener is bound.
    pub local_addr: SocketAddr,
    client: Option<Client>,
    /// Screenshots awaiting the renderer, by request id.
    pending: Vec<(Value, PathBuf)>,
}

struct Client {
    stream: TcpStream,
    incoming: Vec<u8>,
    outgoing: Vec<u8>,
}

fn bind(addr: &str, addr_file: &Path) -> std::io::Result<DevSocket> {
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
        client: None,
        pending: Vec::new(),
    })
}

/// Split one request line into its id and op. The id comes back even when the op is bad. An
/// op whose arguments all have defaults may omit `args`, and an op without arguments may
/// carry `args: {}`; serde wants the opposite in each case, so a failed parse is retried once
/// with `args` toggled.
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
    let object = value
        .as_object_mut()
        .unwrap_or_else(|| unreachable!("checked above"));
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

#[allow(clippy::too_many_arguments)]
fn serve(
    mut commands: Commands,
    socket: Option<ResMut<DevSocket>>,
    mut world: Option<ResMut<SimWorld>>,
    mut data: Option<ResMut<PackData>>,
    config: Res<AppConfig>,
    canvas: Option<Res<CanvasImage>>,
    mut events: MessageWriter<SimEvent>,
    mut replaced: MessageWriter<WorldReplaced>,
    mut shots: MessageReader<ScreenshotSaved>,
) {
    let Some(mut socket) = socket else {
        return;
    };
    socket.accept();
    for shot in shots.read() {
        if let Some(i) = socket.pending.iter().position(|(_, p)| *p == shot.path) {
            let (id, path) = socket.pending.remove(i);
            let result = match &shot.error {
                None => Ok(Reply::Written {
                    path: path.display().to_string(),
                }),
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
        let (Some(world), Some(data)) = (world.as_deref_mut(), data.as_deref_mut()) else {
            socket.queue(&id, &Err(OpError::bad_request("the game is still booting")));
            continue;
        };
        if let Op::Screenshot { path } = &op {
            match screenshot(&mut commands, canvas.as_deref(), path.as_deref()) {
                Ok(path) => socket.pending.push((id, path)),
                Err(e) => socket.queue(&id, &Err(e)),
            }
            continue;
        }
        let result = handle(world, data, &config, &mut events, &mut replaced, &op);
        socket.queue(&id, &result);
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

/// Answer one op with the world in hand: host ops here, the rest through `dispatch`.
fn handle(
    world: &mut SimWorld,
    data: &mut PackData,
    config: &AppConfig,
    events: &mut MessageWriter<SimEvent>,
    replaced: &mut MessageWriter<WorldReplaced>,
    op: &Op,
) -> Result<Reply, OpError> {
    {
        match op {
            Op::SaveWrite { path } => {
                let path = client_path(path, &["ron"])?;
                save(&world.0, Path::new(path)).map_err(OpError::failed)?;
                Ok(Reply::Written { path: path.into() })
            }
            Op::SaveRead { path, force } => {
                let path = client_path(path, &["ron"])?;
                world.0 = load(&data.0, Path::new(path), *force).map_err(OpError::failed)?;
                replaced.write(WorldReplaced);
                status(&world.0, &data.0).map(Reply::Status)
            }
            Op::PackReload => {
                let roots: Vec<&Path> = config.packs.iter().map(PathBuf::as_path).collect();
                let fresh = load_packs(&roots).map_err(OpError::failed)?;
                let p = world.0.position;
                if fresh
                    .maps
                    .get(&p.map)
                    .and_then(|m| m.cell(p.x, p.y))
                    .is_none()
                {
                    return Err(OpError::failed(format!(
                        "the party's tile {p} is not in the reloaded packs; nothing changed"
                    )));
                }
                data.0 = fresh;
                replaced.write(WorldReplaced);
                Ok(Reply::Done {})
            }
            Op::RulesSet { slot, source } => {
                bounded(source)?;
                data.0
                    .rules
                    .set_slot(slot, source)
                    .map_err(OpError::bad_request)?;
                slot_view(&data.0, slot).map(|rule| Reply::Rule { rule })
            }
            other => {
                let reply = dispatch(&mut world.0, &data.0, other)?;
                // Presentation follows the socket's commands the way it follows the keyboard.
                if let Reply::Events { events: produced }
                | Reply::Script {
                    events: produced, ..
                } = &reply
                {
                    for event in produced {
                        events.write(SimEvent(event.clone()));
                    }
                }
                Ok(reply)
            }
        }
    }
}

/// Ask the renderer for the canvas; the reply waits for `ScreenshotSaved`.
fn screenshot(
    commands: &mut Commands,
    canvas: Option<&CanvasImage>,
    path: Option<&str>,
) -> Result<PathBuf, OpError> {
    let canvas =
        canvas.ok_or_else(|| OpError::failed("no canvas to capture; is the renderer running?"))?;
    let path = PathBuf::from(client_path(path.unwrap_or(DEFAULT_SCREENSHOT), &["png"])?);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(OpError::failed)?;
    }
    let target = path.clone();
    commands.spawn(Screenshot::image(canvas.0.clone())).observe(
        move |captured: On<ScreenshotCaptured>,
              mut saved: MessageWriter<ScreenshotSaved>,
              mut commands: Commands| {
            let error = captured
                .image
                .clone()
                .try_into_dynamic()
                .map_err(|e| e.to_string())
                .and_then(|image| image.to_rgb8().save(&target).map_err(|e| e.to_string()))
                .err();
            saved.write(ScreenshotSaved {
                path: target.clone(),
                error,
            });
            commands.entity(captured.entity).despawn();
        },
    );
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnis_sim::Command;

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
            Op::Screenshot { path: None }
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
        assert_eq!(line, "{\"id\":1,\"ok\":true,\"result\":{\"value\":null}}\n");
        let line = encode(&Value::Null, &Err(OpError::HostOnly));
        assert_eq!(
            line,
            "{\"error\":{\"kind\":\"HostOnly\"},\"id\":null,\"ok\":false}\n"
        );
        assert_eq!(line.matches('\n').count(), 1);
    }
}
