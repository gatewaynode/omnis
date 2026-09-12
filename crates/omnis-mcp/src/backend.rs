//! Where ops go: the running game over the dev socket, or an in-process headless game.

use omnis_cli::Headless;
use omnis_cli::omnis_sim::{Op, OpError};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long to wait for the game to answer one op.
pub const GAME_TIMEOUT: Duration = Duration::from_secs(60);

/// The op target.
pub enum Backend {
    /// The running game, connected lazily.
    Game(GameLink),
    /// A headless game in this process.
    Headless(Box<Headless>),
}

/// A connection to the game's dev socket, made on first use and remade after a drop.
pub struct GameLink {
    addr: Option<SocketAddr>,
    addr_file: PathBuf,
    link: Option<(BufReader<TcpStream>, TcpStream)>,
    next_id: u64,
}

impl GameLink {
    /// Connect to `addr`, or to the address in `addr_file` when `addr` is `None`.
    #[must_use]
    pub fn new(addr: Option<SocketAddr>, addr_file: PathBuf) -> GameLink {
        GameLink {
            addr,
            addr_file,
            link: None,
            next_id: 1,
        }
    }

    fn connect(&mut self) -> Result<&mut (BufReader<TcpStream>, TcpStream), String> {
        if self.link.is_none() {
            let addr = match self.addr {
                Some(addr) => addr,
                None => read_addr(&self.addr_file)?,
            };
            let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))
                .map_err(|e| format!("cannot reach the game at {addr}: {e}; is `omnis` running with its dev socket?"))?;
            stream
                .set_read_timeout(Some(GAME_TIMEOUT))
                .map_err(|e| e.to_string())?;
            let reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
            self.link = Some((reader, stream));
        }
        Ok(self
            .link
            .as_mut()
            .unwrap_or_else(|| unreachable!("set above")))
    }

    /// Send one op and wait for its reply: `Ok(result)` or `Err(error object)`.
    pub fn call(&mut self, request: &Value) -> Result<Value, Value> {
        let id = self.next_id;
        self.next_id += 1;
        let mut line = request.clone();
        line["id"] = json!(id);
        let outcome = self.exchange(&line.to_string());
        match outcome {
            Ok(reply) => {
                if reply["id"] != json!(id) {
                    self.link = None;
                    return Err(failed(format!(
                        "the game answered another request: {reply}"
                    )));
                }
                if reply["ok"] == json!(true) {
                    Ok(reply["result"].clone())
                } else {
                    Err(reply["error"].clone())
                }
            }
            Err(message) => {
                self.link = None;
                Err(failed(message))
            }
        }
    }

    fn exchange(&mut self, line: &str) -> Result<Value, String> {
        let (reader, stream) = self.connect()?;
        stream
            .write_all(line.as_bytes())
            .and_then(|()| stream.write_all(b"\n"))
            .map_err(|e| format!("write to the game failed: {e}"))?;
        let mut reply = String::new();
        match reader.read_line(&mut reply) {
            Ok(0) => Err("the game closed the connection".into()),
            Ok(_) => {
                serde_json::from_str(&reply).map_err(|e| format!("bad reply from the game: {e}"))
            }
            Err(e) => Err(format!("read from the game failed: {e}")),
        }
    }
}

fn read_addr(file: &Path) -> Result<SocketAddr, String> {
    let text = std::fs::read_to_string(file).map_err(|e| {
        format!(
            "{}: {e}; start the game (`cargo run -p omnis-app`) so it writes its dev socket address there",
            file.display()
        )
    })?;
    text.trim()
        .parse()
        .map_err(|e| format!("{}: '{}': {e}", file.display(), text.trim()))
}

fn failed(message: String) -> Value {
    serde_json::to_value(OpError::Failed { message }).unwrap_or(Value::Null)
}

impl Backend {
    /// Answer one op given as its wire object `{"op": .., "args": ..}`.
    pub fn call(&mut self, request: &Value) -> Result<Value, Value> {
        match self {
            Backend::Game(link) => link.call(request),
            Backend::Headless(game) => {
                let op: Op = serde_json::from_value(request.clone()).map_err(|e| {
                    serde_json::to_value(OpError::bad_request(e)).unwrap_or(Value::Null)
                })?;
                match game.handle(&op) {
                    Ok(reply) => Ok(serde_json::to_value(reply).unwrap_or(Value::Null)),
                    Err(error) => Err(serde_json::to_value(error).unwrap_or(Value::Null)),
                }
            }
        }
    }
}
