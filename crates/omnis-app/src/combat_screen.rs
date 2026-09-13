//! The fight painted over the viewport (ARCHITECTURE.md §8.1): the stack rows above the 3D
//! view, the action row and the roll log below it, and the modal that follows a wipe. The
//! middle rows stay clear so the scene and the silhouettes show through. Bevy-free.

use crate::actors;
use crate::combat_menu::{CombatMenu, DefeatMenu, EncounterMenu, FightView};
use crate::combat_text::SHORT_CELLS;
use crate::font::fit;
use crate::layout::{CELL, MENU_COLUMNS, Rect, VIEWPORT, row_y, rows};
use crate::screens::{ItemState, MODAL_TEXT_X, item_state, label, label_right, modal};
use crate::widget::{DIM, Frame, HI, Kind, PANEL, TEXT, WidgetId};

/// The first stack row.
const FIRST_STACK_ROW: i32 = 1;
/// Stack rows on the screen.
const STACK_ROWS: i32 = 4;
/// The panel above the clear window: the header and the stack rows.
pub const TOP_PANEL: Rect = rows(0, FIRST_STACK_ROW + STACK_ROWS);
/// The row of the count digits: the last row that ends above the front silhouettes' feet.
const COUNT_ROW: i32 = (actors::FRONT_FEET_Y - 1) / CELL.1;
/// The action row, under the counts.
const ACTION_ROW: i32 = COUNT_ROW + 1;
/// The first roll-log row; the log runs to the last row of the viewport.
const FIRST_LOG_ROW: i32 = ACTION_ROW + 1;
/// The panel below the clear window: from the action row to the viewport's bottom edge.
pub const BOTTOM_PANEL: Rect = Rect::new(
    VIEWPORT.x,
    VIEWPORT.y + row_y(ACTION_ROW),
    VIEWPORT.w,
    VIEWPORT.h - row_y(ACTION_ROW) as u32,
);
/// Rows of the roll log tail.
pub const LOG_ROWS: usize = 4;
/// Cells of a stack row: `99 {name:<20} front`.
const STACK_CELLS: usize = 29;
/// The action row columns for the fight: Attack, Dodge, Exchange, Run.
const COMBAT_ACTION_COLUMNS: [i32; 4] = [1, 9, 16, 26];
/// The action row columns before it: Attack, Bribe …, Hide, Run.
const ENCOUNTER_ACTION_COLUMNS: [i32; 4] = [1, 9, 22, 28];
/// Cells a modal line may take inside the defeat box.
const DEFEAT_LINE_CELLS: usize = 20;
/// The defeat box's height in rows: the title, three log lines, two buttons, and the gaps
/// (`screens::modal` places them).
const DEFEAT_ROWS: i32 = 10;
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
// The rows fit the viewport's grid, the panels and the box sit inside it, and a defeat line
// fits its box with the modal's margins.
const _: () = assert!((STACK_CELLS as i32) < MENU_COLUMNS);
const _: () = assert!(VIEWPORT.encloses(TOP_PANEL) && VIEWPORT.encloses(BOTTOM_PANEL));
const _: () = assert!(!TOP_PANEL.overlaps(BOTTOM_PANEL) && VIEWPORT.encloses(DEFEAT_RECT));
const _: () =
    assert!(CELL.0 as u32 * DEFEAT_LINE_CELLS as u32 + 2 * MODAL_TEXT_X as u32 <= DEFEAT_SIZE.0);

/// The fight: header, stack rows with the target marked, the four actions, the log tail.
pub fn combat(frame: &mut Frame, view: &FightView, menu: &CombatMenu, log: &[String]) {
    panels(frame);
    label(frame, 1, 0, &format!("COMBAT  Round {}", view.round), HI);
    stacks(frame, view, Some(menu.target));
    counts(frame, view);
    for (i, (text, column)) in CombatMenu::ACTIONS
        .iter()
        .zip(COMBAT_ACTION_COLUMNS)
        .enumerate()
    {
        item_state(
            frame,
            WidgetId::Action(i),
            Kind::Button,
            (column, ACTION_ROW),
            text,
            text.len(),
            ItemState::from_selected(menu.cursor == i),
        );
    }
    roll_log(frame, log);
}

/// The choice before a fight: header with the disposition, the stacks, the four choices.
pub fn encounter(frame: &mut Frame, view: &FightView, menu: &EncounterMenu) {
    panels(frame);
    label(frame, 1, 0, "ENCOUNTER", HI);
    label_right(frame, 0, &format!("{:?}", view.disposition), TEXT);
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
        item_state(
            frame,
            WidgetId::Action(i),
            Kind::Button,
            (column, ACTION_ROW),
            text,
            text.chars().count(),
            state,
        );
    }
}

/// Lines of the roll log the defeat modal shows: how the end came.
pub const DEFEAT_LOG_ROWS: usize = 3;
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
/// rows are targets the mouse can pick and the current target is marked; slain rows are
/// inert. Before the fight they are plain text.
fn stacks(frame: &mut Frame, view: &FightView, target: Option<u8>) {
    for (i, stack) in view.stacks.iter().enumerate() {
        let row = FIRST_STACK_ROW + i as i32;
        let text = format!(
            "{:>2} {:<20} {}",
            stack.count,
            fit(&stack.name, 20),
            stack.state()
        );
        let Some(target) = target else {
            label(frame, 1, row, &text, if stack.alive { TEXT } else { DIM });
            continue;
        };
        let state = if !stack.alive {
            ItemState::Disabled
        } else {
            ItemState::from_selected(stack.index == target)
        };
        item_state(
            frame,
            WidgetId::Stack(i),
            Kind::Choice,
            (1, row),
            &text,
            STACK_CELLS,
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

/// The last lines of the log, newest at the bottom in full colour, the older ones dim.
fn roll_log(frame: &mut Frame, log: &[String]) {
    let tail = &log[log.len().saturating_sub(LOG_ROWS)..];
    for (i, line) in tail.iter().enumerate() {
        let newest = i + 1 == tail.len();
        label(
            frame,
            1,
            FIRST_LOG_ROW + i as i32,
            &fit(line, SHORT_CELLS),
            if newest { TEXT } else { DIM },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_menu::StackRow;
    use crate::layout::{MENU_ROWS, cell};
    use crate::screens::tests::assert_laid_out;
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
        }
    }

    fn wide_log() -> Vec<String> {
        (0..6).map(|i| format!("{i}{}", "x".repeat(60))).collect()
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
            message: String::new(),
        };
        combat(&mut frame, &view, &menu, &wide_log());
        assert_laid_out(&frame);
        assert_eq!(frame.widgets.len(), 8, "four stacks, four actions");
        for i in 0..4 {
            let w = frame.widget(WidgetId::Stack(i)).unwrap();
            assert_eq!(w.rect.right(), cell(1 + STACK_CELLS as i32, 0).0);
            assert_eq!(w.enabled, i != 3, "the slain row is inert");
        }
        let run = frame.widget(WidgetId::Action(3)).unwrap();
        assert!(run.rect.right() <= VIEWPORT.right());
        let mid = VIEWPORT.w as i32 / 2;
        let (top, bottom) = (TOP_PANEL, BOTTOM_PANEL);
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
        let (x, y) = cell(1, MENU_ROWS - 1);
        assert_eq!(
            rgb(&frame, x, y + 1),
            Some(TEXT),
            "the newest line is bright"
        );
        let (x, y) = cell(1, FIRST_LOG_ROW);
        assert_eq!(rgb(&frame, x, y + 1), Some(DIM), "older lines are dim");
        let (x, y) = cell(1 + SHORT_CELLS as i32, FIRST_LOG_ROW);
        assert_eq!(frame.raster.get(x, y + 1), Some([0, 0, 0, 0]), "clipped");
        let action = frame.widget(WidgetId::Action(2)).unwrap();
        let h = hit(&frame.widgets, action.rect.x, action.rect.y).unwrap();
        assert_eq!((h.id, h.kind), (WidgetId::Action(2), Kind::Button));
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
        assert_laid_out(&frame);
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
        assert_laid_out(&frame);
        let empty = frame.raster.fingerprint();
        let mut frame = Frame::default();
        let log: Vec<String> = (0..5)
            .map(|i| format!("Line {i} {}", "x".repeat(40)))
            .collect();
        defeat(&mut frame, &DefeatMenu { cursor: 1 }, &log);
        assert_laid_out(&frame);
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
