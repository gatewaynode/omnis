//! The parts of the frame that live outside the menus: the location lines and the movement
//! pad in the right column, and the backdrop under everything (`band.rs` paints the band
//! itself). Text models are fitted to their cells here so the widths are testable.

use crate::canvas::Layout;
use crate::font::{GLYPH_HEIGHT, fit};
use crate::layout::{CELL, HUD_COLUMNS, HUD_LINES, Rect};
use crate::widget::{
    DIM, FRAME, Frame, HI, Kind, PANEL, PadButton, PadState, TEXT, ToolButton, ToolStates, Widget,
    WidgetId,
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

/// Paint the tool pad and register its buttons, each in its own state.
pub fn tools(frame: &mut Frame, states: ToolStates, pressed: Option<WidgetId>) {
    for button in ToolButton::ALL {
        framed_button(
            frame,
            WidgetId::Tool(button),
            button.rect(),
            button.label(),
            states.get(button),
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
    fn the_tool_pad_paints_each_button_in_its_own_state() {
        let mut frame = Frame::default();
        tools(&mut frame, ToolStates::default(), None);
        assert!(frame.widgets.is_empty(), "hidden paints nothing");
        let mut states = ToolStates::all(PadState::Disabled);
        states.set(ToolButton::Menu, PadState::Enabled);
        states.set(ToolButton::Map, PadState::Enabled);
        tools(&mut frame, states, Some(WidgetId::Tool(ToolButton::Menu)));
        assert_eq!(frame.widgets.len(), 6);
        let rgb = |x, y| frame.raster.get(x, y).map(|p| (p[0], p[1], p[2]));
        for button in ToolButton::ALL {
            let w = frame.widget(WidgetId::Tool(button)).expect("painted");
            assert!(w.framed && w.rect == button.rect(), "{button:?}");
            assert_eq!(w.enabled, states.get(button) == PadState::Enabled);
            let corner = rgb(w.rect.x, w.rect.y);
            let expected = match button {
                ToolButton::Menu => HI,
                ToolButton::Map => FRAME,
                _ => DIM,
            };
            assert_eq!(corner, Some(expected), "{button:?}");
        }
        let menu = ToolButton::Menu.rect();
        assert_eq!(
            rgb(menu.x + 2, menu.y + 2),
            Some(HI),
            "the held button is filled"
        );
        let map = ToolButton::Map.rect();
        assert_eq!(
            rgb(map.x + 2, map.y + 2),
            Some((0, 0, 0)),
            "the rest are not"
        );
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
