//! `MenusPlugin`: the title, new game, character creation, and pause screens as lists of text
//! driven by the state machines in `menu.rs`. A screen is spawned on state entry and despawned
//! on exit; keys arrive as logical `KeyboardInput` so names and seeds can be typed.

use crate::AppConfig;
use crate::menu::{
    Catalog, CreationAction, CreationForm, MenuKey, NewGameAction, NewGameForm, Pause, PauseAction,
    Title, TitleAction,
};
use crate::sim::{
    AppState, CommandRefused, MenuState, Notice, PackData, PlayState, PlayerCommand, SimEvent,
    SimSet, SimWorld, StartIn, load,
};
use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use omnis_sim::{Command, Event, PartyCommand, World};

/// Every screen's state, kept so a screen reopens where it was.
#[derive(Resource, Default, Debug)]
pub struct Screens {
    /// The title.
    pub title: Title,
    /// The new game form.
    pub new_game: NewGameForm,
    /// The creation form.
    pub creation: CreationForm,
    /// What creation may choose from.
    pub catalog: Catalog,
    /// The pause overlay.
    pub pause: Pause,
}

/// One line of the active screen.
#[derive(Component)]
struct MenuLine(usize);

/// Which screen is up, if any.
#[derive(SystemParam)]
struct Where<'w> {
    app: Res<'w, State<AppState>>,
    menu: Option<Res<'w, State<MenuState>>>,
    play: Option<Res<'w, State<PlayState>>>,
}

/// The screen as a triple, for matching.
enum Screen {
    Title,
    NewGame,
    CreateParty,
    Paused,
    None,
}

impl Where<'_> {
    fn screen(&self) -> Screen {
        match (
            self.app.get(),
            self.menu.as_deref().map(State::get),
            self.play.as_deref().map(State::get),
        ) {
            (AppState::MainMenu, Some(MenuState::Title), _) => Screen::Title,
            (AppState::MainMenu, Some(MenuState::NewGame), _) => Screen::NewGame,
            (AppState::Playing, _, Some(PlayState::CreateParty)) => Screen::CreateParty,
            (AppState::Playing, _, Some(PlayState::Paused)) => Screen::Paused,
            _ => Screen::None,
        }
    }
}

/// Lines a screen may show; the rest stay blank.
const MAX_LINES: usize = 24;

/// The menus plugin.
pub struct MenusPlugin;

impl Plugin for MenusPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<KeyboardInput>()
            .init_resource::<Screens>()
            .add_systems(OnEnter(MenuState::Title), |c: Commands| {
                spawn_screen(c, DespawnOnExit(MenuState::Title), 32.0);
            })
            .add_systems(OnEnter(MenuState::NewGame), |c: Commands| {
                spawn_screen(c, DespawnOnExit(MenuState::NewGame), 24.0);
            })
            .add_systems(OnEnter(PlayState::CreateParty), open_creation)
            .add_systems(OnEnter(PlayState::Paused), |c: Commands| {
                spawn_screen(c, DespawnOnExit(PlayState::Paused), 24.0);
            })
            .add_systems(Update, menu_keys.in_set(SimSet::Collect))
            .add_systems(Update, refresh.in_set(SimSet::Publish));
    }
}

fn spawn_screen<S: States>(mut commands: Commands, scope: DespawnOnExit<S>, font: f32) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(6.0),
                top: Val::Percent(8.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
            scope,
        ))
        .with_children(|parent| {
            for i in 0..MAX_LINES {
                parent.spawn((
                    Text::new(""),
                    TextFont::from_font_size(font),
                    TextColor(Color::WHITE),
                    MenuLine(i),
                ));
            }
        });
}

fn open_creation(commands: Commands, data: Res<PackData>, mut screens: ResMut<Screens>) {
    screens.catalog = Catalog::from_data(&data.0);
    screens.creation = CreationForm::new(&screens.catalog);
    spawn_screen(commands, DespawnOnExit(PlayState::CreateParty), 20.0);
}

/// A logical key press as the menus see it.
fn menu_key(input: &KeyboardInput) -> Option<MenuKey> {
    if input.state != ButtonState::Pressed {
        return None;
    }
    Some(match &input.logical_key {
        Key::ArrowUp => MenuKey::Up,
        Key::ArrowDown => MenuKey::Down,
        Key::ArrowLeft => MenuKey::Left,
        Key::ArrowRight => MenuKey::Right,
        Key::Enter => MenuKey::Enter,
        Key::Escape => MenuKey::Escape,
        Key::Backspace => MenuKey::Backspace,
        Key::Space => MenuKey::Char(' '),
        Key::Character(text) => {
            let c = text.chars().next()?;
            if c.is_control() {
                return None;
            }
            MenuKey::Char(c)
        }
        _ => return None,
    })
}

/// State transitions the menus request.
#[derive(SystemParam)]
struct Next<'w> {
    app: ResMut<'w, NextState<AppState>>,
    menu: ResMut<'w, NextState<MenuState>>,
    play: ResMut<'w, NextState<PlayState>>,
}

#[allow(clippy::too_many_arguments)]
fn menu_keys(
    mut keys: MessageReader<KeyboardInput>,
    mut commands: Commands,
    at: Where,
    mut next: Next,
    mut screens: ResMut<Screens>,
    config: Res<AppConfig>,
    data: Option<Res<PackData>>,
    world: Option<Res<SimWorld>>,
    mut player: MessageWriter<PlayerCommand>,
    mut exit: MessageWriter<AppExit>,
    mut notice: ResMut<Notice>,
) {
    let members = world.as_ref().map_or(0, |w| w.0.party.members.len());
    for key in keys.read().filter_map(menu_key) {
        match at.screen() {
            Screen::Title => match screens.title.key(key) {
                Some(TitleAction::NewGame) => next.menu.set(MenuState::NewGame),
                Some(TitleAction::Load) => {
                    let Some(data) = data.as_ref() else { continue };
                    match load(&data.0, &config.save_path, false) {
                        Ok(world) => {
                            commands.insert_resource(SimWorld(world));
                            commands.insert_resource(StartIn(PlayState::Explore));
                            next.app.set(AppState::Playing);
                        }
                        Err(e) => notice.0 = format!("Load failed: {e}"),
                    }
                }
                Some(TitleAction::Quit) => {
                    exit.write(AppExit::Success);
                }
                None => {}
            },
            Screen::NewGame => match screens.new_game.key(key) {
                Some(NewGameAction::Start) => {
                    let Some(data) = data.as_ref() else { continue };
                    let form = &screens.new_game;
                    match World::new(&data.0, form.seed(crate::entropy_seed()), form.settings) {
                        Ok(world) => {
                            info!("new game, seed {:#x}, {:?}", world.seed, world.settings);
                            commands.insert_resource(SimWorld(world));
                            commands.insert_resource(StartIn(PlayState::CreateParty));
                            next.app.set(AppState::Playing);
                        }
                        Err(e) => notice.0 = format!("{e}"),
                    }
                }
                Some(NewGameAction::Back) => next.menu.set(MenuState::Title),
                None => {}
            },
            Screen::CreateParty => {
                let Screens {
                    creation, catalog, ..
                } = &mut *screens;
                match creation.key(key, catalog, members) {
                    Some(CreationAction::Add(draft)) => {
                        player.write(PlayerCommand(Command::Party(PartyCommand::Create(draft))));
                    }
                    Some(CreationAction::Begin) => next.play.set(PlayState::Explore),
                    Some(CreationAction::Back) => {
                        commands.remove_resource::<SimWorld>();
                        next.app.set(AppState::MainMenu);
                    }
                    None => {}
                }
            }
            Screen::Paused => match screens.pause.key(key) {
                Some(PauseAction::Resume) => next.play.set(PlayState::Explore),
                Some(PauseAction::QuitToTitle) => {
                    commands.remove_resource::<SimWorld>();
                    next.app.set(AppState::MainMenu);
                }
                Some(PauseAction::Quit) => {
                    exit.write(AppExit::Success);
                }
                None => {}
            },
            Screen::None => {}
        }
    }
}

fn refresh(
    mut events: MessageReader<SimEvent>,
    mut refused: MessageReader<CommandRefused>,
    at: Where,
    mut screens: ResMut<Screens>,
    world: Option<Res<SimWorld>>,
    data: Option<Res<PackData>>,
    mut lines: Query<(&mut Text, &MenuLine)>,
) {
    if events.read().any(|e| e.0 == Event::PartyChanged) {
        let Screens {
            creation, catalog, ..
        } = &mut *screens;
        creation.next_member(catalog);
    }
    for CommandRefused(rejection) in refused.read() {
        screens.creation.message = rejection.to_string();
    }
    let text = match at.screen() {
        Screen::Title => screens.title.lines(),
        Screen::NewGame => screens.new_game.lines(),
        Screen::CreateParty => {
            let names: Vec<String> = world
                .as_ref()
                .map(|w| {
                    w.0.party
                        .members
                        .iter()
                        .map(|m| {
                            let class = data
                                .as_ref()
                                .and_then(|d| d.0.registry.classes.name(m.class))
                                .and_then(|id| id.rsplit(':').next())
                                .unwrap_or("?");
                            format!("{} ({class})", m.name)
                        })
                        .collect()
                })
                .unwrap_or_default();
            screens.creation.lines(&screens.catalog, &names)
        }
        Screen::Paused => {
            let (settings, seed) = world
                .as_ref()
                .map_or((omnis_sim::Settings::default(), 0), |w| {
                    (w.0.settings, w.0.seed)
                });
            screens.pause.lines(settings, seed)
        }
        Screen::None => return,
    };
    for (mut line, MenuLine(i)) in &mut lines {
        let wanted = text.get(*i).map_or("", String::as_str);
        if line.0 != wanted {
            line.0 = wanted.to_owned();
        }
    }
}
