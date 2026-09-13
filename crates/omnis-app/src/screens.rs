//! The menu screens painted over the viewport: title, new game, character creation, pause,
//! and the modal box the defeat screen uses. Each paints its text on the menu grid, a framed
//! box of cells centred in the viewport (`layout::MENU_BOX`), and registers the rows the mouse
//! can hit, keyed by the row indices the models in `menu.rs` already use. `combat_screen.rs`
//! paints the fight on the viewport's own grid with `item_state_at`.

use crate::layout::{CELL, MENU_COLUMNS, Rect, menu_cell};
use crate::menu::{
    Catalog, CreationForm, NewGameForm, Pause, ROW_ADD, ROW_ALIGNMENT, ROW_BACKGROUND, ROW_BEGIN,
    ROW_CLASS, ROW_NAME, ROW_RACE, ROW_SCORES, ROW_SKILLS, Title, rule_label, words,
};
use crate::raster::Rgb;
use crate::widget::{DIM, FRAME, Frame, HI, Kind, PANEL, TEXT, Widget, WidgetId};
use omnis_sim::Settings;
use omnis_sim::omnis_data::{Ability, Alignment};

/// A row's rectangle: `cells` wide from a cell of the menu grid.
pub(crate) fn row_rect(column: i32, row: i32, cells: usize) -> Rect {
    let (x, y) = menu_cell(column, row);
    Rect::new(x, y, cells as u32 * CELL.0 as u32, CELL.1 as u32)
}

/// Paint plain text at a cell of the menu grid.
pub(crate) fn label(frame: &mut Frame, column: i32, row: i32, text: &str, color: Rgb) {
    let (x, y) = menu_cell(column, row);
    frame.raster.text(x, y, text, color);
}

/// Paint text right-aligned to the last menu column.
pub(crate) fn label_right(frame: &mut Frame, row: i32, text: &str, color: Rgb) {
    let column = MENU_COLUMNS - text.chars().count() as i32;
    label(frame, column.max(0), row, text, color);
}

/// The cells of the first `<` and the last `>` in a row's text, as arrow rectangles.
fn arrows(text: &str, rect: Rect) -> (Option<Rect>, Option<Rect>) {
    let at = |i: usize| {
        Rect::new(
            rect.x + i as i32 * CELL.0,
            rect.y,
            CELL.0 as u32,
            CELL.1 as u32,
        )
    };
    (text.find('<').map(at), text.rfind('>').map(at))
}

/// How a row is drawn and whether it answers the mouse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemState {
    /// Plain text, live.
    Normal,
    /// Highlighted with the marker, live.
    Selected,
    /// Dim, inert.
    Disabled,
}

impl ItemState {
    /// Selected or not.
    pub(crate) const fn from_selected(selected: bool) -> ItemState {
        if selected {
            ItemState::Selected
        } else {
            ItemState::Normal
        }
    }
}

/// Paint a row the mouse can hit: highlighted with the marker when selected.
pub(crate) fn item(
    frame: &mut Frame,
    id: WidgetId,
    kind: Kind,
    at: (i32, i32),
    text: &str,
    cells: usize,
    selected: bool,
) {
    item_state(
        frame,
        id,
        kind,
        at,
        text,
        cells,
        ItemState::from_selected(selected),
    );
}

/// Paint a row of the menu grid in a state.
pub(crate) fn item_state(
    frame: &mut Frame,
    id: WidgetId,
    kind: Kind,
    at: (i32, i32),
    text: &str,
    cells: usize,
    state: ItemState,
) {
    let (column, row) = at;
    item_state_at(frame, id, kind, row_rect(column, row, cells), text, state);
}

/// Paint a row in a state at a rectangle: a disabled row is dim and registers an inert
/// widget, so the hover outline and the click both pass it by.
pub(crate) fn item_state_at(
    frame: &mut Frame,
    id: WidgetId,
    kind: Kind,
    rect: Rect,
    text: &str,
    state: ItemState,
) {
    let color = match state {
        ItemState::Normal => TEXT,
        ItemState::Selected => HI,
        ItemState::Disabled => DIM,
    };
    frame.raster.text(rect.x, rect.y, text, color);
    if state == ItemState::Selected {
        frame.raster.marker(rect.x - 5, rect.y + 1, HI);
    }
    let mut widget = Widget::new(id, rect, kind);
    widget.enabled = state != ItemState::Disabled;
    if kind == Kind::Choice {
        let (left, right) = arrows(text, rect);
        widget.left = left;
        widget.right = right;
    }
    frame.push(widget);
}

/// A modal's text starts two cells in from its left edge.
pub(crate) const MODAL_TEXT_X: i32 = 2 * CELL.0;
/// The title's top, from the box's top.
pub(crate) const MODAL_TITLE_Y: i32 = 6;
/// The first line's top, from the box's top.
pub(crate) const MODAL_LINES_Y: i32 = 18;
/// Buttons are a row plus two pixels apart.
pub(crate) const MODAL_BUTTON_PITCH: i32 = CELL.1 + 2;
/// The gap under the last button.
pub(crate) const MODAL_BOTTOM_PAD: i32 = 4;

/// A boxed message over whatever is behind it: a title, some lines, and buttons as
/// `WidgetId::Row(i)`, one per line from the bottom of the box up.
pub(crate) fn modal(
    frame: &mut Frame,
    rect: Rect,
    title: &str,
    lines: &[&str],
    buttons: &[&str],
    cursor: usize,
) {
    frame.raster.fill(rect, PANEL);
    frame.raster.stroke(rect, FRAME);
    let x = rect.x + MODAL_TEXT_X;
    frame.raster.text(x, rect.y + MODAL_TITLE_Y, title, HI);
    for (i, line) in lines.iter().enumerate() {
        let y = rect.y + MODAL_LINES_Y + CELL.1 * i as i32;
        frame.raster.text(x, y, line, TEXT);
    }
    let first = rect.bottom() - MODAL_BUTTON_PITCH * buttons.len() as i32 - MODAL_BOTTOM_PAD;
    for (i, text) in buttons.iter().enumerate() {
        let y = first + MODAL_BUTTON_PITCH * i as i32;
        let cells = text.chars().count();
        let rect = Rect::new(x, y, cells as u32 * CELL.0 as u32, CELL.1 as u32);
        let selected = cursor == i;
        frame
            .raster
            .text(rect.x, rect.y, text, if selected { HI } else { TEXT });
        if selected {
            frame.raster.marker(rect.x - 5, rect.y + 1, HI);
        }
        frame.push(Widget::new(WidgetId::Row(i), rect, Kind::Button));
    }
}

/// The title: three items.
pub fn title(frame: &mut Frame, title: &Title) {
    let column = (MENU_COLUMNS - "OMNIS".len() as i32) / 2;
    label(frame, column, 2, "OMNIS", HI);
    for (i, text) in Title::ITEMS.iter().enumerate() {
        let row = 6 + 2 * i as i32;
        item(
            frame,
            WidgetId::Row(i),
            Kind::Button,
            (column - 4, row),
            text,
            15,
            title.cursor == i,
        );
    }
}

/// The new game form: seed, save rule, permadeath, start, back.
pub fn new_game(frame: &mut Frame, form: &NewGameForm) {
    label(frame, 1, 1, "NEW GAME", HI);
    let caret = if form.cursor == 0 { "_" } else { "" };
    item(
        frame,
        WidgetId::Row(0),
        Kind::TextField,
        (1, 3),
        &format!("Seed {}{caret}", form.seed_text),
        39,
        form.cursor == 0,
    );
    label(frame, 6, 4, "blank = random, words are hashed", DIM);
    item(
        frame,
        WidgetId::Row(1),
        Kind::Choice,
        (1, 6),
        &format!("Saving     < {} >", rule_label(form.settings.save_rule)),
        39,
        form.cursor == 1,
    );
    let permadeath = if form.settings.permadeath {
        "On"
    } else {
        "Off"
    };
    item(
        frame,
        WidgetId::Row(2),
        Kind::Choice,
        (1, 7),
        &format!("Permadeath < {permadeath} >"),
        39,
        form.cursor == 2,
    );
    item(
        frame,
        WidgetId::Row(3),
        Kind::Button,
        (1, 9),
        "Start",
        5,
        form.cursor == 3,
    );
    item(
        frame,
        WidgetId::Row(4),
        Kind::Button,
        (1, 10),
        "Back",
        4,
        form.cursor == 4,
    );
}

/// The creation form.
pub fn creation(frame: &mut Frame, form: &CreationForm, catalog: &Catalog, members: usize) {
    label(frame, 1, 0, "CREATE YOUR PARTY", HI);
    label_right(
        frame,
        0,
        &format!("{members} of {} members", catalog.slots),
        TEXT,
    );
    creation_identity(frame, form, catalog);
    creation_scores(frame, form, catalog);
    creation_skills(frame, form, catalog);
    item(
        frame,
        WidgetId::Row(ROW_ADD),
        Kind::Button,
        (1, 15),
        "Add member",
        10,
        form.cursor == ROW_ADD,
    );
    item(
        frame,
        WidgetId::Row(ROW_BEGIN),
        Kind::Button,
        (21, 15),
        "Begin",
        5,
        form.cursor == ROW_BEGIN,
    );
}

fn creation_identity(frame: &mut Frame, form: &CreationForm, catalog: &Catalog) {
    let caret = if form.cursor == ROW_NAME { "_" } else { "" };
    item(
        frame,
        WidgetId::Row(ROW_NAME),
        Kind::TextField,
        (1, 1),
        &format!("{:<12}{}{caret}", "Name", form.name),
        39,
        form.cursor == ROW_NAME,
    );
    fn pick(list: &[String], i: usize) -> &str {
        list.get(i).map_or("?", String::as_str)
    }
    let alignment = Alignment::ALL[form.alignment % Alignment::ALL.len()];
    let alignment = words(&format!("{alignment:?}"));
    let rows = [
        (
            ROW_RACE,
            "Race",
            catalog.label(pick(&catalog.races, form.race)),
        ),
        (
            ROW_CLASS,
            "Class",
            catalog.label(pick(&catalog.classes, form.class)),
        ),
        (
            ROW_BACKGROUND,
            "Background",
            catalog.label(pick(&catalog.backgrounds, form.background)),
        ),
        (ROW_ALIGNMENT, "Alignment", alignment.as_str()),
    ];
    for (row, name, value) in rows {
        item(
            frame,
            WidgetId::Row(row),
            Kind::Choice,
            (1, 1 + row as i32),
            &format!("{name:<12}< {value} >"),
            39,
            form.cursor == row,
        );
    }
}

fn creation_scores(frame: &mut Frame, form: &CreationForm, catalog: &Catalog) {
    for (k, ability) in Ability::ALL.iter().enumerate() {
        let score = form.scores[k];
        let cost = catalog
            .costs
            .get(usize::from(score.saturating_sub(catalog.min)))
            .copied()
            .unwrap_or(0);
        let row = ROW_SCORES + k;
        item(
            frame,
            WidgetId::Row(row),
            Kind::Choice,
            (1 + 13 * (k as i32 % 3), 6 + k as i32 / 3),
            &format!("{} < {score:>2} > {cost}", ability.short()),
            12,
            form.cursor == row,
        );
    }
    label(
        frame,
        1,
        8,
        &format!(
            "Points left {} of {}",
            catalog.budget - form.spent(catalog),
            catalog.budget
        ),
        TEXT,
    );
}

fn creation_skills(frame: &mut Frame, form: &CreationForm, catalog: &Catalog) {
    let (choose, list) = form.skill_list(catalog);
    label_right(
        frame,
        8,
        &format!("Skills {} of {choose}", form.skills.len()),
        TEXT,
    );
    for (i, skill) in list.iter().enumerate() {
        let picked = if form.skills.contains(skill) {
            'x'
        } else {
            ' '
        };
        item(
            frame,
            WidgetId::Skill(i),
            Kind::Toggle,
            (1 + 20 * (i as i32 % 2), 9 + i as i32 / 2),
            &format!("[{picked}] {}", words(&format!("{skill:?}"))),
            19,
            form.cursor == ROW_SKILLS && form.skill_cursor == i,
        );
    }
}

/// The pause overlay: the settings, read-only, and three items.
pub fn pause(frame: &mut Frame, pause: &Pause, settings: Settings, seed: u64) {
    label(frame, 1, 1, "PAUSED", HI);
    label(frame, 1, 3, &format!("Seed {seed}"), TEXT);
    label(
        frame,
        1,
        4,
        &format!("{:<12}{}", "Saving", rule_label(settings.save_rule)),
        TEXT,
    );
    let permadeath = if settings.permadeath { "on" } else { "off" };
    label(
        frame,
        1,
        5,
        &format!("{:<12}{permadeath}", "Permadeath"),
        TEXT,
    );
    for (i, text) in Pause::ITEMS.iter().enumerate() {
        item(
            frame,
            WidgetId::Row(i),
            Kind::Button,
            (1, 7 + 2 * i as i32),
            text,
            13,
            pause.cursor == i,
        );
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::layout::{MENU_BOX, VIEWPORT};
    use crate::menu::MenuKey;
    use crate::screen::{Target, click};
    use crate::widget::{Hit, Part, hit};
    use omnis_sim::SaveRule;
    use omnis_sim::omnis_data::load_packs;
    use std::path::PathBuf;

    fn catalog() -> Catalog {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = load_packs(&[&repo.join("packs/base")]).unwrap_or_else(|r| panic!("{r}"));
        Catalog::from_data(&data)
    }

    fn widest_creation(catalog: &Catalog) -> CreationForm {
        let mut form = CreationForm::new(catalog);
        form.name = "Bartholomew Longname Jr".into();
        form.race = catalog
            .races
            .iter()
            .position(|r| r == "base:race:halfling")
            .unwrap();
        form.class = catalog
            .classes
            .iter()
            .position(|c| c == "base:class:rogue")
            .unwrap();
        form.alignment = Alignment::ALL
            .iter()
            .position(|a| *a == Alignment::ChaoticNeutral)
            .unwrap();
        form.scores = [15; 6];
        form
    }

    /// Every widget lies inside the menu area and none overlap.
    /// Every widget lies inside `area` and none overlap.
    pub(crate) fn assert_laid_out(frame: &Frame, area: Rect) {
        for (i, a) in frame.widgets.iter().enumerate() {
            assert!(area.encloses(a.rect), "{:?} leaves the menu area", a.id);
            for b in &frame.widgets[i + 1..] {
                assert!(!a.rect.overlaps(b.rect), "{:?} overlaps {:?}", a.id, b.id);
            }
        }
    }

    #[test]
    fn every_screen_fits_the_menu_area_at_its_widest() {
        let mut frame = Frame::default();
        title(&mut frame, &Title::default());
        assert_laid_out(&frame, MENU_BOX);
        assert_eq!(frame.widgets.len(), 3);
        let mut frame = Frame::default();
        let form = NewGameForm {
            seed_text: "a".repeat(32),
            settings: Settings {
                save_rule: SaveRule::Relief,
                ..Settings::default()
            },
            ..NewGameForm::default()
        };
        new_game(&mut frame, &form);
        assert_laid_out(&frame, MENU_BOX);
        assert!(
            frame
                .widget(WidgetId::Row(1))
                .unwrap()
                .right
                .unwrap()
                .right()
                <= menu_cell(MENU_COLUMNS, 0).0
        );
        let mut frame = Frame::default();
        let catalog = catalog();
        creation(&mut frame, &widest_creation(&catalog), &catalog, 6);
        assert_laid_out(&frame, MENU_BOX);
        assert!(
            frame.widget(WidgetId::Skill(10)).is_some(),
            "the rogue's 11 skills"
        );
        for row in (0..14).filter(|r| *r != ROW_SKILLS) {
            assert!(frame.widget(WidgetId::Row(row)).is_some(), "row {row}");
        }
        let mut frame = Frame::default();
        pause(&mut frame, &Pause::default(), Settings::default(), u64::MAX);
        assert_laid_out(&frame, MENU_BOX);
        assert_eq!(frame.widgets.len(), 3);
    }

    #[test]
    fn hits_land_on_rows_arrows_and_skills() {
        let catalog = catalog();
        let mut frame = Frame::default();
        creation(&mut frame, &CreationForm::new(&catalog), &catalog, 0);
        let race = frame.widget(WidgetId::Row(ROW_RACE)).unwrap();
        let left = race.left.unwrap();
        assert_eq!(
            left.x,
            menu_cell(13, 2).0,
            "the < sits after the 12-cell label"
        );
        let h = hit(&frame.widgets, left.x + 2, left.y + 3).unwrap();
        assert_eq!((h.id, h.part), (WidgetId::Row(ROW_RACE), Part::Left));
        let right = race.right.unwrap();
        let h = hit(&frame.widgets, right.x, right.y).unwrap();
        assert_eq!((h.id, h.part), (WidgetId::Row(ROW_RACE), Part::Right));
        let mid = race.rect.x + race.rect.w as i32 / 2;
        let h = hit(&frame.widgets, mid, race.rect.y).unwrap();
        assert_eq!((h.id, h.part), (WidgetId::Row(ROW_RACE), Part::Body));
        let str_row = frame.widget(WidgetId::Row(ROW_SCORES)).unwrap();
        assert_eq!(str_row.left.unwrap().x, menu_cell(5, 6).0);
        let dex_row = frame.widget(WidgetId::Row(ROW_SCORES + 1)).unwrap();
        assert_eq!(dex_row.rect.x, menu_cell(14, 6).0);
        let skill = frame.widget(WidgetId::Skill(1)).unwrap();
        assert_eq!(skill.rect.x, menu_cell(21, 9).0);
        assert_eq!(
            hit(&frame.widgets, skill.rect.x, skill.rect.y)
                .unwrap()
                .kind,
            Kind::Toggle
        );
        assert_eq!(hit(&frame.widgets, 0, 0), None);
        assert_eq!(hit(&frame.widgets, VIEWPORT.right() + 60, 100), None);
    }

    fn press(form: &mut CreationForm, catalog: &Catalog, keys: Vec<MenuKey>) {
        for key in keys {
            form.key(key, catalog, 0);
        }
    }

    #[test]
    fn clicks_reproduce_the_key_paths() {
        let catalog = catalog();
        let mut by_keys = CreationForm::new(&catalog);
        by_keys.key(MenuKey::Down, &catalog, 0);
        by_keys.key(MenuKey::Right, &catalog, 0);
        let mut by_mouse = CreationForm::new(&catalog);
        let hit = Hit {
            id: WidgetId::Row(ROW_RACE),
            part: Part::Right,
            kind: Kind::Choice,
        };
        let keys = click(Target::Creation(&mut by_mouse), hit);
        press(&mut by_mouse, &catalog, keys);
        assert_eq!(by_mouse, by_keys);
        let toggle = Hit {
            id: WidgetId::Skill(2),
            part: Part::Body,
            kind: Kind::Toggle,
        };
        let keys = click(Target::Creation(&mut by_mouse), toggle);
        press(&mut by_mouse, &catalog, keys);
        assert_eq!(by_mouse.skills.len(), 1);
        let keys = click(Target::Creation(&mut by_mouse), toggle);
        press(&mut by_mouse, &catalog, keys);
        assert!(by_mouse.skills.is_empty());
        let begin = Hit {
            id: WidgetId::Row(ROW_BEGIN),
            part: Part::Body,
            kind: Kind::Button,
        };
        let keys = click(Target::Creation(&mut by_mouse), begin);
        press(&mut by_mouse, &catalog, keys);
        assert_eq!(by_mouse.message, "Add at least one member");
    }

    #[test]
    fn a_disabled_item_is_dim_and_inert_and_a_modal_boxes_its_buttons() {
        let mut frame = Frame::default();
        item_state(
            &mut frame,
            WidgetId::Row(0),
            Kind::Button,
            (1, 1),
            "Off",
            3,
            ItemState::Disabled,
        );
        let w = frame.widget(WidgetId::Row(0)).unwrap();
        assert!(!w.enabled);
        assert_eq!(hit(&frame.widgets, w.rect.x, w.rect.y), None);
        let (x, y) = menu_cell(1, 1);
        assert_eq!(frame.raster.get(x, y + 1), Some([DIM.0, DIM.1, DIM.2, 255]));
        let mut frame = Frame::default();
        let rect = Rect::new(48, 32, 144, 64);
        let inside = (rect.x + 2, rect.y + rect.h as i32 / 2);
        let outside = (rect.x - 10, rect.y - 10);
        modal(
            &mut frame,
            rect,
            "The party has fallen",
            &["Every member is down."],
            &["Load last save", "Quit to title"],
            1,
        );
        assert_laid_out(&frame, rect);
        assert_eq!(frame.widgets.len(), 2);
        for w in &frame.widgets {
            assert!(rect.encloses(w.rect), "{:?}", w.id);
        }
        assert_eq!(
            frame.raster.get(rect.x, rect.y),
            Some([FRAME.0, FRAME.1, FRAME.2, 255])
        );
        assert_eq!(
            frame.raster.get(inside.0, inside.1),
            Some([PANEL.0, PANEL.1, PANEL.2, 255]),
            "the margin inside the frame is panel"
        );
        assert_eq!(
            frame.raster.get(outside.0, outside.1),
            Some([0, 0, 0, 0]),
            "outside is untouched"
        );
        let quit = frame.widget(WidgetId::Row(1)).unwrap();
        assert_eq!(
            frame.raster.get(quit.rect.x - 5, quit.rect.y + 1),
            Some([HI.0, HI.1, HI.2, 255]),
            "the marker sits before the cursor's button"
        );
    }

    #[test]
    fn the_title_frame_is_stable() {
        let mut frame = Frame::default();
        title(&mut frame, &Title::default());
        let painted = frame.raster.rgba.chunks(4).filter(|p| p[3] != 0).count();
        assert!(painted > 100, "the title paints glyphs and a marker");
        assert_eq!(
            frame.raster.fingerprint(),
            frame.raster.clone().fingerprint()
        );
    }
}
