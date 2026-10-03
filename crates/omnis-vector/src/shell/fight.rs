//! The fight notice (alt-ARCHITECTURE.md §9): while monsters are met the 3D view stays up and a
//! panel offers the encounter's choices, so the viewer never dead-ends. In a fight it offers a
//! placeholder Attack, Dodge and Flee until the combat screen (Phase B) replaces it; when the
//! party falls it offers a restart.

use super::VectorSet;
use super::controls::{GREEN, button};
use super::session::Session;
use bevy::prelude::*;
use omnis_sim::combat::state::can_fight;
use omnis_sim::event::ActorRef;
use omnis_sim::omnis_data::Data;
use omnis_sim::{
    CombatCommand, CombatView, Command, EncounterChoice, ModeKind, World, apply, bribe_cost,
    combat_view,
};

/// What a notice button does.
#[derive(Component, Debug, Clone, PartialEq)]
pub enum Order {
    /// Send a command to the simulation.
    Command(Command),
    /// Start a fresh world and party.
    Restart,
}

/// One button of the notice.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    /// The label.
    pub label: String,
    /// What it does.
    pub order: Order,
    /// Why it cannot be chosen now, if it cannot.
    pub blocked: Option<String>,
}

/// The panel's content.
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    /// The heading.
    pub title: String,
    /// The monsters, one line a stack.
    pub lines: Vec<String>,
    /// The buttons, in order; the number keys pick them.
    pub choices: Vec<Choice>,
}

/// Whether the party can no longer fight: every member is down, dead or incapacitated.
#[must_use]
pub fn fallen(world: &World, data: &Data) -> bool {
    !world.party.members.iter().any(|m| can_fight(m, data))
}

/// Why the simulation would refuse a command now, found on a copy of the world.
fn refusal(world: &World, data: &Data, command: &Command) -> Option<String> {
    apply(&mut world.clone(), data, command.clone())
        .err()
        .map(|r| r.to_string())
}

fn choice(world: &World, data: &Data, label: String, command: Command) -> Choice {
    Choice {
        blocked: refusal(world, data, &command),
        label,
        order: Order::Command(command),
    }
}

/// The notice for the world as it stands, or `None` while exploring with a party that stands.
#[must_use]
pub fn notice(world: &World, data: &Data) -> Option<Notice> {
    let Some(view) = combat_view(world, data) else {
        return fallen(world, data).then(|| Notice {
            title: "The party has fallen".to_owned(),
            lines: Vec::new(),
            choices: vec![Choice {
                label: "Start again".to_owned(),
                order: Order::Restart,
                blocked: None,
            }],
        });
    };
    let lines = view
        .stacks
        .iter()
        .filter(|s| s.alive)
        .map(|s| {
            let hp: Vec<String> = s.hp.iter().map(ToString::to_string).collect();
            format!(
                "{} x{}  hp {}{}",
                data.label("en", &s.name),
                s.hp.len(),
                hp.join(" "),
                if s.front { "  (front)" } else { "" }
            )
        })
        .collect();
    let (title, choices) = if view.phase == ModeKind::Encounter {
        encounter_choices(world, data, &view)
    } else {
        combat_choices(world, data, &view)
    };
    Some(Notice {
        title,
        lines,
        choices,
    })
}

/// The heading and choices before a fight: the four encounter choices.
fn encounter_choices(world: &World, data: &Data, view: &CombatView) -> (String, Vec<Choice>) {
    let bribe = bribe_cost(world, data).map_or_else(
        |_| "Bribe".to_owned(),
        |cost| format!("Bribe ({cost} gold)"),
    );
    let options = [
        ("Fight".to_owned(), EncounterChoice::Attack),
        (bribe, EncounterChoice::Bribe),
        ("Hide".to_owned(), EncounterChoice::Hide),
        ("Run".to_owned(), EncounterChoice::Run),
    ];
    (
        format!("Monsters ahead ({:?})", view.disposition),
        options
            .into_iter()
            .map(|(label, c)| choice(world, data, label, Command::Encounter(c)))
            .collect(),
    )
}

/// The heading and choices in a fight, the placeholder until Phase B: attack a stack the
/// acting member reaches, dodge, or flee.
fn combat_choices(world: &World, data: &Data, view: &CombatView) -> (String, Vec<Choice>) {
    let actor = match view.current {
        Some(ActorRef::Member(id)) => world
            .party
            .members
            .iter()
            .find(|m| m.id == id)
            .map_or("?", |m| m.name.as_str()),
        _ => "the monsters",
    };
    let attacks = view
        .stacks
        .iter()
        .filter(|s| s.alive && s.reachable)
        .map(|s| {
            let label = format!("Attack {}", data.label("en", &s.name));
            (label, CombatCommand::Attack { stack: s.index })
        });
    let others = [
        ("Dodge".to_owned(), CombatCommand::Dodge),
        ("Flee".to_owned(), CombatCommand::Run),
    ];
    let choices = attacks
        .chain(others)
        .map(|(label, c)| choice(world, data, label, Command::Combat(c)))
        .collect();
    (format!("Round {}: {actor}'s turn", view.round), choices)
}

/// Marks the panel.
#[derive(Component)]
struct Panel;

/// The notice panel and its choices. Headless-safe.
pub struct FightPlugin;

impl Plugin for FightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn)
            .add_systems(Update, (choose, refresh).chain().in_set(VectorSet::Input));
    }
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Panel,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(22.0),
            left: Val::Percent(50.0),
            // Centred on its own width.
            margin: UiRect::left(Val::Px(-260.0)),
            width: Val::Px(520.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            padding: UiRect::all(Val::Px(14.0)),
            border: UiRect::all(Val::Px(1.0)),
            display: Display::None,
            ..default()
        },
        BorderColor::all(GREEN),
        BackgroundColor(Color::srgba(0.0, 0.05, 0.02, 0.85)),
    ));
}

fn text(parent: &mut ChildSpawnerCommands, line: &str, size: f32) {
    parent.spawn((
        Text::new(line),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(GREEN),
    ));
}

/// Rebuild the panel whenever the simulation accepted a command or the party fell.
fn refresh(
    mut commands: Commands,
    session: Res<Session>,
    mut panel: Query<(Entity, &mut Node), With<Panel>>,
    mut last: Local<Option<(usize, bool)>>,
) {
    let key = (
        session.binder.log.len(),
        fallen(&session.world, &session.data),
    );
    if *last == Some(key) {
        return;
    }
    *last = Some(key);
    let Ok((entity, mut node)) = panel.single_mut() else {
        return;
    };
    commands.entity(entity).despawn_related::<Children>();
    let Some(notice) = notice(&session.world, &session.data) else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    commands.entity(entity).with_children(|p| {
        text(p, &notice.title, 22.0);
        for line in &notice.lines {
            text(p, line, 16.0);
        }
        p.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(6.0),
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            for (i, c) in notice.choices.iter().enumerate() {
                let label = match &c.blocked {
                    None => format!("{}  {}", i + 1, c.label),
                    Some(why) => format!("{}: {why}", c.label),
                };
                button(row, &label, c.order.clone(), c.blocked.is_none());
            }
        });
    });
}

const DIGITS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

/// A pressed notice button, or its number key, carries out its order.
fn choose(
    buttons: Query<(&Interaction, &Order), Changed<Interaction>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<Session>,
) {
    let mut order = buttons
        .iter()
        .find(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, o)| o.clone());
    if order.is_none()
        && let Some(n) = DIGITS.iter().position(|k| keys.just_pressed(*k))
        && let Some(notice) = notice(&session.world, &session.data)
        && let Some(c) = notice.choices.get(n)
        && c.blocked.is_none()
    {
        order = Some(c.order.clone());
    }
    match order {
        Some(Order::Command(command)) => session.order(command),
        Some(Order::Restart) => {
            if let Err(e) = session.restart() {
                session.say(format!("Could not start again: {e}"));
            }
        }
        None => {}
    }
}
