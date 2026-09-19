//! The debug menu painted in the menu box: a header, one row per editable thing with `<`
//! and `>` around the field the mouse can push, and a help line. Bevy-free.

use crate::debug_menu::{
    DebugMenu, DebugView, ROW_CONDITION, ROW_FLAG, ROW_FOOD, ROW_GOLD, ROW_HP, ROW_ITEM, ROW_MAP,
    ROW_MEMBER, ROW_SCORE, ROW_SP, ROW_STACK, ROW_XP, ROWS,
};
use crate::font::fit;
use crate::layout::MENU_COLUMNS;
use crate::screens::{ItemState, item_state, label, label_right};
use crate::widget::{DIM, Frame, HI, Kind, WidgetId};
use omnis_sim::omnis_data::Ability;

/// The first row's line on the menu grid.
const FIRST_ROW: i32 = 2;
/// Cells a row may take.
const ROW_CELLS: usize = MENU_COLUMNS as usize - 2;

/// The four facings the map row cycles, as the model orders them.
const FACINGS: [&str; 4] = ["north", "east", "south", "west"];

/// `<text>` when the field is under the cursor, else ` text `.
fn field(text: &str, chosen: bool) -> String {
    if chosen {
        format!("<{text}>")
    } else {
        format!(" {text} ")
    }
}

/// The text of one row: the field under the cursor is bracketed, and on every other row the
/// first field is, so a click on its arrow has something to hit.
fn row_text(menu: &DebugMenu, view: &DebugView, row: usize) -> String {
    let on = |f: usize| {
        if menu.row == row {
            menu.field == f
        } else {
            f == 0
        }
    };
    let member = view.members.get(menu.member);
    match row {
        ROW_MEMBER => {
            let name = member.map_or("nobody", |m| m.name.as_str());
            format!("Member   {}", field(name, on(0)))
        }
        ROW_HP => member.map_or_else(String::new, |m| {
            format!(
                "HP       {} / {}",
                field(&m.hp.0.to_string(), on(0)),
                m.hp.1
            )
        }),
        ROW_SP => member.map_or_else(String::new, |m| {
            format!(
                "SP       {} / {}",
                field(&m.sp.0.to_string(), on(0)),
                m.sp.1
            )
        }),
        ROW_XP => member.map_or_else(String::new, |m| {
            format!("XP       {}", field(&m.xp.to_string(), on(0)))
        }),
        ROW_SCORE => member.map_or_else(String::new, |m| {
            let ability = Ability::ALL[menu.ability.min(5)];
            format!(
                "Score    {}  {}   (hp max unchanged)",
                field(ability.short(), on(0)),
                field(&m.scores[menu.ability.min(5)].to_string(), on(1))
            )
        }),
        ROW_CONDITION => {
            let (id, name) = view
                .conditions
                .get(menu.condition)
                .map_or(("?", "?"), |(id, n)| (id.as_str(), n.as_str()));
            let state = member.is_some_and(|m| m.conditions.iter().any(|c| c == id));
            format!(
                "Cond     {}  {}   Enter toggles",
                field(name, on(0)),
                if state { "on" } else { "off" }
            )
        }
        ROW_ITEM => {
            let name = view.items.get(menu.item).map_or("?", |(_, n)| n.as_str());
            format!(
                "Item     {}  x{}   Enter to member  S to stores",
                field(name, on(0)),
                field(&menu.count.to_string(), on(1))
            )
        }
        ROW_GOLD => format!("Gold     {}", field(&view.gold.to_string(), on(0))),
        ROW_FOOD => format!("Food     {}", field(&view.food.to_string(), on(0))),
        ROW_FLAG => {
            let name = view
                .flags
                .get(menu.flag)
                .map_or("(no flags)", String::as_str);
            format!("Flag     {}   Enter sets 1", field(name, on(0)))
        }
        ROW_MAP => {
            let name = view.maps.get(menu.map).map_or("?", |(n, _, _)| n.as_str());
            format!(
                "Map      {}  x{} y{}  {}   Enter teleports",
                field(name, on(0)),
                field(&menu.tile.0.to_string(), on(1)),
                field(&menu.tile.1.to_string(), on(2)),
                field(FACINGS[menu.facing.min(3)], on(3))
            )
        }
        ROW_STACK => match view.stacks.get(menu.stack) {
            Some(s) => format!(
                "Stack    {}  hp {}   K kills",
                field(
                    &format!("{} {} {}/{}", s.index, s.name, s.count.0, s.count.1),
                    on(0)
                ),
                field(&s.lead_hp.to_string(), on(1))
            ),
            None => "Stack    (no fight)".to_owned(),
        },
        _ => String::new(),
    }
}

/// The debug menu.
pub fn debug(frame: &mut Frame, menu: &DebugMenu, view: &DebugView) {
    label(frame, 1, 0, "DEBUG", HI);
    let mode = if view.fighting {
        "fighting"
    } else {
        "exploring"
    };
    let dev = if view.devtools {
        "dev world"
    } else {
        "dev off: every edit is refused"
    };
    label_right(frame, 0, &format!("{mode}   {dev}"), DIM);
    for row in 0..ROWS {
        let text = fit(&row_text(menu, view, row), ROW_CELLS);
        let inert = (row == ROW_STACK && !view.fighting) || (row == ROW_MAP && view.fighting);
        let state = if inert {
            ItemState::Disabled
        } else {
            ItemState::from_selected(menu.row == row)
        };
        item_state(
            frame,
            WidgetId::Row(row),
            Kind::Choice,
            (1, FIRST_ROW + row as i32),
            &text,
            ROW_CELLS,
            state,
        );
    }
    label(
        frame,
        1,
        FIRST_ROW + ROWS as i32 + 1,
        "Up/Down row  Left/Right 1  -/+ 10  [/] 100  Tab field  Enter act  Esc close",
        DIM,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_menu::{MemberDebug, StackDebug};
    use crate::layout::MENU_BOX;
    use crate::screens::tests::assert_laid_out;
    use omnis_sim::omnis_core::Facing;

    fn view(fighting: bool) -> DebugView {
        DebugView {
            fighting,
            devtools: true,
            members: vec![MemberDebug {
                name: "Bartholomew Longname Jr".to_owned(),
                hp: (999, 999),
                sp: (99, 99),
                xp: 355_000,
                scores: [30; 6],
                conditions: vec!["base:condition:poisoned".to_owned()],
            }],
            gold: 999_999,
            food: 9999,
            items: vec![("base:item:x".to_owned(), "Potion of healing".to_owned())],
            conditions: vec![("base:condition:poisoned".to_owned(), "Poisoned".to_owned())],
            flags: vec![],
            maps: vec![("test:map:dungeon".to_owned(), 24, 24)],
            position: (0, 3, 8, Facing::South),
            stacks: if fighting {
                vec![StackDebug {
                    index: 0,
                    name: "Ancient Red Dragon".to_owned(),
                    count: (99, 99),
                    lead_hp: 999,
                }]
            } else {
                vec![]
            },
        }
    }

    #[test]
    fn every_row_fits_the_box_and_the_inert_rows_follow_the_mode() {
        for fighting in [false, true] {
            let mut frame = Frame::default();
            let mut menu = DebugMenu::default();
            let v = view(fighting);
            menu.open(&v);
            menu.row = ROW_ITEM;
            menu.field = 1;
            debug(&mut frame, &menu, &v);
            assert_laid_out(&frame, MENU_BOX);
            assert_eq!(frame.widgets.len(), ROWS);
            let map = frame.widget(WidgetId::Row(ROW_MAP)).unwrap();
            let stack = frame.widget(WidgetId::Row(ROW_STACK)).unwrap();
            assert_eq!(map.enabled, !fighting);
            assert_eq!(stack.enabled, fighting);
            let item = frame.widget(WidgetId::Row(ROW_ITEM)).unwrap();
            assert!(
                item.left.is_some() && item.right.is_some(),
                "arrows on the field"
            );
            assert!(
                row_text(&menu, &v, ROW_ITEM).contains("<1>"),
                "the count is the chosen field"
            );
            assert!(
                row_text(&menu, &v, ROW_GOLD).contains("<999999>"),
                "another row brackets its first field for the mouse"
            );
            assert!(row_text(&menu, &v, ROW_CONDITION).ends_with("on   Enter toggles"));
        }
    }
}
