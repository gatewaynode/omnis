//! `MenusPlugin`: the title, new game, character creation, and pause screens as lists of text
//! driven by the state machines in `menu.rs`. A screen is spawned on state entry and despawned
//! on exit; keys arrive as logical `KeyboardInput` so names and seeds can be typed, and clicks
//! on the composed frame's widgets arrive as `UiClick`s that become the same keys.

use crate::AppConfig;
use crate::cursor::UiSet;
use crate::menu::{
    Catalog, CreationAction, CreationForm, MenuKey, NewGameAction, NewGameForm, Pause, PauseAction,
    Title, TitleAction,
};
use crate::screen::{self, Hit, Target};
use crate::sim::{
    AppState, CommandRefused, MenuState, Notice, PackData, PlayState, PlayerCommand, SimEvent,
    SimWorld, StartIn, WorldReplaced, load,
};
use crate::ui::UiClick;
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
pub struct Where<'w> {
    app: Res<'w, State<AppState>>,
    menu: Option<Res<'w, State<MenuState>>>,
    play: Option<Res<'w, State<PlayState>>>,
}

/// The active screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Active {
    /// The title.
    Title,
    /// The new game form.
    NewGame,
    /// Character creation.
    CreateParty,
    /// The pause overlay.
    Paused,
    /// No screen: booting or exploring.
    None,
}

impl Where<'_> {
    /// The active screen.
    #[must_use]
    pub fn screen(&self) -> Active {
        match (
            self.app.get(),
            self.menu.as_deref().map(State::get),
            self.play.as_deref().map(State::get),
        ) {
            (AppState::MainMenu, Some(MenuState::Title), _) => Active::Title,
            (AppState::MainMenu, Some(MenuState::NewGame), _) => Active::NewGame,
            (AppState::Playing, _, Some(PlayState::CreateParty)) => Active::CreateParty,
            (AppState::Playing, _, Some(PlayState::Paused)) => Active::Paused,
            _ => Active::None,
        }
    }

    /// Whether the party is walking the map.
    #[must_use]
    pub fn exploring(&self) -> bool {
        *self.app.get() == AppState::Playing
            && self.play.as_deref().map(State::get) == Some(&PlayState::Explore)
    }
}

/// Lines a screen may show; the rest stay blank.
const MAX_LINES: usize = 24;

/// The menus plugin.
pub struct MenusPlugin;

impl Plugin for MenusPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<KeyboardInput>()
            .add_message::<UiClick>()
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
            .add_systems(Update, menu_keys.in_set(UiSet::Dispatch))
            .add_systems(Update, refresh.in_set(UiSet::Model));
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

/// The keys a click on the active screen stands for.
fn click_keys(screens: &mut Screens, active: Active, hit: Hit) -> Vec<MenuKey> {
    let target = match active {
        Active::Title => Target::Title(&mut screens.title),
        Active::NewGame => Target::NewGame(&mut screens.new_game),
        Active::CreateParty => Target::Creation(&mut screens.creation),
        Active::Paused => Target::Pause(&mut screens.pause),
        Active::None => return Vec::new(),
    };
    screen::click(target, hit)
}

/// State transitions the menus request.
#[derive(SystemParam)]
struct Next<'w> {
    app: ResMut<'w, NextState<AppState>>,
    menu: ResMut<'w, NextState<MenuState>>,
    play: ResMut<'w, NextState<PlayState>>,
}

/// What a menu action may touch.
struct Actions<'a, 'c, 'cs, 'n, 'p, 'e, 'r> {
    commands: &'a mut Commands<'c, 'cs>,
    next: &'a mut Next<'n>,
    data: Option<&'a PackData>,
    config: &'a AppConfig,
    player: &'a mut MessageWriter<'p, PlayerCommand>,
    exit: &'a mut MessageWriter<'e, AppExit>,
    replaced: &'a mut MessageWriter<'r, WorldReplaced>,
    notice: &'a mut Notice,
}

impl Actions<'_, '_, '_, '_, '_, '_, '_> {
    fn start_game(&mut self, world: World, start: PlayState) {
        self.commands.insert_resource(SimWorld(world));
        self.commands.insert_resource(StartIn(start));
        self.next.app.set(AppState::Playing);
        // Presentation draws the new world before its first step.
        self.replaced.write(WorldReplaced);
    }

    fn leave_game(&mut self) {
        self.commands.remove_resource::<SimWorld>();
        self.next.app.set(AppState::MainMenu);
    }
}

#[allow(clippy::too_many_arguments)]
fn menu_keys(
    mut keys: MessageReader<KeyboardInput>,
    mut clicks: MessageReader<UiClick>,
    mut commands: Commands,
    at: Where,
    mut next: Next,
    mut screens: ResMut<Screens>,
    config: Res<AppConfig>,
    data: Option<Res<PackData>>,
    world: Option<Res<SimWorld>>,
    mut player: MessageWriter<PlayerCommand>,
    mut exit: MessageWriter<AppExit>,
    mut replaced: MessageWriter<WorldReplaced>,
    mut notice: ResMut<Notice>,
) {
    let members = world.as_ref().map_or(0, |w| w.0.party.members.len());
    let active = at.screen();
    let mut pressed: Vec<MenuKey> = keys.read().filter_map(menu_key).collect();
    for UiClick(hit) in clicks.read() {
        pressed.extend(click_keys(&mut screens, active, *hit));
    }
    let mut act = Actions {
        commands: &mut commands,
        next: &mut next,
        data: data.as_deref(),
        config: &config,
        player: &mut player,
        exit: &mut exit,
        replaced: &mut replaced,
        notice: &mut notice,
    };
    for key in pressed {
        let Screens {
            title,
            new_game,
            creation,
            catalog,
            pause,
        } = &mut *screens;
        match active {
            Active::Title => {
                if let Some(action) = title.key(key) {
                    title_action(action, &mut act);
                }
            }
            Active::NewGame => {
                if let Some(action) = new_game.key(key) {
                    new_game_action(action, new_game, &mut act);
                }
            }
            Active::CreateParty => {
                if let Some(action) = creation.key(key, catalog, members) {
                    creation_action(action, &mut act);
                }
            }
            Active::Paused => {
                if let Some(action) = pause.key(key) {
                    pause_action(action, &mut act);
                }
            }
            Active::None => {}
        }
    }
}

fn title_action(action: TitleAction, act: &mut Actions<'_, '_, '_, '_, '_, '_, '_>) {
    match action {
        TitleAction::NewGame => act.next.menu.set(MenuState::NewGame),
        TitleAction::Load => {
            let Some(data) = act.data else { return };
            match load(&data.0, &act.config.save_path, false) {
                Ok(world) => act.start_game(world, PlayState::Explore),
                Err(e) => act.notice.0 = format!("Load failed: {e}"),
            }
        }
        TitleAction::Quit => {
            act.exit.write(AppExit::Success);
        }
    }
}

fn new_game_action(
    action: NewGameAction,
    form: &NewGameForm,
    act: &mut Actions<'_, '_, '_, '_, '_, '_, '_>,
) {
    match action {
        NewGameAction::Start => {
            let Some(data) = act.data else { return };
            match World::new(&data.0, form.seed(crate::entropy_seed()), form.settings) {
                Ok(world) => {
                    info!("new game, seed {:#x}, {:?}", world.seed, world.settings);
                    act.start_game(world, PlayState::CreateParty);
                }
                Err(e) => act.notice.0 = format!("{e}"),
            }
        }
        NewGameAction::Back => act.next.menu.set(MenuState::Title),
    }
}

fn creation_action(action: CreationAction, act: &mut Actions<'_, '_, '_, '_, '_, '_, '_>) {
    match action {
        CreationAction::Add(draft) => {
            act.player
                .write(PlayerCommand(Command::Party(PartyCommand::Create(draft))));
        }
        CreationAction::Begin => act.next.play.set(PlayState::Explore),
        CreationAction::Back => act.leave_game(),
    }
}

fn pause_action(action: PauseAction, act: &mut Actions<'_, '_, '_, '_, '_, '_, '_>) {
    match action {
        PauseAction::Resume => act.next.play.set(PlayState::Explore),
        PauseAction::QuitToTitle => act.leave_game(),
        PauseAction::Quit => {
            act.exit.write(AppExit::Success);
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
        Active::Title => screens.title.lines(),
        Active::NewGame => screens.new_game.lines(),
        Active::CreateParty => {
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
        Active::Paused => {
            let (settings, seed) = world
                .as_ref()
                .map_or((omnis_sim::Settings::default(), 0), |w| {
                    (w.0.settings, w.0.seed)
                });
            screens.pause.lines(settings, seed)
        }
        Active::None => return,
    };
    for (mut line, MenuLine(i)) in &mut lines {
        let wanted = text.get(*i).map_or("", String::as_str);
        if line.0 != wanted {
            line.0 = wanted.to_owned();
        }
    }
}
