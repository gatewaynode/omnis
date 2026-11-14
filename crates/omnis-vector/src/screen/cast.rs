//! Spells on the road (parity P5; the retired client's road cast menu): the shown member's spells
//! castable outside a fight, each with its cost and a Cast button dim with the reason the view
//! gives, or that the spell has no target to choose. Cast opens the target step
//! (`crate::targeting`): the spell's possible targets as the engine lists them, one of which is
//! chosen (owner, 2026-11-14: a target is chosen after the option, never implied). The screen
//! stays open after a cast, its outcome under the list.

use super::Span;
use super::inventory::{owner, owner_choice, step_owner};
use super::{Control, ControlId, Effect, Forms, Game, Panel, Payload, Row, Screen, ScreenId};
use crate::labels;
use crate::style::{Role, Size};
use crate::targeting::Aim;
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

/// The spell of row `row`, chosen and waiting for its target.
fn aim(row: &CastView) -> Aim {
    Aim::spell(row.caster, &row.spell)
}

/// One spell's row: its name, its Cast button (none in a fight) and its cost. The button is
/// dim with the view's reason, or when the spell has no target to choose now.
fn spell_row(data: &Data, game: Game, at: u16, row: &CastView) -> Row {
    let why_not = row
        .refusal
        .as_ref()
        .map(|r| labels::reason(&game.views.party, r))
        .or_else(|| aim(row).blocked(data, game));
    Row {
        tint: None,
        spans: vec![Span::body(labels::spell(data, &row.spell))],
        controls: if game.views.combat.is_some() {
            Vec::new()
        } else {
            vec![Control {
                why_not,
                ..Control::button(ControlId::Cast(at), "Cast")
            }]
        },
        after: vec![note(format!("{} pt", row.cost))],
        aligned: false,
    }
}

/// The screen: whose spells and Close, then each of their road spells.
#[must_use]
pub fn content(data: &Data, game: Game, forms: &Forms) -> Screen {
    let Some(owner) = owner(game, forms) else {
        return super::empty(ScreenId::Cast);
    };
    if let Some(aim) = forms.aim.as_ref().filter(|_| game.views.combat.is_none()) {
        return crate::targeting::content(data, game, aim, ScreenId::Cast);
    }
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
            rows.push(spell_row(data, game, at, row));
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

/// The caster steps; Close closes; Cast opens the spell's target step, where a target sends
/// the cast and Back returns to the list. The screen stays open after a cast. The control was
/// found live on the screen as it stands.
pub fn apply(
    data: &Data,
    game: Game,
    forms: &mut Forms,
    control: ControlId,
    payload: &Payload,
) -> Effect {
    match control {
        ControlId::Owner => {
            step_owner(game, forms, payload);
            Effect::None
        }
        ControlId::Close => Effect::Back,
        ControlId::Back => {
            forms.aim = None;
            Effect::None
        }
        ControlId::Cast(at) => {
            forms.aim = game.views.casts.get(usize::from(at)).map(aim);
            Effect::None
        }
        ControlId::Target(at) => {
            let command = forms.aim.take().and_then(|aim| aim.chosen(data, game, at));
            // A spell sent at a group opens a fight (the opening strike): the screen gives way
            // to it. Any other cast leaves the screen up for the next.
            let strikes = |command: &Command| {
                matches!(
                    command,
                    Command::Cast {
                        target: Target::Being { .. },
                        ..
                    }
                )
            };
            command.map_or(Effect::None, |command| Effect::Command {
                close: strikes(&command),
                command,
            })
        }
        _ => Effect::None,
    }
}
