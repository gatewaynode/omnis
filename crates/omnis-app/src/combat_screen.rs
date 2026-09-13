//! The fight painted over the viewport (ARCHITECTURE.md §8.1): the stack rows above the 3D
//! view, the action row and the roll log below it, and the modal that follows a wipe. The
//! middle rows stay clear so the scene and the silhouettes show through. Bevy-free.

use crate::actors;
use crate::combat_menu::{CombatMenu, DefeatMenu, EncounterMenu, FightView};
use crate::combat_text::SHORT_CELLS;
use crate::font::fit;
use crate::layout::Rect;
use crate::screens::{ItemState, item_state, label, label_right, modal};
use crate::widget::{DIM, Frame, HI, Kind, PANEL, TEXT, WidgetId};

/// The panel above the clear window: rows 0 to 4.
pub const TOP_PANEL: Rect = Rect::new(0, 0, 240, 40);
/// The panel below it: rows 11 to 15.
pub const BOTTOM_PANEL: Rect = Rect::new(0, 88, 240, 47);
/// The first stack row.
const FIRST_STACK_ROW: i32 = 1;
/// The action row.
const ACTION_ROW: i32 = 11;
/// The first roll-log row; the log runs to the last row of the viewport.
const FIRST_LOG_ROW: i32 = 12;
/// The row of the count digits under the silhouettes.
const COUNT_ROW: i32 = 10;
/// Rows of the roll log tail.
pub const LOG_ROWS: usize = 4;
/// Cells of a stack row: `99 {name:<20} front`.
const STACK_CELLS: usize = 29;
/// The action row columns for the fight: Attack, Dodge, Exchange, Run.
const COMBAT_ACTION_COLUMNS: [i32; 4] = [1, 9, 16, 26];
/// The action row columns before it: Attack, Bribe …, Hide, Run.
const ENCOUNTER_ACTION_COLUMNS: [i32; 4] = [1, 9, 22, 28];
/// Where the defeat modal sits.
pub const DEFEAT_RECT: Rect = Rect::new(48, 32, 144, 64);

/// The fight: header, stack rows with the target marked, the four actions, the log tail.
pub fn combat(frame: &mut Frame, view: &FightView, menu: &CombatMenu, log: &[String]) {
    panels(frame);
    label(frame, 1, 0, &format!("COMBAT  Round {}", view.round), HI);
    if let Some(actor) = &view.actor {
        label_right(frame, 0, &format!("{actor:.10} to act"), TEXT);
    }
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
/// Cells a modal line may take inside the defeat box.
const DEFEAT_LINE_CELLS: usize = 20;

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
        let x = s.rect.x + s.rect.w as i32 / 2 - 3 * text.len() as i32;
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
    use crate::layout::cell;
    use crate::screens::tests::assert_laid_out;
    use crate::widget::hit;
    use omnis_sim::ModeKind;
    use omnis_sim::omnis_data::{Disposition, Size};

    /// Four stacks of the widest names and counts, the widest round and actor.
    fn widest_view(phase: ModeKind) -> FightView {
        FightView {
            phase,
            round: 999,
            actor: Some("Bartholomew Longname".to_owned()),
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
        assert!(frame.widget(WidgetId::Action(3)).unwrap().rect.right() <= 240);
        assert_eq!(rgb(&frame, 120, 20), Some(PANEL), "the top band");
        assert_eq!(rgb(&frame, 120, 100), Some(PANEL), "the bottom band");
        assert_eq!(frame.raster.get(120, 60), Some([0, 0, 0, 0]), "the window");
        assert_eq!(frame.raster.get(120, 87), Some([0, 0, 0, 0]));
        assert_eq!(rgb(&frame, 120, 88), Some(PANEL));
        let target = frame.widget(WidgetId::Stack(1)).unwrap();
        assert_eq!(
            rgb(&frame, target.rect.x - 5, target.rect.y + 1),
            Some(HI),
            "the marker sits before the target"
        );
        let (x, y) = cell(1, 15);
        assert_eq!(
            rgb(&frame, x, y + 1),
            Some(TEXT),
            "the newest line is bright"
        );
        let (x, y) = cell(1, 12);
        assert_eq!(rgb(&frame, x, y + 1), Some(DIM), "older lines are dim");
        let (x, y) = cell(1 + SHORT_CELLS as i32, 12);
        assert_eq!(frame.raster.get(x, y + 1), Some([0, 0, 0, 0]), "clipped");
        let action = frame.widget(WidgetId::Action(2)).unwrap();
        let h = hit(&frame.widgets, action.rect.x, action.rect.y).unwrap();
        assert_eq!((h.id, h.kind), (WidgetId::Action(2), Kind::Button));
        let (_, y) = cell(0, COUNT_ROW);
        let digits = (0..240)
            .filter(|x| rgb(&frame, *x, y + 1) == Some(TEXT))
            .count();
        assert!(digits > 0, "the counts sit under the silhouettes");
        assert_eq!(
            frame.raster.get(120, y - 1),
            Some([0, 0, 0, 0]),
            "row 9 is clear"
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
        assert_eq!(bribe.rect.w, 6 * "Bribe 9999g".len() as u32);
        assert!(frame.widget(WidgetId::Action(0)).unwrap().enabled);
        view.bribe = Some(0);
        let mut frame = Frame::default();
        encounter(&mut frame, &view, &menu);
        let bribe = frame.widget(WidgetId::Action(1)).unwrap();
        assert!(bribe.enabled);
        assert_eq!(bribe.rect.w, 6 * "Bribe free".len() as u32);
        assert_eq!(frame.raster.get(120, 60), Some([0, 0, 0, 0]));
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
        // Three lines of at most 20 cells fit between the title and the buttons: the
        // longest line ends inside the box.
        let right = DEFEAT_RECT.x + 12 + 6 * DEFEAT_LINE_CELLS as i32;
        assert!(right < DEFEAT_RECT.right(), "{right}");
        assert_eq!(
            frame.raster.get(right + 2, DEFEAT_RECT.y + 19),
            Some([PANEL.0, PANEL.1, PANEL.2, 255]),
            "nothing past the clipped line"
        );
        assert_eq!(frame.widgets.len(), 2);
        for w in &frame.widgets {
            assert!(DEFEAT_RECT.encloses(w.rect));
        }
        assert_eq!(
            frame.raster.get(10, 10),
            Some([0, 0, 0, 0]),
            "the world shows"
        );
        // The log lines cross the box's middle; its margin past them stays panel.
        assert_eq!(rgb(&frame, DEFEAT_RECT.right() - 3, 60), Some(PANEL));
        assert_eq!(rgb(&frame, 120, DEFEAT_RECT.y + 16), Some(PANEL));
    }
}
