//! The spell picker in a fight: the rows the acting member's spells make, why one is grey,
//! and the keys while it is open (the `CombatMenu` opens it from Cast). Bevy-free.

use crate::combat_menu::{CombatIntent, CombatMenu, FightView};
use crate::font::fit;
use crate::layout::MENU_COLUMNS;
use crate::menu::{MenuKey, cycle};
use crate::screens::{ItemState, item_state, label};
use crate::sim::Views;
use crate::widget::{DIM, Frame, HI, Kind, WidgetId};
use omnis_sim::omnis_data::Data;
use omnis_sim::{CombatCommand, Command, Pay, Rejection, Target};

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
    /// A reaction spell: cast only by a declared reaction (the tactics panel), never picked.
    pub reaction: bool,
    /// Whether an effect of this spell by this caster is in force.
    pub active: bool,
    /// Why it cannot be cast now, in a few words; `None` when the action or the bonus
    /// action can pay for it.
    pub blocked: Option<String>,
    /// Whether the bonus action pays for it now; it is cast that way when so, leaving the
    /// action.
    pub bonus: bool,
}

impl SpellRow {
    /// The note after the cost: that it is a reaction, the reason it is grey, that it is
    /// in force, or that the bonus action pays for it.
    #[must_use]
    pub fn note(&self) -> String {
        match (&self.blocked, self.reaction, self.active) {
            (_, true, _) => "reaction".to_owned(),
            (Some(why), false, _) => why.clone(),
            (None, false, true) => "in force".to_owned(),
            (None, false, false) if self.bonus => "bonus action".to_owned(),
            (None, false, false) => String::new(),
        }
    }

    /// What pays for a cast from the picker: the bonus action when it can.
    #[must_use]
    pub const fn pay(&self) -> Pay {
        if self.bonus {
            Pay::BonusAction
        } else {
            Pay::Action
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
        if row.reaction {
            self.message = format!("{} is a reaction: declare it in tactics", row.name);
            return None;
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
            pay: row.pay(),
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
pub fn cast_rows(views: &Views, data: &Data) -> Vec<CastRow> {
    views
        .casts
        .iter()
        .map(|c| CastRow {
            caster: c.caster,
            caster_name: views
                .party
                .members
                .get(usize::from(c.caster))
                .map_or_else(String::new, |m| m.name.clone()),
            spell: c.spell,
            name: data.label("en", &c.name).to_owned(),
            cost: c.cost,
            targets_members: c.targets_members,
            blocked: c.refusal.as_ref().map(blocked_note),
        })
        .collect()
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
    use crate::combat_menu::tests::{data, facing, facing_goblins, fight_view};
    use omnis_sim::omnis_data::Data;
    use omnis_sim::{Command, EncounterChoice, Mode, World, apply};

    fn cast_rows(world: &World, data: &Data) -> Vec<CastRow> {
        super::cast_rows(&Views::of(world, data), data)
    }

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
    fn a_bonus_action_spell_is_cast_with_the_bonus_action_and_leaves_the_action() {
        let data = data();
        let mut world = facing(&data, &["cleric"], &[("giant_rat", 1)]);
        let word = data.registry.spells.get("base:spell:healing_word").unwrap();
        world.party.members[0].known_spells.push(word);
        apply(
            &mut world,
            &data,
            Command::Encounter(EncounterChoice::Attack),
        )
        .unwrap();
        let view = fight_view(&world, &data).unwrap();
        let row = |view: &FightView, name: &str| {
            let at = view.spells.iter().position(|s| s.name == name).unwrap();
            (at, view.spells[at].clone())
        };
        let (word_at, word) = row(&view, "Healing Word");
        let (flame_at, flame) = row(&view, "Sacred Flame");
        assert!(word.bonus && word.blocked.is_none(), "{word:?}");
        assert_eq!(word.note(), "bonus action");
        assert!(!flame.bonus && flame.blocked.is_none(), "{flame:?}");
        let mut menu = CombatMenu {
            cursor: crate::combat_menu::ACTION_CAST,
            picker: Some(word_at),
            ..CombatMenu::default()
        };
        let cast = menu.key(MenuKey::Enter, &view, Some(0));
        assert_eq!(
            cast,
            Some(CombatIntent::Command(CombatCommand::Cast {
                spell: word.index,
                target: Target::Member(0),
                pay: Pay::BonusAction,
            }))
        );
        menu.picker = Some(flame_at);
        assert!(matches!(
            menu.key(MenuKey::Enter, &view, None),
            Some(CombatIntent::Command(CombatCommand::Cast {
                pay: Pay::Action,
                ..
            }))
        ));
        let Some(CombatIntent::Command(command)) = cast else {
            unreachable!()
        };
        apply(&mut world, &data, Command::Combat(command)).unwrap();
        let after = fight_view(&world, &data).unwrap();
        assert_eq!((after.budget.actions, after.budget.bonus_actions), (1, 0));
        let (_, word) = row(&after, "Healing Word");
        assert!(!word.bonus, "the bonus action is spent");
        assert!(
            word.blocked.is_some(),
            "and the action may not cast a second spell"
        );
        let (_, flame) = row(&after, "Sacred Flame");
        assert!(
            flame.blocked.is_none(),
            "a cantrip still goes with the action"
        );
    }

    #[test]
    fn a_spent_action_leaves_a_bonus_action_spell_open_and_the_view_reads_own_reactions() {
        let data = data();
        let mut world = facing(&data, &["cleric"], &[("giant_rat", 1)]);
        let word = data.registry.spells.get("base:spell:healing_word").unwrap();
        world.party.members[0].known_spells.push(word);
        apply(
            &mut world,
            &data,
            Command::Encounter(EncounterChoice::Attack),
        )
        .unwrap();
        apply(&mut world, &data, Command::Combat(CombatCommand::Dodge)).unwrap();
        let cleric = world.party.members[0].id;
        let Mode::Combat(state) = &mut world.mode else {
            unreachable!()
        };
        assert_eq!(state.budget.actions, 0, "Dodge spent the action");
        state.set_reactions(omnis_sim::ActorRef::Member(cleric), 0);
        let view = fight_view(&world, &data).unwrap();
        let row = |name: &str| view.spells.iter().find(|s| s.name == name).unwrap();
        assert!(
            row("Healing Word").blocked.is_none() && row("Healing Word").bonus,
            "the bonus action still pays: {:?}",
            row("Healing Word")
        );
        assert!(row("Sacred Flame").blocked.is_some(), "the action is gone");
        assert_eq!(
            view.reactions_left, 0,
            "the cleric's own count, not the rat's"
        );
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
                "Burning Hands",
                "Thunderwave"
            ]
        );
        assert_eq!(view.spells[2].note(), "not here", "mage hand is for doors");
        assert_eq!(view.spells[4].note(), "reaction");
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
                target: Target::Stack(0),
                pay: Pay::Action,
            }))
        );
        assert_eq!(menu.picker, None, "the picker closes on a cast");
        menu.key(MenuKey::Char('c'), &view, None);
        for _ in 0..4 {
            menu.key(MenuKey::Down, &view, None);
        }
        assert_eq!(
            menu.key(MenuKey::Enter, &view, None),
            None,
            "a reaction is declared in tactics, never picked"
        );
        assert_eq!(menu.message, "Shield is a reaction: declare it in tactics");
        assert_eq!(view.spells[4].note(), "reaction");
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
                target: Target::Stack(0),
                pay: Pay::Action,
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
                target: Target::Member(0),
                pay: Pay::Action,
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
