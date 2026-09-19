//! The fight painted over the viewport (ARCHITECTURE.md §8.1): the stack rows above the 3D
//! view, the counts under the silhouettes' feet, the action row below the scene, and the
//! modal that follows a wipe. The rows between stay clear so the scene and the silhouettes
//! show through; the band shows the log. Bevy-free.

use crate::actors;
use crate::combat_menu::{ACTION_USE, CombatMenu, DefeatMenu, EncounterMenu, FightView};
use crate::combat_text::LONG_CELLS;
use crate::font::fit;
use crate::layout::{CELL, Rect, VIEWPORT, VIEWPORT_COLUMNS, VIEWPORT_ROWS, cell, row_y, rows};
use crate::raster::Rgb;
use crate::screens::{ItemState, MODAL_TEXT_X, item_state_at, modal};
use crate::widget::{DIM, Frame, HI, Kind, PANEL, TEXT, WidgetId};

/// The header's row: `COMBAT  Round n` or `ENCOUNTER` and the disposition.
const HEADER_ROW: i32 = 1;
/// The first stack row, a blank row under the header.
const FIRST_STACK_ROW: i32 = 3;
/// Stack rows on the screen.
const STACK_ROWS: i32 = 5;
/// The panel above the clear window: the header, the stack rows, a blank row.
pub const TOP_PANEL: Rect = rows(0, FIRST_STACK_ROW + STACK_ROWS + 1);
/// The row of the count digits: the first row at or under the front silhouettes' feet.
const COUNT_ROW: i32 = (actors::FRONT_FEET_Y + CELL.1 - 1) / CELL.1;
/// Rows of the panel below the clear window.
const BOTTOM_ROWS: i32 = 7;
/// The bottom panel's first row.
const BOTTOM_ROW: i32 = VIEWPORT_ROWS - BOTTOM_ROWS;
/// The panel below the clear window, to the viewport's bottom edge.
pub const BOTTOM_PANEL: Rect = Rect::new(
    VIEWPORT.x,
    VIEWPORT.y + row_y(BOTTOM_ROW),
    VIEWPORT.w,
    VIEWPORT.h - row_y(BOTTOM_ROW) as u32,
);
/// The action row, in the middle of the bottom panel.
const ACTION_ROW: i32 = BOTTOM_ROW + 3;
/// Cells of a stack row's text: `99 {name:<20} front`.
const STACK_TEXT_CELLS: usize = 29;
/// The column a stack's reason for being out of reach starts at, in a fight.
const REASON_COLUMN: i32 = 32;
/// Cells the reason may take.
const REASON_CELLS: usize = 24;
/// Cells of a stack row: the text, a gap, the reason.
const STACK_CELLS: usize = REASON_COLUMN as usize - 1 + REASON_CELLS;
/// The action row columns for the fight: Attack, Cast, Use, Dodge, Exchange, Run.
const COMBAT_ACTION_COLUMNS: [i32; 6] = [1, 9, 15, 20, 27, 37];
/// Cells a spell picker row takes: `{name:<18} {cost:>2} pt  {note:<16}`.
const SPELL_ROW_CELLS: usize = 44;
/// Spell rows the picker shows: the bottom panel's rows under its header.
pub const SPELL_ROWS: usize = (BOTTOM_ROWS - 1) as usize;
// Each action label ends before the next column begins.
const _: () = {
    let mut i = 0;
    while i + 1 < COMBAT_ACTION_COLUMNS.len() {
        assert!(
            COMBAT_ACTION_COLUMNS[i] + (CombatMenu::ACTIONS[i].len() as i32)
                < COMBAT_ACTION_COLUMNS[i + 1]
        );
        i += 1;
    }
};
const _: () = assert!((SPELL_ROW_CELLS as i32) < VIEWPORT_COLUMNS);
/// The action row columns before it: Attack, Bribe …, Hide, Run.
const ENCOUNTER_ACTION_COLUMNS: [i32; 4] = [1, 11, 26, 34];
/// Cells a modal line may take inside the defeat box: a whole log line.
const DEFEAT_LINE_CELLS: usize = LONG_CELLS;
/// The defeat box's height in rows: the title, the log lines, two buttons, and the gaps
/// (`screens::modal` places them).
const DEFEAT_ROWS: i32 = 12;
/// The defeat box's size: the lines plus two cells of margin each side.
const DEFEAT_SIZE: (u32, u32) = (
    (DEFEAT_LINE_CELLS as i32 + 4) as u32 * CELL.0 as u32,
    (DEFEAT_ROWS * CELL.1) as u32,
);
/// Where the defeat modal sits: centred in the viewport, its top on a text row.
pub const DEFEAT_RECT: Rect = Rect::new(
    VIEWPORT.x + (VIEWPORT.w - DEFEAT_SIZE.0) as i32 / 2,
    VIEWPORT.y + row_y((VIEWPORT.h - DEFEAT_SIZE.1) as i32 / 2 / CELL.1),
    DEFEAT_SIZE.0,
    DEFEAT_SIZE.1,
);
// The rows fit the viewport's grid, the panels and the box sit inside it, the counts sit
// between the feet and the bottom panel, and a defeat line fits its box with the modal's
// margins.
const _: () = assert!((STACK_CELLS as i32) < VIEWPORT_COLUMNS);
const _: () = assert!((1 + STACK_TEXT_CELLS as i32) < REASON_COLUMN);
const _: () = assert!(row_y(COUNT_ROW) >= actors::FRONT_FEET_Y);
const _: () = assert!(row_y(COUNT_ROW) + CELL.1 <= BOTTOM_PANEL.y);
const _: () = assert!(TOP_PANEL.bottom() <= row_y(COUNT_ROW));
const _: () = assert!(VIEWPORT.encloses(TOP_PANEL) && VIEWPORT.encloses(BOTTOM_PANEL));
const _: () = assert!(!TOP_PANEL.overlaps(BOTTOM_PANEL) && VIEWPORT.encloses(DEFEAT_RECT));
const _: () =
    assert!(CELL.0 as u32 * DEFEAT_LINE_CELLS as u32 + 2 * MODAL_TEXT_X as u32 <= DEFEAT_SIZE.0);

/// A row's rectangle on the viewport grid: `cells` wide from a cell.
fn vp_rect(column: i32, row: i32, cells: usize) -> Rect {
    let (x, y) = cell(column, row);
    Rect::new(x, y, cells as u32 * CELL.0 as u32, CELL.1 as u32)
}

/// Paint plain text at a cell of the viewport grid.
fn vp_label(frame: &mut Frame, column: i32, row: i32, text: &str, color: Rgb) {
    let (x, y) = cell(column, row);
    frame.raster.text(x, y, text, color);
}

/// Paint text right-aligned a column in from the viewport's edge.
fn vp_label_right(frame: &mut Frame, row: i32, text: &str, color: Rgb) {
    let column = VIEWPORT_COLUMNS - 1 - text.chars().count() as i32;
    vp_label(frame, column.max(0), row, text, color);
}

/// The fight: header, stack rows with the target marked, the six actions or the spell
/// picker in their place; the band shows the log.
pub fn combat(frame: &mut Frame, view: &FightView, menu: &CombatMenu) {
    panels(frame);
    vp_label(
        frame,
        1,
        HEADER_ROW,
        &format!("COMBAT  Round {}", view.round),
        HI,
    );
    stacks(frame, view, Some(menu.target));
    counts(frame, view);
    if let Some(cursor) = menu.picker {
        picker(frame, view, cursor);
        return;
    }
    for (i, (text, column)) in CombatMenu::ACTIONS
        .iter()
        .zip(COMBAT_ACTION_COLUMNS)
        .enumerate()
    {
        let state = if i == ACTION_USE {
            ItemState::Disabled
        } else {
            ItemState::from_selected(menu.cursor == i)
        };
        item_state_at(
            frame,
            WidgetId::Action(i),
            Kind::Button,
            vp_rect(column, ACTION_ROW, text.len()),
            text,
            state,
        );
    }
}

/// The spell picker in the bottom panel: a header with the caster's points, then one row per
/// spell with its cost and note; blocked rows are dim, a reaction row switches its auto-cast.
fn picker(frame: &mut Frame, view: &FightView, cursor: usize) {
    let (points, max) = view.points;
    vp_label(
        frame,
        1,
        BOTTOM_ROW,
        &format!("CAST   SP {points}/{max}   Esc back"),
        HI,
    );
    for (i, spell) in view.spells.iter().enumerate().take(SPELL_ROWS) {
        let text = format!(
            "{:<18} {:>2} pt  {:<16}",
            fit(&spell.name, 18),
            spell.cost.min(99),
            fit(&spell.note(), 16)
        );
        let state = if spell.blocked.is_some() && spell.auto.is_none() {
            ItemState::Disabled
        } else {
            ItemState::from_selected(cursor == i)
        };
        item_state_at(
            frame,
            WidgetId::Spell(usize::from(spell.index)),
            Kind::Button,
            vp_rect(1, BOTTOM_ROW + 1 + i as i32, SPELL_ROW_CELLS),
            &text,
            state,
        );
    }
}

/// The choice before a fight: header with the disposition, the stacks, the four choices.
pub fn encounter(frame: &mut Frame, view: &FightView, menu: &EncounterMenu) {
    panels(frame);
    vp_label(frame, 1, HEADER_ROW, "ENCOUNTER", HI);
    vp_label_right(frame, HEADER_ROW, &format!("{:?}", view.disposition), TEXT);
    stacks(frame, view, None);
    counts(frame, view);
    let bribe = view.bribe_label();
    let texts = [
        EncounterMenu::ACTIONS[0],
        bribe.as_str(),
        EncounterMenu::ACTIONS[2],
        EncounterMenu::ACTIONS[3],
    ];
    for (i, (text, column)) in texts.iter().zip(ENCOUNTER_ACTION_COLUMNS).enumerate() {
        let state = if i == 1 && !view.bribe_allowed() {
            ItemState::Disabled
        } else {
            ItemState::from_selected(menu.cursor == i)
        };
        item_state_at(
            frame,
            WidgetId::Action(i),
            Kind::Button,
            vp_rect(column, ACTION_ROW, text.chars().count()),
            text,
            state,
        );
    }
}

/// Lines of the log the defeat modal shows: how the end came.
pub const DEFEAT_LOG_ROWS: usize = 4;
/// "The party has fallen", boxed over the world, with the last of the roll log so a fight
/// that ended inside one command still reads, and the two ways out.
pub fn defeat(frame: &mut Frame, menu: &DefeatMenu, log: &[String]) {
    let tail = &log[log.len().saturating_sub(DEFEAT_LOG_ROWS)..];
    let lines: Vec<String> = if tail.is_empty() {
        vec!["Every member is down.".to_owned()]
    } else {
        tail.iter().map(|l| fit(l, DEFEAT_LINE_CELLS)).collect()
    };
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    modal(
        frame,
        DEFEAT_RECT,
        "The party has fallen",
        &lines,
        &DefeatMenu::ITEMS,
        menu.cursor,
    );
}

/// The two panel bands; the rows between stay clear for the scene.
fn panels(frame: &mut Frame) {
    frame.raster.fill(TOP_PANEL, PANEL);
    frame.raster.fill(BOTTOM_PANEL, PANEL);
}

/// One row per stack: the living count, the name, and front, back, or slain. In a fight the
/// rows are targets the mouse can pick, the current target is marked, a stack the acting
/// member cannot reach says why, and slain rows are inert. Before the fight they are plain
/// text.
fn stacks(frame: &mut Frame, view: &FightView, target: Option<u8>) {
    for (i, stack) in view.stacks.iter().enumerate().take(STACK_ROWS as usize) {
        let row = FIRST_STACK_ROW + i as i32;
        let text = format!(
            "{:>2} {:<20} {}",
            stack.count,
            fit(&stack.name, 20),
            stack.state()
        );
        let Some(target) = target else {
            vp_label(frame, 1, row, &text, if stack.alive { TEXT } else { DIM });
            continue;
        };
        if let Some(reason) = stack.blocked.as_deref().filter(|_| stack.alive) {
            vp_label(frame, REASON_COLUMN, row, &fit(reason, REASON_CELLS), DIM);
        }
        let state = if !stack.alive {
            ItemState::Disabled
        } else {
            ItemState::from_selected(stack.index == target)
        };
        item_state_at(
            frame,
            WidgetId::Stack(i),
            Kind::Choice,
            vp_rect(1, row, STACK_CELLS),
            &text,
            state,
        );
    }
}

/// The living count of each stack, centred under its silhouette.
fn counts(frame: &mut Frame, view: &FightView) {
    let y = crate::layout::cell(0, COUNT_ROW).1;
    for s in actors::silhouettes(&view.actors()) {
        let Some(stack) = view.stacks.iter().find(|row| row.index == s.index) else {
            continue;
        };
        let text = stack.count.to_string();
        let x = s.rect.x + s.rect.w as i32 / 2 - CELL.0 / 2 * text.len() as i32;
        frame.raster.text(x, y, &text, TEXT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_menu::StackRow;
    use crate::layout::cell;
    use crate::screens::tests::assert_laid_out;

    /// Inside the viewport grid: a full last column ends a pixel past the viewport.
    fn laid_out(frame: &Frame) {
        let area = Rect::new(VIEWPORT.x + 1, VIEWPORT.y, VIEWPORT.w, VIEWPORT.h);
        assert_laid_out(frame, area);
    }
    use crate::screens::{MODAL_BOTTOM_PAD, MODAL_BUTTON_PITCH, MODAL_LINES_Y, MODAL_TEXT_X};
    use crate::widget::hit;
    use omnis_sim::ModeKind;
    use omnis_sim::omnis_data::{Disposition, Size};

    /// Four stacks of the widest names and counts, the widest round.
    fn widest_view(phase: ModeKind) -> FightView {
        FightView {
            phase,
            round: 999,
            own: Some(0),
            disposition: Disposition::Friendly,
            stacks: (0..4)
                .map(|i| StackRow {
                    index: i,
                    name: format!("Ancient Red Dragon {i}x"),
                    count: 99,
                    initial: 99,
                    size: Size::Gargantuan,
                    front: i < 2,
                    alive: i != 3,
                    blocked: (i == 2).then(|| "behind".to_owned()),
                })
                .collect(),
            bribe: Some(9999),
            gold: 0,
            spells: (0..6)
                .map(|i| crate::combat_menu::SpellRow {
                    index: i,
                    name: format!("Spell With A Long Name {i}"),
                    cost: 9,
                    targets_members: i == 5,
                    auto: (i == 4).then_some(false),
                    active: i == 1,
                    blocked: (i == 2).then(|| "need 9 pt".to_owned()),
                })
                .collect(),
            points: (99, 99),
        }
    }

    #[test]
    fn the_picker_lists_the_spells_in_the_bottom_panel() {
        let mut frame = Frame::default();
        let view = widest_view(ModeKind::Combat);
        let menu = CombatMenu {
            picker: Some(1),
            target: 1,
            ..CombatMenu::default()
        };
        combat(&mut frame, &view, &menu);
        laid_out(&frame);
        assert!(
            frame.widget(WidgetId::Action(0)).is_none(),
            "the action row makes way"
        );
        assert_eq!(frame.widgets.len(), 4 + 6, "four stacks, six spells");
        for i in 0..6 {
            let w = frame.widget(WidgetId::Spell(i)).unwrap();
            assert!(BOTTOM_PANEL.encloses(w.rect), "{i}: {:?}", w.rect);
            assert_eq!(w.enabled, i != 2, "the blocked row is inert");
        }
        let chosen = frame.widget(WidgetId::Spell(1)).unwrap();
        assert_eq!(rgb(&frame, chosen.rect.x - 5, chosen.rect.y + 1), Some(HI));
        let (hx, hy) = cell(1, BOTTOM_ROW);
        assert!(
            (0..40).any(|dx| rgb(&frame, hx + dx, hy + 3) == Some(HI)),
            "the header"
        );
    }

    fn rgb(frame: &Frame, x: i32, y: i32) -> Option<(u8, u8, u8)> {
        frame.raster.get(x, y).map(|p| (p[0], p[1], p[2]))
    }

    #[test]
    fn the_fight_fits_at_its_widest_and_leaves_the_window_clear() {
        let mut frame = Frame::default();
        let view = widest_view(ModeKind::Combat);
        let menu = CombatMenu {
            cursor: 2,
            target: 1,
            ..CombatMenu::default()
        };
        combat(&mut frame, &view, &menu);
        laid_out(&frame);
        assert_eq!(frame.widgets.len(), 10, "four stacks, six actions");
        assert!(
            !frame.widget(WidgetId::Action(ACTION_USE)).unwrap().enabled,
            "nothing to use yet"
        );
        for i in 0..4 {
            let w = frame.widget(WidgetId::Stack(i)).unwrap();
            assert_eq!(w.rect.right(), cell(1 + STACK_CELLS as i32, 0).0);
            assert_eq!(w.enabled, i != 3, "the slain row is inert");
        }
        let run = frame.widget(WidgetId::Action(5)).unwrap();
        assert!(run.rect.right() <= VIEWPORT.right());
        let mid = VIEWPORT.w as i32 / 2;
        let (top, bottom) = (TOP_PANEL, BOTTOM_PANEL);
        // The reason a stack is out of reach stands after its row's text, dim.
        let (rx, ry) = cell(REASON_COLUMN, FIRST_STACK_ROW + 2);
        assert!(
            (0..30).any(|dx| rgb(&frame, rx + dx, ry + 3) == Some(DIM)),
            "behind"
        );
        let (rx, ry) = cell(REASON_COLUMN, FIRST_STACK_ROW);
        assert!((0..30).all(|dx| rgb(&frame, rx + dx, ry + 3) == Some(PANEL)));
        // The panels clear the tallest front silhouette's head and stand off its feet.
        let giant = actors::Actor {
            index: 0,
            size: Size::Gargantuan,
            front: true,
        };
        let head = actors::silhouettes(&[giant])[0].rect.y;
        assert!(top.bottom() <= head, "{} vs head {head}", top.bottom());
        assert!(cell(0, COUNT_ROW).1 >= actors::FRONT_FEET_Y);
        assert_eq!(rgb(&frame, top.right() - 2, top.y + 4), Some(PANEL), "top");
        assert_eq!(rgb(&frame, bottom.right() - 2, bottom.y + 4), Some(PANEL));
        let clear = Some([0, 0, 0, 0]);
        assert_eq!(frame.raster.get(mid, top.bottom() + 8), clear, "the window");
        assert_eq!(frame.raster.get(mid, top.bottom()), clear);
        assert_eq!(frame.raster.get(mid, bottom.y - 1), clear);
        assert_eq!(rgb(&frame, mid, bottom.y), Some(PANEL));
        let target = frame.widget(WidgetId::Stack(1)).unwrap();
        assert_eq!(
            rgb(&frame, target.rect.x - 5, target.rect.y + 1),
            Some(HI),
            "the marker sits before the target"
        );
        let action = frame.widget(WidgetId::Action(3)).unwrap();
        let h = hit(&frame.widgets, action.rect.x, action.rect.y).unwrap();
        assert_eq!((h.id, h.kind), (WidgetId::Action(3), Kind::Button));
        let inert = frame.widget(WidgetId::Action(ACTION_USE)).unwrap();
        assert!(hit(&frame.widgets, inert.rect.x, inert.rect.y).is_none());
        let (_, y) = cell(0, COUNT_ROW);
        let digits = (0..VIEWPORT.right())
            .filter(|x| rgb(&frame, *x, y + 1) == Some(TEXT))
            .count();
        assert!(digits > 0, "the counts sit under the silhouettes");
        assert_eq!(
            frame.raster.get(mid, y - 1),
            clear,
            "the row above is clear"
        );
        assert!(
            y + CELL.1 <= bottom.y,
            "the counts end above the bottom panel"
        );
    }

    #[test]
    fn the_encounter_fits_and_gates_the_bribe() {
        let mut frame = Frame::default();
        let mut view = widest_view(ModeKind::Encounter);
        let menu = EncounterMenu::default();
        encounter(&mut frame, &view, &menu);
        laid_out(&frame);
        assert_eq!(frame.widgets.len(), 4, "the stacks are text before a fight");
        let bribe = frame.widget(WidgetId::Action(1)).unwrap();
        assert!(!bribe.enabled, "9999 gold is more than the purse");
        assert_eq!(bribe.rect.w, CELL.0 as u32 * "Bribe 9999g".len() as u32);
        assert!(frame.widget(WidgetId::Action(0)).unwrap().enabled);
        view.bribe = Some(0);
        let mut frame = Frame::default();
        encounter(&mut frame, &view, &menu);
        let bribe = frame.widget(WidgetId::Action(1)).unwrap();
        assert!(bribe.enabled);
        assert_eq!(bribe.rect.w, CELL.0 as u32 * "Bribe free".len() as u32);
        let mid = VIEWPORT.w as i32 / 2;
        assert_eq!(
            frame.raster.get(mid, TOP_PANEL.bottom() + 8),
            Some([0, 0, 0, 0])
        );
    }

    #[test]
    fn the_defeat_modal_sits_in_the_window_and_tells_how_it_ended() {
        let mut frame = Frame::default();
        defeat(&mut frame, &DefeatMenu { cursor: 1 }, &[]);
        laid_out(&frame);
        let empty = frame.raster.fingerprint();
        let mut frame = Frame::default();
        let log: Vec<String> = (0..5)
            .map(|i| format!("Line {i} {}", "x".repeat(120)))
            .collect();
        defeat(&mut frame, &DefeatMenu { cursor: 1 }, &log);
        laid_out(&frame);
        assert_ne!(
            frame.raster.fingerprint(),
            empty,
            "the log's tail is painted"
        );
        // The lines fit between the title and the buttons: the longest ends inside the box.
        let right = DEFEAT_RECT.x + MODAL_TEXT_X + CELL.0 * DEFEAT_LINE_CELLS as i32;
        assert!(right < DEFEAT_RECT.right(), "{right}");
        let first_line = DEFEAT_RECT.y + MODAL_LINES_Y;
        assert_eq!(
            frame.raster.get(right + 2, first_line + 1),
            Some([PANEL.0, PANEL.1, PANEL.2, 255]),
            "nothing past the clipped line"
        );
        assert_eq!(frame.widgets.len(), 2);
        for w in &frame.widgets {
            assert!(DEFEAT_RECT.encloses(w.rect));
        }
        assert_eq!(
            frame.raster.get(DEFEAT_RECT.x - 10, DEFEAT_RECT.y - 10),
            Some([0, 0, 0, 0]),
            "the world shows"
        );
        // The log lines cross the box's middle; its margin past them stays panel.
        assert_eq!(
            rgb(&frame, DEFEAT_RECT.right() - 3, first_line + 4),
            Some(PANEL)
        );
        let mid = DEFEAT_RECT.x + DEFEAT_RECT.w as i32 / 2;
        assert_eq!(rgb(&frame, mid, first_line - 2), Some(PANEL));
        // The last line ends above the first button's row.
        let last_line_bottom = first_line + CELL.1 * DEFEAT_LOG_ROWS as i32;
        let first_button = DEFEAT_RECT.bottom()
            - MODAL_BUTTON_PITCH * DefeatMenu::ITEMS.len() as i32
            - MODAL_BOTTOM_PAD;
        assert!(
            last_line_bottom < first_button,
            "{last_line_bottom} vs {first_button}"
        );
        let load = frame.widget(WidgetId::Row(0)).unwrap();
        assert_eq!(load.rect.y, first_button);
        for y in last_line_bottom..first_button {
            assert!(
                (DEFEAT_RECT.x + 1..DEFEAT_RECT.right() - 1)
                    .all(|x| rgb(&frame, x, y) == Some(PANEL)),
                "a blank row {y} between the lines and the buttons"
            );
        }
    }
}
