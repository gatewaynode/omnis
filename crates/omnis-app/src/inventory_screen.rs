//! The inventory overlay painted in the menu box: the pane tabs on row 0 (`Row(0..panes)`),
//! the pane's summary, its rows (`Row(panes + i)`), and the action buttons (`Action(i)`),
//! each dim when it does not apply to the row under the cursor. Bevy-free.

use crate::font::fit;
use crate::inventory_menu::{InventoryAction, InventoryMenu, InventoryView, ItemRow};
use crate::layout::MENU_COLUMNS;
use crate::screens::{ItemState, item_state, label};
use crate::sheet_menu::slot_label;
use crate::widget::{DIM, Frame, HI, Kind, WidgetId};

/// Cells a tab may take; seven tabs at this pitch fit the grid.
const TAB_CELLS: usize = 8;
/// Cells from one tab to the next.
const TAB_PITCH: i32 = 10;
/// The first row of items.
const FIRST_ROW: i32 = 3;
/// Rows of items a pane shows.
pub const ROWS: usize = 10;
/// The row of the action buttons.
const ACTION_ROW: i32 = 14;
/// The action buttons' columns.
const ACTION_COLUMNS: [i32; 5] = [1, 12, 22, 32, 42];
/// Cells a full-width row may take.
const ROW_CELLS: usize = MENU_COLUMNS as usize - 2;

/// One row's text: the name, the count, the slot, and the marks.
#[must_use]
pub fn row_text(row: &ItemRow) -> String {
    let slot = row.slot.map_or("", slot_label);
    let marks = match (row.equipped, row.usable) {
        (true, _) => "worn",
        (false, true) => "use",
        (false, false) => "",
    };
    format!(
        "{:<34} x{:>3}   {:<10} {marks}",
        fit(&row.name, 34),
        row.count.min(999),
        slot
    )
}

fn tabs(frame: &mut Frame, menu: &InventoryMenu, view: &InventoryView) {
    label(frame, 1, 0, "ITEMS", HI);
    for (i, pane) in view.panes.iter().enumerate().take(7) {
        item_state(
            frame,
            WidgetId::Row(i),
            Kind::Button,
            (8 + i as i32 * TAB_PITCH, 0),
            &fit(&pane.title, TAB_CELLS),
            TAB_CELLS,
            ItemState::from_selected(i == menu.pane),
        );
    }
}

fn rows(frame: &mut Frame, menu: &InventoryMenu, view: &InventoryView) {
    let Some(pane) = view.panes.get(menu.pane) else {
        return;
    };
    label(frame, 1, 1, &fit(&pane.summary, ROW_CELLS), DIM);
    if pane.rows.is_empty() {
        label(frame, 1, FIRST_ROW, "nothing", DIM);
    }
    // The window of rows keeps the cursor in view.
    let first = menu.cursor.saturating_sub(ROWS - 1);
    for (i, row) in pane.rows.iter().enumerate().skip(first).take(ROWS) {
        item_state(
            frame,
            WidgetId::Row(view.panes.len() + i),
            Kind::Button,
            (1, FIRST_ROW + (i - first) as i32),
            &row_text(row),
            ROW_CELLS,
            ItemState::from_selected(i == menu.cursor),
        );
    }
}

fn actions(frame: &mut Frame, menu: &InventoryMenu, view: &InventoryView) {
    let stores = menu.stores(view);
    let row = menu.row(view);
    for (i, (action, column)) in InventoryAction::ALL.iter().zip(ACTION_COLUMNS).enumerate() {
        let state = if action.applies(stores, row) {
            ItemState::Normal
        } else {
            ItemState::Disabled
        };
        item_state(
            frame,
            WidgetId::Action(i),
            Kind::Button,
            (column, ACTION_ROW),
            action.label(),
            action.label().len(),
            state,
        );
    }
}

/// The overlay: tabs, the pane, the actions.
pub fn inventory(frame: &mut Frame, menu: &InventoryMenu, view: &InventoryView) {
    tabs(frame, menu, view);
    rows(frame, menu, view);
    actions(frame, menu, view);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory_menu::tests::sample;
    use crate::layout::{MENU_BOX, menu_cell};
    use crate::screens::tests::assert_laid_out;
    use crate::widget::{Part, hit};
    use omnis_sim::omnis_data::EquipSlot;

    #[test]
    fn the_overlay_lays_out_tabs_rows_and_actions_inside_the_box() {
        let view = sample();
        let mut menu = InventoryMenu::default();
        menu.sync(&view);
        let mut frame = Frame::default();
        inventory(&mut frame, &menu, &view);
        assert_laid_out(&frame, MENU_BOX);
        assert_eq!(frame.widgets.len(), 3 + 8 + 5, "tabs, rows, actions");
        let tab = frame.widget(WidgetId::Row(2)).unwrap();
        let h = hit(&frame.widgets, tab.rect.x, tab.rect.y).unwrap();
        assert_eq!((h.id, h.part), (WidgetId::Row(2), Part::Body));
        let first = frame.widget(WidgetId::Row(3)).unwrap();
        assert_eq!((first.rect.x, first.rect.y), menu_cell(1, FIRST_ROW));
        let equip = frame.widget(WidgetId::Action(0)).unwrap();
        let take = frame.widget(WidgetId::Action(3)).unwrap();
        assert!(
            equip.enabled && !take.enabled,
            "chain mail: equip yes, take no"
        );
        // The stores: take on, the rest off; an empty cursor pane says nothing.
        menu.click_row(2);
        let mut frame = Frame::default();
        inventory(&mut frame, &menu, &view);
        assert_laid_out(&frame, MENU_BOX);
        assert!(frame.widget(WidgetId::Action(3)).unwrap().enabled);
        assert!(!frame.widget(WidgetId::Action(0)).unwrap().enabled);
        assert!(!frame.widget(WidgetId::Action(4)).unwrap().enabled);
    }

    #[test]
    fn long_panes_scroll_to_the_cursor_and_wide_names_stay_in_the_grid() {
        let long = "x".repeat(60);
        let rows: Vec<ItemRow> = (0..25)
            .map(|i| ItemRow {
                name: format!("{long}{i}"),
                count: 65535,
                slot: Some(EquipSlot::MainHand),
                equipped: true,
                usable: true,
            })
            .collect();
        let view = InventoryView {
            panes: (0..7)
                .map(|i| crate::inventory_menu::Pane {
                    title: format!("{long}{i}"),
                    summary: long.clone(),
                    rows: rows.clone(),
                })
                .collect(),
        };
        let mut menu = InventoryMenu {
            pane: 6,
            cursor: 24,
            ..InventoryMenu::default()
        };
        menu.sync(&view);
        let mut frame = Frame::default();
        inventory(&mut frame, &menu, &view);
        assert_laid_out(&frame, MENU_BOX);
        assert_eq!(frame.widgets.len(), 7 + ROWS + 5);
        let last = frame.widget(WidgetId::Row(7 + 24)).unwrap();
        assert_eq!(last.rect.y, menu_cell(1, FIRST_ROW + ROWS as i32 - 1).1);
        assert!(frame.widget(WidgetId::Row(7)).is_none(), "scrolled off");
        let edge = menu_cell(MENU_COLUMNS, 0).0;
        for y in MENU_BOX.y..MENU_BOX.bottom() {
            for x in edge..MENU_BOX.right() {
                assert!(
                    !frame.raster.get(x, y).is_some_and(|p| p[3] != 0),
                    "text past column {MENU_COLUMNS} at ({x}, {y})"
                );
            }
        }
        assert_eq!(
            row_text(&rows[0]).len(),
            34 + 1 + 4 + 3 + 10 + 1 + 4,
            "{}",
            row_text(&rows[0])
        );
    }
}
