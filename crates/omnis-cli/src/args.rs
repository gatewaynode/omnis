//! Hand-rolled argument parsing (self-supporting rule). Flags: `--pack <dir>` (repeatable),
//! `--seed <n>`, `--script <file>`, `--time`, `--commands <n>`, `--out <file>`, `--yes`;
//! everything else is positional.

use std::path::PathBuf;

/// Parsed command-line words.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Args {
    /// Words that are not flags or flag values, in order.
    pub positional: Vec<String>,
    /// `--pack` directories, in order.
    pub packs: Vec<PathBuf>,
    /// `--seed`.
    pub seed: Option<u64>,
    /// `--script`.
    pub script: Option<PathBuf>,
    /// `--time`: report how long the work took.
    pub time: bool,
    /// `--commands`.
    pub commands: Option<usize>,
    /// `--out`.
    pub out: Option<PathBuf>,
    /// `--yes`: do what was only listed (`world tidy` deletes).
    pub yes: bool,
}

impl Args {
    /// Parse words; an unknown `--flag` or a flag without its value is an error.
    pub fn parse(words: &[&str]) -> Result<Args, String> {
        let mut args = Args::default();
        let mut it = words.iter();
        while let Some(word) = it.next() {
            match *word {
                "--pack" => args
                    .packs
                    .push(PathBuf::from(it.next().ok_or("--pack needs a directory")?)),
                "--seed" => {
                    let value = it.next().ok_or("--seed needs a number")?;
                    args.seed = Some(value.parse().map_err(|_| format!("bad seed '{value}'"))?);
                }
                "--script" => {
                    args.script = Some(PathBuf::from(it.next().ok_or("--script needs a file")?));
                }
                "--time" => args.time = true,
                "--yes" => args.yes = true,
                "--commands" => {
                    let value = it.next().ok_or("--commands needs a number")?;
                    args.commands =
                        Some(value.parse().map_err(|_| format!("bad count '{value}'"))?);
                }
                "--out" => args.out = Some(PathBuf::from(it.next().ok_or("--out needs a file")?)),
                flag if flag.starts_with("--") => return Err(format!("unknown flag '{flag}'")),
                other => args.positional.push(other.to_owned()),
            }
        }
        Ok(args)
    }

    /// The packs given, or `packs/base` then `packs/test` when none were.
    #[must_use]
    pub fn packs_or_default(&self) -> Vec<PathBuf> {
        if self.packs.is_empty() {
            vec![PathBuf::from("packs/base"), PathBuf::from("packs/test")]
        } else {
            self.packs.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_and_positionals_separate() {
        let args = Args::parse(&["map", "text", "--pack", "p", "--seed", "7", "x:map:y"]).unwrap();
        assert_eq!(args.positional, ["map", "text", "x:map:y"]);
        assert_eq!(args.packs, [PathBuf::from("p")]);
        assert_eq!(args.seed, Some(7));
        assert_eq!(
            Args::parse(&[]).unwrap().packs_or_default(),
            [PathBuf::from("packs/base"), PathBuf::from("packs/test")]
        );
        assert!(Args::parse(&["--seed"]).is_err());
        assert!(Args::parse(&["--seed", "x"]).is_err());
        assert!(Args::parse(&["--what"]).is_err());
    }

    #[test]
    fn measuring_flags() {
        let args =
            Args::parse(&["w.ron", "--time", "--commands", "500", "--out", "o.ron"]).unwrap();
        assert_eq!(args.positional, ["w.ron"]);
        assert!(args.time);
        assert_eq!(args.commands, Some(500));
        assert_eq!(args.out, Some(PathBuf::from("o.ron")));
        assert!(!Args::parse(&[]).unwrap().time);
        assert!(Args::parse(&["--commands", "many"]).is_err());
        assert!(Args::parse(&["--out"]).is_err());
    }
}
