//! `omnis-cli`: headless subcommands. Argument parsing is by hand (self-supporting rule).
#![forbid(unsafe_code)]

use omnis_cli::args::Args;
use omnis_cli::{Headless, bake, schema};
use omnis_data::load_packs;
use omnis_data::ron_io::read_ron;
use omnis_sim::command::parse_script;
use omnis_sim::{Op, Replay, Reply};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage:
  omnis-cli validate <pack-dir>...                 load packs and report every error
  omnis-cli schema dump                            the data files, save, replay, and ops by example
  omnis-cli map text <map-id> [--pack <dir>]...    a map as text with the party at its start
  omnis-cli play --script <file> [--pack <dir>]... [--seed <n>]
                                                   run a command script, print events and the fingerprint
  omnis-cli replay <replay.ron> [--pack <dir>]...  re-run a replay and assert its fingerprint
  omnis-cli tileset bake <spec.ron>...             bake viewport slot sprites and write the tileset file
";

fn main() -> ExitCode {
    let words: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let result = match words.as_slice() {
        ["validate", roots @ ..] if !roots.is_empty() => validate(roots),
        ["schema", "dump"] => schema::dump()
            .map_err(|e| e.to_string())
            .map(|s| print!("{s}")),
        ["tileset", "bake", specs @ ..] if !specs.is_empty() => bake_all(specs),
        ["map", "text", rest @ ..] => Args::parse(rest).and_then(map_text),
        ["play", rest @ ..] => Args::parse(rest).and_then(play),
        ["replay", rest @ ..] => Args::parse(rest).and_then(replay),
        _ => Err(USAGE.trim_end().to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("omnis-cli: {e}");
            ExitCode::FAILURE
        }
    }
}

fn validate(roots: &[&str]) -> Result<(), String> {
    let roots: Vec<&Path> = roots.iter().map(Path::new).collect();
    let data = load_packs(&roots).map_err(|report| report.to_string())?;
    println!(
        "ok: {} packs, {} maps, {} tilesets, {} languages, {} races, {} classes, {} backgrounds, {} items, {} conditions, {} spells, {} monsters, {} rule slots",
        data.packs.len(),
        data.maps.len(),
        data.tilesets.len(),
        data.text.len(),
        data.races.len(),
        data.classes.len(),
        data.backgrounds.len(),
        data.items.len(),
        data.conditions.len(),
        data.spells.len(),
        data.monsters.len(),
        data.rules.slot_names().count(),
    );
    Ok(())
}

fn bake_all(specs: &[&str]) -> Result<(), String> {
    let mut failed = Vec::new();
    for spec in specs {
        match bake::bake_spec(Path::new(spec)) {
            Ok(report) => println!("{report}"),
            Err(e) => failed.push(format!("{spec}: {e}")),
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(failed.join("\n"))
    }
}

fn map_text(args: Args) -> Result<(), String> {
    let [map] = args.positional.as_slice() else {
        return Err("map text needs exactly one map id".into());
    };
    let mut game = Headless::new(args.packs_or_default(), args.seed.unwrap_or(0))
        .map_err(|e| e.to_string())?;
    match game.handle(&Op::MapText {
        map: Some(map.clone()),
    }) {
        Ok(Reply::Text { text }) => {
            print!("{text}");
            Ok(())
        }
        Ok(other) => Err(format!("unexpected reply {other:?}")),
        Err(e) => Err(e.to_string()),
    }
}

fn play(args: Args) -> Result<(), String> {
    let script = args.script.as_ref().ok_or("play needs --script <file>")?;
    let text = std::fs::read_to_string(script).map_err(|e| format!("{}: {e}", script.display()))?;
    let commands = parse_script(&text).map_err(|e| format!("{}: {e}", script.display()))?;
    let mut game = Headless::new(args.packs_or_default(), args.seed.unwrap_or(0))
        .map_err(|e| e.to_string())?;
    for command in commands {
        let turn = game.world.turn;
        let word = command.word();
        match game.handle(&Op::SimCommand { command }) {
            Ok(Reply::Events { events }) => {
                for event in events {
                    println!("{turn} {word}: {event:?}");
                }
            }
            Ok(other) => return Err(format!("unexpected reply {other:?}")),
            Err(e) => return Err(format!("turn {turn}: {e}")),
        }
    }
    let fingerprint = game.world.fingerprint().map_err(|e| e.to_string())?;
    println!("fingerprint: {fingerprint:016x}");
    Ok(())
}

fn replay(args: Args) -> Result<(), String> {
    let [file] = args.positional.as_slice() else {
        return Err("replay needs exactly one replay file".into());
    };
    let file = PathBuf::from(file);
    let replay: Replay = read_ron(&file, &file).map_err(|e| e.to_string())?;
    let packs = args.packs_or_default();
    let roots: Vec<&Path> = packs.iter().map(PathBuf::as_path).collect();
    let data = load_packs(&roots).map_err(|report| report.to_string())?;
    replay.check(&data).map_err(|e| e.to_string())?;
    println!(
        "ok: {} commands reproduce fingerprint {:016x}",
        replay.commands.len(),
        replay.fingerprint
    );
    Ok(())
}
