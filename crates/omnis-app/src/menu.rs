//! The menus as pure state machines: title, new game (seed and difficulty), character
//! creation by point buy, and the pause overlay. Each screen is a value that takes a key and
//! answers with an action, and renders itself as lines of text. Bevy-free, so every
//! transition is unit-tested; `menus.rs` only spawns text and feeds keys.

use omnis_sim::omnis_core::fnv1a64;
use omnis_sim::{SaveRule, Settings};

/// A key as the menus see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Enter or Space on a button.
    Enter,
    /// Escape.
    Escape,
    /// Backspace.
    Backspace,
    /// A printable character.
    Char(char),
}

/// The help line every screen ends with.
pub(crate) const HELP: &str = "Up/Down select   Left/Right change   Enter confirm   Esc back";

/// Camel-case variant names as words: `LawfulGood` to `Lawful Good`.
#[must_use]
pub fn words(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_ascii_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

pub(crate) fn cycle(index: usize, len: usize, key: MenuKey) -> usize {
    if len == 0 {
        return 0;
    }
    match key {
        MenuKey::Left | MenuKey::Up => (index + len - 1) % len,
        MenuKey::Right | MenuKey::Down => (index + 1) % len,
        _ => index,
    }
}

pub(crate) fn mark(cursor: usize, row: usize, text: String) -> String {
    if cursor == row {
        format!("> {text}")
    } else {
        format!("  {text}")
    }
}

// ---------------------------------------------------------------- title

/// What the title screen asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleAction {
    /// Go to the new game screen.
    NewGame,
    /// Load the quick save.
    Load,
    /// Exit.
    Quit,
}

/// The title screen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Title {
    /// Selected item.
    pub cursor: usize,
}

impl Title {
    /// The items, in cursor order.
    pub const ITEMS: [&'static str; 3] = ["New game", "Load quick save", "Quit"];

    /// Handle a key.
    pub fn key(&mut self, key: MenuKey) -> Option<TitleAction> {
        match key {
            MenuKey::Up | MenuKey::Down => self.cursor = cycle(self.cursor, 3, key),
            MenuKey::Enter => {
                return Some(match self.cursor {
                    0 => TitleAction::NewGame,
                    1 => TitleAction::Load,
                    _ => TitleAction::Quit,
                });
            }
            _ => {}
        }
        None
    }

    /// The screen as lines.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec!["OMNIS".to_owned(), String::new()];
        lines.extend(
            Self::ITEMS
                .iter()
                .enumerate()
                .map(|(i, item)| mark(self.cursor, i, (*item).to_owned())),
        );
        lines.push(String::new());
        lines.push(HELP.to_owned());
        lines
    }
}

/// "on" or "off".
#[must_use]
pub const fn on_off(flag: bool) -> &'static str {
    if flag { "on" } else { "off" }
}

// ---------------------------------------------------------------- new game

/// What the new game screen asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewGameAction {
    /// Start with the form's seed and settings.
    Start,
    /// Back to the title.
    Back,
}

/// Seed text plus the difficulty controls (PRD D17).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewGameForm {
    /// What was typed; blank means a random seed.
    pub seed_text: String,
    /// The difficulty.
    pub settings: Settings,
    /// Selected row: seed, save rule, permadeath, start, back.
    pub cursor: usize,
}

/// The save rule as the screens name it.
#[must_use]
pub fn rule_label(rule: SaveRule) -> &'static str {
    match rule {
        SaveRule::Anywhere => "Anywhere",
        SaveRule::Relief => "Inns, items, and spells",
        SaveRule::InnOnly => "Inns only",
    }
}

impl NewGameForm {
    const RULES: [SaveRule; 3] = [SaveRule::Anywhere, SaveRule::Relief, SaveRule::InnOnly];

    /// The seed: a number as typed, any other text hashed, blank as `entropy`.
    #[must_use]
    pub fn seed(&self, entropy: u64) -> u64 {
        let text = self.seed_text.trim();
        if text.is_empty() {
            entropy
        } else {
            text.parse().unwrap_or_else(|_| fnv1a64(text.as_bytes()))
        }
    }

    /// Handle a key.
    pub fn key(&mut self, key: MenuKey) -> Option<NewGameAction> {
        match key {
            MenuKey::Up | MenuKey::Down => self.cursor = cycle(self.cursor, 5, key),
            MenuKey::Left | MenuKey::Right => match self.cursor {
                1 => {
                    let at = Self::RULES
                        .iter()
                        .position(|r| *r == self.settings.save_rule)
                        .unwrap_or(0);
                    self.settings.save_rule = Self::RULES[cycle(at, 3, key)];
                }
                2 => self.settings.permadeath = !self.settings.permadeath,
                _ => {}
            },
            MenuKey::Char(c) if self.cursor == 0 && self.seed_text.len() < 32 => {
                self.seed_text.push(c);
            }
            MenuKey::Backspace if self.cursor == 0 => {
                self.seed_text.pop();
            }
            MenuKey::Enter => match self.cursor {
                3 => return Some(NewGameAction::Start),
                4 => return Some(NewGameAction::Back),
                _ => {}
            },
            MenuKey::Escape => return Some(NewGameAction::Back),
            _ => {}
        }
        None
    }

    /// The screen as lines.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let rule = rule_label(self.settings.save_rule);
        vec![
            "NEW GAME".to_owned(),
            String::new(),
            mark(
                self.cursor,
                0,
                format!(
                    "Seed: {}_   (blank for random; words are hashed)",
                    self.seed_text
                ),
            ),
            mark(self.cursor, 1, format!("Saving: < {rule} >")),
            mark(
                self.cursor,
                2,
                format!(
                    "Permadeath: < {} >",
                    if self.settings.permadeath {
                        "On"
                    } else {
                        "Off"
                    }
                ),
            ),
            mark(self.cursor, 3, "Start".to_owned()),
            mark(self.cursor, 4, "Back".to_owned()),
            String::new(),
            HELP.to_owned(),
        ]
    }
}

// ---------------------------------------------------------------- creation

pub use crate::creation_menu::{
    Catalog, CreationAction, CreationForm, ROW_ADD, ROW_ALIGNMENT, ROW_BACKGROUND, ROW_BEGIN,
    ROW_CLASS, ROW_NAME, ROW_RACE, ROW_SCORES, ROW_SKILLS, ROWS,
};

// ---------------------------------------------------------------- pause

/// What the pause overlay asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseAction {
    /// Back to the world.
    Resume,
    /// Write the quick save.
    Save,
    /// Read the quick save.
    Load,
    /// Open the character sheet.
    Sheet,
    /// Open the debug menu (a dev build with devtools on in this game).
    Debug,
    /// Drop the game and return to the title.
    QuitToTitle,
    /// Exit.
    Quit,
}

/// The pause overlay: the settings, read-only, and the items.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pause {
    /// Selected item.
    pub cursor: usize,
}

/// Whether the debug menu can open: this build carries the dev tools and the game allows
/// `Dev` commands.
#[must_use]
pub fn debug_available(settings: Settings) -> bool {
    settings.devtools && cfg!(feature = "devtools")
}

impl Pause {
    /// The items, in cursor order.
    pub const ITEMS: [&'static str; 7] = [
        "Resume",
        "Save",
        "Load",
        "Character sheet",
        "Debug menu",
        "Quit to title",
        "Quit",
    ];
    /// The action of each item, in the same order.
    const ACTIONS: [PauseAction; 7] = [
        PauseAction::Resume,
        PauseAction::Save,
        PauseAction::Load,
        PauseAction::Sheet,
        PauseAction::Debug,
        PauseAction::QuitToTitle,
        PauseAction::Quit,
    ];
    /// The row of the debug item, dim when `debug_available` says no.
    pub const DEBUG: usize = 4;

    /// Handle a key.
    pub fn key(&mut self, key: MenuKey) -> Option<PauseAction> {
        match key {
            MenuKey::Up | MenuKey::Down => {
                self.cursor = cycle(self.cursor, Self::ITEMS.len(), key);
            }
            MenuKey::Escape => return Some(PauseAction::Resume),
            MenuKey::Enter => return Self::ACTIONS.get(self.cursor).copied(),
            _ => {}
        }
        None
    }

    /// The overlay as lines.
    #[must_use]
    pub fn lines(&self, settings: Settings, seed: u64) -> Vec<String> {
        let mut lines = vec![
            "PAUSED".to_owned(),
            format!("Seed {seed}"),
            format!(
                "Saving: {:?}   Permadeath: {}   Devtools: {}",
                settings.save_rule,
                on_off(settings.permadeath),
                on_off(settings.devtools)
            ),
            String::new(),
        ];
        lines.extend(
            Self::ITEMS
                .iter()
                .enumerate()
                .map(|(i, item)| mark(self.cursor, i, (*item).to_owned())),
        );
        lines.push(String::new());
        lines.push("Up/Down select   Enter confirm   Esc resume".to_owned());
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_cycles_and_chooses() {
        let mut title = Title::default();
        assert_eq!(title.key(MenuKey::Up), None);
        assert_eq!(title.cursor, 2, "wraps");
        assert_eq!(title.key(MenuKey::Enter), Some(TitleAction::Quit));
        title.key(MenuKey::Down);
        assert_eq!(title.key(MenuKey::Enter), Some(TitleAction::NewGame));
        assert!(title.lines()[2].starts_with("> New game"));
        assert!(title.lines()[3].starts_with("  Load"));
    }

    #[test]
    fn the_new_game_form_makes_a_seed_and_settings() {
        let mut form = NewGameForm::default();
        assert_eq!(form.seed(99), 99, "blank is entropy");
        for c in "42".chars() {
            form.key(MenuKey::Char(c));
        }
        assert_eq!(form.seed(99), 42);
        form.key(MenuKey::Backspace);
        form.key(MenuKey::Backspace);
        for c in "toel".chars() {
            form.key(MenuKey::Char(c));
        }
        assert_eq!(form.seed(99), fnv1a64(b"toel"), "words are hashed");
        form.key(MenuKey::Down);
        form.key(MenuKey::Right);
        assert_eq!(form.settings.save_rule, SaveRule::Relief);
        form.key(MenuKey::Right);
        form.key(MenuKey::Right);
        assert_eq!(form.settings.save_rule, SaveRule::Anywhere, "cycles");
        form.key(MenuKey::Left);
        assert_eq!(form.settings.save_rule, SaveRule::InnOnly);
        form.key(MenuKey::Down);
        form.key(MenuKey::Left);
        assert!(form.settings.permadeath);
        form.key(MenuKey::Down);
        assert_eq!(form.key(MenuKey::Enter), Some(NewGameAction::Start));
        form.key(MenuKey::Down);
        assert_eq!(form.key(MenuKey::Enter), Some(NewGameAction::Back));
        assert_eq!(form.key(MenuKey::Escape), Some(NewGameAction::Back));
        assert!(form.lines()[3].contains("Inns only"), "{:?}", form.lines());
        assert!(form.lines()[4].contains("On"));
    }

    #[test]
    fn the_pause_overlay_shows_the_settings_and_every_item_answers() {
        let mut pause = Pause::default();
        let settings = Settings {
            save_rule: SaveRule::InnOnly,
            permadeath: true,
            devtools: true,
        };
        let lines = pause.lines(settings, 7);
        assert_eq!(lines[1], "Seed 7");
        assert_eq!(lines[2], "Saving: InnOnly   Permadeath: on   Devtools: on");
        assert_eq!(
            &lines[4..11],
            [
                "> Resume",
                "  Save",
                "  Load",
                "  Character sheet",
                "  Debug menu",
                "  Quit to title",
                "  Quit"
            ]
        );
        assert_eq!(pause.key(MenuKey::Escape), Some(PauseAction::Resume));
        let expected = [
            PauseAction::Resume,
            PauseAction::Save,
            PauseAction::Load,
            PauseAction::Sheet,
            PauseAction::Debug,
            PauseAction::QuitToTitle,
            PauseAction::Quit,
        ];
        assert_eq!(Pause::ITEMS[Pause::DEBUG], "Debug menu");
        assert_eq!(
            debug_available(settings),
            cfg!(feature = "devtools"),
            "on in this game, so the build decides"
        );
        assert!(!debug_available(Settings::default()));
        for (i, action) in expected.iter().enumerate() {
            assert_eq!(pause.cursor, i);
            assert_eq!(pause.key(MenuKey::Enter), Some(*action), "item {i}");
            assert!(pause.lines(settings, 7)[4 + i].starts_with("> "));
            pause.key(MenuKey::Down);
        }
        assert_eq!(pause.cursor, 0, "wraps after the last item");
        assert_eq!(words("LawfulGood"), "Lawful Good");
        assert_eq!(words("SleightOfHand"), "Sleight Of Hand");
    }
}
