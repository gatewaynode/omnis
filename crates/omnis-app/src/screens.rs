//! The menu screens painted over the viewport: title, new game, character creation, pause.
//! Each paints its text on the 40×16 cell grid of the viewport and registers the rows the
//! mouse can hit, keyed by the row indices the models in `menu.rs` already use.

use crate::layout::{CELL, MENU_COLUMNS, Rect, cell};
use crate::menu::{
    Catalog, CreationForm, NewGameForm, Pause, ROW_ADD, ROW_ALIGNMENT, ROW_BACKGROUND, ROW_BEGIN,
    ROW_CLASS, ROW_NAME, ROW_RACE, ROW_SCORES, ROW_SKILLS, Title, rule_label, words,
};
use crate::raster::Rgb;
use crate::widget::{DIM, Frame, HI, Kind, TEXT, Widget, WidgetId};
use omnis_sim::Settings;
use omnis_sim::omnis_data::{Ability, Alignment};

/// A row's rectangle: `cells` wide from a grid cell.
fn row_rect(column: i32, row: i32, cells: usize) -> Rect {
    let (x, y) = cell(column, row);
    Rect::new(x, y, cells as u32 * CELL.0 as u32, CELL.1 as u32)
}

/// Paint plain text at a grid cell.
fn label(frame: &mut Frame, column: i32, row: i32, text: &str, color: Rgb) {
    let (x, y) = cell(column, row);
    frame.raster.text(x, y, text, color);
}

/// Paint text right-aligned to the last menu column.
fn label_right(frame: &mut Frame, row: i32, text: &str, color: Rgb) {
    let column = MENU_COLUMNS - text.chars().count() as i32;
    label(frame, column.max(0), row, text, color);
}

/// The cells of the first `<` and the last `>` in the text, as arrow rectangles.
fn arrows(text: &str, column: i32, row: i32) -> (Option<Rect>, Option<Rect>) {
    let left = text.find('<').map(|i| row_rect(column + i as i32, row, 1));
    let right = text.rfind('>').map(|i| row_rect(column + i as i32, row, 1));
    (left, right)
}

/// Paint a row the mouse can hit: highlighted with the marker when selected.
fn item(
    frame: &mut Frame,
    id: WidgetId,
    kind: Kind,
    at: (i32, i32),
    text: &str,
    cells: usize,
    selected: bool,
) {
    let (column, row) = at;
    let rect = row_rect(column, row, cells);
    let color = if selected { HI } else { TEXT };
    frame.raster.text(rect.x, rect.y, text, color);
    if selected {
        frame.raster.marker(rect.x - 5, rect.y + 1, HI);
    }
    let mut widget = Widget::new(id, rect, kind);
    if kind == Kind::Choice {
        let (left, right) = arrows(text, column, row);
        widget.left = left;
        widget.right = right;
    }
    frame.widgets.push(widget);
}

/// The title: three items.
pub fn title(frame: &mut Frame, title: &Title) {
    label(frame, 17, 2, "OMNIS", HI);
    for (i, text) in Title::ITEMS.iter().enumerate() {
        let row = 6 + 2 * i as i32;
        item(
            frame,
            WidgetId::Row(i),
            Kind::Button,
            (13, row),
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
mod tests {
    use super::*;
    use crate::layout::VIEWPORT;
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

    fn assert_laid_out(frame: &Frame) {
        // Column 39's blank sixth pixel is x 240, one past the viewport.
        let area = Rect::new(1, 0, VIEWPORT.w, VIEWPORT.h);
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
        assert_laid_out(&frame);
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
        assert_laid_out(&frame);
        assert!(
            frame
                .widget(WidgetId::Row(1))
                .unwrap()
                .right
                .unwrap()
                .right()
                <= 241
        );
        let mut frame = Frame::default();
        let catalog = catalog();
        creation(&mut frame, &widest_creation(&catalog), &catalog, 6);
        assert_laid_out(&frame);
        assert!(
            frame.widget(WidgetId::Skill(10)).is_some(),
            "the rogue's 11 skills"
        );
        for row in (0..14).filter(|r| *r != ROW_SKILLS) {
            assert!(frame.widget(WidgetId::Row(row)).is_some(), "row {row}");
        }
        let mut frame = Frame::default();
        pause(&mut frame, &Pause::default(), Settings::default(), u64::MAX);
        assert_laid_out(&frame);
        assert_eq!(frame.widgets.len(), 3);
    }

    #[test]
    fn hits_land_on_rows_arrows_and_skills() {
        let catalog = catalog();
        let mut frame = Frame::default();
        creation(&mut frame, &CreationForm::new(&catalog), &catalog, 0);
        let race = frame.widget(WidgetId::Row(ROW_RACE)).unwrap();
        let left = race.left.unwrap();
        assert_eq!(left.x, cell(13, 2).0, "the < sits after the 12-cell label");
        let h = hit(&frame.widgets, left.x + 2, left.y + 3).unwrap();
        assert_eq!((h.id, h.part), (WidgetId::Row(ROW_RACE), Part::Left));
        let right = race.right.unwrap();
        let h = hit(&frame.widgets, right.x, right.y).unwrap();
        assert_eq!((h.id, h.part), (WidgetId::Row(ROW_RACE), Part::Right));
        let h = hit(&frame.widgets, race.rect.x + 100, race.rect.y).unwrap();
        assert_eq!((h.id, h.part), (WidgetId::Row(ROW_RACE), Part::Body));
        let str_row = frame.widget(WidgetId::Row(ROW_SCORES)).unwrap();
        assert_eq!(str_row.left.unwrap().x, cell(5, 6).0);
        let dex_row = frame.widget(WidgetId::Row(ROW_SCORES + 1)).unwrap();
        assert_eq!(dex_row.rect.x, cell(14, 6).0);
        let skill = frame.widget(WidgetId::Skill(1)).unwrap();
        assert_eq!(skill.rect.x, cell(21, 9).0);
        assert_eq!(
            hit(&frame.widgets, skill.rect.x, skill.rect.y)
                .unwrap()
                .kind,
            Kind::Toggle
        );
        assert_eq!(hit(&frame.widgets, 0, 0), None);
        assert_eq!(hit(&frame.widgets, 300, 100), None);
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
