//! The loot window (owner, 2026-11-12): what lies in one place within the party's reach, for
//! the party to take. It is opened on an [`ItemPlace`], so the same window serves the ground
//! (the piles in reach, `PickUp`) and an open container (`TakeOut`), and whatever place the
//! engine later lets items be taken from. Each item has Take (one) and, where there are
//! several, Take all; Take everything empties the place. Who receives is chosen at the top: the
//! member selected in the party column when it opens, else the first on their feet. Every
//! button is lit by a trial of its exact command, so a dim one carries the engine's reason. The
//! window closes itself when the place is empty or out of reach ([`gone`]).

use super::{Control, ControlId, Effect, Forms, Game, Kind, Panel, Payload, Row, Screen};
use super::{ScreenId, Span};
use crate::labels;
use crate::sim::Views;
use crate::style::{Role, Size};
use omnis_sim::api::{ItemCount, MemberView};
use omnis_sim::event::ItemPlace;
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, ItemCommand};

/// The window's form: where the items are and who takes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loot {
    /// The place the window is open on.
    pub from: ItemPlace,
    /// Who receives, once chosen in the window.
    pub receiver: Option<CharacterId>,
}

impl Default for Loot {
    fn default() -> Loot {
        Loot {
            from: ItemPlace::Ground,
            receiver: None,
        }
    }
}

/// What a control of the window does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LootAct {
    /// Who receives.
    Receiver,
    /// Take one of the item at this row.
    Take(u16),
    /// Take every one of the item at this row.
    TakeAll(u16),
    /// Take every item of the place.
    Everything,
}

/// The place's name and its items as the views have them, or `None` when it is not in reach:
/// no pile there, or no open container of that id.
fn place<'v>(
    data: &'v Data,
    views: &'v Views,
    from: &ItemPlace,
) -> Option<(&'v str, &'v [ItemCount])> {
    let objects = &views.objects;
    match from {
        ItemPlace::Ground => Some(("On the ground", objects.ground.as_slice())),
        ItemPlace::Object(id) => {
            let object = objects.objects.iter().find(|o| o.object == *id)?;
            Some((
                labels::text(data, &object.name),
                object.contents.as_deref()?,
            ))
        }
        ItemPlace::Member(_) | ItemPlace::Stores => None,
    }
}

/// Whether the window has nothing left to show: the place is empty, out of reach, shut, or the
/// party is no longer exploring.
#[must_use]
pub fn gone(data: &Data, views: &Views, forms: &Forms) -> bool {
    !views.exploring()
        || place(data, views, &forms.loot.from).is_none_or(|(_, items)| items.is_empty())
}

/// Who receives: the one chosen in the window, else the one selected in the party column, else
/// the first member on their feet, else the first.
#[must_use]
pub fn receiver<'v>(game: Game<'v>, forms: &Forms) -> Option<&'v MemberView> {
    let members = &game.views.party.members;
    let find = |id: CharacterId| members.iter().find(|m| m.member == id);
    forms
        .loot
        .receiver
        .and_then(find)
        .or_else(|| forms.selected.and_then(find))
        .or_else(|| members.iter().find(|m| !m.down && !m.dead))
        .or_else(|| members.first())
}

/// The command that takes `count` of `item` from the form's place into `member`'s kit.
fn take(from: &ItemPlace, member: CharacterId, item: &str, count: u16) -> Option<Command> {
    let item = item.to_owned();
    Some(Command::Item(match from {
        ItemPlace::Ground => ItemCommand::PickUp {
            member,
            item,
            count,
        },
        ItemPlace::Object(id) => ItemCommand::TakeOut {
            member,
            object: id.clone(),
            item,
            count,
        },
        ItemPlace::Member(_) | ItemPlace::Stores => return None,
    }))
}

/// What a control sends: one command for a row's buttons, one an item for Take everything.
fn commands(data: &Data, game: Game, forms: &Forms, act: LootAct) -> Vec<Command> {
    let from = &forms.loot.from;
    let (Some((_, items)), Some(to)) = (place(data, game.views, from), receiver(game, forms))
    else {
        return Vec::new();
    };
    let row = |at: u16, all: bool| {
        let item = items.get(usize::from(at))?;
        take(
            from,
            to.member,
            &item.item,
            if all { item.count } else { 1 },
        )
    };
    match act {
        LootAct::Receiver => Vec::new(),
        LootAct::Take(at) => row(at, false).into_iter().collect(),
        LootAct::TakeAll(at) => row(at, true).into_iter().collect(),
        LootAct::Everything => items
            .iter()
            .filter_map(|i| take(from, to.member, &i.item, i.count))
            .collect(),
    }
}

/// A button of the window, dim with the engine's reason for the first command it would send.
fn button(data: &Data, game: Game, forms: &Forms, act: LootAct, label: &str) -> Control {
    let why_not = match commands(data, game, forms, act).first() {
        None => Some("Gone".to_owned()),
        Some(command) => game
            .world
            .trial(data, command)
            .err()
            .map(|r| labels::refused(data, &game.views.party, &r)),
    };
    Control {
        why_not,
        ..Control::button(ControlId::Loot(act), label)
    }
}

/// The screen: who takes and Close, each item with its buttons, then Take everything.
#[must_use]
pub fn content(data: &Data, game: Game, forms: &Forms) -> Screen {
    let members = &game.views.party.members;
    let to = receiver(game, forms).map(|m| m.member);
    let mut rows = vec![Row::controls(vec![
        Control {
            id: ControlId::Loot(LootAct::Receiver),
            label: "Taken by".to_owned(),
            kind: Kind::Choice {
                options: members.iter().map(|m| m.name.clone()).collect(),
                at: members
                    .iter()
                    .position(|m| Some(m.member) == to)
                    .unwrap_or(0),
            },
            why_not: None,
        },
        Control::button(ControlId::Close, "Close"),
    ])];
    let (title, items) = place(data, game.views, &forms.loot.from).unwrap_or(("Nothing here", &[]));
    for (at, item) in (0u16..).zip(items) {
        let mut controls = vec![button(data, game, forms, LootAct::Take(at), "Take")];
        if item.count > 1 {
            controls.push(button(data, game, forms, LootAct::TakeAll(at), "Take all"));
        }
        rows.push(Row {
            spans: vec![Span::body(labels::count_label(
                labels::item(data, &item.item),
                item.count,
            ))],
            controls,
            ..Row::default()
        });
    }
    if items.is_empty() {
        rows.push(Row::text(Span {
            text: "Nothing left".to_owned(),
            role: Role::Dim,
            size: Size::Small,
        }));
    } else {
        rows.push(Row::controls(vec![button(
            data,
            game,
            forms,
            LootAct::Everything,
            "Take everything",
        )]));
    }
    Screen {
        id: ScreenId::Loot,
        title: Span::title(title),
        panels: vec![Panel { rows }],
        keys: Vec::new(),
        message: None,
    }
}

/// The receiver steps around the party; a row's button sends its command and the window stays
/// open; Take everything sends one an item; Close closes. The control was found live.
pub fn apply(
    data: &Data,
    game: Game,
    forms: &mut Forms,
    control: ControlId,
    payload: &Payload,
) -> Effect {
    match control {
        ControlId::Loot(LootAct::Receiver) => {
            let members = &game.views.party.members;
            let now = receiver(game, forms).map(|m| m.member);
            let n = members.len();
            if let Some(at) = members.iter().position(|m| Some(m.member) == now) {
                let next = match payload {
                    Payload::Left => (at + n - 1) % n,
                    _ => (at + 1) % n,
                };
                forms.loot.receiver = Some(members[next].member);
            }
            Effect::None
        }
        ControlId::Loot(act) => {
            let mut commands = commands(data, game, forms, act);
            match commands.len() {
                0 => Effect::None,
                1 => Effect::Command {
                    command: commands.remove(0),
                    close: false,
                },
                _ => Effect::Commands(commands),
            }
        }
        _ => Effect::Back,
    }
}
