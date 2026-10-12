//! The new game's form (presentation-ARCHITECTURE §6.7, from the retired client's `NewGameForm`):
//! the seed, the save rule and permadeath, then Start or Back. A blank seed is drawn from the
//! clock, a number is the seed as typed, and any other text is hashed, so a word names a world.

use super::{
    Control, ControlId, Effect, HostOp, Kind, Panel, Payload, Row, Screen, ScreenId, Span,
};
use crate::labels;
use crate::sim::game_settings;
use crate::style::{Role, Size};
use omnis_sim::omnis_core::rng::fnv1a64;
use omnis_sim::{SaveRule, Settings};

/// The longest seed text, in bytes.
pub const SEED_MAX: usize = 32;

/// The save rules, easiest first, in the order the choice steps through them.
pub const RULES: [SaveRule; 3] = [SaveRule::Anywhere, SaveRule::Relief, SaveRule::InnOnly];

/// What the form holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewGameForm {
    /// The seed as typed.
    pub seed_text: String,
    /// The save rule and permadeath chosen.
    pub settings: Settings,
}

impl NewGameForm {
    /// The seed the text names: `entropy` when blank, the number when it is one, its hash
    /// otherwise.
    #[must_use]
    pub fn seed(&self, entropy: u64) -> u64 {
        let text = self.seed_text.trim();
        if text.is_empty() {
            entropy
        } else {
            text.parse().unwrap_or_else(|_| fnv1a64(text.as_bytes()))
        }
    }

    fn rule_at(&self) -> usize {
        RULES
            .iter()
            .position(|r| *r == self.settings.save_rule)
            .unwrap_or(0)
    }

    /// Step the save rule one along, wrapping; `back` steps the other way.
    fn step_rule(&mut self, back: bool) {
        let n = RULES.len();
        let at = if back {
            (self.rule_at() + n - 1) % n
        } else {
            (self.rule_at() + 1) % n
        };
        self.settings.save_rule = RULES[at];
    }
}

/// The screen.
#[must_use]
pub fn content(form: &NewGameForm) -> Screen {
    let seed = Control {
        id: ControlId::Seed,
        label: "Seed".to_owned(),
        kind: Kind::Text {
            value: form.seed_text.clone(),
            max: SEED_MAX,
        },
        why_not: None,
    };
    let rule = Control {
        id: ControlId::Rule,
        label: "Saving".to_owned(),
        kind: Kind::Choice {
            options: RULES
                .iter()
                .map(|r| labels::save_rule(*r).to_owned())
                .collect(),
            at: form.rule_at(),
        },
        why_not: None,
    };
    let permadeath = if form.settings.permadeath {
        "On"
    } else {
        "Off"
    };
    let note = Span {
        text: "blank = random, words are hashed".to_owned(),
        role: Role::Dim,
        size: Size::Small,
    };
    Screen {
        id: ScreenId::NewGame,
        title: Span::title("NEW GAME"),
        panels: vec![Panel {
            rows: vec![
                Row::controls(vec![seed]),
                Row::text(note),
                Row::controls(vec![rule]),
                Row::controls(vec![Control::button(
                    ControlId::Permadeath,
                    format!("Permadeath: {permadeath}"),
                )]),
                Row::controls(vec![
                    Control::button(ControlId::Start, "Start"),
                    Control::button(ControlId::Back, "Back"),
                ]),
            ],
        }],
        keys: Vec::new(),
        message: None,
    }
}

/// Fill the form, or start the game it describes with `entropy` for a blank seed.
#[must_use]
pub fn apply(
    form: &mut NewGameForm,
    control: ControlId,
    payload: &Payload,
    entropy: u64,
) -> Effect {
    match (control, payload) {
        (ControlId::Seed, Payload::Text(text)) => {
            form.seed_text.clone_from(text);
            Effect::None
        }
        (ControlId::Rule, Payload::Left) => {
            form.step_rule(true);
            Effect::None
        }
        (ControlId::Rule, _) => {
            form.step_rule(false);
            Effect::None
        }
        (ControlId::Permadeath, _) => {
            form.settings.permadeath = !form.settings.permadeath;
            Effect::None
        }
        (ControlId::Start, _) => Effect::Host(HostOp::NewGame {
            seed: form.seed(entropy),
            settings: game_settings(form.settings),
        }),
        (ControlId::Back, _) => Effect::Back,
        _ => Effect::None,
    }
}
