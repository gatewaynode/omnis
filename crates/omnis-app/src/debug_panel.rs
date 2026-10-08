//! The debug panel as a model, Bevy-free (`feathers_debug.rs` draws it): what each control is,
//! what each text shows, which controls are grey, and how a report becomes a `Dev` command.
//! A number bound to the world is typed and sent once, when Enter or leaving the field commits
//! it; the selectors, the item count, the flag value and the teleport's tile are kept here
//! until a button acts on them. No limits beyond the commands' integer types: the simulation
//! decides what a value means (owner, 2026-10-04).

use crate::debug_menu::DebugView;
use crate::ui_model::Payload;
use omnis_sim::DevCommand;
use omnis_sim::omnis_core::{Facing, fnv1a64};
use omnis_sim::omnis_data::Ability;

/// The four facings the facing menu offers.
pub const FACINGS: [Facing; 4] = [Facing::North, Facing::East, Facing::South, Facing::West];

/// One control of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DebugPanelId {
    /// The member menu.
    Member,
    /// A member in the menu.
    MemberPick(usize),
    /// Hit points.
    Hp,
    /// Spell points.
    Sp,
    /// Experience.
    Xp,
    /// An ability score, in SRD order.
    Score(usize),
    /// The condition menu.
    Condition,
    /// A condition in the menu.
    ConditionPick(usize),
    /// The chosen condition on or off.
    Toggle,
    /// The purse, in copper.
    Gold,
    /// Food.
    Food,
    /// An item in the list.
    Item(usize),
    /// How many to give.
    Count,
    /// Give to the member.
    GiveMember,
    /// Give to the party's stores.
    GiveStores,
    /// The flag menu.
    Flag,
    /// A flag in the menu.
    FlagPick(usize),
    /// The value to set.
    FlagValue,
    /// Set the flag.
    SetFlag,
    /// The map menu.
    Map,
    /// A map in the menu.
    MapPick(usize),
    /// The teleport's column.
    X,
    /// The teleport's row.
    Y,
    /// The facing menu.
    Facing,
    /// A facing in the menu.
    FacingPick(usize),
    /// Teleport.
    Go,
    /// The stack menu.
    Stack,
    /// A stack in the menu.
    StackPick(usize),
    /// The stack's lead individual's hit points.
    StackHp,
    /// Kill the stack.
    Kill,
    /// Close the panel.
    Close,
}

/// One text the panel rewrites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugLabelId {
    /// The member menu's caption.
    Member,
    /// "of N" after hit points.
    HpMax,
    /// "of N" after spell points.
    SpMax,
    /// The level after experience.
    Level,
    /// The condition menu's caption.
    Condition,
    /// Whether the member has the chosen condition.
    ConditionState,
    /// The chosen item.
    Item,
    /// The flag menu's caption.
    Flag,
    /// The map menu's caption.
    Map,
    /// The facing menu's caption.
    Facing,
    /// The stack menu's caption.
    Stack,
    /// The last refusal.
    Message,
}

/// What the panel asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugAsk {
    /// A dev command.
    Send(DevCommand),
    /// Close the panel.
    Close,
}

/// What the panel keeps between reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugForm {
    /// The member the member rows edit, by slot.
    pub member: usize,
    /// The condition the toggle sets.
    pub condition: usize,
    /// The item the give buttons hand out.
    pub item: usize,
    /// How many.
    pub count: u16,
    /// The flag Set writes.
    pub flag: usize,
    /// The value it writes.
    pub flag_value: i64,
    /// The map the teleport goes to.
    pub map: usize,
    /// The tile it goes to.
    pub tile: (u16, u16),
    /// The facing it ends with, by index into `FACINGS`.
    pub facing: usize,
    /// The stack the fight rows edit.
    pub stack: usize,
    /// The last number sent, so the commit that follows Enter when the field loses focus
    /// sends nothing again.
    pub sent: Option<(DebugPanelId, i64)>,
    /// Why the last report did nothing; empty when it did something.
    pub message: String,
}

impl DebugForm {
    /// A form for the world as it is: the teleport starts where the party stands.
    #[must_use]
    pub fn open(view: &DebugView) -> Self {
        let (map, x, y, facing) = view.position;
        DebugForm {
            member: 0,
            condition: 0,
            item: 0,
            count: 1,
            flag: 0,
            flag_value: 1,
            map,
            tile: (x, y),
            facing: FACINGS.iter().position(|f| *f == facing).unwrap_or(0),
            stack: 0,
            sent: None,
            message: String::new(),
        }
    }

    /// Keep every selector inside its list (a member who left, a stack that fell).
    pub fn sync(&mut self, view: &DebugView) {
        let clamp = |at: &mut usize, len: usize| *at = (*at).min(len.saturating_sub(1));
        clamp(&mut self.member, view.members.len());
        clamp(&mut self.condition, view.conditions.len());
        clamp(&mut self.item, view.items.len());
        clamp(&mut self.flag, view.flags.len());
        clamp(&mut self.map, view.maps.len());
        clamp(&mut self.stack, view.stacks.len());
    }

    /// The member's slot as the commands want it.
    fn slot(&self) -> u8 {
        u8::try_from(self.member).unwrap_or(u8::MAX)
    }
}

/// `n` as the command's integer type, saturating at its ends.
fn fit<T: TryFrom<i64> + Bounded>(n: i64) -> T {
    T::try_from(n).unwrap_or(if n < 0 { T::LOW } else { T::HIGH })
}

/// The ends of an integer type.
trait Bounded {
    const LOW: Self;
    const HIGH: Self;
}

macro_rules! bounded {
    ($($t:ty),*) => {
        $(impl Bounded for $t {
            const LOW: Self = <$t>::MIN;
            const HIGH: Self = <$t>::MAX;
        })*
    };
}

bounded!(u8, u16, u32, i32);

/// Answer a report: move a selector, keep a staged value, or ask for a command.
pub fn apply(
    id: DebugPanelId,
    payload: &Payload,
    view: &DebugView,
    form: &mut DebugForm,
) -> Option<DebugAsk> {
    use DebugPanelId as P;
    let typed = match payload {
        Payload::Number(n) | Payload::Commit(n) => Some(*n),
        _ => None,
    };
    let committed = match payload {
        Payload::Commit(n) if form.sent == Some((id, *n)) => return None,
        Payload::Commit(n) if number(id, view, form) == Some(*n) => return None,
        Payload::Commit(n) => {
            form.sent = Some((id, *n));
            Some(*n)
        }
        Payload::Activate => {
            form.sent = None;
            None
        }
        _ => None,
    };
    let activated = *payload == Payload::Activate;
    let member = view.members.get(form.member);
    let command = match id {
        P::Close if activated => return Some(DebugAsk::Close),
        P::MemberPick(at) if activated => {
            form.member = at;
            return None;
        }
        P::ConditionPick(at) if activated => {
            form.condition = at;
            return None;
        }
        P::Item(at) if activated => {
            form.item = at;
            return None;
        }
        P::FlagPick(at) if activated => {
            form.flag = at;
            form.flag_value = 1;
            return None;
        }
        P::MapPick(at) if activated => {
            form.map = at;
            let (w, h) = view.maps.get(at).map_or((1, 1), |m| (m.1, m.2));
            form.tile = (
                form.tile.0.min(w.saturating_sub(1)),
                form.tile.1.min(h.saturating_sub(1)),
            );
            return None;
        }
        P::FacingPick(at) if activated => {
            form.facing = at.min(FACINGS.len() - 1);
            return None;
        }
        P::StackPick(at) if activated => {
            form.stack = at;
            return None;
        }
        P::Count => {
            form.count = fit::<u16>(typed?).max(1);
            return None;
        }
        P::FlagValue => {
            form.flag_value = typed?;
            return None;
        }
        P::X => {
            form.tile.0 = fit(typed?);
            return None;
        }
        P::Y => {
            form.tile.1 = fit(typed?);
            return None;
        }
        P::Gold => DevCommand::SetGold {
            gold: fit(committed?),
        },
        P::Food => DevCommand::SetFood {
            food: fit(committed?),
        },
        P::Hp | P::Sp | P::Xp | P::Score(_) | P::Toggle | P::GiveMember if member.is_none() => {
            form.message = "No member to edit".to_owned();
            return None;
        }
        P::Hp => DevCommand::SetHp {
            member: form.slot(),
            hp: fit(committed?),
        },
        P::Sp => DevCommand::SetSpellPoints {
            member: form.slot(),
            points: fit(committed?),
        },
        P::Xp => DevCommand::SetXp {
            member: form.slot(),
            xp: fit(committed?),
        },
        P::Score(at) => DevCommand::SetScore {
            member: form.slot(),
            ability: *Ability::ALL.get(at)?,
            score: fit(committed?),
        },
        P::Toggle if activated => {
            let (condition, _) = view.conditions.get(form.condition)?;
            DevCommand::SetCondition {
                member: form.slot(),
                applied: !member?.conditions.contains(condition),
                condition: condition.clone(),
            }
        }
        P::GiveMember | P::GiveStores if activated => DevCommand::GiveItem {
            member: (id == P::GiveMember).then(|| form.slot()),
            item: view.items.get(form.item)?.0.clone(),
            count: form.count,
        },
        P::SetFlag if activated => DevCommand::SetFlag {
            flag: view.flags.get(form.flag)?.0.clone(),
            value: form.flag_value,
        },
        P::Go if activated && view.fighting => {
            form.message = "No teleport in a fight".to_owned();
            return None;
        }
        P::Go if activated => DevCommand::Teleport {
            map: view.maps.get(form.map)?.0.clone(),
            x: form.tile.0,
            y: form.tile.1,
            facing: FACINGS[form.facing.min(FACINGS.len() - 1)],
        },
        P::StackHp | P::Kill if !view.fighting => {
            form.message = "No fight is on".to_owned();
            return None;
        }
        P::StackHp => DevCommand::SetMonsterHp {
            stack: view.stacks.get(form.stack)?.index,
            index: 0,
            hp: fit(committed?),
        },
        P::Kill if activated => DevCommand::KillStack {
            stack: view.stacks.get(form.stack)?.index,
        },
        _ => return None,
    };
    Some(DebugAsk::Send(command))
}

/// The value a number field shows: the world's for the bound ones, the form's for the staged.
#[must_use]
pub fn number(id: DebugPanelId, view: &DebugView, form: &DebugForm) -> Option<i64> {
    use DebugPanelId as P;
    let member = view.members.get(form.member);
    Some(match id {
        P::Hp => i64::from(member?.hp.0),
        P::Sp => i64::from(member?.sp.0),
        P::Xp => i64::from(member?.xp),
        P::Score(at) => i64::from(*member?.scores.get(at)?),
        P::Gold => i64::from(view.gold),
        P::Food => i64::from(view.food),
        P::Count => i64::from(form.count),
        P::FlagValue => form.flag_value,
        P::X => i64::from(form.tile.0),
        P::Y => i64::from(form.tile.1),
        P::StackHp => i64::from(view.stacks.get(form.stack)?.lead_hp),
        _ => return None,
    })
}

/// Whether a control would do nothing now: the member rows without members, Go in a fight, the
/// stack rows outside one.
#[must_use]
pub fn dim(id: DebugPanelId, view: &DebugView) -> bool {
    use DebugPanelId as P;
    match id {
        P::Hp | P::Sp | P::Xp | P::Score(_) | P::Toggle | P::GiveMember => view.members.is_empty(),
        P::Go => view.fighting,
        P::StackHp | P::Kill => !view.fighting || view.stacks.is_empty(),
        _ => false,
    }
}

/// The text a label shows.
#[must_use]
pub fn label_text(label: DebugLabelId, view: &DebugView, form: &DebugForm) -> String {
    use DebugLabelId as L;
    let member = view.members.get(form.member);
    match label {
        L::Member => member.map_or_else(|| "no members".to_owned(), |m| m.name.clone()),
        L::HpMax => member.map_or_else(String::new, |m| format!("of {}", m.hp.1)),
        L::SpMax => member.map_or_else(String::new, |m| format!("of {}", m.sp.1)),
        L::Level => member.map_or_else(String::new, |m| {
            format!(
                "level {} (more experience levels at once; less keeps it)",
                m.level
            )
        }),
        L::Condition => view
            .conditions
            .get(form.condition)
            .map_or_else(String::new, |c| c.1.clone()),
        L::ConditionState => match (member, view.conditions.get(form.condition)) {
            (Some(m), Some((id, _))) if m.conditions.contains(id) => "on".to_owned(),
            (Some(_), Some(_)) => "off".to_owned(),
            _ => String::new(),
        },
        L::Item => view
            .items
            .get(form.item)
            .map_or_else(String::new, |i| format!("Item: {}", i.1)),
        L::Flag => view
            .flags
            .get(form.flag)
            .map_or_else(String::new, |f| format!("{} (now {})", f.0, f.1)),
        L::Map => view
            .maps
            .get(form.map)
            .map_or_else(String::new, |m| format!("{} ({} by {})", m.0, m.1, m.2)),
        L::Facing => FACINGS[form.facing.min(FACINGS.len() - 1)].to_string(),
        L::Stack => view.stacks.get(form.stack).map_or_else(
            || "no fight".to_owned(),
            |s| format!("{} ({} of {})", s.name, s.count.0, s.count.1),
        ),
        L::Message => form.message.clone(),
    }
}

/// What the entity tree depends on: the menus' and the list's entries and whether a fight is
/// on. Numbers, captions and the grey state are rewritten in place.
#[must_use]
pub fn shape(view: &DebugView) -> u64 {
    let mut bytes = Vec::new();
    for m in &view.members {
        bytes.extend(m.name.as_bytes());
        bytes.push(0);
    }
    for n in [
        view.conditions.len(),
        view.items.len(),
        view.flags.len(),
        view.maps.len(),
    ] {
        bytes.extend(n.to_le_bytes());
    }
    for s in &view.stacks {
        bytes.extend(s.name.as_bytes());
        bytes.push(s.count.0);
    }
    bytes.push(u8::from(view.fighting));
    fnv1a64(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_menu::tests::debug_view;
    use crate::debug_menu::tests::world_and_data;

    #[test]
    fn a_number_is_sent_once_when_committed_and_has_no_limit_but_its_type() {
        let (world, data) = world_and_data();
        let view = debug_view(&world, &data);
        let mut form = DebugForm::open(&view);
        let mut put = |id, payload: Payload| apply(id, &payload, &view, &mut form);
        assert_eq!(put(DebugPanelId::Hp, Payload::Number(12)), None, "typing");
        assert_eq!(
            put(DebugPanelId::Hp, Payload::Commit(120)),
            Some(DebugAsk::Send(DevCommand::SetHp { member: 0, hp: 120 })),
            "above the maximum"
        );
        assert_eq!(
            put(DebugPanelId::Hp, Payload::Commit(120)),
            None,
            "the commit when the field loses focus after Enter sends nothing again"
        );
        assert_eq!(
            put(DebugPanelId::Score(2), Payload::Commit(300)),
            Some(DebugAsk::Send(DevCommand::SetScore {
                member: 0,
                ability: Ability::Constitution,
                score: 255
            })),
            "a score saturates at its type"
        );
        assert_eq!(
            put(DebugPanelId::Gold, Payload::Commit(-5)),
            Some(DebugAsk::Send(DevCommand::SetGold { gold: 0 }))
        );
        assert_eq!(put(DebugPanelId::Count, Payload::Number(3)), None);
        assert_eq!(
            put(DebugPanelId::GiveStores, Payload::Activate),
            Some(DebugAsk::Send(DevCommand::GiveItem {
                member: None,
                item: view.items[0].0.clone(),
                count: 3
            }))
        );
        assert_eq!(number(DebugPanelId::Count, &view, &form), Some(3));
        assert_eq!(
            number(DebugPanelId::Hp, &view, &form),
            Some(i64::from(view.members[0].hp.0)),
            "a bound number shows the world"
        );
    }

    #[test]
    fn selectors_stage_and_buttons_act() {
        let (world, data) = world_and_data();
        let view = debug_view(&world, &data);
        let mut form = DebugForm::open(&view);
        assert_eq!(form.tile, (view.position.1, view.position.2));
        let mut put = |id, payload: Payload| apply(id, &payload, &view, &mut form);
        put(DebugPanelId::Item(1), Payload::Activate);
        assert_eq!(
            put(DebugPanelId::GiveMember, Payload::Activate),
            Some(DebugAsk::Send(DevCommand::GiveItem {
                member: Some(0),
                item: view.items[1].0.clone(),
                count: 1
            }))
        );
        put(DebugPanelId::ConditionPick(2), Payload::Activate);
        assert_eq!(
            put(DebugPanelId::Toggle, Payload::Activate),
            Some(DebugAsk::Send(DevCommand::SetCondition {
                member: 0,
                condition: view.conditions[2].0.clone(),
                applied: true
            }))
        );
        put(DebugPanelId::FacingPick(0), Payload::Activate);
        put(DebugPanelId::X, Payload::Number(4));
        assert_eq!(
            put(DebugPanelId::Go, Payload::Activate),
            Some(DebugAsk::Send(DevCommand::Teleport {
                map: view.maps[view.position.0].0.clone(),
                x: 4,
                y: view.position.2,
                facing: Facing::North
            }))
        );
        assert_eq!(put(DebugPanelId::Kill, Payload::Activate), None);
        assert_eq!(form.message, "No fight is on");
        assert!(dim(DebugPanelId::Kill, &view) && !dim(DebugPanelId::Go, &view));
        assert_eq!(
            apply(DebugPanelId::Close, &Payload::Activate, &view, &mut form),
            Some(DebugAsk::Close)
        );
        assert_eq!(
            label_text(DebugLabelId::ConditionState, &view, &form),
            "off"
        );
        assert!(label_text(DebugLabelId::Level, &view, &form).starts_with("level 1"));
    }
}
