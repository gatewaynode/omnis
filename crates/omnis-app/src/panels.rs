//! The parts of the frame that live outside the menus: the location lines and the movement
//! pad in the right column, and the backdrop under everything (`band.rs` paints the band
//! itself). Text models are fitted to their cells here so the widths are testable.

use crate::canvas::Layout;
use crate::font::{GLYPH_HEIGHT, fit};
use crate::layout::{CELL, HUD_COLUMNS, HUD_LINES, Rect};
use crate::widget::{
    DIM, FRAME, Frame, HI, Kind, PANEL, PadButton, PadState, TEXT, Widget, WidgetId,
};
use omnis_sim::omnis_core::Calendar;

/// The three location lines, each fitted to the column.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hud {
    /// The map's display name.
    pub map: String,
    /// `x,y F` with the facing's initial.
    pub position: String,
    /// The party's date, the night, and its age (`clock_text`).
    pub clock: String,
}

impl Hud {
    /// From the map name, the position, the facing as displayed, and the clock line.
    #[must_use]
    pub fn new(map: &str, x: i32, y: i32, facing: &str, clock: &str) -> Hud {
        let initial = facing
            .chars()
            .next()
            .map(|c| c.to_ascii_uppercase())
            .unwrap_or('?');
        Hud {
            map: fit(map, HUD_COLUMNS),
            position: fit(&format!("{x},{y} {initial}"), HUD_COLUMNS),
            clock: fit(clock, HUD_COLUMNS),
        }
    }
}

/// The party's date on `calendar` (`Year 1 day 40 14:20`, counted from 1 for people), `night`
/// while it is, and the party's age (M8): `Year 1 day 40 14:20 night  age 2y 40d`.
#[must_use]
pub fn clock_text(date: i64, age: i64, calendar: Calendar) -> String {
    clock_text_in(date, age, calendar, HUD_COLUMNS)
}

/// The clock line fitted to `columns` cells: without the age, then as `Y1 D40 14:20`, when the
/// long form does not fit.
pub(crate) fn clock_text_in(date: i64, age: i64, calendar: Calendar, columns: usize) -> String {
    let d = calendar.date(date);
    let (year, day) = (d.year.saturating_add(1), d.day + 1);
    let time = format!("{:02}:{:02}", d.minute / 60, d.minute % 60);
    let night = if calendar.night(date) { " night" } else { "" };
    let days = calendar.day(age.max(0));
    let per_year = i64::from(calendar.days_per_year.max(1));
    let age = match days / per_year {
        0 => format!("age {days}d"),
        years => format!("age {years}y {}d", days % per_year),
    };
    let date = format!("Year {year} day {day} {time}{night}");
    [
        format!("{date}  {age}"),
        date,
        format!("Y{year} D{day} {time}{night}"),
    ]
    .into_iter()
    .find(|line| line.len() <= columns)
    .unwrap_or_else(|| fit(&format!("Y{year} D{day} {time}"), columns))
}

/// The message line: the last event, notice, or rejection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Message {
    /// The text.
    pub text: String,
    /// Whether it is a rejection or a failure.
    pub alert: bool,
}

/// Paint the backdrop: opaque panel everywhere but the viewport and the minimap, so viewport
/// sprites that overhang the viewport's edges never show outside it.
pub fn backdrop(frame: &mut Frame, layout: &Layout) {
    frame.raster.fill(layout.canvas(), PANEL);
    frame.raster.erase(layout.viewport());
    let (x, y, w, h) = layout.minimap();
    frame.raster.erase(Rect::new(x, y, w, h));
}

/// Paint the location lines.
pub fn hud(frame: &mut Frame, hud: &Hud) {
    for ((x, y), text) in HUD_LINES.iter().zip([&hud.map, &hud.position, &hud.clock]) {
        frame.raster.text(*x, *y, text, TEXT);
    }
}

/// Paint the pad and register its buttons.
pub fn pad(frame: &mut Frame, state: PadState, pressed: Option<WidgetId>) {
    for button in PadButton::ALL {
        framed_button(
            frame,
            WidgetId::Pad(button),
            button.rect(),
            button.label(),
            state,
            pressed,
        );
    }
}

/// One framed button with a centred label: bright when held, dim when disabled, nothing
/// when hidden; registered as a framed widget, enabled only when its state is.
fn framed_button(
    frame: &mut Frame,
    id: WidgetId,
    rect: Rect,
    label: &str,
    state: PadState,
    pressed: Option<WidgetId>,
) {
    if state == PadState::Hidden {
        return;
    }
    let held = pressed == Some(id) && state == PadState::Enabled;
    let (frame_color, ink) = match state {
        PadState::Enabled if held => (HI, PANEL),
        PadState::Enabled => (FRAME, TEXT),
        PadState::Disabled | PadState::Hidden => (DIM, DIM),
    };
    if held {
        frame.raster.fill(rect, HI);
    }
    frame.raster.stroke(rect, frame_color);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::NARROW;
    use crate::layout::{BAND, CANVAS, RIGHT_COLUMN, SIDEBAR_MAP, VIEWPORT};

    #[test]
    fn location_lines_fit_the_hud() {
        let c = Calendar::default();
        let hud = Hud::new("Test Dungeon", 3, 4, "south", &clock_text(208, 208, c));
        assert_eq!(hud.map, "Test Dungeon");
        assert_eq!(hud.position, "3,4 S");
        assert_eq!(hud.clock, "Year 1 day 1 03:28 night  age 0d");
        let name = "The Sunken Cathedral of the Drowned Kings and Their Court";
        let wide = Hud::new(name, 65535, 65535, "north", "");
        assert_eq!(wide.map, fit(name, HUD_COLUMNS));
        assert!(wide.map.len() <= HUD_COLUMNS && wide.position.len() <= HUD_COLUMNS);
        assert_eq!(wide.position, "65535,65535 N");
        // The date runs on its own; the age is the party's.
        let year = 360 * 1440;
        assert_eq!(
            clock_text(2 * year + 39 * 1440 + 14 * 60 + 20, year + 3 * 1440, c),
            "Year 3 day 40 14:20  age 1y 3d"
        );
        // Narrower, the age goes first, then the words.
        assert_eq!(clock_text_in(208, 208, c, 26), "Year 1 day 1 03:28 night");
        assert_eq!(clock_text_in(208, 208, c, 18), "Y1 D1 03:28 night");
        assert_eq!(clock_text_in(208, 208, c, 12), "Y1 D1 03:28");
        assert!(clock_text(i64::MAX, i64::MAX, c).len() <= HUD_COLUMNS);
        assert_eq!(Hud::new("", 0, 0, "", "").position, "0,0 ?");
    }

    #[test]
    fn the_backdrop_covers_the_column_and_band_but_not_the_minimap() {
        let mut frame = Frame::default();
        backdrop(&mut frame, &NARROW);
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
