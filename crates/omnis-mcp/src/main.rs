//! `omnis-mcp [--headless [--pack <dir>]... [--seed <n>]] [--root <dir>] [--addr <ip:port>]
//! [--addr-file <file>] [--log <file>]`: the MCP bridge on stdio. Without `--headless` it talks
//! to the running game through the address in `<root>/.omnis/dev.addr`; `--root` is the
//! project directory the game runs in (default: the current directory). Logs go to stderr
//! and, when `--log` is given, to that file; stdout carries only the protocol. Exits on stdin
//! EOF.
#![forbid(unsafe_code)]

use omnis_cli::Headless;
use omnis_mcp::backend::{Backend, GameLink};
use omnis_mcp::bridge::Bridge;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

struct Options {
    headless: bool,
    packs: Vec<PathBuf>,
    seed: u64,
    root: PathBuf,
    addr: Option<std::net::SocketAddr>,
    addr_file: Option<PathBuf>,
    log: Option<PathBuf>,
}

fn parse_args() -> Result<Options, String> {
    let mut options = Options {
        headless: false,
        packs: Vec::new(),
        seed: 1,
        root: PathBuf::from("."),
        addr: None,
        addr_file: None,
        log: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |what: &str| args.next().ok_or(format!("{arg} needs {what}"));
        match arg.as_str() {
            "--headless" => options.headless = true,
            "--pack" => options.packs.push(PathBuf::from(value("a directory")?)),
            "--seed" => {
                let text = value("a number")?;
                options.seed = text.parse().map_err(|_| format!("bad seed '{text}'"))?;
            }
            "--root" => options.root = PathBuf::from(value("a directory")?),
            "--addr" => {
                let text = value("<ip:port>")?;
                options.addr = Some(
                    text.parse()
                        .map_err(|e| format!("bad address '{text}': {e}"))?,
                );
            }
            "--addr-file" => options.addr_file = Some(PathBuf::from(value("a file")?)),
            "--log" => options.log = Some(PathBuf::from(value("a file")?)),
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    if options.packs.is_empty() {
        options.packs.push(options.root.join("packs/base"));
        options.packs.push(options.root.join("packs/test"));
    }
    Ok(options)
}

fn open_log(path: &PathBuf) -> Option<std::fs::File> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .ok()
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(options) => options,
        Err(e) => {
            eprintln!("omnis-mcp: {e}");
            return ExitCode::FAILURE;
        }
    };
    let backend = if options.headless {
        match Headless::new(options.packs.clone(), options.seed) {
            Ok(game) => Backend::Headless(Box::new(game)),
            Err(e) => {
                eprintln!("omnis-mcp: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        let addr_file = options
            .addr_file
            .clone()
            .unwrap_or_else(|| options.root.join(".omnis/dev.addr"));
        Backend::Game(GameLink::new(options.addr, addr_file))
    };
    let mut log = options.log.as_ref().and_then(open_log);
    let mut bridge = Bridge::new(backend, options.root.clone());
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(log) = &mut log {
            let _ = writeln!(log, "> {line}");
        }
        if let Some(reply) = bridge.handle_line(&line) {
            if let Some(log) = &mut log {
                let _ = write!(log, "< {reply}");
            }
            if stdout
                .write_all(reply.as_bytes())
                .and_then(|()| stdout.flush())
                .is_err()
            {
                break;
            }
        }
    }
    eprintln!(
        "omnis-mcp: stdin closed, exiting (handshake era: {:?})",
        bridge.era
    );
    ExitCode::SUCCESS
}
