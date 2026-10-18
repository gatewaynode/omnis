//! `omnis-cli`: headless subcommands. Argument parsing is by hand (self-supporting rule).
#![forbid(unsafe_code)]

use omnis_cli::args::Args;
use omnis_cli::{Headless, bake, schema};
use omnis_data::ron_io::{self, read_ron, write_ron};
use omnis_data::{Data, load_packs};
use omnis_sim::replay::replay_world;
use omnis_sim::{Op, Replay, Reply, parse_script};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

/// Commands a wander takes when `--commands` is not given.
const WANDER_COMMANDS: usize = 10_000;

const USAGE: &str = "usage:
  omnis-cli validate <pack-dir>...                 load packs and report every error
  omnis-cli schema dump                            the data files, save, replay, and ops by example
  omnis-cli map text <map-id> [--pack <dir>]...    a map as text with the party at its start
  omnis-cli play --script <file> [--pack <dir>]... [--seed <n>]
                                                   run a command script, print events and the fingerprint
  omnis-cli replay <replay.ron> [--pack <dir>]... [--time]
                                                   re-run a replay and assert its fingerprint;
                                                   --time reports how long the replay took
  omnis-cli wander --out <replay.ron> [--commands <n>] [--seed <n>] [--pack <dir>]...
                                                   a seeded party wanders the maps; writes the replay
  omnis-cli tileset bake <spec.ron>...             bake viewport slot sprites and write the tileset file
  omnis-cli world tidy [--yes]                     list the world-folder files no save under .omnis/
                                                   names; --yes deletes them
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
        ["wander", rest @ ..] => Args::parse(rest).and_then(wander),
        ["world", "tidy", rest @ ..] => Args::parse(rest).and_then(world_tidy),
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
        "ok: {} packs, {} maps, {} tilesets, {} languages, {} races, {} classes, {} backgrounds, {} items, {} conditions, {} spells, {} monsters, {} rule slots, {} services, {} objects",
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
        data.services.len(),
        data.objects.len(),
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
        level: None,
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
    let words = parse_script(&text).map_err(|e| format!("{}: {e}", script.display()))?;
    let mut game = Headless::new(args.packs_or_default(), args.seed.unwrap_or(0))
        .map_err(|e| e.to_string())?;
    for word in words {
        let turn = game.world.turn;
        // A word counts slots and rows: it means whoever and whatever stands there now.
        let command = word
            .command(&game.world, &game.data)
            .ok_or_else(|| format!("turn {turn}: '{word}' names an empty slot or row"))?;
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
    let data = packs(&args)?;
    if args.time {
        return timed(&replay, &data);
    }
    replay.check(&data).map_err(|e| e.to_string())?;
    println!(
        "ok: {} commands reproduce fingerprint {:016x}",
        replay.commands.len(),
        replay.fingerprint
    );
    Ok(())
}

fn packs(args: &Args) -> Result<Data, String> {
    let packs = args.packs_or_default();
    let roots: Vec<&Path> = packs.iter().map(PathBuf::as_path).collect();
    load_packs(&roots).map_err(|report| report.to_string())
}

/// A replay checked as `replay` does, with what it cost: the replay itself, the fingerprint, and
/// the journal of interactions it left (what a save carries, PRD D28).
fn timed(replay: &Replay, data: &Data) -> Result<(), String> {
    if replay.packs != data.fingerprints {
        return Err(omnis_sim::ReplayError::PackMismatch.to_string());
    }
    let start = Instant::now();
    let world = replay_world(data, replay.seed, replay.settings, &replay.commands)
        .map_err(|e| e.to_string())?;
    let replayed = start.elapsed();
    let start = Instant::now();
    let fingerprint = world.fingerprint().map_err(|e| e.to_string())?;
    let hashed = start.elapsed();
    if fingerprint != replay.fingerprint {
        let diverged = omnis_sim::ReplayError::Diverged {
            expected: replay.fingerprint,
            actual: fingerprint,
        };
        return Err(diverged.to_string());
    }
    let n = replay.commands.len().max(1);
    let journal = ron_io::to_compact(&world.journal).map_err(|e| e.to_string())?;
    println!(
        "ok: {} commands reproduce fingerprint {fingerprint:016x}",
        replay.commands.len()
    );
    println!(
        "replay: {} ms, {} µs a command; fingerprint: {} ms",
        replayed.as_millis(),
        replayed.as_micros() / n as u128,
        hashed.as_millis()
    );
    println!(
        "journal: {} interactions, {} bytes; the party lived {} minutes",
        world.journal.len(),
        journal.len(),
        world.party_clock().elapsed
    );
    Ok(())
}

fn wander(args: Args) -> Result<(), String> {
    let out = args.out.as_ref().ok_or("wander needs --out <replay.ron>")?;
    let data = packs(&args)?;
    let commands = args.commands.unwrap_or(WANDER_COMMANDS);
    let walk = omnis_cli::wander::wander(&data, args.seed.unwrap_or(0), commands)?;
    write_ron(out, &walk.replay).map_err(|e| e.to_string())?;
    println!(
        "wrote {} commands to {}",
        walk.replay.commands.len(),
        out.display()
    );
    println!(
        "{} bytes of events a command; {} maps, {} fights, {} minutes lived",
        walk.event_bytes / commands.max(1),
        walk.maps,
        walk.fights,
        walk.minutes
    );
    Ok(())
}

/// The world-folder files no character save under `.omnis/` names (M8.5 7e,
/// `save::tidy_list`), listed with their sizes; with `--yes`, deleted. The window's debug page
/// does the same (`game_io::tidy`).
fn world_tidy(args: Args) -> Result<(), String> {
    if !args.positional.is_empty() {
        return Err("world tidy takes no paths: it tidies .omnis/ under this folder".to_owned());
    }
    let root = Path::new(".");
    let all = ron_io::list_under(root, ".omnis").map_err(|e| e.to_string())?;
    let files = omnis_sim::save::tidy_list(&all, &mut |p| {
        ron_io::read_save_under(root, p).map_err(|e| e.to_string())
    })?;
    let mut bytes = 0;
    for file in &files {
        let size = ron_io::size_under(root, file).map_err(|e| e.to_string())?;
        bytes += size;
        println!("{size:>10}  {file}");
    }
    if args.yes {
        for file in &files {
            ron_io::remove_under(root, file).map_err(|e| e.to_string())?;
        }
        println!("deleted {} files, {bytes} bytes", files.len());
    } else {
        println!(
            "{} files, {bytes} bytes no save names; run again with --yes to delete them",
            files.len()
        );
    }
    Ok(())
}
