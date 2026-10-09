//! Script words: a typing aid for `omnis-cli play`, the app's dev script and tests. A word counts
//! as a player does: members by marching-order slot, spells, items and features by their row in
//! the list on screen. A command names members by identity and definitions by string id
//! (protocol 2), so a word is checked when the script is read and turned into a command against
//! the world and the packs when it is applied: a slot means whoever stands there at that moment,
//! a row whatever that list holds then.

use crate::combat::{CombatCommand, FeatureChoice, Pay, Target};
use crate::command::Command;
use crate::encounter::EncounterChoice;
use crate::event::ActorRef;
use crate::party::PartyCommand;
use crate::rest::{HitDiceSpend, RestCommand};
use crate::service::ServiceCommand;
use crate::tactics::TacticsCommand;
use crate::world::{Mode, World};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use omnis_core::{CharacterId, Direction, ItemId, Rotation};
use omnis_data::Data;
use omnis_rules::{Character, combat_features};

/// One checked script word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word(String);

impl Word {
    /// The word, if it is one: its syntax is checked, its slots and rows are not.
    #[must_use]
    pub fn parse(word: &str) -> Option<Word> {
        command(word, &Scope::Any).map(|_| Word(word.to_string()))
    }

    /// The command this word means in `world` now; `None` when a slot or a row it names is
    /// empty, or the word does not fit the world's state (`buy-0` outside a shop, `cast-0-1`
    /// with nobody's turn).
    #[must_use]
    pub fn command(&self, world: &World, data: &Data) -> Option<Command> {
        command(&self.0, &Scope::Live { world, data })
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

/// What a word's numbers resolve against: anything (a syntax check), or a world and its packs.
enum Scope<'a> {
    /// Every slot and row is filled.
    Any,
    /// The world as it stands.
    Live {
        /// The world.
        world: &'a World,
        /// Its packs.
        data: &'a Data,
    },
}

/// What a row stands for in a syntax check: a placeholder, when the row is a number.
fn any_row(row: &str) -> Option<String> {
    row.parse::<u8>().is_ok().then(String::new)
}

/// Row `row` of a list.
fn nth<T: Clone>(list: &[T], row: &str) -> Option<T> {
    list.get(row.parse::<usize>().ok()?).cloned()
}

impl Scope<'_> {
    /// The member standing in a slot.
    fn member(&self, slot: &str) -> Option<CharacterId> {
        let slot: u8 = slot.parse().ok()?;
        match self {
            Scope::Any => Some(CharacterId(u32::from(slot))),
            Scope::Live { world, .. } => world.party.members.get(usize::from(slot)).map(|m| m.id),
        }
    }

    /// The member whose turn it is in a fight.
    fn acting(&self) -> Option<&Character> {
        let Scope::Live { world, .. } = self else {
            return None;
        };
        let Mode::Combat(state) = &world.mode else {
            return None;
        };
        let ActorRef::Member(id) = state.order.get(usize::from(state.current))?.actor else {
            return None;
        };
        world.party.members.iter().find(|m| m.id == id)
    }

    /// The string id of an item kind.
    fn item_id(&self, item: ItemId) -> Option<String> {
        let Scope::Live { data, .. } = self else {
            return None;
        };
        data.registry.items.name(item).map(String::from)
    }

    /// Row `row` of the acting member's known spells.
    fn known_spell(&self, row: &str) -> Option<String> {
        let Scope::Live { data, .. } = self else {
            return any_row(row);
        };
        let spell = nth(&self.acting()?.known_spells, row)?;
        data.registry.spells.name(spell).map(String::from)
    }

    /// Row `row` of the acting member's kit.
    fn kit_item(&self, row: &str) -> Option<String> {
        if let Scope::Any = self {
            return any_row(row);
        }
        let (item, _) = nth(&self.acting()?.equipment, row)?;
        self.item_id(item)
    }

    /// Row `row` of the acting member's features with effect, as its name key.
    fn feature(&self, row: &str) -> Option<String> {
        let Scope::Live { data, .. } = self else {
            return any_row(row);
        };
        let features = combat_features(self.acting()?, data);
        nth(&features, row).map(|f| f.name.clone())
    }

    /// Row `row` of the party's stores.
    fn stores_item(&self, row: &str) -> Option<String> {
        let Scope::Live { world, .. } = self else {
            return any_row(row);
        };
        let (item, _) = nth(&world.party.inventory, row)?;
        self.item_id(item)
    }

    /// The definition of the service the party is inside.
    fn service(&self) -> Option<&omnis_data::ServiceDef> {
        let Scope::Live { world, data } = self else {
            return None;
        };
        let Mode::Town(state) = &world.mode else {
            return None;
        };
        data.services.get(&state.service)
    }

    /// Row `row` of the service's stock.
    fn stock_item(&self, row: &str) -> Option<String> {
        if let Scope::Any = self {
            return any_row(row);
        }
        nth(&self.service()?.items, row)
    }

    /// Row `row` of the service's spells.
    fn service_spell(&self, row: &str) -> Option<String> {
        if let Scope::Any = self {
            return any_row(row);
        }
        nth(&self.service()?.spells, row)
    }

    /// Row `row` of the class list of the member in a slot.
    fn class_spell(&self, member: CharacterId, row: &str) -> Option<String> {
        let Scope::Live { world, data } = self else {
            return any_row(row);
        };
        let who = world.party.members.iter().find(|m| m.id == member)?;
        let casting = data.classes.get(&who.class)?.casting.as_ref()?;
        nth(&casting.list, row)
    }
}

fn command(word: &str, scope: &Scope<'_>) -> Option<Command> {
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
                return parse_react(rest, scope).map(Command::Party);
            }
            if let Some(rest) = word.strip_prefix("feature-") {
                return parse_feature(rest, scope).map(Command::Combat);
            }
            if let Some(n) = word.strip_prefix("attack-") {
                return n
                    .parse()
                    .ok()
                    .map(|stack| Command::Combat(CombatCommand::Attack { stack }));
            }
            if let Some(n) = word.strip_prefix("swap-") {
                return scope
                    .member(n)
                    .map(|with| Command::Combat(CombatCommand::Exchange { with }));
            }
            if let Some(rest) = word.strip_prefix("cast-") {
                return parse_cast(rest, scope).map(Command::Combat);
            }
            if let Some(rest) = word.strip_prefix("use-item-") {
                return parse_use(rest, scope).map(Command::Combat);
            }
            return parse_town(word, scope);
        }
    })
}

/// `N-M` casts the acting member's spell row `N` at stack `M`; `N-mM` at member `M`; a
/// trailing `-bonus` pays with the bonus action.
fn parse_cast(rest: &str, scope: &Scope<'_>) -> Option<CombatCommand> {
    let (rest, pay) = match rest.strip_suffix("-bonus") {
        Some(rest) => (rest, Pay::BonusAction),
        None => (rest, Pay::Action),
    };
    let (spell, target) = rest.split_once('-')?;
    let target = match target.strip_prefix('m') {
        Some(slot) => Target::Member(scope.member(slot)?),
        None => Target::Stack(target.parse().ok()?),
    };
    let spell = scope.known_spell(spell)?;
    Some(CombatCommand::Cast { spell, target, pay })
}

/// `M-on` or `M-off` switches member `M`'s reactions.
fn parse_react(rest: &str, scope: &Scope<'_>) -> Option<PartyCommand> {
    let (slot, on) = match rest.split_once('-')? {
        (slot, "on") => (slot, true),
        (slot, "off") => (slot, false),
        _ => return None,
    };
    Some(PartyCommand::Tactics(TacticsCommand::SetReactions {
        member: scope.member(slot)?,
        on,
    }))
}

/// `F` uses the acting member's feature row `F`; `F-W` exchanges with slot `W` (Cunning
/// Action); `F-hide` hides.
fn parse_feature(rest: &str, scope: &Scope<'_>) -> Option<CombatCommand> {
    let (feature, choice) = match rest.split_once('-') {
        Some((feature, "hide")) => (feature, FeatureChoice::Hide),
        Some((feature, with)) => (
            feature,
            FeatureChoice::Exchange {
                with: scope.member(with)?,
            },
        ),
        None => (rest, FeatureChoice::None),
    };
    Some(CombatCommand::Feature {
        feature: scope.feature(feature)?,
        choice,
    })
}

/// `N` uses row `N` of the acting member's kit on themselves; `N-mM` on member `M`.
fn parse_use(rest: &str, scope: &Scope<'_>) -> Option<CombatCommand> {
    let (item, target) = match rest.split_once('-') {
        Some((item, target)) => (item, Some(scope.member(target.strip_prefix('m')?)?)),
        None => (rest, None),
    };
    Some(CombatCommand::Use {
        item: scope.kit_item(item)?,
        target,
    })
}

/// Town and rest words that carry numbers: `food-N`, `heal-M`, `cure-M`, `raise-M`, `buy-R`
/// and `buy-R-N` (row `R` of the stock), `sell-R` and `sell-R-N` (row `R` of the stores),
/// `deposit-N` and `withdraw-N` (copper), `train-M`, `choose-M-R` (row `R` of the member's
/// class list), `learn-M-R` (row `R` of the service's spells), `short-rest-A-B-…` (hit dice per
/// member in marching order).
fn parse_town(word: &str, scope: &Scope<'_>) -> Option<Command> {
    if let Some(dice) = word.strip_prefix("short-rest-") {
        let dice = dice
            .split('-')
            .enumerate()
            .map(|(slot, count)| {
                Some(HitDiceSpend {
                    member: scope.member(&slot.to_string())?,
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
            member: scope.member(args)?,
        },
        "cure" => ServiceCommand::Cure {
            member: scope.member(args)?,
        },
        "raise" => ServiceCommand::Raise {
            member: scope.member(args)?,
        },
        "buy" | "sell" => {
            let (row, count) = match args.split_once('-') {
                Some((row, count)) => (row, count.parse().ok()?),
                None => (args, 1),
            };
            if verb == "buy" {
                let item = scope.stock_item(row)?;
                ServiceCommand::Buy { item, count }
            } else {
                let item = scope.stores_item(row)?;
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
            member: scope.member(args)?,
        },
        "choose" | "learn" => {
            let (slot, row) = args.split_once('-')?;
            let member = scope.member(slot)?;
            if verb == "choose" {
                let spell = scope.class_spell(member, row)?;
                ServiceCommand::Choose { member, spell }
            } else {
                let spell = scope.service_spell(row)?;
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
/// and in one `attack` (the first stack), `attack-N`, `cast-N-M` (the acting member's spell
/// row `N` at stack `M`),
/// `cast-N-mM` (at member `M`), either with `-bonus` to pay with the bonus action,
/// `use-item-N` (row `N` of the acting member's kit, on themselves), `use-item-N-mM` (on
/// member `M`), `dodge`, `swap-N`, `flee`, `feature-F` (feature row `F`), `feature-F-W`
/// (Cunning Action's exchange with slot `W`), `feature-F-hide`, `end` (ends the turn), at any
/// time `react-M-on` and `react-M-off` (member `M`'s reactions switch), inside a service
/// `leave`, `room`, `rumor` and the words with numbers of `parse_town` (`buy-0`, `heal-1`, …),
/// outside one `rest`, `short-rest` and `short-rest-A-B-…`, separated by whitespace or commas;
/// `#` starts a comment that runs to the end of the line. Members are named by marching-order
/// slot and definitions by their row in the list on screen; [`Word::command`] turns each into
/// a command against the world it is applied to.
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
    fn a_word_is_checked_for_its_syntax_alone() {
        for word in [
            "heal-1",
            "short-rest-0-2",
            "cast-0-m2",
            "cast-3-1-bonus",
            "use-item-4-m0",
            "feature-1-hide",
            "buy-2-5",
            "sell-0",
            "choose-1-3",
            "learn-0-2",
        ] {
            assert!(Word::parse(word).is_some(), "{word}");
        }
        for word in [
            "heal-x",
            "cast-a-m1",
            "use-item-",
            "buy-1-x",
            "choose-1",
            "fly",
        ] {
            assert!(Word::parse(word).is_none(), "{word}");
        }
        assert_eq!(
            command("heal-3", &Scope::Any),
            Some(Command::Service(ServiceCommand::Heal {
                member: CharacterId(3)
            })),
            "with nothing to resolve against, a slot stands for itself"
        );
    }
}
