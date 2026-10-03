//! The camp panel, Bevy-free (M7 step 8b): which control is which (`CampPanelId`), the texts it
//! rewrites (`CampLabelId`), and `apply`, which turns a control's report into the rest to ask
//! for. Everything shown comes from `omnis_sim::rest_view`: each member's hit dice and how many
//! a short rest may spend, the long rest's food and why it would be refused, so the panel adds
//! no rule of its own. The widgets are not trusted: a count is held to what the member may
//! spend, and a rest the view refuses sends nothing. `feathers_camp.rs` draws it.

use crate::service_panel::reason;
use crate::ui_model::{Payload, whole};
use omnis_sim::omnis_core::fnv1a64;
use omnis_sim::{RestCommand, RestView, World};

/// One control of the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum CampPanelId {
    /// A member's hit dice to spend, by party slot.
    Dice(usize),
    /// Rest an hour, spending the dice chosen.
    Short,
    /// Rest the night.
    Long,
    /// Back to the map.
    #[default]
    Close,
}

/// A text the panel rewrites from the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CampLabelId {
    /// "The night eats 3 food; the stores hold 7".
    #[default]
    Food,
    /// A member's line, by party slot: "Brenna  HP 5/12 · Hit dice 2/3 d10".
    Member(usize),
    /// How many dice a member spends, or why none.
    Dice(usize),
    /// Why the long rest would be refused, or what it gives.
    Long,
    /// The last refusal.
    Message,
}

/// What the panel keeps between frames: the dice chosen and the last refusal.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CampForm {
    /// Hit dice each member spends, in party order.
    pub dice: Vec<u8>,
    /// The last refusal, or empty.
    pub message: String,
}

impl CampForm {
    /// Hold the choices to what the view allows now: one count per member, none above what
    /// that member may spend.
    pub fn fit(&mut self, view: &RestView) {
        self.dice.resize(view.members.len(), 0);
        for (count, member) in self.dice.iter_mut().zip(&view.members) {
            *count = (*count).min(member.spendable);
        }
    }

    /// Whether any member spends a die.
    #[must_use]
    pub fn spends(&self) -> bool {
        self.dice.iter().any(|d| *d > 0)
    }
}

/// What a control asks of the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CampAsk {
    /// Send this rest.
    Rest(RestCommand),
    /// Back to the map.
    Close,
}

/// Apply one control's report. `view` is the camp as it is now.
pub fn apply(
    id: CampPanelId,
    payload: &Payload,
    view: &RestView,
    form: &mut CampForm,
) -> Option<CampAsk> {
    form.fit(view);
    match (id, payload) {
        (CampPanelId::Dice(slot), Payload::Slide(value)) => {
            let member = view.members.get(slot)?;
            let wanted = whole(*value)?.clamp(0, i64::from(member.spendable));
            form.dice[slot] = u8::try_from(wanted).ok()?;
            None
        }
        (CampPanelId::Short, Payload::Activate) if view.refusal.is_none() && form.spends() => {
            Some(CampAsk::Rest(RestCommand::Short {
                dice: form.dice.clone(),
            }))
        }
        (CampPanelId::Long, Payload::Activate) if view.long.is_none() => {
            Some(CampAsk::Rest(RestCommand::Long))
        }
        (CampPanelId::Close, Payload::Activate) => Some(CampAsk::Close),
        _ => None,
    }
}

/// A member's line: name, hit points and hit dice.
#[must_use]
pub fn member_line(view: &RestView, world: &World, slot: usize) -> String {
    let (Some(member), Some(character)) = (view.members.get(slot), world.party.members.get(slot))
    else {
        return String::new();
    };
    format!(
        "{}  HP {}/{} · Hit dice {}/{} d{}",
        character.name, member.hp, member.hp_max, member.dice_left, member.dice, member.die
    )
}

/// How many dice a member spends, or why none may be.
#[must_use]
pub fn dice_note(view: &RestView, form: &CampForm, slot: usize) -> String {
    let Some(member) = view.members.get(slot) else {
        return String::new();
    };
    match (member.spendable, member.dice_left) {
        (0, 0) => "no hit dice left".to_owned(),
        (0, _) if member.hp >= member.hp_max => "unhurt".to_owned(),
        (0, _) => "cannot spend".to_owned(),
        _ => {
            let count = form.dice.get(slot).copied().unwrap_or(0);
            format!("spend {count}")
        }
    }
}

/// The food line.
#[must_use]
pub fn food_line(view: &RestView) -> String {
    format!(
        "The night eats {} food; the stores hold {}",
        view.long_food, view.food
    )
}

/// What the long rest gives, or why it would be refused.
#[must_use]
pub fn long_note(view: &RestView) -> String {
    view.long.as_ref().map_or_else(
        || "eight hours: full hit points and spell points, half the hit dice back".to_owned(),
        reason,
    )
}

/// What the panel's entity tree depends on: who may spend dice and how many (a slider's
/// range), and how many members there are. Counts and notes are rewritten in place.
#[must_use]
pub fn shape(view: &RestView) -> u64 {
    let mut bytes = view.members.len().to_le_bytes().to_vec();
    bytes.extend(view.members.iter().map(|m| m.spendable));
    bytes.push(u8::from(view.refusal.is_some()));
    fnv1a64(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnis_sim::{CampMember, Rejection};

    fn member(hp: i32, dice_left: u8, spendable: u8) -> CampMember {
        CampMember {
            hp,
            hp_max: 12,
            dice_left,
            dice: 3,
            die: 10,
            spendable,
        }
    }

    fn view() -> RestView {
        RestView {
            refusal: None,
            members: vec![member(5, 2, 2), member(12, 3, 0), member(4, 0, 0)],
            long_food: 3,
            food: 7,
            long: None,
        }
    }

    #[test]
    fn counts_are_held_to_what_a_member_may_spend() {
        let view = view();
        let mut form = CampForm::default();
        let slide = |form: &mut CampForm, slot, value| {
            apply(CampPanelId::Dice(slot), &Payload::Slide(value), &view, form)
        };
        assert_eq!(slide(&mut form, 0, 9.0), None);
        assert_eq!(form.dice, vec![2, 0, 0], "held to two");
        slide(&mut form, 0, -3.0);
        assert_eq!(form.dice[0], 0);
        slide(&mut form, 1, 2.0);
        assert_eq!(form.dice[1], 0, "an unhurt member spends none");
        slide(&mut form, 0, f32::NAN);
        assert_eq!(form.dice[0], 0, "not a number changes nothing");
        assert_eq!(slide(&mut form, 9, 1.0), None, "no such member");
        assert_eq!(
            apply(CampPanelId::Short, &Payload::Activate, &view, &mut form),
            None,
            "nothing chosen, nothing sent"
        );
        slide(&mut form, 0, 1.4);
        assert_eq!(
            apply(CampPanelId::Short, &Payload::Activate, &view, &mut form),
            Some(CampAsk::Rest(RestCommand::Short {
                dice: vec![1, 0, 0]
            }))
        );
    }

    #[test]
    fn a_refused_rest_sends_nothing() {
        let mut refused = view();
        refused.long = Some(Rejection::NoFood { need: 3, have: 1 });
        let mut form = CampForm {
            dice: vec![1, 0, 0],
            message: String::new(),
        };
        assert_eq!(
            apply(CampPanelId::Long, &Payload::Activate, &refused, &mut form),
            None
        );
        refused.refusal = Some(Rejection::WrongMode);
        assert_eq!(
            apply(CampPanelId::Short, &Payload::Activate, &refused, &mut form),
            None
        );
        assert_eq!(
            apply(CampPanelId::Long, &Payload::Activate, &view(), &mut form),
            Some(CampAsk::Rest(RestCommand::Long))
        );
        assert_eq!(
            apply(CampPanelId::Close, &Payload::Activate, &refused, &mut form),
            Some(CampAsk::Close)
        );
    }

    #[test]
    fn a_choice_above_a_new_view_is_cut_back() {
        let mut form = CampForm {
            dice: vec![2, 0, 0, 1],
            message: String::new(),
        };
        let mut healed = view();
        healed.members[0].spendable = 1;
        form.fit(&healed);
        assert_eq!(form.dice, vec![1, 0, 0]);
    }

    #[test]
    fn the_notes_say_why_none_may_be_spent() {
        let view = view();
        let form = CampForm {
            dice: vec![2, 0, 0],
            message: String::new(),
        };
        assert_eq!(dice_note(&view, &form, 0), "spend 2");
        assert_eq!(dice_note(&view, &form, 1), "unhurt");
        assert_eq!(dice_note(&view, &form, 2), "no hit dice left");
        assert_eq!(food_line(&view), "The night eats 3 food; the stores hold 7");
        let mut soon = view.clone();
        soon.long = Some(Rejection::RestTooSoon { minutes: 605 });
        assert_eq!(long_note(&soon), "rested too recently (10h 05m)");
        assert_eq!(
            shape(&view),
            shape(&soon),
            "a refusal is rewritten in place"
        );
        let mut fewer = view.clone();
        fewer.members[0].spendable = 1;
        assert_ne!(shape(&view), shape(&fewer), "a slider's range changed");
    }
}
