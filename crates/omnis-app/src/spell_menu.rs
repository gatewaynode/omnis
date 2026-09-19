//! The spell picker in a fight: the rows the acting member's spells make, why one is grey,
//! and the keys while it is open (the `CombatMenu` opens it from Cast). Bevy-free.

use crate::combat_menu::{CombatIntent, CombatMenu, FightView};
use crate::font::fit;
use crate::layout::MENU_COLUMNS;
use crate::menu::{MenuKey, cycle};
use crate::screens::{ItemState, item_state, label};
use crate::widget::{DIM, Frame, HI, Kind, WidgetId};
use omnis_sim::combat::cast;
use omnis_sim::omnis_core::{Pcg32, StreamName};
use omnis_sim::omnis_data::Data;
use omnis_sim::{CombatCommand, Command, Rejection, Target, World};

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

// ---------------------------------------------------------------- outside a fight

/// One spell a member can cast while exploring, as the cast menu lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastRow {
    /// The caster's slot.
    pub caster: u8,
    /// The caster's name.
    pub caster_name: String,
    /// The spell's index in the caster's list.
    pub spell: u8,
    /// The spell's name.
    pub name: String,
    /// Points it costs.
    pub cost: u32,
    /// Aimed at a member (the band's selected one), else at the caster.
    pub targets_members: bool,
    /// Why it cannot be cast now, in a few words.
    pub blocked: Option<String>,
}

/// Every member's spells that can be cast outside a fight, in marching order.
#[must_use]
pub fn cast_rows(world: &World, data: &Data) -> Vec<CastRow> {
    let stream = StreamName::new("cast");
    let mut rng = world
        .rngs
        .get(&stream)
        .copied()
        .unwrap_or_else(|| Pcg32::for_stream(world.seed, &stream));
    let mut rows = Vec::new();
    for (own, member) in world.party.members.iter().enumerate() {
        for (i, id) in member.known_spells.iter().enumerate() {
            let Some(spell) = data.spells.get(id) else {
                continue;
            };
            let explore = spell.effect.as_ref().is_some_and(|e| e.explore_castable());
            if !explore {
                continue;
            }
            let (caster, index) = (
                u8::try_from(own).unwrap_or(u8::MAX),
                u8::try_from(i).unwrap_or(u8::MAX),
            );
            rows.push(CastRow {
                caster,
                caster_name: member.name.clone(),
                spell: index,
                name: data.label("en", &spell.name).to_owned(),
                cost: spell.point_cost(),
                targets_members: spell.effect.as_ref().is_some_and(|e| e.targets_members()),
                blocked: cast::check(world, data, own, index, false, &mut rng)
                    .err()
                    .map(|r| blocked_note(&r)),
            });
        }
    }
    rows
}

/// What the cast menu asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastIntent {
    /// A cast.
    Command(Command),
    /// Close the menu.
    Close,
}

/// The cast menu outside a fight: Up/Down choose a row, Enter casts it at the band's
/// selected member (or the caster), Escape closes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CastMenu {
    /// The row under the cursor.
    pub cursor: usize,
    /// Why the last confirmation did nothing; empty when it did something.
    pub message: String,
}

impl CastMenu {
    /// Keep the cursor on a row.
    pub fn sync(&mut self, rows: &[CastRow]) {
        self.cursor = self.cursor.min(rows.len().saturating_sub(1));
    }

    /// Handle a key.
    pub fn key(
        &mut self,
        key: MenuKey,
        rows: &[CastRow],
        selected: Option<usize>,
    ) -> Option<CastIntent> {
        match key {
            MenuKey::Up | MenuKey::Down => self.cursor = cycle(self.cursor, rows.len(), key),
            MenuKey::Enter => return self.confirm(rows, selected),
            MenuKey::Escape | MenuKey::Char('c') => return Some(CastIntent::Close),
            _ => {}
        }
        None
    }

    fn confirm(&mut self, rows: &[CastRow], selected: Option<usize>) -> Option<CastIntent> {
        self.message.clear();
        let Some(row) = rows.get(self.cursor) else {
            self.message = "Nobody can cast anything here".to_owned();
            return None;
        };
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
            Target::Member(row.caster)
        };
        Some(CastIntent::Command(Command::Cast {
            caster: row.caster,
            spell: row.spell,
            target,
        }))
    }
}

/// The cast menu painted in the menu box: a header, one row per castable spell, a help line.
pub fn cast_screen(frame: &mut Frame, menu: &CastMenu, rows: &[CastRow]) {
    label(frame, 1, 1, "CAST", HI);
    if rows.is_empty() {
        label(frame, 1, 3, "Nobody knows a spell for the road.", DIM);
    }
    let cells = MENU_COLUMNS as usize - 2;
    for (i, row) in rows.iter().enumerate().take(12) {
        let note = row.blocked.as_deref().unwrap_or("");
        let text = format!(
            "{:<12} {:<18} {:>2} pt  {}",
            fit(&row.caster_name, 12),
            fit(&row.name, 18),
            row.cost.min(99),
            fit(note, 16)
        );
        let state = if row.blocked.is_some() {
            ItemState::Disabled
        } else {
            ItemState::from_selected(menu.cursor == i)
        };
        item_state(
            frame,
            WidgetId::Row(i),
            Kind::Button,
            (1, 3 + i as i32),
            &text,
            cells,
            state,
        );
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
        assert_eq!(
            (menu.use_picker, menu.message.as_str()),
            (Some(0), ""),
            "the acolyte's potion opens the item picker"
        );
    }

    #[test]
    fn the_cast_menu_lists_the_road_spells_and_casts_them() {
        let data = data();
        let mut world = facing(&data, &["fighter", "cleric", "wizard"], &[("giant_rat", 1)]);
        world.mode = Mode::Explore;
        let rows = cast_rows(&world, &data);
        let names: Vec<(&str, &str)> = rows
            .iter()
            .map(|r| (r.caster_name.as_str(), r.name.as_str()))
            .collect();
        assert_eq!(
            names,
            [
                ("Gorm", "Guidance"),
                ("Gorm", "Light"),
                ("Gorm", "Bless"),
                ("Gorm", "Cure Wounds"),
                ("Wren", "Light"),
                ("Wren", "Mage Hand"),
            ],
            "the cleric's and the wizard's road spells, in marching order"
        );
        let mut menu = CastMenu::default();
        for _ in 0..3 {
            menu.key(MenuKey::Down, &rows, None);
        }
        assert_eq!(menu.key(MenuKey::Enter, &rows, None), None);
        assert_eq!(menu.message, "Select a member to cast Cure Wounds on");
        assert_eq!(
            menu.key(MenuKey::Enter, &rows, Some(0)),
            Some(CastIntent::Command(Command::Cast {
                caster: 1,
                spell: rows[3].spell,
                target: Target::Member(0)
            }))
        );
        menu.key(MenuKey::Down, &rows, None);
        assert_eq!(
            menu.key(MenuKey::Enter, &rows, None),
            Some(CastIntent::Command(Command::Cast {
                caster: 2,
                spell: rows[4].spell,
                target: Target::Member(2)
            })),
            "light needs no target"
        );
        world.party.members[1].spell_points = 0;
        let rows = cast_rows(&world, &data);
        assert_eq!(rows[3].blocked.as_deref(), Some("need 1 pt"));
        assert_eq!(rows[0].blocked, None, "guidance is free");
        assert_eq!(
            menu.key(MenuKey::Escape, &rows, None),
            Some(CastIntent::Close)
        );
        let mut frame = crate::widget::Frame::default();
        cast_screen(&mut frame, &menu, &rows);
        assert_eq!(frame.widgets.len(), 6);
        assert!(!frame.widget(WidgetId::Row(3)).unwrap().enabled);
    }
}
