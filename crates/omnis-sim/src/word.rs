//! Script words: a typing aid for `omnis-cli play`, the app's dev script and tests. A word names
//! members by marching-order slot, as a player counts them, while a command names them by
//! identity (protocol 2), so a word is checked when the script is read and turned into a command
//! against the party when it is applied: the slot means whoever stands there at that moment.

use crate::combat::{CombatCommand, FeatureChoice, Pay, Target};
use crate::command::Command;
use crate::encounter::EncounterChoice;
use crate::party::PartyCommand;
use crate::rest::{HitDiceSpend, RestCommand};
use crate::service::ServiceCommand;
use crate::tactics::TacticsCommand;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use omnis_core::{CharacterId, Direction, Rotation};

/// One checked script word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word(String);

impl Word {
    /// The word, if it is one: its syntax is checked, its slots are not.
    #[must_use]
    pub fn parse(word: &str) -> Option<Word> {
        let any = |slot: u8| Some(CharacterId(u32::from(slot)));
        command(word, &any).map(|_| Word(word.to_string()))
    }

    /// The command for a party whose members, in marching order, have these ids; `None` when
    /// the word names a slot no member stands in.
    #[must_use]
    pub fn command(&self, members: &[CharacterId]) -> Option<Command> {
        command(&self.0, &|slot| members.get(usize::from(slot)).copied())
    }

    /// The word as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Word {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A member's identity for a slot named in a word.
type Slots<'a> = dyn Fn(u8) -> Option<CharacterId> + 'a;

fn member(text: &str, slots: &Slots<'_>) -> Option<CharacterId> {
    slots(text.parse().ok()?)
}

fn command(word: &str, slots: &Slots<'_>) -> Option<Command> {
    Some(match word {
        "forward" => Command::Step(Direction::Forward),
        "back" => Command::Step(Direction::Back),
        "left" => Command::Step(Direction::Left),
        "right" => Command::Step(Direction::Right),
        "turn-left" => Command::Turn(Rotation::Left),
        "turn-right" => Command::Turn(Rotation::Right),
        "around" => Command::Turn(Rotation::Around),
        "use" => Command::Interact,
        "fight" => Command::Encounter(EncounterChoice::Attack),
        "bribe" => Command::Encounter(EncounterChoice::Bribe),
        "hide" => Command::Encounter(EncounterChoice::Hide),
        "run" => Command::Encounter(EncounterChoice::Run),
        "attack" => Command::Combat(CombatCommand::Attack { stack: 0 }),
        "dodge" => Command::Combat(CombatCommand::Dodge),
        "flee" => Command::Combat(CombatCommand::Run),
        "leave" => Command::Service(ServiceCommand::Leave),
        "room" => Command::Service(ServiceCommand::Room),
        "rumor" => Command::Service(ServiceCommand::Rumor),
        "rest" => Command::Rest(RestCommand::Long),
        "short-rest" => Command::Rest(RestCommand::Short { dice: Vec::new() }),
        "end" => Command::Combat(CombatCommand::EndTurn),
        _ => {
            if let Some(rest) = word.strip_prefix("react-") {
                return parse_react(rest, slots).map(Command::Party);
            }
            if let Some(rest) = word.strip_prefix("feature-") {
                return parse_feature(rest, slots).map(Command::Combat);
            }
            if let Some(n) = word.strip_prefix("attack-") {
                return n
                    .parse()
                    .ok()
                    .map(|stack| Command::Combat(CombatCommand::Attack { stack }));
            }
            if let Some(n) = word.strip_prefix("swap-") {
                return member(n, slots)
                    .map(|with| Command::Combat(CombatCommand::Exchange { with }));
            }
            if let Some(rest) = word.strip_prefix("cast-") {
                return parse_cast(rest, slots).map(Command::Combat);
            }
            if let Some(rest) = word.strip_prefix("use-item-") {
                return parse_use(rest, slots).map(Command::Combat);
            }
            return parse_town(word, slots);
        }
    })
}

/// `N-M` casts spell `N` at stack `M`; `N-mM` at member `M`; a trailing `-bonus` pays with
/// the bonus action.
fn parse_cast(rest: &str, slots: &Slots<'_>) -> Option<CombatCommand> {
    let (rest, pay) = match rest.strip_suffix("-bonus") {
        Some(rest) => (rest, Pay::BonusAction),
        None => (rest, Pay::Action),
    };
    let (spell, target) = rest.split_once('-')?;
    let spell = spell.parse().ok()?;
    let target = match target.strip_prefix('m') {
        Some(slot) => Target::Member(member(slot, slots)?),
        None => Target::Stack(target.parse().ok()?),
    };
    Some(CombatCommand::Cast { spell, target, pay })
}

/// `M-on` or `M-off` switches member `M`'s reactions.
fn parse_react(rest: &str, slots: &Slots<'_>) -> Option<PartyCommand> {
    let (slot, on) = match rest.split_once('-')? {
        (slot, "on") => (slot, true),
        (slot, "off") => (slot, false),
        _ => return None,
    };
    Some(PartyCommand::Tactics(TacticsCommand::SetReactions {
        member: member(slot, slots)?,
        on,
    }))
}

/// `F` uses feature row `F`; `F-W` exchanges with slot `W` (Cunning Action); `F-hide` hides.
fn parse_feature(rest: &str, slots: &Slots<'_>) -> Option<CombatCommand> {
    let (feature, choice) = match rest.split_once('-') {
        Some((feature, "hide")) => (feature, FeatureChoice::Hide),
        Some((feature, with)) => (
            feature,
            FeatureChoice::Exchange {
                with: member(with, slots)?,
            },
        ),
        None => (rest, FeatureChoice::None),
    };
    Some(CombatCommand::Feature {
        feature: feature.parse().ok()?,
        choice,
    })
}

/// `N` uses item `N` of the acting member's kit on themselves; `N-mM` on member `M`.
fn parse_use(rest: &str, slots: &Slots<'_>) -> Option<CombatCommand> {
    let (item, target) = match rest.split_once('-') {
        Some((item, target)) => (item, Some(member(target.strip_prefix('m')?, slots)?)),
        None => (rest, None),
    };
    Some(CombatCommand::Use {
        item: item.parse().ok()?,
        target,
    })
}

/// Town and rest words that carry numbers: `food-N`, `heal-M`, `cure-M`, `raise-M`, `buy-R`
/// and `buy-R-N` (row `R` of the stock), `sell-R` and `sell-R-N` (row `R` of the stores),
/// `deposit-N` and `withdraw-N` (copper), `train-M`, `choose-M-R` (row `R` of the member's
/// class list), `learn-M-R` (row `R` of the service's spells), `short-rest-A-B-…` (hit dice per
/// member in marching order).
fn parse_town(word: &str, slots: &Slots<'_>) -> Option<Command> {
    if let Some(dice) = word.strip_prefix("short-rest-") {
        let dice = dice
            .split('-')
            .enumerate()
            .map(|(slot, count)| {
                Some(HitDiceSpend {
                    member: slots(u8::try_from(slot).ok()?)?,
                    count: count.parse().ok()?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        return Some(Command::Rest(RestCommand::Short { dice }));
    }
    let (verb, args) = word.split_once('-')?;
    let service = match verb {
        "food" => ServiceCommand::BuyFood {
            count: args.parse().ok()?,
        },
        "heal" => ServiceCommand::Heal {
            member: member(args, slots)?,
        },
        "cure" => ServiceCommand::Cure {
            member: member(args, slots)?,
        },
        "raise" => ServiceCommand::Raise {
            member: member(args, slots)?,
        },
        "buy" | "sell" => {
            let (item, count) = match args.split_once('-') {
                Some((item, count)) => (item.parse().ok()?, count.parse().ok()?),
                None => (args.parse().ok()?, 1),
            };
            if verb == "buy" {
                ServiceCommand::Buy { item, count }
            } else {
                ServiceCommand::Sell { item, count }
            }
        }
        "deposit" => ServiceCommand::Deposit {
            amount: args.parse().ok()?,
        },
        "withdraw" => ServiceCommand::Withdraw {
            amount: args.parse().ok()?,
        },
        "train" => ServiceCommand::Train {
            member: member(args, slots)?,
        },
        "choose" | "learn" => {
            let (slot, spell) = args.split_once('-')?;
            let (member, spell) = (member(slot, slots)?, spell.parse().ok()?);
            if verb == "choose" {
                ServiceCommand::Choose { member, spell }
            } else {
                ServiceCommand::Learn { member, spell }
            }
        }
        _ => return None,
    };
    Some(Command::Service(service))
}

/// A word in a script that is not a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError {
    /// One-based line of the word.
    pub line: usize,
    /// The word.
    pub word: String,
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: unknown command '{}'", self.line, self.word)
    }
}

impl core::error::Error for ScriptError {}

/// Parse a command script: words `forward`, `back`, `left`, `right` (sidesteps),
/// `turn-left`, `turn-right`, `around`, `use`, before a fight `fight`, `bribe`, `hide`, `run`,
/// and in one `attack` (the first stack), `attack-N`, `cast-N-M` (spell `N` at stack `M`),
/// `cast-N-mM` (at member `M`), either with `-bonus` to pay with the bonus action,
/// `use-item-N` (item `N` of the acting member's kit, on themselves), `use-item-N-mM` (on
/// member `M`), `dodge`, `swap-N`, `flee`, `feature-F` (feature row `F`), `feature-F-W`
/// (Cunning Action's exchange with slot `W`), `feature-F-hide`, `end` (ends the turn), at any
/// time `react-M-on` and `react-M-off` (member `M`'s reactions switch), inside a service
/// `leave`, `room`, `rumor` and the words with numbers of `parse_town` (`buy-0`, `heal-1`, …),
/// outside one `rest`, `short-rest` and `short-rest-A-B-…`, separated by whitespace or commas;
/// `#` starts a comment that runs to the end of the line. Members are named by marching-order
/// slot; [`Word::command`] turns each into a command against the party it is applied to.
pub fn parse_script(text: &str) -> Result<Vec<Word>, ScriptError> {
    let mut words = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let code = line.split('#').next().unwrap_or("");
        for word in code.split(|c: char| c.is_whitespace() || c == ',') {
            if word.is_empty() {
                continue;
            }
            match Word::parse(word) {
                Some(word) => words.push(word),
                None => {
                    return Err(ScriptError {
                        line: index + 1,
                        word: word.to_string(),
                    });
                }
            }
        }
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_s_slot_names_whoever_stands_there() {
        let members = [CharacterId(7), CharacterId(3), CharacterId(9)];
        let command = |word: &str| Word::parse(word).and_then(|w| w.command(&members));
        assert_eq!(
            command("heal-1"),
            Some(Command::Service(ServiceCommand::Heal {
                member: CharacterId(3)
            }))
        );
        assert_eq!(
            command("short-rest-0-2"),
            Some(Command::Rest(RestCommand::Short {
                dice: alloc::vec![
                    HitDiceSpend {
                        member: CharacterId(7),
                        count: 0
                    },
                    HitDiceSpend {
                        member: CharacterId(3),
                        count: 2
                    },
                ]
            }))
        );
        assert_eq!(
            command("cast-0-m2"),
            Some(Command::Combat(CombatCommand::Cast {
                spell: 0,
                target: Target::Member(CharacterId(9)),
                pay: Pay::Action,
            }))
        );
        assert_eq!(command("heal-3"), None, "nobody stands in slot 3");
        assert!(
            Word::parse("heal-3").is_some(),
            "but the word itself is sound"
        );
    }
}
