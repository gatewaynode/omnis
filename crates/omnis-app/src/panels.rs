//! The parts of the frame that live outside the menus: the location lines and the movement
//! pad in the right column, and the backdrop under them and the band (`band.rs` paints the
//! band itself). Text models are fitted to their cells here so the widths are testable.

use crate::font::{GLYPH_HEIGHT, fit};
use crate::layout::{BAND, CELL, HUD_COLUMNS, HUD_LINES, RIGHT_COLUMN, Rect, SIDEBAR_MAP};
use crate::widget::{
    DIM, FRAME, Frame, HI, Kind, PANEL, PadButton, PadState, TEXT, Widget, WidgetId,
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
        frame.push(widget);
    }
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
}
