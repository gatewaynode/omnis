//! The session resource and the command line (alt-ARCHITECTURE.md §6, §12).

use super::text::describe;
use crate::bind::Binder;
use crate::grid::cell_of;
use crate::party;
use crate::pose::Pose;
use bevy::prelude::Resource;
use omnis_sim::omnis_data::{Data, load_packs};
use omnis_sim::{Command, Event, PartyCommand, Settings, World};
use std::path::{Component, Path, PathBuf};

/// What the command line asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Pack roots, loaded in order.
    pub packs: Vec<PathBuf>,
    /// The world seed.
    pub seed: u64,
    /// A window instead of borderless fullscreen.
    pub windowed: bool,
    /// The display to go fullscreen on, by index (0 is the first); the current one when `None`.
    pub monitor: Option<usize>,
    /// Wait for the display's refresh before presenting a frame (off measures the frame's cost).
    pub vsync: bool,
    /// Where the command log is written.
    pub log: PathBuf,
    /// Capture the window to this file and exit (a check of the view without a person).
    pub screenshot: Option<PathBuf>,
    /// Frames to hold forward before the capture.
    pub walk: u32,
    /// The capture's size in pixels.
    pub size: (u32, u32),
}

impl Default for Config {
    fn default() -> Config {
        Config {
            packs: vec![PathBuf::from("packs/base"), PathBuf::from("packs/test")],
            seed: 1,
            windowed: false,
            vsync: true,
            monitor: None,
            log: PathBuf::from(".omnis/vector-session.ron"),
            screenshot: None,
            walk: 0,
            size: (1600, 900),
        }
    }
}

/// Parse `--pack <dir>` (repeatable; replaces the defaults), `--seed <n>`, `--windowed`,
/// `--no-vsync`, `--monitor <n>` (the display to go fullscreen on, 0 to 15),
/// `--log <path>` and `--screenshot <path>` (relative, without `..`, so a session cannot write
/// outside the working tree), `--walk <frames>` (hold forward before the screenshot), and
/// `--size WxH` (the offscreen capture's size).
///
/// # Errors
/// A message naming the bad argument.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Config, String> {
    let mut config = Config::default();
    let mut packs = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--pack" => packs.push(PathBuf::from(value("--pack")?)),
            "--seed" => {
                config.seed = value("--seed")?
                    .parse()
                    .map_err(|_| "--seed takes a number".to_owned())?
            }
            "--windowed" => config.windowed = true,
            "--no-vsync" => config.vsync = false,
            "--monitor" => {
                let n: usize = value("--monitor")?
                    .parse()
                    .map_err(|_| "--monitor takes a display number".to_owned())?;
                if n > 15 {
                    return Err("--monitor takes a display number from 0 to 15".to_owned());
                }
                config.monitor = Some(n);
            }
            "--log" => {
                let path = PathBuf::from(value("--log")?);
                if !safe_relative(&path) {
                    return Err("--log takes a relative path without '..'".to_owned());
                }
                config.log = path;
            }
            "--screenshot" => {
                let path = PathBuf::from(value("--screenshot")?);
                if !safe_relative(&path) {
                    return Err("--screenshot takes a relative path without '..'".to_owned());
                }
                config.screenshot = Some(path);
            }
            "--size" => config.size = size(&value("--size")?)?,
            "--walk" => {
                config.walk = value("--walk")?
                    .parse()
                    .map_err(|_| "--walk takes a frame count".to_owned())?
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if !packs.is_empty() {
        config.packs = packs;
    }
    Ok(config)
}

/// `WIDTHxHEIGHT`, each 16..=8192.
fn size(text: &str) -> Result<(u32, u32), String> {
    let bad = || format!("--size takes WIDTHxHEIGHT, got {text}");
    let (w, h) = text.split_once('x').ok_or_else(bad)?;
    let (w, h): (u32, u32) = (w.parse().map_err(|_| bad())?, h.parse().map_err(|_| bad())?);
    if (16..=8192).contains(&w) && (16..=8192).contains(&h) {
        Ok((w, h))
    } else {
        Err(bad())
    }
}

fn safe_relative(path: &Path) -> bool {
    path.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        && path.components().next().is_some()
}

/// The simulation and everything the binder keeps, in one resource so a system can borrow the
/// world, the data and the binder together.
#[derive(Resource)]
pub struct Session {
    /// The loaded packs.
    pub data: Data,
    /// The authoritative world.
    pub world: World,
    /// Pose-to-command translation and the accepted-command log.
    pub binder: Binder,
    /// The camera's pose.
    pub pose: Pose,
    /// The seed and settings the world started with, for the replay.
    pub seed: u64,
    /// See `seed`.
    pub settings: Settings,
    /// Where the log is written.
    pub log_path: PathBuf,
    /// Recent event lines for the HUD, newest last.
    pub lines: Vec<String>,
    /// A door moved: the 3D lines must be rebuilt.
    pub reshape: bool,
    /// What the session started from, for a restart.
    pub config: Config,
}

impl Session {
    /// Load the packs, start a world, and create the fixed party through the binder.
    ///
    /// # Errors
    /// A message for a pack that fails to load or a world that cannot start.
    pub fn start(config: &Config) -> Result<Session, String> {
        let roots: Vec<&Path> = config.packs.iter().map(PathBuf::as_path).collect();
        let data = load_packs(&roots).map_err(|report| format!("{report:?}"))?;
        let settings = Settings::default();
        let mut world = World::new(&data, config.seed, settings).map_err(|e| format!("{e:?}"))?;
        let mut binder = Binder::default();
        for draft in party::fixed() {
            binder
                .apply(
                    &mut world,
                    &data,
                    Command::Party(PartyCommand::Create(draft)),
                )
                .map_err(|e| format!("party: {e:?}"))?;
        }
        let pose = Pose::at(world.position);
        Ok(Session {
            data,
            world,
            binder,
            pose,
            seed: config.seed,
            settings,
            log_path: config.log.clone(),
            lines: Vec::new(),
            reshape: false,
            config: config.clone(),
        })
    }

    /// Record the session as a replay at `log_path`.
    ///
    /// # Errors
    /// The log did not reproduce, or the file could not be written.
    pub fn save_log(&self) -> Result<PathBuf, String> {
        let replay = self
            .binder
            .replay(&self.data, self.seed, self.settings)
            .map_err(|e| e.to_string())?;
        if let Some(dir) = self.log_path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        omnis_sim::omnis_data::ron_io::write_ron(&self.log_path, &replay)
            .map_err(|e| e.to_string())?;
        Ok(self.log_path.clone())
    }

    /// Apply a command that is not movement (an encounter choice, a combat action) and report
    /// what happened. The pose follows when the command moved the party, as a retreat does.
    pub fn order(&mut self, command: Command) {
        match self.binder.apply(&mut self.world, &self.data, command) {
            Ok(events) => self.note(&events),
            Err(rejection) => self.say(format!("Refused: {rejection}")),
        }
        self.follow();
    }

    /// Snap the pose to the simulation's position when the party is no longer where the pose
    /// is, keeping the pose otherwise.
    pub fn follow(&mut self) {
        let p = self.world.position;
        let here = cell_of(self.pose.x, self.pose.z);
        if here != (i32::from(p.x), i32::from(p.y)) {
            self.pose = Pose::at(p);
        }
    }

    /// Start over with a fresh world and party from the same command line.
    ///
    /// # Errors
    /// As [`Session::start`]; the old session is kept.
    pub fn restart(&mut self) -> Result<(), String> {
        *self = Session::start(&self.config)?;
        // The doors are shut again.
        self.reshape = true;
        self.say("A new party sets out".to_owned());
        Ok(())
    }

    /// Take in a frame's events: HUD lines for the ones worth showing, and the reshape flag.
    pub fn note(&mut self, events: &[Event]) {
        self.reshape |= events.iter().any(|e| matches!(e, Event::Door { .. }));
        for line in events.iter().filter_map(describe) {
            self.say(line);
        }
    }

    /// Add a line to the HUD's recent events, keeping the last eight.
    pub fn say(&mut self, line: String) {
        self.lines.push(line);
        let excess = self.lines.len().saturating_sub(8);
        self.lines.drain(..excess);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn defaults_load_both_packs() {
        let config = parse(Vec::new()).expect("no arguments");
        assert_eq!(config, Config::default());
    }

    #[test]
    fn flags_parse_and_packs_replace_the_defaults() {
        let config = parse(args(&[
            "--pack",
            "a",
            "--pack",
            "b",
            "--seed",
            "9",
            "--windowed",
            "--no-vsync",
            "--monitor",
            "1",
        ]))
        .expect("valid");
        assert_eq!(config.packs, vec![PathBuf::from("a"), PathBuf::from("b")]);
        assert_eq!(
            (config.seed, config.windowed, config.vsync),
            (9, true, false)
        );
        assert_eq!(config.monitor, Some(1));
        assert!(parse(args(&["--monitor", "x"])).is_err());
        assert!(parse(args(&["--monitor", "99"])).is_err());
    }

    #[test]
    fn the_log_path_stays_inside_the_working_tree() {
        assert!(parse(args(&["--log", "out/session.ron"])).is_ok());
        assert!(parse(args(&["--log", "../escape.ron"])).is_err());
        assert!(parse(args(&["--log", "/etc/passwd"])).is_err());
        assert!(parse(args(&["--log", "a/../../b"])).is_err());
        assert!(parse(args(&["--bogus"])).is_err());
    }
}
