//! The marching order (owner, 2026-11-24): the party as it marches, front row first, each
//! member with an Up and a Down. A press sends the engine's `Reorder` with two neighbours
//! swapped, and the screen stays open on the new order. Opened from the party column's
//! Order button on the road; the column stands beside it. The front row's size is a rule
//! (`front_row`): the order decides who is in it, not how many.

use super::{Control, ControlId, Effect, Game, Panel, Row, Screen, ScreenId, Span};
use crate::labels;
use crate::style::{Role, Size};
use omnis_sim::Command;
use omnis_sim::api::PartyCommand;
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::Data;

/// A small dim note.
fn note(text: impl Into<String>) -> Span {
    Span {
        text: text.into(),
        role: Role::Dim,
        size: Size::Small,
    }
}

/// The party's order as it stands.
fn standing(game: Game) -> Vec<CharacterId> {
    game.views.party.members.iter().map(|m| m.member).collect()
}

/// The command that keeps the order as it is: what the engine says of any reorder here.
#[must_use]
pub fn unchanged(game: Game) -> Command {
    Command::Party(PartyCommand::Reorder {
        order: standing(game),
    })
}

/// The order with `member` one place nearer the front (`up`) or the back, when there is such
/// a place.
fn moved(game: Game, member: CharacterId, up: bool) -> Option<Command> {
    let mut order = standing(game);
    let at = order.iter().position(|m| *m == member)?;
    let to = if up {
        at.checked_sub(1)?
    } else {
        Some(at + 1).filter(|n| *n < order.len())?
    };
    order.swap(at, to);
    Some(Command::Party(PartyCommand::Reorder { order }))
}

/// Why `member` cannot move that way: no place to move to, else the engine's refusal.
fn why_not(data: &Data, game: Game, member: CharacterId, up: bool) -> Option<String> {
    let Some(command) = moved(game, member, up) else {
        return Some(if up { "Already first" } else { "Already last" }.to_owned());
    };
    game.world
        .trial(data, &command)
        .err()
        .map(|r| labels::reason(&game.views.party, &r))
}

/// The order as it stands.
#[must_use]
pub fn content(data: &Data, game: Game) -> Screen {
    let party = &game.views.party;
    let mut rows = vec![Row {
        spans: vec![Span::body(format!(
            "The first {} march in front and meet a foe hand to hand",
            party.front_row
        ))],
        controls: vec![Control::button(ControlId::Close, "Close")],
        ..Row::default()
    }];
    let mut group = None;
    for m in &party.members {
        if group != Some(m.in_front) {
            group = Some(m.in_front);
            rows.push(Row::text(note(if m.in_front { "front" } else { "back" })));
        }
        let shift = |up: bool, label: &str| Control {
            why_not: why_not(data, game, m.member, up),
            ..Control::button(
                ControlId::Shift {
                    member: m.member,
                    up,
                },
                label,
            )
        };
        rows.push(Row {
            spans: vec![Span::body(format!(
                "{}  {} {}",
                m.name,
                labels::class(data, &m.class),
                m.level
            ))],
            controls: vec![shift(true, "Up"), shift(false, "Down")],
            ..Row::default()
        });
    }
    Screen {
        id: ScreenId::Order,
        title: Span::title("Marching order"),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// A member moves a place and the screen stays open; Close closes. The control was found
/// live on the screen as it stands.
#[must_use]
pub fn apply(game: Game, control: ControlId) -> Effect {
    match control {
        ControlId::Shift { member, up } => {
            moved(game, member, up).map_or(Effect::None, |command| Effect::Command {
                command,
                close: false,
            })
        }
        ControlId::Close => Effect::Back,
        _ => Effect::None,
    }
}
