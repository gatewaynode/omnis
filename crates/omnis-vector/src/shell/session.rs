//! The session resource and the command line (presentation-ARCHITECTURE.md §6, §12).

use super::hud::LOG_LINES;
use super::text::{describe, searched};
use crate::bind::Binder;
use crate::grid::cell_of;
use crate::history::History;
use crate::notice::Notice;
use crate::party;
use crate::pose::Pose;
use crate::rolllog::{self, Names};
use crate::sim::{SimWorld, Views, game_settings};
use crate::style::Role;
use bevy::prelude::Resource;
use omnis_sim::api::Draft;
use omnis_sim::omnis_data::{Data, load_packs};
use omnis_sim::ops::client_path;
use omnis_sim::{CheckKind, Command, Event, ItemPlace, PartyCommand, Settings, World};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// What the command line asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Pack roots, loaded in order.
    pub packs: Vec<PathBuf>,
    /// The world seed for a game started from the command line.
    pub seed: u64,
    /// Start in a game at once (`--seed`), not at the title.
    pub play: bool,
    /// The screen to open at the start (`--screen`), for captures and checks.
    pub screen: Option<StartScreen>,
    /// The quick save, relative to the working directory.
    pub save: String,
    /// The player's settings file, relative to the working directory.
    pub settings: String,
    /// A window instead of borderless fullscreen.
    pub windowed: bool,
    /// The display to go fullscreen on, by index (0 is the first); the current one when `None`.
    pub monitor: Option<usize>,
    /// Wait for the display's refresh before presenting a frame (off measures the frame's cost).
    pub vsync: bool,
    /// Log the frame times (`--frame-stats`), with vsync off: a frame time capped by the display
    /// says nothing.
    pub frame_stats: bool,
    /// Where the command log is written.
    pub log: PathBuf,
    /// Capture the window to this file and exit (a check of the view without a person).
    pub screenshot: Option<PathBuf>,
    /// Frames to hold forward before the capture.
    pub walk: u32,
    /// The capture's size in pixels.
    pub size: (u32, u32),
    /// Run the capture uncapped and report its frame times (`--timing`).
    pub timing: bool,
    /// The dev socket's loopback address (feature `devtools`); `None` with `--no-dev-socket`.
    pub dev_socket: Option<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            packs: vec![PathBuf::from("packs/base"), PathBuf::from("packs/test")],
            seed: 1,
            play: false,
            screen: None,
            save: ".omnis/quick.ron".to_owned(),
            settings: ".omnis/settings.ron".to_owned(),
            windowed: false,
            vsync: true,
            frame_stats: false,
            monitor: None,
            log: PathBuf::from(".omnis/vector-session.ron"),
            screenshot: None,
            walk: 0,
            size: (1600, 900),
            timing: false,
            dev_socket: Some("127.0.0.1:0".to_owned()),
        }
    }
}

/// The most lines a notice keeps: a command's newest.
const NOTICE_LINES: usize = 6;

/// A screen the command line can open at the start.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartScreen {
    /// The title (no game).
    Title,
    /// The new-game form (no game).
    NewGame,
    /// Party creation, in a new game with no party.
    CreateParty,
    /// Pause, over a game.
    Pause,
    /// The square's actions, over a game.
    Actions,
    /// The verbs of what is in reach, over a game.
    Verbs,
    /// The first member's kit beside the party column, over a game.
    Inventory,
    /// The full log, over a game.
    Log,
}

impl StartScreen {
    /// Whether the screen is shown over a running game.
    #[must_use]
    pub const fn in_game(self) -> bool {
        matches!(
            self,
            StartScreen::Pause
                | StartScreen::Actions
                | StartScreen::Verbs
                | StartScreen::Inventory
                | StartScreen::Log
                | StartScreen::CreateParty
        )
    }
}

impl Config {
    /// Whether to start in a game rather than at the title: `--seed`, a screen shown over a game,
    /// or a capture of the game view.
    #[must_use]
    pub fn starts_in_game(&self) -> bool {
        match self.screen {
            Some(screen) => screen.in_game(),
            None => self.play || self.screenshot.is_some(),
        }
    }
}

/// Parse `--pack <dir>` (repeatable; replaces the defaults), `--seed <n>` (and start in the game
/// rather than at the title), `--save <file>` (the quick save, a relative `.ron` path),
/// `--settings <file>` (the player's settings, a relative `.ron` path),
/// `--screen title|new-game|create-party|pause|actions|verbs` (open it at the start), `--windowed`,
/// `--no-vsync`, `--frame-stats` (log the frame times, vsync off), `--monitor <n>` (the display to go fullscreen on, 0 to 15),
/// `--log <path>` and `--screenshot <path>` (relative, without `..`, so a session cannot write
/// outside the working tree), `--walk <frames>` (hold forward before the screenshot), and
/// `--size WxH` (the offscreen capture's size), `--timing` (with `--screenshot`: run uncapped and
/// report the frame times), and with feature `devtools`
/// `--dev-socket <ip:port>` (a loopback address; port 0, the default, picks a free one) and
/// `--no-dev-socket`.
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
                    .map_err(|_| "--seed takes a number".to_owned())?;
                config.play = true;
            }
            "--save" => {
                let path = value("--save")?;
                client_path(&path, &["ron"])
                    .map_err(|e| format!("--save takes a relative .ron path: {e}"))?;
                config.save = path;
            }
            "--settings" => {
                let path = value("--settings")?;
                client_path(&path, &["ron"])
                    .map_err(|e| format!("--settings takes a relative .ron path: {e}"))?;
                config.settings = path;
            }
            "--screen" => {
                config.screen = Some(match value("--screen")?.as_str() {
                    "title" => StartScreen::Title,
                    "new-game" => StartScreen::NewGame,
                    "create-party" => StartScreen::CreateParty,
                    "pause" => StartScreen::Pause,
                    "actions" => StartScreen::Actions,
                    "verbs" => StartScreen::Verbs,
                    "inventory" => StartScreen::Inventory,
                    "log" => StartScreen::Log,
                    other => {
                        return Err(format!(
                            "--screen takes title, new-game, create-party, pause, actions, verbs, inventory or log, got {other}"
                        ));
                    }
                });
            }
            "--windowed" => config.windowed = true,
            "--no-vsync" => config.vsync = false,
            "--frame-stats" => {
                config.frame_stats = true;
                config.vsync = false;
            }
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
            "--timing" => config.timing = true,
            "--dev-socket" if cfg!(feature = "devtools") => {
                config.dev_socket = Some(value("--dev-socket")?);
            }
            "--no-dev-socket" if cfg!(feature = "devtools") => config.dev_socket = None,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if !packs.is_empty() {
        config.packs = packs;
    }
    if config.timing && config.screenshot.is_none() {
        return Err("--timing times a capture: it needs --screenshot".to_owned());
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

/// The loaded packs and the command line, there from the start whether or not a game runs: the
/// title starts games from them.
#[derive(Resource, Debug, Clone)]
pub struct Packs {
    /// The packs, shared with the running game.
    pub data: Arc<Data>,
    /// The command line.
    pub config: Config,
}

impl Packs {
    /// Load the packs the command line names.
    ///
    /// # Errors
    /// The loader's report for packs that fail to load.
    pub fn load(config: &Config) -> Result<Packs, String> {
        let roots: Vec<&Path> = config.packs.iter().map(PathBuf::as_path).collect();
        let data = load_packs(&roots).map_err(|report| format!("{report:?}"))?;
        Ok(Packs {
            data: Arc::new(data),
            config: config.clone(),
        })
    }
}

/// The simulation and everything the binder keeps, in one resource so a system can borrow the
/// world, the data and the binder together. It exists only while a game runs.
#[derive(Resource)]
pub struct Session {
    /// The loaded packs (a pack reload or a rule set over the dev socket replaces this game's
    /// copy only).
    pub data: Arc<Data>,
    /// The authoritative world, reached only through the engine's API.
    pub world: SimWorld,
    /// What the shell reads of the world, refreshed after every accepted command.
    pub views: Views,
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
    /// Every line said this session, the fight's own too, stamped: the full log
    /// (`screen::log`).
    pub history: History,
    /// What the last command the player started said, or its refusal, and where it was done:
    /// shown centred while exploring, fading as the party walks away (`crate::notice`).
    pub notice: Option<Notice>,
    /// The current or last fight's roll log, newest last.
    pub fight_log: Vec<String>,
    /// Who is in the fight, for the roll log.
    pub names: Names,
    /// A door moved: the 3D lines must be rebuilt.
    pub reshape: bool,
    /// What the session started from, for a restart.
    pub config: Config,
    /// Why the binder's log no longer replays this world, once something outside it changed
    /// the world or the packs (a save loaded, packs reloaded, a rule set over the dev socket).
    pub unreplayable: Option<String>,
}

impl Session {
    /// Load the packs and start a game with the command line's seed and the default settings
    /// (with `Dev` commands in devtools builds) and the fixed party: a game started without the
    /// title.
    ///
    /// # Errors
    /// A message for a pack that fails to load or a world that cannot start.
    pub fn start(config: &Config) -> Result<Session, String> {
        let packs = Packs::load(config)?;
        Session::with_party(
            &packs,
            config.seed,
            game_settings(Settings::default()),
            &party::fixed(),
        )
    }

    /// A new game from `packs` with `seed` and `settings`, its party not yet made: party
    /// creation comes first.
    ///
    /// # Errors
    /// A message for a world that cannot start.
    pub fn new_game(packs: &Packs, seed: u64, settings: Settings) -> Result<Session, String> {
        Session::with_party(packs, seed, settings, &[])
    }

    /// A new game whose party is `drafts`, each created through the binder, so the log
    /// replays them.
    ///
    /// # Errors
    /// A message for a world that cannot start or a draft the rules refuse.
    pub fn with_party(
        packs: &Packs,
        seed: u64,
        settings: Settings,
        drafts: &[Draft],
    ) -> Result<Session, String> {
        Session::begin(
            Arc::clone(&packs.data),
            packs.config.clone(),
            (seed, settings),
            drafts,
        )
    }

    fn begin(
        data: Arc<Data>,
        config: Config,
        (seed, settings): (u64, Settings),
        drafts: &[Draft],
    ) -> Result<Session, String> {
        let mut world = SimWorld::start(&data, seed, settings).map_err(|e| format!("{e:?}"))?;
        let mut binder = Binder::default();
        for draft in drafts {
            binder
                .apply(
                    &mut world,
                    &data,
                    Command::Party(PartyCommand::Create(draft.clone())),
                )
                .map_err(|e| format!("party: {e}"))?;
        }
        Ok(Session::around(data, config, world, binder, seed, settings))
    }

    /// The members this game's log created, in order: the party a restart makes again. Empty
    /// for a loaded save, whose log starts empty.
    #[must_use]
    pub fn drafts(&self) -> Vec<Draft> {
        self.binder
            .log
            .iter()
            .filter_map(|c| match c {
                Command::Party(PartyCommand::Create(draft)) => Some(draft.clone()),
                _ => None,
            })
            .collect()
    }

    /// A game from a loaded save: its log starts empty and does not replay, since a replay
    /// starts from a new world.
    #[must_use]
    pub fn loaded(packs: &Packs, world: World, why: &str) -> Session {
        let world = SimWorld::new(world);
        let here = world.here(&packs.data);
        let mut session = Session::around(
            Arc::clone(&packs.data),
            packs.config.clone(),
            world,
            Binder::default(),
            here.seed,
            here.settings,
        );
        session.unreplayable = Some(why.to_owned());
        session
    }

    fn around(
        data: Arc<Data>,
        config: Config,
        world: SimWorld,
        binder: Binder,
        seed: u64,
        settings: Settings,
    ) -> Session {
        let views = world.views(&data);
        let pose = Pose::at(views.position);
        let mut names = Names::default();
        names.observe(&views.party);
        Session {
            data,
            world,
            views,
            binder,
            pose,
            seed,
            settings,
            log_path: config.log.clone(),
            lines: Vec::new(),
            history: History::default(),
            notice: None,
            fight_log: Vec::new(),
            names,
            reshape: true,
            config,
            unreplayable: None,
        }
    }

    /// Record the session as a replay at `log_path`.
    ///
    /// # Errors
    /// The log did not reproduce, or the file could not be written.
    pub fn save_log(&self) -> Result<PathBuf, String> {
        if let Some(why) = &self.unreplayable {
            return Err(format!("the log no longer replays: {why}"));
        }
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

    /// Apply a command that is not movement (an encounter choice, a combat action, a new
    /// member) and report what happened; `true` when it was accepted. The pose follows when the
    /// command moved the party, as a retreat does.
    pub fn order(&mut self, command: Command) -> bool {
        self.ordered(command).is_ok()
    }

    /// [`Session::order`], with its outcome: the newest line it added when accepted (none when
    /// nothing worth a line happened), or the refusal's line.
    ///
    /// # Errors
    /// The refusal's line, also added to the lines, when the command was refused.
    pub fn ordered(&mut self, command: Command) -> Result<Option<String>, String> {
        let (outcome, said, role) = match self.binder.apply(&mut self.world, &self.data, command) {
            Ok(events) => {
                let lines = self.take_in(&events);
                let newest = lines.last().map(|(_, line)| line.clone());
                let said = lines
                    .into_iter()
                    .filter(|(moved, _)| !moved)
                    .map(|(_, l)| l);
                (Ok(newest), said.collect(), Role::Interface)
            }
            Err(rejection) => {
                let line = format!("Refused: {rejection}");
                self.say(line.clone());
                (Err(line.clone()), vec![line], Role::Alert)
            }
        };
        self.follow();
        // A command that only moved the party leaves the last notice to fade where it was.
        if !said.is_empty() && self.views.exploring() {
            let keep = said.len().saturating_sub(NOTICE_LINES);
            self.notice = Some(Notice::new(
                said[keep..].to_vec(),
                role,
                self.views.position,
                self.pose,
            ));
        }
        outcome
    }

    /// Snap the pose to the simulation's position when the party is no longer where the pose
    /// is, keeping the pose otherwise.
    pub fn follow(&mut self) {
        let p = self.views.position;
        let here = cell_of(self.pose.x, self.pose.z);
        if here != (i32::from(p.x), i32::from(p.y)) {
            self.pose = Pose::at(p);
        }
    }

    /// Start the same game over: a fresh world with this game's seed and settings, and the
    /// same party made again from the log (none after a loaded save: it is created anew).
    ///
    /// # Errors
    /// As [`Session::with_party`]; the old session is kept.
    pub fn restart(&mut self) -> Result<(), String> {
        let drafts = self.drafts();
        *self = Session::begin(
            Arc::clone(&self.data),
            self.config.clone(),
            (self.seed, self.settings),
            &drafts,
        )?;
        // The doors are shut again.
        self.reshape = true;
        if !drafts.is_empty() {
            self.say("A new party sets out".to_owned());
        }
        Ok(())
    }

    /// Take in a frame's events: HUD lines for the ones worth showing, the roll log (cleared
    /// when monsters are met, the last 40 lines kept), and the reshape flag. Answers the newest
    /// line added.
    pub fn note(&mut self, events: &[Event]) -> Option<String> {
        self.take_in(events).pop().map(|(_, line)| line)
    }

    /// [`Session::note`], answering every line added and whether a move said it.
    fn take_in(&mut self, events: &[Event]) -> Vec<(bool, String)> {
        self.refresh();
        self.reshape |= events.iter().any(reshapes);
        self.names.observe(&self.views.party);
        self.names.locate(self.views.position.map);
        // Each event's fight line, for the full log when the HUD has none for it.
        let mut fight = Vec::with_capacity(events.len());
        for event in events {
            if matches!(event, Event::EncounterStarted { .. }) {
                self.fight_log.clear();
            }
            let line = rolllog::describe(event, &self.names, &self.data);
            if let Some(line) = &line {
                self.fight_log.push(line.clone());
            }
            fight.push(line);
            self.names.follow(event, &self.data);
        }
        let excess = self.fight_log.len().saturating_sub(40);
        self.fight_log.drain(..excess);
        // Each line, and whether a move said it: a notice leaves out a move's. The full log
        // keeps each event once: the HUD's line, else the fight's.
        let mut lines: Vec<(bool, String)> = Vec::new();
        let age = self.views.here.age;
        for (event, fight) in events.iter().zip(fight) {
            match (describe(event, &self.names, &self.data), fight) {
                (Some(line), _) => {
                    self.history.push(age, line.clone());
                    lines.push((matches!(event, Event::Moved { .. }), line));
                }
                (None, Some(line)) => self.history.push(age, line),
                (None, None) => {}
            }
        }
        for line in searched(
            events,
            &self.views,
            &self.data,
            self.world.faced_door(&self.data),
        ) {
            self.history.push(age, line.clone());
            lines.push((false, line));
        }
        for (_, line) in &lines {
            self.show(line.clone());
        }
        lines
    }

    /// A world from outside the binder (a loaded save) takes this one's place: the views, the
    /// pose and the names follow it, the lines are redrawn, and the log stops replaying. The
    /// game is now the save's: its seed and settings, and none of this log's members, so a
    /// restart starts the save's game and has its party made anew.
    pub fn replace(&mut self, world: omnis_sim::World, why: &str) {
        self.world.replace(world);
        let here = self.world.here(&self.data);
        (self.seed, self.settings) = (here.seed, here.settings);
        self.binder.log.clear();
        self.refresh();
        self.pose = Pose::at(self.views.position);
        self.names.observe(&self.views.party);
        self.reshape = true;
        self.unreplayable = Some(why.to_owned());
        // The old world's last event is not this one's, nor its history.
        self.notice = None;
        self.history = History::default();
    }

    /// Read the views afresh. Every change to the world ends here: `note` calls it, and `note`
    /// follows every command the binder applies.
    pub fn refresh(&mut self) {
        self.views = self.world.views(&self.data);
    }

    /// Add a line to the HUD's recent events and the full log.
    pub fn say(&mut self, line: String) {
        self.history.push(self.views.here.age, line.clone());
        self.show(line);
    }

    /// Add a line to the HUD's recent events, keeping the last eight.
    fn show(&mut self, line: String) {
        self.lines.push(line);
        let excess = self.lines.len().saturating_sub(LOG_LINES);
        self.lines.drain(..excess);
    }
}

/// Whether an event changes what the view draws of the map: a door, an object's state, an
/// object taken away or found, items laid on or taken from the ground, loot left by a fight, or
/// a standing group met, bought off, hidden from or run from (M8.5 step 2c).
#[must_use]
pub fn reshapes(event: &Event) -> bool {
    match event {
        Event::Door { .. }
        | Event::ObjectState { .. }
        | Event::ObjectRemoved { .. }
        | Event::Noticed { .. }
        | Event::EncounterStarted { .. }
        | Event::Bribed { .. }
        | Event::Check {
            kind: CheckKind::Hide | CheckKind::Run,
            ..
        }
        | Event::CombatEnded { .. } => true,
        Event::ItemMoved { from, to, .. } => {
            [from, to].iter().any(|p| matches!(p, ItemPlace::Ground))
        }
        _ => false,
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
    fn frame_stats_log_the_frame_times_with_vsync_off() {
        let config = parse(args(&["--frame-stats"])).expect("valid");
        assert_eq!((config.frame_stats, config.vsync), (true, false));
        let config = parse(args(&["--windowed"])).expect("valid");
        assert_eq!((config.frame_stats, config.vsync), (false, true));
    }

    #[test]
    fn timing_needs_a_capture() {
        let config = parse(args(&["--screenshot", "x.png", "--timing"])).expect("valid");
        assert!(config.timing);
        assert!(
            !parse(args(&["--screenshot", "x.png"]))
                .expect("valid")
                .timing
        );
        assert!(parse(args(&["--timing"])).is_err());
    }

    #[test]
    fn the_title_is_the_start_unless_a_game_is_asked_for() {
        let start = |list: &[&str]| parse(args(list)).expect("valid").starts_in_game();
        assert!(!start(&[]), "the title");
        assert!(start(&["--seed", "7"]), "a seed starts a game");
        assert!(
            start(&["--screenshot", "x.png"]),
            "a capture of the game view"
        );
        assert!(!start(&["--screenshot", "x.png", "--screen", "title"]));
        assert!(!start(&["--seed", "7", "--screen", "new-game"]));
        assert!(start(&["--screen", "pause"]), "the pause needs a game");
        assert!(
            start(&["--screen", "create-party"]),
            "a new world to make a party in"
        );
        assert!(parse(args(&["--screen", "sheet"])).is_err());
    }

    #[test]
    fn the_view_is_redrawn_for_doors_objects_found_items_on_the_ground_and_loot() {
        use omnis_sim::api::{CombatOutcome, Place};
        use omnis_sim::event::Notice;
        let place = Place {
            map: "test:map:town".into(),
            level: omnis_sim::api::Level::GROUND,
            x: 0,
            y: 0,
            facing: omnis_sim::api::Facing::North,
        };
        let moved = |from, to| Event::ItemMoved {
            item: "base:item:arrows".into(),
            count: 1,
            from,
            to,
        };
        let member = ItemPlace::Member(omnis_sim::api::CharacterId(1));
        let object = |o: &str| o.to_owned();
        for yes in [
            Event::ObjectState {
                object: object("gate_1"),
                state: "open".into(),
            },
            Event::ObjectRemoved {
                object: object("stone_1"),
            },
            Event::Noticed {
                object: object("lever_1"),
                how: Notice::Passive,
            },
            moved(member.clone(), ItemPlace::Ground),
            moved(ItemPlace::Ground, member.clone()),
            Event::CombatEnded {
                outcome: CombatOutcome::Victory,
                xp: 0,
                gold: 0,
                fallen: Vec::new(),
                loot: Vec::new(),
            },
        ] {
            assert!(reshapes(&yes), "{yes:?}");
        }
        for no in [
            moved(member.clone(), ItemPlace::Stores),
            moved(member, ItemPlace::Object(object("chest_1"))),
            Event::Moved {
                from: place.clone(),
                to: place,
            },
        ] {
            assert!(!reshapes(&no), "{no:?}");
        }
    }

    #[test]
    fn the_quick_save_is_a_relative_ron_file() {
        let config = parse(args(&["--save", "saves/a.ron"])).expect("valid");
        assert_eq!(config.save, "saves/a.ron");
        for bad in ["../a.ron", "/tmp/a.ron", "saves/a.txt"] {
            assert!(parse(args(&["--save", bad])).is_err(), "{bad}");
        }
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
