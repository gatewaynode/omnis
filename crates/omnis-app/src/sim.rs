//! `SimPlugin`: owns the `World`, applies commands in one ordered set, publishes events as
//! messages (ARCHITECTURE.md §8.1). Runs headless.

use crate::AppConfig;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use omnis_sim::omnis_data::{Data, load_packs};
use omnis_sim::{Command, Event, Mode, Rejection, Settings, World, apply};
use std::path::{Path, PathBuf};

/// Top-level app state (ARCHITECTURE.md §8.1).
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Loading packs.
    #[default]
    Boot,
    /// The title and new game screens; no world exists.
    MainMenu,
    /// A game is running.
    Playing,
}

/// Which menu screen is up.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::MainMenu)]
pub enum MenuState {
    /// New game, load, quit.
    #[default]
    Title,
    /// Seed and difficulty.
    NewGame,
}

/// What the player is doing while playing.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Playing)]
pub enum PlayState {
    /// Building the party before the first step.
    #[default]
    CreateParty,
    /// Walking the map.
    Explore,
    /// Monsters ahead: attack, bribe, hide, or run.
    Encounter,
    /// The fight.
    Combat,
    /// The pause overlay.
    Paused,
    /// Every member is down: load or quit.
    Defeat,
    /// The debug menu over the world; commands still apply (feature `devtools`).
    Debug,
    /// The cast menu while exploring.
    Cast,
}

/// The settings a new game starts with: the player's choices, and `devtools` when this build
/// carries the dev socket, so the debug menu and `Dev` commands work in a dev build and never
/// in a release.
#[must_use]
pub fn game_settings(base: Settings) -> Settings {
    Settings {
        devtools: cfg!(feature = "devtools"),
        ..base
    }
}

impl PlayState {
    /// The play state the world's mode calls for.
    #[must_use]
    pub const fn for_mode(mode: &Mode) -> PlayState {
        match mode {
            Mode::Explore => PlayState::Explore,
            Mode::Encounter(_) => PlayState::Encounter,
            Mode::Combat(_) => PlayState::Combat,
        }
    }
}

/// Whether a game is running and not paused: commands apply in every other play state.
#[must_use]
pub fn unpaused(state: Option<Res<State<PlayState>>>) -> bool {
    state.is_some_and(|s| *s.get() != PlayState::Paused)
}

/// Where a newly started game begins; read once on entering `Playing`.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartIn(pub PlayState);

/// The loaded packs.
#[derive(Resource)]
pub struct PackData(pub Data);

/// The game state. Only `SimPlugin` systems mutate it.
#[derive(Resource)]
pub struct SimWorld(pub World);

/// A player action for the simulation.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct PlayerCommand(pub Command);

/// Something outside the simulation: saving, loading, overlays, pausing, quitting.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellCommand {
    /// Write the quick save.
    Save,
    /// Read the quick save.
    Load,
    /// Show or hide the automap.
    ToggleAutomap,
    /// Open the pause overlay.
    Pause,
    /// Open the cast menu while exploring.
    Cast,
    /// Exit the application.
    Quit,
}

/// One simulation event, re-published one to one.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct SimEvent(pub Event);

/// A command the rules refused, for the screen that sent it.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct CommandRefused(pub Rejection);

/// The whole world changed (a load); presentation redraws everything.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldReplaced;

/// A line for the player that is not a simulation event: saved, loaded, errors.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct Notice(pub String);

/// The ordered sets of the frame: collect commands, apply them, publish for presentation.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSet {
    /// Input and shells turn into messages.
    Collect,
    /// Commands hit the world.
    Apply,
    /// Presentation reads the results.
    Publish,
}

/// The simulation plugin.
pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_sub_state::<MenuState>()
            .add_sub_state::<PlayState>()
            .add_message::<PlayerCommand>()
            .add_message::<ShellCommand>()
            .add_message::<SimEvent>()
            .add_message::<CommandRefused>()
            .add_message::<WorldReplaced>()
            .init_resource::<Notice>()
            .configure_sets(
                Update,
                (SimSet::Collect, SimSet::Apply, SimSet::Publish).chain(),
            )
            .add_systems(Startup, boot)
            .add_systems(OnEnter(AppState::Playing), start_in)
            .add_systems(
                Update,
                (apply_commands, shell)
                    .chain()
                    .in_set(SimSet::Apply)
                    // The world may leave mid-frame (quit to title): skip until the state follows.
                    .run_if(unpaused.and_then(resource_exists::<SimWorld>)),
            );
    }
}

/// Load the packs and start a new game, or report why not and exit.
fn boot(
    mut commands: Commands,
    config: Res<AppConfig>,
    mut next: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    let roots: Vec<&Path> = config.packs.iter().map(PathBuf::as_path).collect();
    let data = match load_packs(&roots) {
        Ok(data) => data,
        Err(report) => {
            error!("packs refused:\n{report}");
            exit.write(AppExit::error());
            return;
        }
    };
    if !config.autostart {
        info!("{} pack(s) loaded; main menu", data.packs.len());
        commands.insert_resource(PackData(data));
        next.set(AppState::MainMenu);
        return;
    }
    match World::new(&data, config.seed, game_settings(Settings::default())) {
        Ok(world) => {
            info!(
                "new game on {} pack(s), seed {:#x}",
                data.packs.len(),
                config.seed
            );
            commands.insert_resource(SimWorld(world));
            commands.insert_resource(PackData(data));
            commands.insert_resource(StartIn(PlayState::Explore));
            next.set(AppState::Playing);
        }
        Err(e) => {
            error!("{e}");
            exit.write(AppExit::error());
        }
    }
}

/// Enter the play state the starter asked for; the sub-state's default is party creation.
fn start_in(
    mut commands: Commands,
    start: Option<Res<StartIn>>,
    mut next: ResMut<NextState<PlayState>>,
) {
    if let Some(start) = start {
        next.set(start.0);
        commands.remove_resource::<StartIn>();
    }
}

fn apply_commands(
    mut incoming: MessageReader<PlayerCommand>,
    mut world: ResMut<SimWorld>,
    data: Res<PackData>,
    mut events: MessageWriter<SimEvent>,
    mut refused: MessageWriter<CommandRefused>,
) {
    for PlayerCommand(command) in incoming.read() {
        match apply(&mut world.0, &data.0, command.clone()) {
            Ok(produced) => {
                for event in produced {
                    events.write(SimEvent(event));
                }
            }
            Err(rejection) => {
                info!("{command:?} refused: {rejection}");
                refused.write(CommandRefused(rejection));
            }
        }
    }
}

/// What the shell writes: the notice line, the world-replaced flag, exit, and the pause.
#[derive(SystemParam)]
struct ShellOut<'w> {
    notice: ResMut<'w, Notice>,
    replaced: MessageWriter<'w, WorldReplaced>,
    exit: MessageWriter<'w, AppExit>,
    next_play: ResMut<'w, NextState<PlayState>>,
}

fn shell(
    mut incoming: MessageReader<ShellCommand>,
    mut world: ResMut<SimWorld>,
    data: Res<PackData>,
    config: Res<AppConfig>,
    mut out: ShellOut,
) {
    for command in incoming.read() {
        match command {
            ShellCommand::Save if !world.0.may_save() => {
                out.notice.0 = "The save rule forbids saving here".to_owned();
            }
            ShellCommand::Save => match save(&world.0, &config.save_path) {
                Ok(()) => out.notice.0 = format!("Saved to {}", config.save_path.display()),
                Err(e) => out.notice.0 = format!("Save failed: {e}"),
            },
            ShellCommand::Load => match load(&data.0, &config.save_path, false) {
                Ok(loaded) => {
                    world.0 = loaded;
                    out.replaced.write(WorldReplaced);
                    out.notice.0 = format!("Loaded {}", config.save_path.display());
                }
                Err(e) => out.notice.0 = format!("Load failed: {e}"),
            },
            ShellCommand::Pause => out.next_play.set(PlayState::Paused),
            ShellCommand::Cast => out.next_play.set(PlayState::Cast),
            ShellCommand::Quit => {
                out.exit.write(AppExit::Success);
            }
            ShellCommand::ToggleAutomap => {}
        }
    }
}

/// Write the world as RON to `path`, creating the directory.
pub fn save(world: &World, path: &Path) -> Result<(), String> {
    let text = world.to_ron().map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, text).map_err(|e| e.to_string())
}

/// Read a world from `path`, checked against the loaded packs unless `force`.
pub fn load(data: &Data, path: &Path, force: bool) -> Result<World, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    World::from_ron(&text, data, force).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_play_state_follows_the_mode_and_only_the_pause_stops_commands() {
        assert_eq!(PlayState::for_mode(&Mode::Explore), PlayState::Explore);
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>()
            .add_sub_state::<PlayState>();
        assert!(!app.world_mut().run_system_cached(unpaused).unwrap());
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        app.update();
        assert!(app.world_mut().run_system_cached(unpaused).unwrap());
        app.world_mut()
            .resource_mut::<NextState<PlayState>>()
            .set(PlayState::Paused);
        app.update();
        assert!(!app.world_mut().run_system_cached(unpaused).unwrap());
        app.world_mut()
            .resource_mut::<NextState<PlayState>>()
            .set(PlayState::Defeat);
        app.update();
        assert!(app.world_mut().run_system_cached(unpaused).unwrap());
    }
}
