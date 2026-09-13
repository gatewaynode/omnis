//! The parts of the frame that live outside the menus: the location lines and the movement
//! pad in the right column, and the bottom band with the message, the party, and the help
//! line. Text models are fitted to their cells here so the widths are testable.

use crate::font::{GLYPH_HEIGHT, fit};
use crate::layout::{
    BAND, BAND_BACK_X, BAND_COLUMNS, BAND_FRONT_X, BAND_HELP, BAND_MESSAGE, BAND_ROW_COLUMNS,
    BAND_ROWS, CELL, HUD_COLUMNS, HUD_LINES, RIGHT_COLUMN, Rect, SIDEBAR_MAP,
};
use crate::widget::{
    ALERT, DIM, FRAME, Frame, HI, Kind, PANEL, PadButton, PadState, SP, TEXT, Widget, WidgetId,
};
use omnis_sim::MINUTES_PER_DAY;

/// The three location lines, each fitted to the column.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hud {
    /// The map's display name.
    pub map: String,
    /// `x,y F` with the facing's initial.
    pub position: String,
    /// `Day d hh:mm`.
    pub clock: String,
}

impl Hud {
    /// From the map name, the position, the facing as displayed, and the party clock.
    #[must_use]
    pub fn new(map: &str, x: i32, y: i32, facing: &str, elapsed: i64) -> Hud {
        let initial = facing
            .chars()
            .next()
            .map(|c| c.to_ascii_uppercase())
            .unwrap_or('?');
        Hud {
            map: fit(map, HUD_COLUMNS),
            position: fit(&format!("{x},{y} {initial}"), HUD_COLUMNS),
            clock: clock_text(elapsed),
        }
    }
}

/// The clock as `Day d hh:mm`, or `Dd hh:mm` once the day number no longer fits the HUD.
#[must_use]
pub fn clock_text(elapsed: i64) -> String {
    clock_text_in(elapsed, HUD_COLUMNS)
}

/// The clock fitted to `columns` cells.
pub(crate) fn clock_text_in(elapsed: i64, columns: usize) -> String {
    let per_day = i64::from(MINUTES_PER_DAY);
    let day = elapsed.div_euclid(per_day) + 1;
    let minute = elapsed.rem_euclid(per_day);
    let long = format!("Day {day} {:02}:{:02}", minute / 60, minute % 60);
    if long.len() <= columns {
        long
    } else {
        fit(
            &format!("D{day} {:02}:{:02}", minute / 60, minute % 60),
            columns,
        )
    }
}

/// One party slot as the band shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemberRow {
    /// The name.
    pub name: String,
    /// The class's display name.
    pub class: String,
    /// Hit points.
    pub hp: i32,
    /// Hit point maximum.
    pub hp_max: i32,
    /// Spell points.
    pub sp: u32,
    /// The first condition's initial.
    pub condition: Option<char>,
}

impl MemberRow {
    /// The name column: ten cells.
    #[must_use]
    pub fn name_text(&self) -> String {
        format!("{:<10}", fit(&self.name, 10))
    }

    /// The points column after the name: ` hp/max sp c`, twelve cells at most.
    #[must_use]
    pub fn points_text(&self) -> String {
        format!(
            " {:>3}/{:<3} {:>2} {}",
            self.hp.clamp(-99, 999),
            self.hp_max.clamp(0, 999),
            self.sp.min(99),
            crate::font::printable(self.condition.unwrap_or(' '))
        )
    }

    /// The class column after the name, while creating.
    #[must_use]
    pub fn class_text(&self) -> String {
        format!(" {}", fit(&self.class, BAND_ROW_COLUMNS - 11))
    }

    /// Whether hit points are below a quarter.
    #[must_use]
    pub fn hp_low(&self) -> bool {
        self.hp * 4 < self.hp_max
    }
}

/// The message line: the last event, notice, or rejection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Message {
    /// The text.
    pub text: String,
    /// Whether it is a rejection or a failure.
    pub alert: bool,
}

/// Paint the column and band backdrop: opaque panel around the minimap and across the band,
/// so viewport sprites that overhang the viewport's edges never show there.
pub fn backdrop(frame: &mut Frame) {
    let map = Rect::new(SIDEBAR_MAP.0, SIDEBAR_MAP.1, SIDEBAR_MAP.2, SIDEBAR_MAP.3);
    let column = RIGHT_COLUMN;
    let above = Rect::new(column.x, column.y, column.w, (map.y - column.y) as u32);
    let left = Rect::new(column.x, map.y, (map.x - column.x) as u32, map.h);
    let right = Rect::new(
        map.right(),
        map.y,
        (column.right() - map.right()) as u32,
        map.h,
    );
    let below = Rect::new(
        column.x,
        map.bottom(),
        column.w,
        (column.bottom() - map.bottom()) as u32,
    );
    for rect in [above, left, right, below, BAND] {
        frame.raster.fill(rect, PANEL);
    }
}

/// Paint the location lines.
pub fn hud(frame: &mut Frame, hud: &Hud) {
    for ((x, y), text) in HUD_LINES.iter().zip([&hud.map, &hud.position, &hud.clock]) {
        frame.raster.text(*x, *y, text, TEXT);
    }
}

/// Paint the pad and register its buttons.
pub fn pad(frame: &mut Frame, state: PadState, pressed: Option<WidgetId>) {
    if state == PadState::Hidden {
        return;
    }
    for button in PadButton::ALL {
        let rect = button.rect();
        let id = WidgetId::Pad(button);
        let (frame_color, ink) = match state {
            PadState::Enabled if pressed == Some(id) => (HI, PANEL),
            PadState::Enabled => (FRAME, TEXT),
            PadState::Disabled | PadState::Hidden => (DIM, DIM),
        };
        if pressed == Some(id) && state == PadState::Enabled {
            frame.raster.fill(rect, HI);
        }
        frame.raster.stroke(rect, frame_color);
        let label = button.label();
        let width = label.len() as i32 * CELL.0 - 1;
        let y = rect.y + (rect.h as i32 - GLYPH_HEIGHT) / 2;
        frame
            .raster
            .text(rect.x + (rect.w as i32 - width) / 2, y, label, ink);
        let mut widget = Widget::new(id, rect, Kind::Button);
        widget.framed = true;
        widget.enabled = state == PadState::Enabled;
        frame.widgets.push(widget);
    }
}

/// Where a party slot's row starts, if it is shown: the front row fills the left column,
/// the back row the right one.
#[must_use]
pub fn slot_origin(slot: usize, front_row: usize) -> Option<(i32, i32)> {
    let front = front_row.min(BAND_ROWS.len());
    if slot < front {
        Some((BAND_FRONT_X, BAND_ROWS[slot]))
    } else {
        BAND_ROWS.get(slot - front).map(|y| (BAND_BACK_X, *y))
    }
}

/// What the band shows.
pub struct Band<'a> {
    /// The message line.
    pub message: &'a Message,
    /// The party, in marching order.
    pub members: &'a [MemberRow],
    /// How many members stand in front.
    pub front_row: usize,
    /// The member the mouse selected.
    pub selected: Option<usize>,
    /// Whether rows show classes (creation) instead of points.
    pub creating: bool,
    /// The help line.
    pub help: &'a str,
}

/// Paint the band: message, party rows, help.
pub fn band(frame: &mut Frame, view: &Band<'_>) {
    let color = if view.message.alert { ALERT } else { TEXT };
    frame.raster.text(
        BAND_MESSAGE.0,
        BAND_MESSAGE.1,
        &fit(&view.message.text, BAND_COLUMNS),
        color,
    );
    for (slot, member) in view.members.iter().enumerate() {
        let Some((x, y)) = slot_origin(slot, view.front_row) else {
            continue;
        };
        let selected = view.selected == Some(slot);
        let name_color = if selected { HI } else { TEXT };
        frame.raster.text(x, y, &member.name_text(), name_color);
        let after = x + 10 * CELL.0;
        if view.creating {
            frame.raster.text(after, y, &member.class_text(), TEXT);
        } else {
            let points = member.points_text();
            let hp_color = if member.hp_low() { ALERT } else { TEXT };
            frame.raster.text(after, y, &points[..8], hp_color);
            frame.raster.text(after + 8 * CELL.0, y, &points[8..11], SP);
            frame
                .raster
                .text(after + 11 * CELL.0, y, &points[11..], ALERT);
        }
        let rect = Rect::new(x, y, BAND_ROW_COLUMNS as u32 * CELL.0 as u32, CELL.1 as u32);
        frame
            .widgets
            .push(Widget::new(WidgetId::Member(slot), rect, Kind::Button));
    }
    frame
        .raster
        .text(BAND_HELP.0, BAND_HELP.1, &fit(view.help, BAND_COLUMNS), DIM);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{CANVAS, VIEWPORT};

    #[test]
    fn location_lines_fit_the_hud() {
        let hud = Hud::new("Test Dungeon", 3, 4, "south", 208);
        assert_eq!(hud.map, "Test Dungeon");
        assert_eq!(hud.position, "3,4 S");
        assert_eq!(hud.clock, "Day 1 03:28");
        let name = "The Sunken Cathedral of the Drowned Kings and Their Court";
        let wide = Hud::new(name, 65535, 65535, "north", 0);
        assert_eq!(wide.map, fit(name, HUD_COLUMNS));
        assert!(wide.map.len() <= HUD_COLUMNS && wide.position.len() <= HUD_COLUMNS);
        assert_eq!(wide.position, "65535,65535 N");
        // At thirteen cells the day number runs out of room after day 999.
        assert_eq!(clock_text_in(1440 * 998 + 208, 13), "Day 999 03:28");
        assert_eq!(clock_text_in(1440 * 999 + 208, 13), "D1000 03:28");
        assert!(clock_text(i64::MAX).len() <= HUD_COLUMNS);
        assert_eq!(Hud::new("", 0, 0, "", 0).position, "0,0 ?");
    }

    #[test]
    fn party_rows_fit_twenty_six_cells_at_their_widest() {
        let row = MemberRow {
            name: "Bartholomew Longname".into(),
            class: "Fighter".into(),
            hp: -99,
            hp_max: 999,
            sp: 99,
            condition: Some('P'),
        };
        let text = row.name_text() + row.points_text().as_str();
        assert_eq!(text, "Bartholome -99/999 99 P");
        assert!(text.len() <= BAND_ROW_COLUMNS);
        let brenna = MemberRow {
            name: "Brenna".into(),
            class: "Wizard".into(),
            hp: 12,
            hp_max: 12,
            sp: 4,
            condition: None,
        };
        assert_eq!(
            brenna.name_text() + brenna.points_text().as_str(),
            "Brenna      12/12   4  "
        );
        assert_eq!(
            brenna.name_text() + brenna.class_text().as_str(),
            "Brenna     Wizard"
        );
        assert!(!brenna.hp_low());
        assert!(
            MemberRow {
                hp: 2,
                hp_max: 9,
                ..brenna.clone()
            }
            .hp_low()
        );
        let long_class = MemberRow {
            class: "Battle Chaplain of the Dawn".into(),
            ..brenna
        };
        assert!(
            (long_class.name_text() + long_class.class_text().as_str()).len() <= BAND_ROW_COLUMNS
        );
    }

    #[test]
    fn the_backdrop_covers_the_column_and_band_but_not_the_minimap() {
        let mut frame = Frame::default();
        backdrop(&mut frame);
        let panel = [PANEL.0, PANEL.1, PANEL.2, 255];
        let (mx, my, mw, mh) = SIDEBAR_MAP;
        let clear = Some([0, 0, 0, 0]);
        let get = |x, y| frame.raster.get(x, y);
        assert_eq!(
            get(RIGHT_COLUMN.x, my + 10),
            Some(panel),
            "left of the minimap"
        );
        assert_eq!(
            get(CANVAS.right() - 1, 0),
            Some(panel),
            "the column's corner"
        );
        assert_eq!(
            get(mx + 10, my + mh as i32 + 2),
            Some(panel),
            "under the minimap"
        );
        assert_eq!(get(mx + mw as i32, my + 10), Some(panel), "right of it");
        assert_eq!(get(100, BAND.y + 10), Some(panel), "the band");
        assert_eq!(get(mx + 10, my + 10), clear, "the minimap shows through");
        assert_eq!(get(mx + mw as i32 - 1, my + mh as i32 - 1), clear);
        assert_eq!(
            get(VIEWPORT.w as i32 / 2, VIEWPORT.h as i32 / 2),
            clear,
            "the viewport"
        );
        assert_eq!(get(VIEWPORT.right() - 1, VIEWPORT.bottom() - 1), clear);
    }

    #[test]
    fn slots_fill_the_front_column_then_the_back_column() {
        assert_eq!(slot_origin(0, 3), Some((BAND_FRONT_X, BAND_ROWS[0])));
        assert_eq!(slot_origin(2, 3), Some((BAND_FRONT_X, BAND_ROWS[2])));
        assert_eq!(slot_origin(3, 3), Some((BAND_BACK_X, BAND_ROWS[0])));
        assert_eq!(slot_origin(5, 3), Some((BAND_BACK_X, BAND_ROWS[2])));
        assert_eq!(slot_origin(6, 3), None);
        assert_eq!(slot_origin(1, 1), Some((BAND_BACK_X, BAND_ROWS[0])));
        assert_eq!(slot_origin(3, 4), Some((BAND_BACK_X, BAND_ROWS[0])));
    }
}
