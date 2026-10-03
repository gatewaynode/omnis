//! The encounter and defeat menus as pure state machines (ARCHITECTURE.md §8.1): the choice
//! before a fight and the modal after a lost one. Each takes a key and answers with an intent;
//! `combat.rs` feeds keys and applies intents. Bevy-free, split from `combat_menu.rs`.

use crate::combat_menu::FightView;
use crate::menu::{MenuKey, cycle};
use omnis_sim::EncounterChoice;
use omnis_sim::omnis_core::money::gp_floor;

// ---------------------------------------------------------------- encounter

/// What the encounter menu asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterIntent {
    /// One of the four choices.
    Choice(EncounterChoice),
    /// Open the pause overlay.
    Pause,
}

/// The choice before a fight: arrows cycle the four, Enter confirms, `a b h r` are hotkeys,
/// Escape pauses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EncounterMenu {
    /// The choice under the cursor.
    pub cursor: usize,
    /// Why the last confirmation did nothing; empty when it did something.
    pub message: String,
}

impl EncounterMenu {
    /// The choices, in cursor order (the bribe's text comes from the view).
    pub const ACTIONS: [&'static str; 4] = ["Attack", "Bribe", "Hide", "Run"];
    /// The hotkeys, in the same order.
    pub const HOTKEYS: [char; 4] = ['a', 'b', 'h', 'r'];
    /// The choices, in the same order.
    const CHOICES: [EncounterChoice; 4] = [
        EncounterChoice::Attack,
        EncounterChoice::Bribe,
        EncounterChoice::Hide,
        EncounterChoice::Run,
    ];

    /// Handle a key.
    pub fn key(&mut self, key: MenuKey, view: &FightView) -> Option<EncounterIntent> {
        match key {
            MenuKey::Up | MenuKey::Down | MenuKey::Left | MenuKey::Right => {
                self.cursor = cycle(self.cursor, Self::ACTIONS.len(), key);
            }
            MenuKey::Enter => return self.confirm(view),
            MenuKey::Char(c) => {
                if let Some(at) = Self::HOTKEYS.iter().position(|h| *h == c) {
                    self.cursor = at;
                    return self.confirm(view);
                }
            }
            MenuKey::Escape => return Some(EncounterIntent::Pause),
            MenuKey::Backspace => {}
        }
        None
    }

    fn confirm(&mut self, view: &FightView) -> Option<EncounterIntent> {
        self.message.clear();
        let choice = Self::CHOICES[self.cursor.min(3)];
        if choice == EncounterChoice::Bribe && !view.bribe_allowed() {
            self.message = match view.bribe {
                Some(cost) => format!(
                    "Not enough gold: {} needed, {} carried",
                    gp_floor(cost),
                    gp_floor(view.gold)
                ),
                None => "They cannot be bribed".to_owned(),
            };
            return None;
        }
        Some(EncounterIntent::Choice(choice))
    }
}

// ---------------------------------------------------------------- defeat

/// What the defeat modal asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefeatAction {
    /// Load the last save.
    Load,
    /// Drop the game and return to the title.
    QuitToTitle,
}

/// "The party has fallen": two choices, no escape.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefeatMenu {
    /// The choice under the cursor.
    pub cursor: usize,
}

impl DefeatMenu {
    /// The choices, in cursor order.
    pub const ITEMS: [&'static str; 2] = ["Load last save", "Quit to title"];

    /// Handle a key.
    pub fn key(&mut self, key: MenuKey) -> Option<DefeatAction> {
        match key {
            MenuKey::Up | MenuKey::Down => self.cursor = cycle(self.cursor, 2, key),
            MenuKey::Enter => {
                return Some(if self.cursor == 0 {
                    DefeatAction::Load
                } else {
                    DefeatAction::QuitToTitle
                });
            }
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_menu::tests::view_with;
    use omnis_sim::ModeKind;

    #[test]
    fn encounter_keys_cycle_and_gate_the_bribe() {
        let mut view = view_with(&[(0, true, None)], None);
        view.phase = ModeKind::Encounter;
        view.bribe = Some(1200);
        view.gold = 1199;
        let mut menu = EncounterMenu::default();
        assert_eq!(menu.key(MenuKey::Right, &view), None);
        assert_eq!(menu.cursor, 1);
        assert_eq!(menu.key(MenuKey::Enter, &view), None);
        assert_eq!(
            menu.message, "Not enough gold: 12 needed, 11 carried",
            "a copper short shows the purse rounded down"
        );
        assert_eq!(view.bribe_label(), "Bribe 12g");
        view.gold = 1200;
        assert_eq!(
            menu.key(MenuKey::Enter, &view),
            Some(EncounterIntent::Choice(EncounterChoice::Bribe))
        );
        assert!(menu.message.is_empty());
        view.bribe = Some(0);
        assert_eq!(view.bribe_label(), "Bribe free");
        assert!(view.bribe_allowed());
        view.bribe = None;
        assert_eq!(menu.key(MenuKey::Enter, &view), None);
        assert_eq!(menu.message, "They cannot be bribed");
        menu.key(MenuKey::Up, &view);
        assert_eq!(menu.cursor, 0);
        menu.key(MenuKey::Left, &view);
        assert_eq!(menu.cursor, 3, "wraps");
        assert_eq!(
            menu.key(MenuKey::Char('h'), &view),
            Some(EncounterIntent::Choice(EncounterChoice::Hide))
        );
        assert_eq!(
            menu.key(MenuKey::Char('r'), &view),
            Some(EncounterIntent::Choice(EncounterChoice::Run))
        );
        assert_eq!(
            menu.key(MenuKey::Char('a'), &view),
            Some(EncounterIntent::Choice(EncounterChoice::Attack))
        );
        assert_eq!(
            menu.key(MenuKey::Escape, &view),
            Some(EncounterIntent::Pause)
        );
    }

    #[test]
    fn the_defeat_menu_loads_or_quits() {
        let mut menu = DefeatMenu::default();
        assert_eq!(menu.key(MenuKey::Escape), None, "no way out but the two");
        assert_eq!(menu.key(MenuKey::Enter), Some(DefeatAction::Load));
        menu.key(MenuKey::Down);
        assert_eq!(menu.key(MenuKey::Enter), Some(DefeatAction::QuitToTitle));
        menu.key(MenuKey::Down);
        assert_eq!(menu.cursor, 0, "wraps");
    }
}
