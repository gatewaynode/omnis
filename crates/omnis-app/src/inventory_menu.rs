//! The inventory overlay's model: one pane per member and one for the stores, the rows of a
//! pane, and the actions on the row under the cursor. Every action is an `ItemCommand` for
//! the simulation; the menu stays open so the player can keep sorting, and the rejection or
//! the events say what happened. Bevy-free.

use crate::defs;
use crate::menu::{MenuKey, cycle};
use crate::text::coins;
use omnis_sim::api::{ItemView, PartyView};
use omnis_sim::omnis_core::CharacterId;
use omnis_sim::omnis_data::{Data, EquipSlot};
use omnis_sim::{Command, ItemCommand};

/// One row of a pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRow {
    /// The item's id, the `item` the item commands take.
    pub item: String,
    /// The display name.
    pub name: String,
    /// How many.
    pub count: u16,
    /// The slot it equips into, if any.
    pub slot: Option<EquipSlot>,
    /// Worn or wielded by the pane's member.
    pub equipped: bool,
    /// Use does something with it.
    pub usable: bool,
}

/// One pane: a member's kit, or the stores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pane {
    /// The member whose kit it is, by id; `None` for the stores.
    pub member: Option<CharacterId>,
    /// The tab's text: the member's name, or `STORES`.
    pub title: String,
    /// The line under the tabs: what the member wears, or the purse and larder.
    pub summary: String,
    /// The rows, in the order the item commands index them.
    pub rows: Vec<ItemRow>,
}

/// What the overlay shows: the members in marching order, then the stores.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InventoryView {
    /// The panes.
    pub panes: Vec<Pane>,
}

impl InventoryView {
    /// How many panes are members.
    #[must_use]
    pub fn members(&self) -> usize {
        self.panes.len().saturating_sub(1)
    }
}

/// The view for the party as it is.
#[must_use]
pub fn inventory_view(party: &PartyView, data: &Data) -> InventoryView {
    let name = |id: &str| {
        defs::item(data, id)
            .map_or("?", |item| data.label("en", &item.name))
            .to_owned()
    };
    let row = |item: &ItemView| ItemRow {
        item: item.item.clone(),
        name: name(&item.item),
        count: item.count,
        slot: item.slot,
        equipped: item.equipped,
        usable: item.usable,
    };
    let mut panes: Vec<Pane> = party
        .members
        .iter()
        .map(|member| {
            let worn: Vec<String> = EquipSlot::ALL
                .iter()
                .filter_map(|slot| member.worn.iter().find(|(s, _)| s == slot))
                .map(|(_, id)| name(id))
                .collect();
            Pane {
                member: Some(member.member),
                title: member.name.clone(),
                summary: if worn.is_empty() {
                    "wearing nothing".to_owned()
                } else {
                    format!("wearing {}", worn.join(", "))
                },
                rows: member.equipment.iter().map(row).collect(),
            }
        })
        .collect();
    panes.push(Pane {
        member: None,
        title: "STORES".to_owned(),
        summary: format!("{}  food {}", coins(party.gold), party.food),
        rows: party
            .inventory
            .iter()
            .map(|item| ItemRow {
                slot: None,
                equipped: false,
                ..row(item)
            })
            .collect(),
    });
    InventoryView { panes }
}

/// What the action row offers; each has a key and a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryAction {
    /// Wear or wield the row, or take it off when worn.
    Equip,
    /// Use the row on the band's selected member, or its owner.
    Use,
    /// One of the row into the stores.
    Stow,
    /// One of the row from the stores to the band's selected member, or the first.
    Take,
    /// One of the row to the band's selected member.
    Give,
}

impl InventoryAction {
    /// Every action in button order.
    pub const ALL: [InventoryAction; 5] = [
        InventoryAction::Equip,
        InventoryAction::Use,
        InventoryAction::Stow,
        InventoryAction::Take,
        InventoryAction::Give,
    ];

    /// The button's text, the key first.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            InventoryAction::Equip => "E equip",
            InventoryAction::Use => "U use",
            InventoryAction::Stow => "S stow",
            InventoryAction::Take => "T take",
            InventoryAction::Give => "G give",
        }
    }

    /// The key that fires it.
    #[must_use]
    pub const fn key(self) -> char {
        match self {
            InventoryAction::Equip => 'e',
            InventoryAction::Use => 'u',
            InventoryAction::Stow => 's',
            InventoryAction::Take => 't',
            InventoryAction::Give => 'g',
        }
    }

    /// Whether the action applies to a pane and its row: the stores only take from, a kit
    /// does everything else, and the rest depends on the row.
    #[must_use]
    pub fn applies(self, stores: bool, row: Option<&ItemRow>) -> bool {
        let Some(row) = row else {
            return false;
        };
        match self {
            InventoryAction::Take => stores,
            InventoryAction::Equip => !stores && row.slot.is_some(),
            InventoryAction::Use => !stores && row.usable,
            InventoryAction::Stow | InventoryAction::Give => !stores,
        }
    }
}

/// What the overlay asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryIntent {
    /// An item command; the overlay stays open.
    Command(Command),
    /// Close the overlay.
    Close,
}

/// The overlay: Left/Right or Tab change the pane, Up/Down the row, a letter or Enter acts,
/// Escape or `i` closes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InventoryMenu {
    /// The pane shown.
    pub pane: usize,
    /// The row under the cursor.
    pub cursor: usize,
    /// How many panes the last view had, so a click knows where the rows start.
    pub panes: usize,
    /// Why the last action did nothing; empty when it did something.
    pub message: String,
}

impl InventoryMenu {
    /// Open on the band's selected member, or the first pane.
    pub fn open(&mut self, selected: Option<usize>, members: usize) {
        self.pane = selected.filter(|s| *s < members).unwrap_or(0);
        self.cursor = 0;
        self.message.clear();
    }

    /// Keep the pane and the cursor on what the view has.
    pub fn sync(&mut self, view: &InventoryView) {
        self.panes = view.panes.len();
        self.pane = self.pane.min(self.panes.saturating_sub(1));
        let rows = view.panes.get(self.pane).map_or(0, |p| p.rows.len());
        self.cursor = self.cursor.min(rows.saturating_sub(1));
    }

    /// A click on a widget row: the tabs come first, then the rows of the pane.
    pub fn click_row(&mut self, row: usize) {
        if row < self.panes {
            if self.pane != row {
                self.pane = row;
                self.cursor = 0;
            }
        } else {
            self.cursor = row - self.panes;
        }
    }

    /// Whether the pane shown is the stores.
    #[must_use]
    pub fn stores(&self, view: &InventoryView) -> bool {
        self.pane + 1 == view.panes.len()
    }

    /// The row under the cursor.
    #[must_use]
    pub fn row<'a>(&self, view: &'a InventoryView) -> Option<&'a ItemRow> {
        view.panes.get(self.pane)?.rows.get(self.cursor)
    }

    /// Handle a key; `selected` is the band's selected member.
    pub fn key(
        &mut self,
        key: MenuKey,
        view: &InventoryView,
        selected: Option<usize>,
    ) -> Option<InventoryIntent> {
        let rows = view.panes.get(self.pane).map_or(0, |p| p.rows.len());
        match key {
            MenuKey::Up | MenuKey::Down => self.cursor = cycle(self.cursor, rows, key),
            MenuKey::Left | MenuKey::Right => {
                self.pane = cycle(self.pane, view.panes.len(), key);
                self.cursor = 0;
            }
            MenuKey::Char('\t') => {
                self.pane = cycle(self.pane, view.panes.len(), MenuKey::Right);
                self.cursor = 0;
            }
            MenuKey::Escape | MenuKey::Char('i') => return Some(InventoryIntent::Close),
            MenuKey::Enter => return self.act(self.primary(view), view, selected),
            MenuKey::Char(c) => {
                let action = InventoryAction::ALL.iter().find(|a| a.key() == c)?;
                return self.act(*action, view, selected);
            }
            MenuKey::Backspace => {}
        }
        None
    }

    /// What Enter does on the row: take from the stores; wear, use, or stow from a kit.
    fn primary(&self, view: &InventoryView) -> InventoryAction {
        if self.stores(view) {
            return InventoryAction::Take;
        }
        match self.row(view) {
            Some(row) if row.slot.is_some() => InventoryAction::Equip,
            Some(row) if row.usable => InventoryAction::Use,
            _ => InventoryAction::Stow,
        }
    }

    /// The command an action on the row stands for, or the message why there is none.
    fn act(
        &mut self,
        action: InventoryAction,
        view: &InventoryView,
        selected: Option<usize>,
    ) -> Option<InventoryIntent> {
        self.message.clear();
        let stores = self.stores(view);
        let Some(row) = self.row(view) else {
            self.message = "Nothing here".to_owned();
            return None;
        };
        if !action.applies(stores, Some(row)) {
            self.message = match action {
                InventoryAction::Take => {
                    "Take is for the stores; Give hands things over".to_owned()
                }
                _ if stores => "Take it to a member first".to_owned(),
                InventoryAction::Equip => format!("{} cannot be worn or wielded", row.name),
                InventoryAction::Use => format!("{} does nothing when used", row.name),
                InventoryAction::Stow | InventoryAction::Give => "Nothing to do".to_owned(),
            };
            return None;
        }
        let id_at = |pane: usize| view.panes.get(pane).and_then(|p| p.member);
        let item = row.item.clone();
        let target = selected.and_then(id_at);
        // The stores pane has no member: `applies` lets only Take, which names its taker, act there.
        let command = match (action, id_at(self.pane)) {
            (InventoryAction::Take, _) => {
                let Some(member) = target.or_else(|| id_at(0)) else {
                    self.message = "Nobody to take it".to_owned();
                    return None;
                };
                ItemCommand::Take {
                    member,
                    item,
                    count: 1,
                }
            }
            (_, None) => return None,
            (InventoryAction::Equip, Some(member)) => match row.slot.filter(|_| row.equipped) {
                Some(slot) => ItemCommand::Unequip { member, slot },
                None => ItemCommand::Equip { member, item },
            },
            (InventoryAction::Use, Some(member)) => ItemCommand::Use {
                member,
                item,
                receiver: target,
            },
            (InventoryAction::Stow, Some(member)) => ItemCommand::Stow {
                member,
                item,
                count: 1,
            },
            (InventoryAction::Give, Some(member)) => match target {
                Some(to) if to != member => ItemCommand::Give {
                    giver: member,
                    receiver: to,
                    item,
                    count: 1,
                },
                _ => {
                    self.message = "Click another member on the band to give to".to_owned();
                    return None;
                }
            },
        };
        Some(InventoryIntent::Command(Command::Item(command)))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::combat_menu::tests::{data, facing};
    use omnis_sim::items::item_id;
    use omnis_sim::{World, party_view};

    fn inventory_view(world: &World, data: &Data) -> InventoryView {
        super::inventory_view(&party_view(world, data), data)
    }

    /// The id a sample row's name stands for: `Potion of healing` is `base:item:potion_of_healing`.
    pub(crate) fn id_of(name: &str) -> String {
        format!("base:item:{}", name.to_lowercase().replace(' ', "_"))
    }

    fn row(name: &str, slot: Option<EquipSlot>, equipped: bool, usable: bool) -> ItemRow {
        ItemRow {
            item: id_of(name),
            name: name.to_owned(),
            count: 1,
            slot,
            equipped,
            usable,
        }
    }

    /// The sample's members, whose ids are not their slots, so a slot sent as an id shows.
    const BRENNA: CharacterId = CharacterId(30);
    const DURIN: CharacterId = CharacterId(31);

    /// A sample view: a fighter's kit and a stocked store.
    pub(crate) fn sample() -> InventoryView {
        InventoryView {
            panes: vec![
                Pane {
                    member: Some(BRENNA),
                    title: "Brenna".to_owned(),
                    summary: "wearing Longsword, Shield, Light crossbow, Chain mail".to_owned(),
                    rows: vec![
                        row("Chain mail", Some(EquipSlot::Body), true, false),
                        row("Longsword", Some(EquipSlot::MainHand), true, false),
                        row("Shield", Some(EquipSlot::OffHand), true, false),
                        row("Light crossbow", Some(EquipSlot::Ranged), true, false),
                        ItemRow {
                            count: 20,
                            ..row("Crossbow bolt", None, false, false)
                        },
                        row("Holy symbol", None, false, false),
                        row("Potion of healing", None, false, true),
                        row("Leather", Some(EquipSlot::Body), false, false),
                    ],
                },
                Pane {
                    member: Some(DURIN),
                    title: "Durin".to_owned(),
                    summary: "wearing Mace, Shield, Scale mail".to_owned(),
                    rows: vec![row("Mace", Some(EquipSlot::MainHand), true, false)],
                },
                Pane {
                    member: None,
                    title: "STORES".to_owned(),
                    summary: "gold 90  food 60".to_owned(),
                    rows: vec![
                        ItemRow {
                            count: 3,
                            ..row("Potion of healing", None, false, true)
                        },
                        row("Spyglass", None, false, true),
                    ],
                },
            ],
        }
    }

    #[test]
    fn the_view_lists_kits_in_command_order_then_the_stores() {
        let data = data();
        let mut world = facing(&data, &["fighter", "cleric"], &[]);
        let potion = item_id(&data, "potion_of_healing").unwrap();
        world.party.inventory.push((potion, 2));
        let view = inventory_view(&world, &data);
        assert_eq!(view.panes.len(), 3);
        assert_eq!(view.members(), 2);
        let brenna = &view.panes[0];
        assert_eq!(brenna.title, world.party.members[0].name);
        assert_eq!(
            brenna.summary,
            "wearing Longsword, Shield, Light crossbow, Chain mail"
        );
        let names: Vec<&str> = brenna.rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Chain mail",
                "Longsword",
                "Shield",
                "Light crossbow",
                "Crossbow bolt",
                "Holy symbol (amulet)",
                "Potion of healing"
            ]
        );
        assert!(brenna.rows[0].equipped && brenna.rows[0].slot == Some(EquipSlot::Body));
        assert_eq!(brenna.rows[4].count, 20);
        assert!(brenna.rows[6].usable && !brenna.rows[6].equipped);
        let stores = &view.panes[2];
        assert_eq!(stores.title, "STORES");
        assert_eq!(stores.summary, "30 gp 0 sp 0 cp  food 20");
        assert_eq!(
            (stores.rows[0].name.as_str(), stores.rows[0].count),
            ("Potion of healing", 2)
        );
        world.party.members[0].equipped.clear();
        assert_eq!(
            inventory_view(&world, &data).panes[0].summary,
            "wearing nothing"
        );
    }

    #[test]
    fn keys_move_between_panes_and_rows_and_act_on_the_row() {
        let view = sample();
        let mut menu = InventoryMenu::default();
        menu.open(Some(1), 2);
        assert_eq!(menu.pane, 1);
        menu.sync(&view);
        assert_eq!(menu.panes, 3);
        assert_eq!(menu.key(MenuKey::Left, &view, None), None);
        assert_eq!(menu.pane, 0);
        menu.key(MenuKey::Up, &view, None);
        assert_eq!(menu.cursor, 7, "up from the top wraps");
        let equip = menu.key(MenuKey::Enter, &view, None);
        assert_eq!(
            equip,
            Some(InventoryIntent::Command(Command::Item(
                ItemCommand::Equip {
                    member: BRENNA,
                    item: id_of("Leather")
                }
            )))
        );
        menu.cursor = 0;
        assert_eq!(
            menu.key(MenuKey::Char('e'), &view, None),
            Some(InventoryIntent::Command(Command::Item(
                ItemCommand::Unequip {
                    member: BRENNA,
                    slot: EquipSlot::Body
                }
            ))),
            "a worn row comes off"
        );
        menu.cursor = 6;
        assert_eq!(
            menu.key(MenuKey::Enter, &view, Some(1)),
            Some(InventoryIntent::Command(Command::Item(ItemCommand::Use {
                member: BRENNA,
                item: id_of("Potion of healing"),
                receiver: Some(DURIN)
            }))),
            "a potion's Enter uses it on the selected member"
        );
        assert_eq!(
            menu.key(MenuKey::Char('g'), &view, Some(1)),
            Some(InventoryIntent::Command(Command::Item(ItemCommand::Give {
                giver: BRENNA,
                receiver: DURIN,
                item: id_of("Potion of healing"),
                count: 1
            })))
        );
        assert_eq!(menu.key(MenuKey::Char('g'), &view, Some(0)), None);
        assert!(menu.message.starts_with("Click another member"));
        assert_eq!(menu.key(MenuKey::Char('t'), &view, None), None);
        assert!(menu.message.starts_with("Take is for the stores"));
        menu.cursor = 5;
        assert_eq!(menu.key(MenuKey::Char('e'), &view, None), None);
        assert_eq!(menu.message, "Holy symbol cannot be worn or wielded");
        assert_eq!(menu.key(MenuKey::Char('u'), &view, None), None);
        assert_eq!(menu.message, "Holy symbol does nothing when used");
        assert_eq!(
            menu.key(MenuKey::Enter, &view, None),
            Some(InventoryIntent::Command(Command::Item(ItemCommand::Stow {
                member: BRENNA,
                item: id_of("Holy symbol"),
                count: 1
            }))),
            "Enter on plain gear stows it"
        );
        assert_eq!(menu.key(MenuKey::Char('\t'), &view, None), None);
        assert_eq!((menu.pane, menu.cursor), (1, 0));
        menu.key(MenuKey::Char('\t'), &view, None);
        assert!(menu.stores(&view));
        assert_eq!(
            menu.key(MenuKey::Enter, &view, Some(1)),
            Some(InventoryIntent::Command(Command::Item(ItemCommand::Take {
                member: DURIN,
                item: id_of("Potion of healing"),
                count: 1
            })))
        );
        assert_eq!(menu.key(MenuKey::Char('u'), &view, None), None);
        assert_eq!(menu.message, "Take it to a member first");
        assert_eq!(
            menu.key(MenuKey::Escape, &view, None),
            Some(InventoryIntent::Close)
        );
        assert_eq!(
            menu.key(MenuKey::Char('i'), &view, None),
            Some(InventoryIntent::Close)
        );
        assert_eq!(menu.key(MenuKey::Char('x'), &view, None), None);
    }

    #[test]
    fn clicks_pick_tabs_then_rows_and_an_empty_pane_says_so() {
        let view = sample();
        let mut menu = InventoryMenu::default();
        menu.sync(&view);
        menu.click_row(2);
        assert_eq!((menu.pane, menu.cursor), (2, 0));
        menu.click_row(4);
        assert_eq!((menu.pane, menu.cursor), (2, 1));
        menu.click_row(2);
        assert_eq!(menu.cursor, 1, "the same tab keeps the cursor");
        let empty = InventoryView {
            panes: vec![Pane {
                member: None,
                title: "STORES".to_owned(),
                summary: String::new(),
                rows: Vec::new(),
            }],
        };
        menu.sync(&empty);
        assert_eq!((menu.pane, menu.cursor), (0, 0));
        assert_eq!(menu.key(MenuKey::Enter, &empty, None), None);
        assert_eq!(menu.message, "Nothing here");
        assert!(!InventoryAction::Take.applies(true, None));
        assert!(InventoryAction::Take.applies(true, Some(&view.panes[2].rows[0])));
        assert!(!InventoryAction::Give.applies(true, Some(&view.panes[2].rows[0])));
    }
}
