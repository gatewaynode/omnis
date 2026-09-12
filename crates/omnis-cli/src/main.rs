//! `omnis-cli`: headless subcommands. Argument parsing is by hand (self-supporting rule).
#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage:
  omnis-cli tileset bake <spec.ron>...   bake viewport slot sprites and write the tileset file
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["tileset", "bake", specs @ ..] if !specs.is_empty() => {
            let mut failed = false;
            for spec in specs {
                match omnis_cli::bake::bake_spec(Path::new(spec)) {
                    Ok(report) => println!("{report}"),
                    Err(e) => {
                        eprintln!("omnis-cli: {spec}: {e}");
                        failed = true;
                    }
                }
            }
            if failed {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        _ => {
            eprint!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}
