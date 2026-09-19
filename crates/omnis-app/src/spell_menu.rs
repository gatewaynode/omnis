//! The spell picker in a fight: the rows the acting member's spells make, why one is grey,
//! and the keys while it is open (the `CombatMenu` opens it from Cast). Bevy-free.

use crate::combat_menu::{CombatIntent, CombatMenu, FightView};
use crate::menu::{MenuKey, cycle};
use omnis_sim::{CombatCommand, Rejection, Target};

/// One spell the acting member knows, as the picker shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellRow {
    /// Its index in the caster's list, the number `cast` takes.
    pub index: u8,
    /// The spell's name.
    pub name: String,
    /// Points it costs.
    pub cost: u32,
    /// Aimed at a member rather than a stack.
    pub targets_members: bool,
    /// For a reaction spell, whether the caster casts it on their own.
    pub auto: Option<bool>,
    /// Whether an effect of this spell by this caster is in force.
    pub active: bool,
    /// Why it cannot be cast now, in a few words.
    pub blocked: Option<String>,
}

impl SpellRow {
    /// The note after the cost: the reason it is grey, the auto-cast switch, or that it is
    /// in force.
    #[must_use]
    pub fn note(&self) -> String {
        match (&self.blocked, self.auto, self.active) {
            (_, Some(on), _) => format!("auto: {}", if on { "on" } else { "off" }),
            (Some(why), _, _) => why.clone(),
            (None, None, true) => "in force".to_owned(),
            (None, None, false) => String::new(),
        }
    }
}

/// A few words for why a spell row is grey.
pub(crate) fn blocked_note(rejection: &Rejection) -> String {
    match rejection {
        Rejection::NotEnoughPoints { need, .. } => format!("need {need} pt"),
        Rejection::MissingComponents { .. } => "needs components".to_owned(),
        Rejection::NotCastable { .. } => "not here".to_owned(),
        other => other.to_string(),
    }
}

impl CombatMenu {
    /// Keys while the picker is open.
    pub(crate) fn picker_key(
        &mut self,
        cursor: usize,
        key: MenuKey,
        view: &FightView,
        selected: Option<usize>,
    ) -> Option<CombatIntent> {
        match key {
            MenuKey::Up | MenuKey::Down => {
                self.picker = Some(cycle(cursor, view.spells.len(), key));
            }
            MenuKey::Left | MenuKey::Right => self.step_target(view, key),
            MenuKey::Enter => return self.confirm_cast(cursor, view, selected),
            MenuKey::Escape | MenuKey::Char('c') => self.picker = None,
            MenuKey::Char(_) | MenuKey::Backspace => {}
        }
        None
    }

    /// Cast the spell under the picker's cursor, or say why not.
    pub(crate) fn confirm_cast(
        &mut self,
        cursor: usize,
        view: &FightView,
        selected: Option<usize>,
    ) -> Option<CombatIntent> {
        self.message.clear();
        let Some(row) = view.spells.get(cursor) else {
            self.picker = None;
            return None;
        };
        if let Some(on) = row.auto {
            return Some(CombatIntent::AutoCast {
                spell: row.index,
                on: !on,
            });
        }
        if let Some(why) = &row.blocked {
            self.message = format!("{}: {why}", row.name);
            return None;
        }
        let target = if row.targets_members {
            match selected {
                Some(member) => Target::Member(u8::try_from(member).unwrap_or(u8::MAX)),
                None => {
                    self.message = format!("Select a member to cast {} on", row.name);
                    return None;
                }
            }
        } else {
            Target::Stack(self.target)
        };
        self.picker = None;
        Some(CombatIntent::Command(CombatCommand::Cast {
            spell: row.index,
            target,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_menu::fight_view;
    use crate::combat_menu::tests::{data, facing, facing_goblins};
    use omnis_sim::omnis_data::Data;
    use omnis_sim::{Command, EncounterChoice, Mode, World, apply};

    /// A lone wizard against a rat: the fight parks on her turn with her six spells.
    fn wizard_in_a_fight(data: &Data) -> World {
        let mut world = facing(data, &["wizard"], &[("giant_rat", 1)]);
        apply(
            &mut world,
            data,
            Command::Encounter(EncounterChoice::Attack),
        )
        .unwrap();
        assert!(matches!(world.mode, Mode::Combat(_)));
        world
    }

    #[test]
    fn cast_opens_the_picker_and_a_row_casts_at_the_target() {
        let data = data();
        let mut world = wizard_in_a_fight(&data);
        let view = fight_view(&world, &data).unwrap();
        assert_eq!(view.own, Some(0));
        assert_eq!(view.points, (1, 1), "Int 12: one point at level one");
        let names: Vec<&str> = view.spells.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Fire Bolt",
                "Light",
                "Mage Hand",
                "Magic Missile",
                "Shield",
                "Burning Hands"
            ]
        );
        assert_eq!(view.spells[2].note(), "not here", "mage hand is for doors");
        assert_eq!(view.spells[4].note(), "auto: off");
        let mut menu = CombatMenu::default();
        menu.sync(&view);
        assert_eq!(menu.key(MenuKey::Char('c'), &view, None), None);
        assert_eq!(menu.picker, Some(0));
        for _ in 0..3 {
            menu.key(MenuKey::Down, &view, None);
        }
        assert_eq!(
            menu.key(MenuKey::Enter, &view, None),
            Some(CombatIntent::Command(CombatCommand::Cast {
                spell: 3,
                target: Target::Stack(0)
            }))
        );
        assert_eq!(menu.picker, None, "the picker closes on a cast");
        menu.key(MenuKey::Char('c'), &view, None);
        for _ in 0..4 {
            menu.key(MenuKey::Down, &view, None);
        }
        assert_eq!(
            menu.key(MenuKey::Enter, &view, None),
            Some(CombatIntent::AutoCast { spell: 4, on: true }),
            "a reaction row is a switch"
        );
        menu.key(MenuKey::Escape, &view, None);
        assert_eq!(menu.picker, None, "escape closes the picker, not the fight");
        world.party.members[0].spell_points = 0;
        let view = fight_view(&world, &data).unwrap();
        assert_eq!(view.spells[3].blocked.as_deref(), Some("need 1 pt"));
        assert_eq!(view.spells[0].blocked, None, "a cantrip stays free");
        menu.key(MenuKey::Char('c'), &view, None);
        for _ in 0..3 {
            menu.key(MenuKey::Down, &view, None);
        }
        assert_eq!(menu.key(MenuKey::Enter, &view, None), None);
        assert_eq!(menu.message, "Magic Missile: need 1 pt");
        menu.key(MenuKey::Up, &view, None);
        menu.key(MenuKey::Up, &view, None);
        menu.key(MenuKey::Up, &view, None);
        assert_eq!(
            menu.key(MenuKey::Enter, &view, None),
            Some(CombatIntent::Command(CombatCommand::Cast {
                spell: 0,
                target: Target::Stack(0)
            }))
        );
    }

    #[test]
    fn heals_need_a_selected_member_and_fighters_have_no_spells() {
        let data = data();
        let mut world = facing(&data, &["cleric"], &[("giant_rat", 1)]);
        apply(
            &mut world,
            &data,
            Command::Encounter(EncounterChoice::Attack),
        )
        .unwrap();
        let view = fight_view(&world, &data).unwrap();
        let cure = view
            .spells
            .iter()
            .position(|s| s.name == "Cure Wounds")
            .expect("the cleric knows cure wounds");
        assert!(view.spells[cure].targets_members);
        let mut menu = CombatMenu::default();
        menu.key(MenuKey::Char('c'), &view, None);
        for _ in 0..cure {
            menu.key(MenuKey::Down, &view, None);
        }
        assert_eq!(menu.key(MenuKey::Enter, &view, None), None);
        assert_eq!(menu.message, "Select a member to cast Cure Wounds on");
        assert_eq!(
            menu.key(MenuKey::Enter, &view, Some(0)),
            Some(CombatIntent::Command(CombatCommand::Cast {
                spell: u8::try_from(cure).unwrap(),
                target: Target::Member(0)
            }))
        );
        let mut world = facing_goblins(&data);
        apply(
            &mut world,
            &data,
            Command::Encounter(EncounterChoice::Attack),
        )
        .unwrap();
        let view = fight_view(&world, &data).unwrap();
        assert!(view.spells.is_empty());
        let mut menu = CombatMenu::default();
        assert_eq!(menu.key(MenuKey::Char('c'), &view, None), None);
        assert_eq!(
            (menu.picker, menu.message.as_str()),
            (None, "No spells known")
        );
        assert_eq!(menu.key(MenuKey::Char('u'), &view, None), None);
        assert_eq!(menu.message, "Nothing to use yet");
    }
}
