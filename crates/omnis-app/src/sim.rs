//! `SimPlugin`: owns the `World`, applies commands in one ordered set, publishes events as
//! messages (ARCHITECTURE.md §8.1). Runs headless.

use crate::AppConfig;
use bevy::prelude::*;
use omnis_sim::omnis_data::{Data, load_packs};
use omnis_sim::{Command, Event, World, apply};
use std::path::{Path, PathBuf};

/// Top-level app state. Menus arrive with M3.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Loading packs and creating the world.
    #[default]
    Boot,
    /// A game is running.
    Playing,
}

/// What the player is doing while playing.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Playing)]
pub enum PlayState {
    /// Walking the map.
    #[default]
    Explore,
}

/// The loaded packs.
#[derive(Resource)]
pub struct PackData(pub Data);

/// The game state. Only `SimPlugin` systems mutate it.
#[derive(Resource)]
pub struct SimWorld(pub World);

/// A player action for the simulation.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCommand(pub Command);

/// Something outside the simulation: saving, loading, overlays, quitting.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellCommand {
    /// Write the quick save.
    Save,
    /// Read the quick save.
    Load,
    /// Show or hide the automap.
    ToggleAutomap,
    /// Exit the application.
    Quit,
}

/// One simulation event, re-published one to one.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct SimEvent(pub Event);

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
            .add_sub_state::<PlayState>()
            .add_message::<PlayerCommand>()
            .add_message::<ShellCommand>()
            .add_message::<SimEvent>()
            .add_message::<WorldReplaced>()
            .init_resource::<Notice>()
            .configure_sets(
                Update,
                (SimSet::Collect, SimSet::Apply, SimSet::Publish).chain(),
            )
            .add_systems(Startup, boot)
            .add_systems(
                Update,
                (apply_commands, shell)
                    .chain()
                    .in_set(SimSet::Apply)
                    .run_if(in_state(PlayState::Explore)),
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
    match World::new(&data, config.seed) {
        Ok(world) => {
            info!(
                "new game on {} pack(s), seed {:#x}",
                data.packs.len(),
                config.seed
            );
            commands.insert_resource(SimWorld(world));
            commands.insert_resource(PackData(data));
            next.set(AppState::Playing);
        }
        Err(e) => {
            error!("{e}");
            exit.write(AppExit::error());
        }
    }
}

fn apply_commands(
    mut incoming: MessageReader<PlayerCommand>,
    mut world: ResMut<SimWorld>,
    data: Res<PackData>,
    mut events: MessageWriter<SimEvent>,
) {
    for PlayerCommand(command) in incoming.read() {
        match apply(&mut world.0, &data.0, *command) {
            Ok(produced) => {
                for event in produced {
                    events.write(SimEvent(event));
                }
            }
            Err(rejection) => info!("{command:?} refused: {rejection}"),
        }
    }
}

fn shell(
    mut incoming: MessageReader<ShellCommand>,
    mut world: ResMut<SimWorld>,
    data: Res<PackData>,
    config: Res<AppConfig>,
    mut notice: ResMut<Notice>,
    mut replaced: MessageWriter<WorldReplaced>,
    mut exit: MessageWriter<AppExit>,
) {
    for command in incoming.read() {
        match command {
            ShellCommand::Save => match save(&world.0, &config.save_path) {
                Ok(()) => notice.0 = format!("Saved to {}", config.save_path.display()),
                Err(e) => notice.0 = format!("Save failed: {e}"),
            },
            ShellCommand::Load => match load(&data.0, &config.save_path, false) {
                Ok(loaded) => {
                    world.0 = loaded;
                    replaced.write(WorldReplaced);
                    notice.0 = format!("Loaded {}", config.save_path.display());
                }
                Err(e) => notice.0 = format!("Load failed: {e}"),
            },
            ShellCommand::Quit => {
                exit.write(AppExit::Success);
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
