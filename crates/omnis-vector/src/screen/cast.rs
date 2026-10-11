//! Spells on the road (parity P5; the other client's road cast menu): the shown member's spells
//! castable outside a fight, each with its cost and a Cast button dim with the reason the view
//! or a trial of the exact command gives. A spell aimed at a member goes to the one selected in
//! the party column, which stands beside this screen, or else to the caster. The screen stays
//! open after a cast, its outcome under the list.

use super::Span;
use super::inventory::{owner, owner_choice, step_owner};
use super::{Control, ControlId, Effect, Forms, Game, Panel, Payload, Row, Screen, ScreenId};
use crate::labels;
use crate::style::{Role, Size};
use omnis_sim::api::CastView;
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, Target};

/// A small dim note.
fn note(text: impl Into<String>) -> Span {
    Span {
        text: text.into(),
        role: Role::Dim,
        size: Size::Small,
    }
}

/// Where row `row` goes: the selected member for a spell aimed at members, else the caster
/// (which the engine ignores for light and the like).
fn target(row: &CastView, forms: &Forms) -> Target {
    let to = if row.targets_members {
        forms.selected.unwrap_or(row.caster)
    } else {
        row.caster
    };
    Target::Member(to)
}

/// The command row `row` sends.
fn command(row: &CastView, forms: &Forms) -> Command {
    Command::Cast {
        caster: row.caster,
        spell: row.spell.clone(),
        target: target(row, forms),
    }
}

/// A member's name in the party, or "someone".
fn name(game: Game<'_>, id: omnis_sim::omnis_core::CharacterId) -> &str {
    game.views
        .party
        .members
        .iter()
        .find(|m| m.member == id)
        .map_or("someone", |m| m.name.as_str())
}

/// One spell's row: its name, its Cast button (none in a fight), its cost and target.
fn spell_row(data: &Data, game: Game, forms: &Forms, at: u16, row: &CastView) -> Row {
    let why_not = row
        .refusal
        .clone()
        .or_else(|| game.world.trial(data, &command(row, forms)).err())
        .map(|r| labels::reason(&game.views.party, &r));
    let mut after = format!("{} pt", row.cost);
    if row.targets_members {
        let Target::Member(to) = target(row, forms) else {
            unreachable!("a road cast goes to a member")
        };
        after.push_str(&format!("  on {}", name(game, to)));
    }
    Row {
        spans: vec![Span::body(labels::spell(data, &row.spell))],
        controls: if game.views.combat.is_some() {
            Vec::new()
        } else {
            vec![Control {
                why_not,
                ..Control::button(ControlId::Cast(at), "Cast")
            }]
        },
        after: vec![note(after)],
    }
}

/// The screen: whose spells and Close, then each of their road spells.
#[must_use]
pub fn content(data: &Data, game: Game, forms: &Forms) -> Screen {
    let Some(owner) = owner(game, forms) else {
        return super::empty(ScreenId::Cast);
    };
    let mut rows = vec![Row::controls(vec![
        owner_choice(game, owner.member, "Caster"),
        Control::button(ControlId::Close, "Close"),
    ])];
    if game.views.combat.is_some() {
        rows.push(Row::text(note(
            "In a fight: look here, cast from the fight's actions",
        )));
    }
    let casts = &game.views.casts;
    let before = rows.len();
    for (i, row) in casts.iter().enumerate() {
        if row.caster == owner.member
            && let Ok(at) = u16::try_from(i)
        {
            rows.push(spell_row(data, game, forms, at, row));
        }
    }
    if rows.len() == before {
        rows.push(Row::text(note(if casts.is_empty() {
            "Nobody knows a spell for the road.".to_owned()
        } else {
            format!("{} knows no spell for the road", owner.name)
        })));
    }
    Screen {
        id: ScreenId::Cast,
        title: Span::title("Spells"),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// The caster steps; Close closes; Cast sends the row's command and the screen stays open. The
/// control was found live on the screen as it stands.
pub fn apply(game: Game, forms: &mut Forms, control: ControlId, payload: &Payload) -> Effect {
    match control {
        ControlId::Owner => {
            step_owner(game, forms, payload);
            Effect::None
        }
        ControlId::Close => Effect::Back,
        ControlId::Cast(at) => game
            .views
            .casts
            .get(usize::from(at))
            .map_or(Effect::None, |row| Effect::Command {
                command: command(row, forms),
                close: false,
            }),
        _ => Effect::None,
    }
}
