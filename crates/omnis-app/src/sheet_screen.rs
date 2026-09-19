//! The character sheet painted in the menu box: a header with three tabs and the member
//! choice, then one of three pages laid out in columns. Bevy-free; the only widgets are
//! the tabs (`Row(0..3)`) and the member row (`Row(3)`, a choice with arrows).

use crate::font::fit;
use crate::layout::MENU_COLUMNS;
use crate::screens::{ItemState, item_state, label, label_right};
use crate::sheet_menu::{SheetMenu, SheetPage, SheetView, signed};
use crate::widget::{DIM, Frame, HI, Kind, TEXT, WidgetId};

/// The widget row of the member choice, after the three tabs.
pub const ROW_MEMBER: usize = 3;
/// The tabs' columns on row 0.
const TAB_COLUMNS: [i32; 3] = [8, 16, 24];
/// Cells a full-width row may take.
const ROW_CELLS: usize = MENU_COLUMNS as usize - 2;

/// A list laid out `per_row` entries to a row from `first_row`, `pitch` cells apart, over
/// at most `rows` rows; an empty list says `empty`.
fn columns(
    frame: &mut Frame,
    first_row: i32,
    per_row: usize,
    pitch: i32,
    rows: usize,
    entries: &[String],
    empty: &str,
) {
    if entries.is_empty() {
        label(frame, 1, first_row, empty, DIM);
        return;
    }
    let width = usize::try_from(pitch).unwrap_or(1).saturating_sub(1);
    for (i, entry) in entries.iter().enumerate().take(per_row * rows) {
        let (column, row) = (i % per_row, i / per_row);
        label(
            frame,
            1 + column as i32 * pitch,
            first_row + row as i32,
            &fit(entry, width),
            TEXT,
        );
    }
}

/// The header: the title, the tabs, the member count, and the member row.
fn header(frame: &mut Frame, menu: &SheetMenu, view: &SheetView, members: usize) {
    label(frame, 1, 0, "SHEET", HI);
    for (page, column) in SheetPage::ALL.iter().zip(TAB_COLUMNS) {
        item_state(
            frame,
            WidgetId::Row(page.index()),
            Kind::Button,
            (column, 0),
            page.label(),
            page.label().len(),
            ItemState::from_selected(*page == menu.page),
        );
    }
    label_right(
        frame,
        0,
        &format!("member {} of {}", menu.member + 1, members),
        DIM,
    );
    let text = format!(
        "< {:<24} >  {:<20}  {:<10} {:>2}",
        fit(&view.name, 24),
        fit(&view.race, 20),
        fit(&view.class, 10),
        view.level.min(99)
    );
    item_state(
        frame,
        WidgetId::Row(ROW_MEMBER),
        Kind::Choice,
        (1, 1),
        &text,
        ROW_CELLS,
        ItemState::Normal,
    );
}

fn stats(frame: &mut Frame, view: &SheetView) {
    let next = view
        .next_xp
        .map_or("max".to_owned(), |n| n.min(999_999).to_string());
    label(
        frame,
        1,
        3,
        &format!(
            "HP {}/{}   SP {}/{}   AC {}   Proficiency {}   XP {} / {next}",
            view.hp.0.clamp(-99, 999),
            view.hp.1.clamp(0, 999),
            view.sp.0.min(99),
            view.sp.1.min(99),
            view.ac.clamp(0, 99),
            signed(view.proficiency),
            view.xp.min(999_999),
        ),
        TEXT,
    );
    let scores: Vec<String> = view
        .scores
        .iter()
        .map(|(name, score, bonus)| format!("{name} {score:>2} {}", signed(*bonus)))
        .collect();
    columns(frame, 5, 6, 13, 1, &scores, "");
    let saves: Vec<String> = view
        .saves
        .iter()
        .map(|(name, bonus)| format!("{name} {}", signed(*bonus)))
        .collect();
    label(frame, 1, 6, &format!("Saves  {}", saves.join("   ")), TEXT);
    label(frame, 1, 8, "SKILLS", DIM);
    let skills: Vec<String> = view
        .skills
        .iter()
        .map(|(name, bonus)| format!("{:<17} {}", fit(name, 17), signed(*bonus)))
        .collect();
    columns(frame, 9, 3, 26, 3, &skills, "none");
    let conditions = if view.conditions.is_empty() {
        "none".to_owned()
    } else {
        view.conditions.join(", ")
    };
    label(
        frame,
        1,
        13,
        &fit(&format!("Conditions  {conditions}"), ROW_CELLS),
        TEXT,
    );
    label(
        frame,
        1,
        14,
        &fit(
            &format!(
                "{}   {}   {} years",
                view.background, view.alignment, view.age_years
            ),
            ROW_CELLS,
        ),
        TEXT,
    );
}

fn magic(frame: &mut Frame, view: &SheetView) {
    let magic = &view.magic;
    let line = match &magic.casting {
        Some(ability) => format!(
            "Casting {ability}   Points {}/{}",
            magic.points.0.min(99),
            magic.points.1.min(99)
        ),
        None => "No spellcasting".to_owned(),
    };
    label(frame, 1, 3, &line, TEXT);
    label(frame, 1, 5, "SPELLS", DIM);
    let spells: Vec<String> = magic
        .spells
        .iter()
        .map(|(name, cost)| format!("{:<16} {cost}", fit(name, 16)))
        .collect();
    columns(frame, 6, 3, 26, 4, &spells, "none");
    label(frame, 1, 11, "EFFECTS", DIM);
    let effects: Vec<String> = magic
        .effects
        .iter()
        .map(|(name, left)| format!("{:<20} {left}", fit(name, 20)))
        .collect();
    columns(frame, 12, 2, 39, 3, &effects, "none");
}

fn gear(frame: &mut Frame, view: &SheetView) {
    label(frame, 1, 3, "WORN AND WIELDED", DIM);
    for (i, (slot, item)) in view.gear.slots.iter().enumerate().take(4) {
        label(
            frame,
            1,
            4 + i as i32,
            &format!("{:<10} {}", fit(slot, 10), fit(item, 40)),
            TEXT,
        );
    }
    label(frame, 1, 9, "CARRIED", DIM);
    let carried: Vec<String> = view
        .gear
        .carried
        .iter()
        .map(|(name, count)| format!("{:<28} x{}", fit(name, 28), count))
        .collect();
    columns(frame, 10, 2, 39, 6, &carried, "none");
}

/// The sheet: the header and the page under the cursor.
pub fn sheet(frame: &mut Frame, menu: &SheetMenu, view: &SheetView, members: usize) {
    header(frame, menu, view, members);
    match menu.page {
        SheetPage::Stats => stats(frame, view),
        SheetPage::Magic => magic(frame, view),
        SheetPage::Gear => gear(frame, view),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{MENU_BOX, menu_cell};
    use crate::screens::tests::assert_laid_out;
    use crate::sheet_menu::{SheetGear, SheetMagic};
    use crate::widget::Part;
    use crate::widget::hit;

    /// A sheet with every field at its widest.
    fn widest() -> SheetView {
        let long = "x".repeat(40);
        SheetView {
            name: long.clone(),
            race: long.clone(),
            class: long.clone(),
            level: 20,
            background: long.clone(),
            alignment: "Chaotic Neutral".to_owned(),
            age_years: 99_999,
            hp: (-999, 9999),
            sp: (999, 999),
            ac: 999,
            proficiency: 10,
            xp: 9_999_999,
            next_xp: Some(9_999_999),
            scores: [
                ("STR", 30, 10),
                ("DEX", 30, 10),
                ("CON", 30, 10),
                ("INT", 30, 10),
                ("WIS", 30, 10),
                ("CHA", 30, 10),
            ],
            saves: vec![("STR", 10), ("CON", 10)],
            skills: (0..18).map(|i| (format!("{long}{i}"), -10)).collect(),
            conditions: (0..8).map(|i| format!("{long}{i}")).collect(),
            magic: SheetMagic {
                casting: Some("Intelligence".to_owned()),
                points: (999, 999),
                spells: (0..20)
                    .map(|i| (format!("{long}{i}"), "99 pt".to_owned()))
                    .collect(),
                effects: (0..9)
                    .map(|i| (format!("{long}{i}"), "999999 min".to_owned()))
                    .collect(),
            },
            gear: SheetGear {
                slots: vec![
                    ("Main hand".to_owned(), long.clone()),
                    ("Off hand".to_owned(), long.clone()),
                    ("Ranged".to_owned(), long.clone()),
                    ("Body".to_owned(), long.clone()),
                ],
                carried: (0..20).map(|i| (format!("{long}{i}"), 65535)).collect(),
            },
        }
    }

    /// No lit pixel to the right of the menu grid's last column inside the box.
    fn assert_no_text_past_the_grid(frame: &Frame) {
        let edge = menu_cell(MENU_COLUMNS, 0).0;
        for y in MENU_BOX.y..MENU_BOX.bottom() {
            for x in edge..MENU_BOX.right() {
                let lit = frame.raster.get(x, y).is_some_and(|p| p[3] != 0);
                assert!(!lit, "text past column {MENU_COLUMNS} at ({x}, {y})");
            }
        }
    }

    #[test]
    fn every_page_fits_the_grid_at_its_widest() {
        let view = widest();
        for page in SheetPage::ALL {
            let menu = SheetMenu {
                member: 5,
                page,
                message: String::new(),
            };
            let mut frame = Frame::default();
            sheet(&mut frame, &menu, &view, 6);
            assert_laid_out(&frame, MENU_BOX);
            assert_no_text_past_the_grid(&frame);
            assert_eq!(
                frame.widgets.len(),
                4,
                "{page:?}: three tabs and the member"
            );
            let member = frame.widget(WidgetId::Row(ROW_MEMBER)).unwrap();
            assert!(member.left.is_some() && member.right.is_some());
            let tab = frame.widget(WidgetId::Row(page.index())).unwrap();
            let h = hit(&frame.widgets, tab.rect.x, tab.rect.y).unwrap();
            assert_eq!((h.id, h.part), (WidgetId::Row(page.index()), Part::Body));
        }
        let mut frame = Frame::default();
        sheet(&mut frame, &SheetMenu::default(), &SheetView::default(), 1);
        assert_laid_out(&frame, MENU_BOX);
        let mut blank = SheetView::default();
        blank.gear.slots = vec![("Main hand".to_owned(), "-".to_owned())];
        for page in SheetPage::ALL {
            let menu = SheetMenu {
                page,
                ..SheetMenu::default()
            };
            let mut frame = Frame::default();
            sheet(&mut frame, &menu, &blank, 1);
            assert_laid_out(&frame, MENU_BOX);
        }
    }

    /// Whether any pixel of the cell at this column and row is painted.
    fn lit(frame: &Frame, column: i32, row: i32) -> bool {
        let (x, y) = menu_cell(column, row);
        (0..6)
            .any(|dx| (0..8).any(|dy| frame.raster.get(x + dx, y + dy).is_some_and(|p| p[3] != 0)))
    }

    #[test]
    fn columns_lay_entries_out_by_pitch_and_say_none_when_empty() {
        let mut frame = Frame::default();
        let entries: Vec<String> = (0..7).map(|i| format!("e{i}")).collect();
        columns(&mut frame, 2, 3, 26, 2, &entries, "none");
        assert!(
            lit(&frame, 1, 2) && lit(&frame, 27, 2) && lit(&frame, 53, 2),
            "three to a row"
        );
        assert!(lit(&frame, 1, 3) && lit(&frame, 53, 3), "the second row");
        assert!(!lit(&frame, 1, 4), "the seventh entry is past the rows");
        let mut frame = Frame::default();
        columns(&mut frame, 2, 3, 26, 2, &[], "none");
        assert!(lit(&frame, 1, 2), "says none");
    }
}
