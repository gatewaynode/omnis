//! The item picker in a fight: the rows the acting member's usable kit makes, why one is
//! grey, and the keys while it is open (the `CombatMenu` opens it from Use). A use goes to
//! the band's selected member, or the user. Bevy-free.

use crate::combat_menu::{CombatIntent, CombatMenu, FightView};
use crate::menu::{MenuKey, cycle};
use omnis_sim::omnis_data::{Data, UseEffect};
use omnis_sim::{CombatCommand, World};

/// One usable item of the acting member's kit, as the picker shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseRow {
    /// Its row in the kit, the number `use-item` takes.
    pub index: u8,
    /// The item's name.
    pub name: String,
    /// How many.
    pub count: u16,
    /// Why it cannot be used here, in a few words.
    pub blocked: Option<String>,
}

/// The acting member's kit rows that have a use; a sense item is not used from a fight.
#[must_use]
pub fn use_rows(world: &World, data: &Data, own: usize) -> Vec<UseRow> {
    let Some(member) = world.party.members.get(own) else {
        return Vec::new();
    };
    member
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
                index: u8::try_from(i).unwrap_or(u8::MAX),
                name: data.label("en", &item.name).to_owned(),
                count: *count,
                blocked,
            })
        })
        .collect()
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

    /// Use the item under the picker's cursor on the selected member (or the user), or say
    /// why not.
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
        self.use_picker = None;
        Some(CombatIntent::Command(CombatCommand::Use {
            item: row.index,
            target: selected.map(|s| u8::try_from(s).unwrap_or(u8::MAX)),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_menu::tests::{data, facing};
    use crate::combat_menu::{ACTION_USE, fight_view};
    use omnis_sim::items::item_id;
    use omnis_sim::{Command, EncounterChoice, Mode, apply};

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
        let rows: Vec<(u8, &str, u16, Option<&str>)> = view
            .usable
            .iter()
            .map(|r| (r.index, r.name.as_str(), r.count, r.blocked.as_deref()))
            .collect();
        assert_eq!(
            rows,
            [
                (6, "Potion of healing", 1, None),
                (7, "Spyglass", 1, Some("not here"))
            ]
        );
        let mut menu = CombatMenu::default();
        assert_eq!(menu.key(MenuKey::Char('u'), &view, None), None);
        assert_eq!((menu.cursor, menu.use_picker), (ACTION_USE, Some(0)));
        menu.key(MenuKey::Down, &view, None);
        assert_eq!(menu.use_picker, Some(1));
        assert_eq!(menu.key(MenuKey::Enter, &view, None), None);
        assert_eq!(menu.message, "Spyglass: not here");
        assert_eq!(menu.use_picker, Some(1), "a refusal keeps the picker open");
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
        assert_eq!(
            menu.key(MenuKey::Enter, &view, None),
            Some(CombatIntent::Command(CombatCommand::Use {
                item: 6,
                target: None
            })),
            "no selection: the user"
        );
        menu.key(MenuKey::Char('u'), &view, None);
        assert_eq!(menu.key(MenuKey::Char('u'), &view, None), None);
        assert_eq!(menu.use_picker, None, "u closes it too");
        menu.key(MenuKey::Char('u'), &view, None);
        assert_eq!(menu.key(MenuKey::Escape, &view, None), None);
        assert_eq!(menu.use_picker, None, "so does Escape");
        // Nothing usable: Use says so and opens nothing.
        world.party.members[0].equipment.clear();
        let bare = fight_view(&world, &data).unwrap();
        assert!(bare.usable.is_empty());
        menu.use_picker = Some(1);
        menu.sync(&bare);
        assert_eq!(menu.use_picker, None, "sync closes an empty picker");
        assert_eq!(menu.key(MenuKey::Char('u'), &bare, None), None);
        assert_eq!(menu.message, "Nothing to use");
    }
}
