//! The use picker in a fight: the acting member's class features, then the rows their usable
//! kit makes, why one is grey, and the keys while it is open (the `CombatMenu` opens it from
//! Use). An item goes to the band's selected member, or the user; Cunning Action's exchange
//! goes to the selected member. Bevy-free.

use crate::combat_menu::{CombatIntent, CombatMenu, FightView};
use crate::menu::{MenuKey, cycle};
use crate::spell_menu::blocked_note;
use omnis_sim::omnis_data::{Data, FeatureEffect, UseEffect};
use omnis_sim::omnis_rules::combat_features;
use omnis_sim::{CombatCommand, FeatureChoice, FighterView, World};

/// What a feature row asks of the feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Its one thing.
    None,
    /// Cunning Action's exchange with the selected member.
    Exchange,
    /// Cunning Action's hide.
    Hide,
}

/// What a row uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseKind {
    /// A class feature.
    Feature {
        /// Its row, the number `feature` takes.
        index: u8,
        /// Uses left before a rest; `None` at will.
        uses_left: Option<u8>,
        /// Which of its choices.
        choice: Choice,
    },
    /// An item of the kit.
    Item {
        /// Its row in the kit, the number `use-item` takes.
        index: u8,
        /// How many.
        count: u16,
    },
}

/// One row of the picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseRow {
    /// The feature's or item's name.
    pub name: String,
    /// What it uses.
    pub kind: UseKind,
    /// Why it cannot be used now, in a few words.
    pub blocked: Option<String>,
}

impl UseRow {
    /// The amount after the name: `x3` for an item, `1 use` or `at will` for a feature.
    #[must_use]
    pub fn amount(&self) -> String {
        match self.kind {
            UseKind::Item { count, .. } => format!("x{count}"),
            UseKind::Feature {
                uses_left: Some(1), ..
            } => "1 use".to_owned(),
            UseKind::Feature {
                uses_left: Some(n), ..
            } => format!("{n} uses"),
            UseKind::Feature {
                uses_left: None, ..
            } => "at will".to_owned(),
        }
    }
}

/// The acting member's feature rows (Cunning Action makes two: exchange and hide), then the
/// kit rows that have a use; a sense item is not used from a fight.
#[must_use]
pub fn use_rows(
    world: &World,
    data: &Data,
    own: usize,
    fighter: Option<&FighterView>,
) -> Vec<UseRow> {
    let Some(member) = world.party.members.get(own) else {
        return Vec::new();
    };
    let defs = combat_features(member, data);
    let features = fighter.into_iter().flat_map(|f| &f.features).flat_map(|f| {
        let name = data.label("en", &f.name).to_owned();
        let blocked = f.blocked.as_ref().map(blocked_note);
        let cunning = defs
            .get(usize::from(f.index))
            .is_some_and(|d| d.effect == Some(FeatureEffect::Cunning));
        let choices: &[(Choice, &str)] = if cunning {
            &[(Choice::Exchange, ": exchange"), (Choice::Hide, ": hide")]
        } else {
            &[(Choice::None, "")]
        };
        choices
            .iter()
            .map(|(choice, suffix)| UseRow {
                name: format!("{name}{suffix}"),
                kind: UseKind::Feature {
                    index: f.index,
                    uses_left: f.uses_left,
                    choice: *choice,
                },
                blocked: blocked.clone(),
            })
            .collect::<Vec<_>>()
    });
    let items = member
        .equipment
        .iter()
        .enumerate()
        .filter_map(|(i, (id, count))| {
            let item = data.items.get(id)?;
            let blocked = match item.use_effect.as_ref()? {
                UseEffect::Heal { .. } => None,
                UseEffect::Sense(_) => Some("not here".to_owned()),
            };
            Some(UseRow {
                name: data.label("en", &item.name).to_owned(),
                kind: UseKind::Item {
                    index: u8::try_from(i).unwrap_or(u8::MAX),
                    count: *count,
                },
                blocked,
            })
        });
    features.chain(items).collect()
}

impl CombatMenu {
    /// Keys while the item picker is open.
    pub(crate) fn use_key(
        &mut self,
        cursor: usize,
        key: MenuKey,
        view: &FightView,
        selected: Option<usize>,
    ) -> Option<CombatIntent> {
        match key {
            MenuKey::Up | MenuKey::Down => {
                self.use_picker = Some(cycle(cursor, view.usable.len(), key));
            }
            MenuKey::Left | MenuKey::Right => self.step_target(view, key),
            MenuKey::Enter => return self.confirm_use(cursor, view, selected),
            MenuKey::Escape | MenuKey::Char('u') => self.use_picker = None,
            MenuKey::Char(_) | MenuKey::Backspace => {}
        }
        None
    }

    /// Use the feature or item under the picker's cursor (an item on the selected member, or
    /// the user), or say why not.
    pub(crate) fn confirm_use(
        &mut self,
        cursor: usize,
        view: &FightView,
        selected: Option<usize>,
    ) -> Option<CombatIntent> {
        self.message.clear();
        let Some(row) = view.usable.get(cursor) else {
            self.use_picker = None;
            return None;
        };
        if let Some(why) = &row.blocked {
            self.message = format!("{}: {why}", row.name);
            return None;
        }
        let command = match row.kind {
            UseKind::Item { index, .. } => CombatCommand::Use {
                item: index,
                target: selected.map(|s| u8::try_from(s).unwrap_or(u8::MAX)),
            },
            UseKind::Feature { index, choice, .. } => CombatCommand::Feature {
                feature: index,
                choice: match choice {
                    Choice::None => FeatureChoice::None,
                    Choice::Exchange => FeatureChoice::Exchange {
                        with: self.partner(view, selected)?,
                    },
                    Choice::Hide => FeatureChoice::Hide,
                },
            },
        };
        self.use_picker = None;
        Some(CombatIntent::Command(command))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_menu::tests::{data, facing};
    use crate::combat_menu::{ACTION_USE, fight_view};
    use omnis_sim::items::item_id;
    use omnis_sim::{Command, EncounterChoice, Mode, apply, combat_view};

    #[test]
    fn the_picker_lists_the_kit_s_usable_rows_and_uses_one_on_the_selection() {
        let data = data();
        let mut world = facing(&data, &["fighter"], &[("giant_rat", 1)]);
        apply(
            &mut world,
            &data,
            Command::Encounter(EncounterChoice::Attack),
        )
        .unwrap();
        assert!(matches!(world.mode, Mode::Combat(_)));
        world.party.members[0]
            .equipment
            .push((item_id(&data, "spyglass").unwrap(), 1));
        let view = fight_view(&world, &data).unwrap();
        assert_eq!(view.own, Some(0));
        let rows: Vec<(UseKind, &str, String, Option<&str>)> = view
            .usable
            .iter()
            .map(|r| (r.kind, r.name.as_str(), r.amount(), r.blocked.as_deref()))
            .collect();
        let second_wind = UseKind::Feature {
            index: 0,
            uses_left: Some(1),
            choice: Choice::None,
        };
        assert_eq!(
            rows,
            [
                (second_wind, "Second Wind", "1 use".to_owned(), None),
                (
                    UseKind::Item { index: 6, count: 1 },
                    "Potion of healing",
                    "x1".to_owned(),
                    None
                ),
                (
                    UseKind::Item { index: 7, count: 1 },
                    "Spyglass",
                    "x1".to_owned(),
                    Some("not here")
                )
            ],
            "the features first, then the kit"
        );
        let mut menu = CombatMenu::default();
        assert_eq!(menu.key(MenuKey::Char('u'), &view, None), None);
        assert_eq!((menu.cursor, menu.use_picker), (ACTION_USE, Some(0)));
        menu.key(MenuKey::Down, &view, None);
        menu.key(MenuKey::Down, &view, None);
        assert_eq!(menu.use_picker, Some(2));
        assert_eq!(menu.key(MenuKey::Enter, &view, None), None);
        assert_eq!(menu.message, "Spyglass: not here");
        assert_eq!(menu.use_picker, Some(2), "a refusal keeps the picker open");
        menu.key(MenuKey::Up, &view, None);
        assert_eq!(
            menu.key(MenuKey::Enter, &view, Some(0)),
            Some(CombatIntent::Command(CombatCommand::Use {
                item: 6,
                target: Some(0)
            }))
        );
        assert_eq!(menu.use_picker, None, "a use closes the picker");
        menu.key(MenuKey::Char('u'), &view, None);
        menu.key(MenuKey::Down, &view, None);
        assert_eq!(
            menu.key(MenuKey::Enter, &view, None),
            Some(CombatIntent::Command(CombatCommand::Use {
                item: 6,
                target: None
            })),
            "no selection: the user"
        );
        menu.key(MenuKey::Char('u'), &view, None);
        assert_eq!(
            menu.key(MenuKey::Enter, &view, Some(2)),
            Some(CombatIntent::Command(CombatCommand::Feature {
                feature: 0,
                choice: FeatureChoice::None
            })),
            "a feature row is the Feature command; the selection does not matter"
        );
        menu.key(MenuKey::Char('u'), &view, None);
        assert_eq!(menu.key(MenuKey::Char('u'), &view, None), None);
        assert_eq!(menu.use_picker, None, "u closes it too");
        menu.key(MenuKey::Char('u'), &view, None);
        assert_eq!(menu.key(MenuKey::Escape, &view, None), None);
        assert_eq!(menu.use_picker, None, "so does Escape");
        // Nothing usable: Use says so and opens nothing.
        world.party.members[0].equipment.clear();
        let mut bare = fight_view(&world, &data).unwrap();
        assert_eq!(bare.usable.len(), 1, "Second Wind stays");
        bare.usable.clear();
        menu.use_picker = Some(1);
        menu.sync(&bare);
        assert_eq!(menu.use_picker, None, "sync closes an empty picker");
        assert_eq!(menu.key(MenuKey::Char('u'), &bare, None), None);
        assert_eq!(menu.message, "Nothing to use");
    }

    /// A party of a level-2 fighter and a level-2 rogue in a fight against one rat.
    fn level_two() -> (Data, World) {
        let data = data();
        let mut world = facing(&data, &["fighter", "rogue"], &[("giant_rat", 1)]);
        for member in &mut world.party.members {
            member.level = 2;
        }
        apply(
            &mut world,
            &data,
            Command::Encounter(EncounterChoice::Attack),
        )
        .unwrap();
        (data, world)
    }

    #[test]
    fn each_member_s_features_make_rows_and_cunning_action_makes_two() {
        let (data, world) = level_two();
        let view = combat_view(&world, &data).unwrap();
        let rows = |own: usize| -> Vec<(String, UseKind)> {
            let fighter = view.members.iter().find(|m| usize::from(m.index) == own);
            use_rows(&world, &data, own, fighter)
                .into_iter()
                .filter(|r| matches!(r.kind, UseKind::Feature { .. }))
                .map(|r| (r.name, r.kind))
                .collect()
        };
        let feature = |index, uses_left, choice| UseKind::Feature {
            index,
            uses_left,
            choice,
        };
        assert_eq!(
            rows(0),
            [
                ("Second Wind".to_owned(), feature(0, Some(1), Choice::None)),
                ("Action Surge".to_owned(), feature(1, Some(1), Choice::None)),
            ]
        );
        assert_eq!(
            rows(1),
            [
                (
                    "Cunning Action: exchange".to_owned(),
                    feature(0, None, Choice::Exchange)
                ),
                (
                    "Cunning Action: hide".to_owned(),
                    feature(0, None, Choice::Hide)
                ),
            ]
        );
    }

    #[test]
    fn cunning_action_s_exchange_goes_to_the_selected_member_and_hide_needs_none() {
        let (data, world) = level_two();
        let mut view = fight_view(&world, &data).unwrap();
        let fighter = combat_view(&world, &data).unwrap();
        let rogue = fighter.members.iter().find(|m| m.index == 1);
        view.own = Some(1);
        view.usable = use_rows(&world, &data, 1, rogue);
        for row in &mut view.usable {
            row.blocked = None;
        }
        let mut menu = CombatMenu {
            use_picker: Some(0),
            ..CombatMenu::default()
        };
        assert_eq!(menu.key(MenuKey::Enter, &view, None), None);
        assert_eq!(menu.message, "Select a member to exchange with");
        assert_eq!(menu.key(MenuKey::Enter, &view, Some(1)), None);
        assert_eq!(menu.message, "Select another member to exchange with");
        assert_eq!(
            menu.key(MenuKey::Enter, &view, Some(0)),
            Some(CombatIntent::Command(CombatCommand::Feature {
                feature: 0,
                choice: FeatureChoice::Exchange { with: 0 }
            }))
        );
        menu.use_picker = Some(1);
        assert_eq!(
            menu.key(MenuKey::Enter, &view, None),
            Some(CombatIntent::Command(CombatCommand::Feature {
                feature: 0,
                choice: FeatureChoice::Hide
            }))
        );
    }
}
