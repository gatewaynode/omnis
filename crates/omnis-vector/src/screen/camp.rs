//! Camp (parity P6; the retired client's camp panel): each member's hit points and hit dice with
//! how many to spend, a short rest that spends them, and a long rest with the food it eats or
//! why it cannot be taken. Opened from the Camp button while exploring; the party column
//! stands beside it. Every button is lit by the rest view's refusal, then a trial of its exact
//! command. The screen stays open after a rest, its outcome under the rows.

use super::{
    Control, ControlId, Effect, Forms, Game, Kind, Panel, Payload, Row, Screen, ScreenId, Span,
    stepped,
};
use crate::labels;
use crate::style::{Role, Size};
use omnis_sim::api::{CampMember, HitDiceSpend, RestCommand};
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, Rejection};

/// A small dim note.
fn note(text: impl Into<String>) -> Span {
    Span {
        text: text.into(),
        role: Role::Dim,
        size: Size::Small,
    }
}

/// The hit dice each member is to spend: the form's counts, one per member, each held within
/// what that member may spend now.
#[must_use]
pub fn counts(game: Game, forms: &Forms) -> Vec<u8> {
    game.views
        .rest
        .members
        .iter()
        .enumerate()
        .map(|(i, m)| forms.dice.get(i).copied().unwrap_or(0).min(m.spendable))
        .collect()
}

/// The short rest the counts ask for: each member with a count above 0, in marching order.
fn short(game: Game, counts: &[u8]) -> Command {
    let spend = game
        .views
        .party
        .members
        .iter()
        .zip(counts)
        .filter(|(_, n)| **n > 0)
        .map(|(m, n)| HitDiceSpend {
            member: m.member,
            count: *n,
        })
        .collect();
    Command::Rest(RestCommand::Short { spend })
}

/// Why a rest cannot be taken: the view's refusal, else a trial of its command.
fn refused(data: &Data, game: Game, view: Option<&Rejection>, command: &Command) -> Option<String> {
    view.cloned()
        .or_else(|| game.world.trial(data, command).err())
        .map(|r| labels::reason(&game.views.party, &r))
}

/// Why a member spends no dice (the retired client's `dice_note`), or how many.
fn dice_note(member: &CampMember, count: u8) -> String {
    match member.spendable {
        0 if member.hit_dice_left == 0 => "no hit dice left".to_owned(),
        0 if member.hp >= member.hp_max => "unhurt".to_owned(),
        0 => "cannot spend".to_owned(),
        _ => format!("spend {count}"),
    }
}

/// The camp as it stands.
#[must_use]
pub fn content(data: &Data, game: Game, forms: &Forms) -> Screen {
    let rest = &game.views.rest;
    let counts = counts(game, forms);
    let mut rows = vec![Row {
        tint: None,
        spans: vec![Span::body(format!(
            "The night eats {} food; the stores hold {}",
            rest.long_food, rest.food
        ))],
        controls: vec![Control::button(ControlId::Close, "Close")],
        after: Vec::new(),
    }];
    for (i, (member, shown)) in rest
        .members
        .iter()
        .zip(&game.views.party.members)
        .enumerate()
    {
        let count = counts.get(i).copied().unwrap_or(0);
        let controls = if member.spendable > 0 {
            vec![Control {
                id: ControlId::Dice(u8::try_from(i).unwrap_or(u8::MAX)),
                label: "Spend".to_owned(),
                kind: Kind::Number {
                    value: i64::from(count),
                    min: 0,
                    max: i64::from(member.spendable),
                },
                why_not: None,
            }]
        } else {
            Vec::new()
        };
        rows.push(Row {
            tint: None,
            spans: vec![Span::body(format!(
                "{}  HP {}/{}  Hit dice {}/{} d{}",
                shown.name,
                member.hp,
                member.hp_max,
                member.hit_dice_left,
                member.hit_dice,
                member.die
            ))],
            controls,
            after: vec![note(dice_note(member, count))],
        });
    }
    let short_why = if rest.refusal.is_none() && counts.iter().all(|n| *n == 0) {
        Some("Choose hit dice to spend".to_owned())
    } else {
        refused(data, game, rest.refusal.as_ref(), &short(game, &counts))
    };
    rows.push(Row {
        tint: None,
        spans: Vec::new(),
        controls: vec![Control {
            why_not: short_why,
            ..Control::button(ControlId::ShortRest, "Short rest")
        }],
        after: vec![note("An hour; each member spends the dice chosen")],
    });
    let long = Command::Rest(RestCommand::Long);
    rows.push(Row {
        tint: None,
        spans: Vec::new(),
        controls: vec![Control {
            why_not: refused(data, game, rest.long_refusal.as_ref(), &long),
            ..Control::button(ControlId::LongRest, "Long rest")
        }],
        after: vec![note(
            "eight hours: full hit points and spell points, half the hit dice back",
        )],
    });
    Screen {
        id: ScreenId::Camp,
        title: Span::title("Camp"),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// A count steps or is set; a rest is sent and the screen stays open (a short rest clears the
/// counts); Close closes. The control was found live on the screen as it stands.
pub fn apply(game: Game, forms: &mut Forms, control: ControlId, payload: &Payload) -> Effect {
    forms.dice = counts(game, forms);
    match control {
        ControlId::Dice(i) => {
            let i = usize::from(i);
            if let (Some(member), Some(count)) =
                (game.views.rest.members.get(i), forms.dice.get_mut(i))
            {
                let n = stepped(i64::from(*count), 0, i64::from(member.spendable), payload);
                *count = u8::try_from(n).unwrap_or(0);
            }
            Effect::None
        }
        ControlId::ShortRest => {
            let command = short(game, &forms.dice);
            forms.dice.clear();
            Effect::Command {
                command,
                close: false,
            }
        }
        ControlId::LongRest => Effect::Command {
            command: Command::Rest(RestCommand::Long),
            close: false,
        },
        ControlId::Close => Effect::Back,
        _ => Effect::None,
    }
}
