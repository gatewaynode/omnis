//! The fight's controls as a screen (presentation-ARCHITECTURE §6.5, §9): the step's prompt, the
//! budget line, the reactions switch and the menu's entries, described here and drawn by the
//! fight screen (`shell/combat.rs`) at the arena's rectangles: the turn's list in the action
//! column, the spell, feature and item pickers in a panel over the picture. It is never on the
//! overlays' stack. The entries are `CombatMenu`'s; a press carries one out. Bevy-free.

use super::{Control, ControlId, Effect, Game, Key, Kind, Panel, Row, Screen, ScreenId, Span};
use crate::combat_menu::{self, CombatMenu, Step};
use crate::style::{Role, Size};
use omnis_sim::api::TacticsCommand;
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::Data;
use omnis_sim::{ActorRef, Command, ModeKind, PartyCommand};

/// A control of the fight screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FightAct {
    /// The menu's nth entry, as `CombatMenu::entries` lists them now.
    Entry(u8),
    /// The acting member's reactions switch.
    Reactions,
}

/// What the fight screen keeps: the menu's step, and the turn it was set on.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FightForm {
    /// The menu.
    pub menu: CombatMenu,
    /// The phase, round and actor the step belongs to.
    turn: Option<(ModeKind, u32, Option<ActorRef>)>,
}

/// Where the fight screen stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// In the action column, under the picture.
    Column,
    /// In the panel over the picture, the column and the field: a picker.
    Over,
}

/// Where the screen stands at the menu's step: the pickers over the picture, the rest in the
/// column (a target is clicked on the field, so it must be in view).
#[must_use]
pub const fn place(form: &FightForm) -> Place {
    if form.menu.picking() {
        Place::Over
    } else {
        Place::Column
    }
}

/// The menu goes back to the turn's list when the turn is no longer the one its step was set
/// on: a phase, a round or an actor changed (a load, or an order from elsewhere).
pub fn settle(game: Game, form: &mut FightForm) {
    let turn = game
        .views
        .combat
        .as_ref()
        .map(|v| (v.phase, v.round, v.current));
    if form.turn != turn {
        form.turn = turn;
        form.menu = CombatMenu::default();
    }
}

/// Dim words: a note after a control.
fn note(text: impl Into<String>) -> Span {
    Span {
        text: text.into(),
        role: Role::Dim,
        size: Size::Body,
    }
}

/// The acting member and whether their reactions are on.
fn reacting(game: Game) -> Option<(CharacterId, bool)> {
    let view = game.views.combat.as_ref()?;
    let own = combat_menu::acting(view)?;
    let on = view.members.iter().find(|f| f.member == own)?.reactions_on;
    Some((own, on))
}

/// The reactions switch's command: the acting member's reactions the other way.
fn switch(game: Game) -> Option<Command> {
    let (member, on) = reacting(game)?;
    Some(Command::Party(PartyCommand::Tactics(
        TacticsCommand::SetReactions { member, on: !on },
    )))
}

/// The fight's controls as they stand.
#[must_use]
pub fn content(data: &Data, game: Game, form: &FightForm) -> Screen {
    let (world, menu) = (game.world, form.menu);
    let mut rows = Vec::new();
    if let Some(line) = combat_menu::budget_line(game.views) {
        let line = match (menu.picking(), combat_menu::points(game.views)) {
            (true, Some(points)) => format!("{points}   {line}"),
            _ => line,
        };
        rows.push(Row::text(Span::body(line)));
    }
    if menu.step == Step::Top
        && let (Some((_, on)), Some(command)) = (reacting(game), switch(game))
    {
        rows.push(Row::controls(vec![Control {
            id: ControlId::Fight(FightAct::Reactions),
            label: "Reactions".to_owned(),
            kind: Kind::Choice {
                options: vec!["off".to_owned(), "on".to_owned()],
                at: usize::from(on),
            },
            why_not: crate::trial::why(world, data, &game.views.party, &command),
        }]));
    }
    let entries = menu.entries(world, data);
    let controls = entries.iter().enumerate().filter_map(|(i, e)| {
        Some(Control {
            id: ControlId::Fight(FightAct::Entry(u8::try_from(i).ok()?)),
            label: e.label.clone(),
            kind: Kind::Button,
            why_not: e.blocked.clone(),
        })
    });
    if menu.picking() {
        for (control, e) in controls.zip(&entries) {
            rows.push(Row {
                after: e.note.iter().map(note).collect(),
                ..Row::controls(vec![control])
            });
        }
    } else {
        rows.push(Row::controls(controls.collect()));
    }
    let keys = (0..entries.len().min(9))
        .filter_map(|i| {
            let n = u8::try_from(i).ok()?;
            Some((Key::Digit(n + 1), ControlId::Fight(FightAct::Entry(n))))
        })
        .collect();
    Screen {
        id: ScreenId::Fight,
        title: Span::title(menu.prompt(world, data)),
        panels: vec![Panel { rows }],
        keys,
        message: None,
    }
}

/// Use a fight control: an entry carried out (a command to send, or the menu moved), or the
/// reactions switch flipped.
pub fn apply(data: &Data, game: Game, form: &mut FightForm, control: ControlId) -> Effect {
    let command = match control {
        ControlId::Fight(FightAct::Entry(n)) => {
            let entries = form.menu.entries(game.world, data);
            let Some(entry) = entries.get(usize::from(n)) else {
                return Effect::None;
            };
            form.menu.choose(&entry.act)
        }
        ControlId::Fight(FightAct::Reactions) => switch(game),
        _ => None,
    };
    command.map_or(Effect::None, |command| Effect::Command {
        command,
        close: false,
    })
}
