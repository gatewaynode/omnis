//! A member's kit beside the party's stores (parity P5; the retired client's `inventory_menu`).
//! Each row offers what applies to it by the view's own facts (a slot to wear it in, a use), and
//! every button is lit by a trial of its exact command, so a dim one carries the engine's
//! reason. The member selected in the party column, which stands beside this screen, receives a
//! give or a potion; Take fills the kit shown. Counts move one at a time, as in the retired client.

use super::{Control, ControlId, Effect, Forms, Game, Kind, Panel, Payload, Row, Screen};
use super::{ScreenId, Span};
use crate::labels;
use crate::style::{Role, Size};
use omnis_sim::api::{ItemView, MemberView};
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::{Data, EquipSlot};
use omnis_sim::{Command, ItemCommand};

/// What a row's button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemAct {
    /// Wear or wield it.
    Equip,
    /// Take it off.
    Unequip,
    /// Use it, on the selected member or else the user.
    Use,
    /// Put one in the stores.
    Stow,
    /// Hand one to the selected member.
    Give,
    /// Take one from the stores into the kit shown.
    Take,
}

impl ItemAct {
    /// The button's word.
    const fn caption(self) -> &'static str {
        match self {
            ItemAct::Equip => "Equip",
            ItemAct::Unequip => "Take off",
            ItemAct::Use => "Use",
            ItemAct::Stow => "Stow",
            ItemAct::Give => "Give",
            ItemAct::Take => "Take",
        }
    }
}

/// The member whose kit or spells a screen shows: the form's, while they are in the party, else
/// the first member.
#[must_use]
pub fn owner<'v>(game: Game<'v>, forms: &Forms) -> Option<&'v MemberView> {
    let members = &game.views.party.members;
    forms
        .owner
        .and_then(|id| members.iter().find(|m| m.member == id))
        .or_else(|| members.first())
}

/// The choice of whose kit or spells are shown, by name.
#[must_use]
pub fn owner_choice(game: Game, owner: CharacterId, label: &str) -> Control {
    let members = &game.views.party.members;
    Control {
        id: ControlId::Owner,
        label: label.to_owned(),
        kind: Kind::Choice {
            options: members.iter().map(|m| m.name.clone()).collect(),
            at: members.iter().position(|m| m.member == owner).unwrap_or(0),
        },
        why_not: None,
    }
}

/// Step the shown member left or right (a press steps right), around the party.
pub fn step_owner(game: Game, forms: &mut Forms, payload: &Payload) {
    let members = &game.views.party.members;
    let Some(now) = owner(game, forms).map(|m| m.member) else {
        return;
    };
    let at = members.iter().position(|m| m.member == now).unwrap_or(0);
    let n = members.len();
    let next = match payload {
        Payload::Left => (at + n - 1) % n,
        _ => (at + 1) % n,
    };
    forms.owner = Some(members[next].member);
}

/// A small dim note.
fn note(text: impl Into<String>) -> Span {
    Span {
        text: text.into(),
        role: Role::Dim,
        size: Size::Small,
    }
}

/// A list's heading.
fn heading(text: &str) -> Row {
    Row::text(Span {
        text: text.to_owned(),
        role: Role::Title,
        size: Size::Body,
    })
}

/// The kit's item at `at`, or the stores' for Take.
fn item_at<'v>(
    game: Game<'v>,
    owner: &'v MemberView,
    at: u16,
    act: ItemAct,
) -> Option<&'v ItemView> {
    let list = if act == ItemAct::Take {
        &game.views.party.inventory
    } else {
        &owner.equipment
    };
    list.get(usize::from(at))
}

/// The command a row's button sends, or why it has none to send (a give with no one to receive).
fn command(game: Game, forms: &Forms, at: u16, act: ItemAct) -> Result<Command, String> {
    let owner = owner(game, forms).ok_or_else(|| "No one in the party".to_owned())?;
    let item = item_at(game, owner, at, act).ok_or_else(|| "Gone".to_owned())?;
    let (member, id) = (owner.member, item.item.clone());
    Ok(Command::Item(match act {
        ItemAct::Equip => ItemCommand::Equip { member, item: id },
        ItemAct::Unequip => ItemCommand::Unequip {
            member,
            slot: item.slot.ok_or_else(|| "Not worn".to_owned())?,
        },
        ItemAct::Use => ItemCommand::Use {
            member,
            item: id,
            receiver: forms.selected,
        },
        ItemAct::Stow => ItemCommand::Stow {
            member,
            item: id,
            count: 1,
        },
        ItemAct::Give => match forms.selected {
            None => return Err("Select a member in the party column".to_owned()),
            Some(to) if to == member => {
                return Err("Select another member in the party column".to_owned());
            }
            Some(receiver) => ItemCommand::Give {
                giver: member,
                receiver,
                item: id,
                count: 1,
            },
        },
        ItemAct::Take => ItemCommand::Take {
            member,
            item: id,
            count: 1,
        },
    }))
}

/// A row's button, dim with the client's reason or the trial's.
fn button(data: &Data, game: Game, forms: &Forms, at: u16, act: ItemAct) -> Control {
    let why_not = match command(game, forms, at, act) {
        Err(why) => Some(why),
        Ok(command) => game
            .world
            .trial(data, &command)
            .err()
            .map(|r| labels::reason(&game.views.party, &r)),
    };
    Control {
        why_not,
        ..Control::button(ControlId::Item { at, act }, act.caption())
    }
}

/// The buttons that apply to a kit row, by the view's facts.
fn kit_acts(item: &ItemView) -> Vec<ItemAct> {
    let mut acts = Vec::new();
    if item.slot.is_some() {
        acts.push(if item.equipped {
            ItemAct::Unequip
        } else {
            ItemAct::Equip
        });
    }
    if item.usable {
        acts.push(ItemAct::Use);
    }
    acts.extend([ItemAct::Stow, ItemAct::Give]);
    acts
}

/// One item's row: its name and count, its buttons (none in a fight), its slot and marks.
fn item_row(
    data: &Data,
    game: Game,
    forms: &Forms,
    at: u16,
    item: &ItemView,
    acts: &[ItemAct],
) -> Row {
    let marks: Vec<&str> = item
        .slot
        .map(labels::slot)
        .into_iter()
        .chain(item.equipped.then_some("worn"))
        .chain((item.usable && !item.equipped).then_some("use"))
        .collect();
    let looking = game.views.combat.is_some();
    Row {
        tint: None,
        spans: vec![Span::body(labels::count_label(
            labels::item(data, &item.item),
            item.count,
        ))],
        controls: if looking {
            Vec::new()
        } else {
            acts.iter()
                .map(|act| button(data, game, forms, at, *act))
                .collect()
        },
        after: if marks.is_empty() {
            Vec::new()
        } else {
            vec![note(marks.join("  "))]
        },
    }
}

/// "wearing Longsword, Shield, Chain mail" in slot order, or "wearing nothing".
fn wearing(data: &Data, owner: &MemberView) -> String {
    let worn: Vec<&str> = EquipSlot::ALL
        .iter()
        .filter_map(|slot| owner.worn.iter().find(|(s, _)| s == slot))
        .map(|(_, id)| labels::item(data, id))
        .collect();
    if worn.is_empty() {
        "wearing nothing".to_owned()
    } else {
        format!("wearing {}", worn.join(", "))
    }
}

/// The screen: whose kit and Close, what they wear, the kit, then the stores beside it.
#[must_use]
pub fn content(data: &Data, game: Game, forms: &Forms) -> Screen {
    let Some(owner) = owner(game, forms) else {
        return super::empty(ScreenId::Inventory);
    };
    let party = &game.views.party;
    let mut kit = vec![
        Row::controls(vec![
            owner_choice(game, owner.member, "Kit of"),
            Control::button(ControlId::Close, "Close"),
        ]),
        Row::text(note(wearing(data, owner))),
    ];
    if game.views.combat.is_some() {
        kit.push(Row::text(note(
            "In a fight: look here, act from the fight's actions",
        )));
    }
    kit.push(heading("Kit"));
    for (i, item) in owner.equipment.iter().enumerate() {
        if let Ok(at) = u16::try_from(i) {
            kit.push(item_row(data, game, forms, at, item, &kit_acts(item)));
        }
    }
    if owner.equipment.is_empty() {
        kit.push(Row::text(note("Nothing carried")));
    }
    let mut stores = vec![
        heading("Stores"),
        Row::text(note(format!(
            "{}  food {}",
            labels::coins(party.gold),
            party.food
        ))),
    ];
    for (i, item) in party.inventory.iter().enumerate() {
        if let Ok(at) = u16::try_from(i) {
            stores.push(item_row(data, game, forms, at, item, &[ItemAct::Take]));
        }
    }
    if party.inventory.is_empty() {
        stores.push(Row::text(note("Nothing in the stores")));
    }
    Screen {
        id: ScreenId::Inventory,
        title: Span::title("Inventory"),
        panels: vec![Panel { rows: kit }, Panel { rows: stores }],
        keys: Vec::new(),
        message: None,
    }
}

/// The owner steps; Close closes; a row's button sends its command and the screen stays open.
/// The control was found live on the screen as it stands.
pub fn apply(game: Game, forms: &mut Forms, control: ControlId, payload: &Payload) -> Effect {
    match control {
        ControlId::Owner => {
            step_owner(game, forms, payload);
            Effect::None
        }
        ControlId::Close => Effect::Back,
        ControlId::Item { at, act } => {
            command(game, forms, at, act).map_or(Effect::None, |command| Effect::Command {
                command,
                close: false,
            })
        }
        _ => Effect::None,
    }
}
