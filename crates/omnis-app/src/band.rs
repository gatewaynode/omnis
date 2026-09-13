//! The bottom band: the message line, the roster with one row per member (their class,
//! level, points, armour class, and condition), the event log with the roll math beside it,
//! and the help line. The acting member's row is barred and marked, the way the menus mark
//! their cursor; the mouse's selection keeps its highlighted name, so both read at once.
//! Bevy-free; the rows and columns are `canvas.rs` constants.

use crate::canvas::Layout;
use crate::font::fit;
use crate::layout::{CELL, Rect};
use crate::panels::Message;
use crate::raster::Rgb;
use crate::widget::{ALERT, DIM, FRAME, Frame, HI, Kind, SP, TEXT, Widget, WidgetId};

pub use crate::canvas::{
    BAND_COLUMNS, BAND_ROW0, CAPTION_ROW, FIRST_MEMBER_ROW, HELP_ROW, LOG_CELLS, LOG_COLUMN,
    LOG_ROWS, MEMBER_ROWS, MESSAGE_ROW, ROSTER_CELLS, ROWS_USED, band_cell,
};

/// The roster's columns: where each run starts and its caption.
const COLUMNS: [(i32, &str); 8] = [
    (0, ""),
    (6, "NAME"),
    (23, "CLASS"),
    (34, "LV"),
    (37, "HP"),
    (45, "SP"),
    (48, "AC"),
    (51, "CONDITION"),
];

/// One party slot as the roster shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemberRow {
    /// The name.
    pub name: String,
    /// The class's display name.
    pub class: String,
    /// The level.
    pub level: u8,
    /// Hit points.
    pub hp: i32,
    /// Hit point maximum.
    pub hp_max: i32,
    /// Spell points.
    pub sp: u32,
    /// Armour class.
    pub ac: i64,
    /// The first condition's label.
    pub condition: Option<String>,
}

impl MemberRow {
    /// Whether hit points are below a quarter.
    #[must_use]
    pub fn hp_low(&self) -> bool {
        self.hp * 4 < self.hp_max
    }

    /// The row's runs of text by column with their colours; `group` is the word before the
    /// name ("front", "back", or nothing).
    #[must_use]
    pub fn cells(&self, group: &str, style: RowStyle) -> Vec<(i32, String, Rgb)> {
        let hp = if self.hp_low() { ALERT } else { TEXT };
        let condition = self.condition.as_deref().unwrap_or("");
        vec![
            (COLUMNS[0].0, format!("{:<5}", fit(group, 5)), DIM),
            (
                COLUMNS[1].0,
                format!("{:<16}", fit(&self.name, 16)),
                style.name,
            ),
            (COLUMNS[2].0, format!("{:<10}", fit(&self.class, 10)), TEXT),
            (COLUMNS[3].0, format!("{:>2}", self.level.min(99)), TEXT),
            (
                COLUMNS[4].0,
                format!(
                    "{:>3}/{:<3}",
                    self.hp.clamp(-99, 999),
                    self.hp_max.clamp(0, 999)
                ),
                hp,
            ),
            (COLUMNS[5].0, format!("{:>2}", self.sp.min(99)), SP),
            (COLUMNS[6].0, format!("{:>2}", self.ac.clamp(-9, 99)), TEXT),
            (COLUMNS[7].0, fit(condition, 10), ALERT),
        ]
    }
}

/// How a roster row is drawn: a bar under it, the name's colour, the marker before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowStyle {
    /// The bar's colour, when the member is acting.
    pub fill: Option<Rgb>,
    /// The name's colour: highlighted when the mouse selected the member.
    pub name: Rgb,
    /// Whether the marker points at the row.
    pub marker: bool,
}

/// The style for a row: acting and selected are independent, so all four states differ.
#[must_use]
pub fn row_style(acting: bool, selected: bool) -> RowStyle {
    RowStyle {
        fill: acting.then_some(FRAME),
        name: if selected { HI } else { TEXT },
        marker: acting,
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
    /// The member whose turn it is.
    pub acting: Option<usize>,
    /// The event log, oldest first.
    pub log: &'a [String],
    /// The help line.
    pub help: &'a str,
}

/// Paint the band: message, captions, roster, the rule, the log, help. The roster sits in
/// the wing when the layout has one, else beside the log with a rule between.
pub fn band(frame: &mut Frame, layout: &Layout, view: &Band<'_>) {
    let columns = layout.band_columns();
    let (x, y) = layout.band_cell(0, MESSAGE_ROW);
    let color = if view.message.alert { ALERT } else { TEXT };
    frame
        .raster
        .text(x, y, &fit(&view.message.text, columns), color);
    roster(frame, layout, view);
    if !layout.is_wide() {
        let (x, y) = layout.band_cell(LOG_COLUMN - 1, CAPTION_ROW);
        let height = ((HELP_ROW - CAPTION_ROW) * CELL.1) as u32;
        frame.raster.fill(Rect::new(x + 2, y, 1, height), FRAME);
    }
    let (x, y) = layout.band_cell(layout.log_column(), CAPTION_ROW);
    frame.raster.text(x, y, "EVENTS", DIM);
    log(frame, layout, view.log);
    let (x, y) = layout.band_cell(0, HELP_ROW);
    frame.raster.text(x, y, &fit(view.help, columns), DIM);
}

/// The captions and one row per member, the acting one barred and marked.
fn roster(frame: &mut Frame, layout: &Layout, view: &Band<'_>) {
    let (x, y) = layout.roster_origin();
    for (column, caption) in COLUMNS {
        frame.raster.text(x + column * CELL.0, y, caption, DIM);
    }
    for (slot, member) in view.members.iter().enumerate() {
        let Some((x, y)) = layout.slot_origin(slot) else {
            break;
        };
        let group = if slot == 0 {
            "front"
        } else if slot == view.front_row {
            "back"
        } else {
            ""
        };
        let style = row_style(view.acting == Some(slot), view.selected == Some(slot));
        let rect = Rect::new(x, y, ROSTER_CELLS as u32 * CELL.0 as u32, CELL.1 as u32);
        if let Some(fill) = style.fill {
            frame.raster.fill(rect, fill);
        }
        for (column, text, color) in member.cells(group, style) {
            frame.raster.text(x + column * CELL.0, y, &text, color);
        }
        if style.marker {
            frame.raster.marker(x - 5, y + 1, HI);
        }
        frame.push(Widget::new(WidgetId::Member(slot), rect, Kind::Button));
    }
}

/// The log's tail, newest at the bottom in full colour, the older lines dim.
fn log(frame: &mut Frame, layout: &Layout, lines: &[String]) {
    let tail = &lines[lines.len().saturating_sub(LOG_ROWS)..];
    let first = FIRST_MEMBER_ROW + LOG_ROWS as i32 - tail.len() as i32;
    let cells = layout.log_cells();
    for (i, line) in tail.iter().enumerate() {
        let (x, y) = layout.band_cell(layout.log_column(), first + i as i32);
        let newest = i + 1 == tail.len();
        let color = if newest { TEXT } else { DIM };
        frame.raster.text(x, y, &fit(line, cells), color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::NARROW;
    use crate::layout::BAND;
    use crate::widget::PANEL;

    fn member(name: &str) -> MemberRow {
        MemberRow {
            name: name.to_owned(),
            class: "Fighter".to_owned(),
            level: 1,
            hp: 12,
            hp_max: 12,
            sp: 0,
            ac: 16,
            condition: None,
        }
    }

    fn view<'a>(
        members: &'a [MemberRow],
        selected: Option<usize>,
        acting: Option<usize>,
        log: &'a [String],
        message: &'a Message,
    ) -> Band<'a> {
        Band {
            message,
            members,
            front_row: 3,
            selected,
            acting,
            log,
            help: "help",
        }
    }

    fn rgb(frame: &Frame, x: i32, y: i32) -> Option<(u8, u8, u8)> {
        frame.raster.get(x, y).map(|p| (p[0], p[1], p[2]))
    }

    fn row(column: i32, r: i32, cells: usize) -> Rect {
        let (x, y) = band_cell(column, r);
        Rect::new(x, y, cells as u32 * CELL.0 as u32, CELL.1 as u32)
    }

    #[test]
    fn regions_fit_the_band_and_do_not_overlap() {
        let message = row(0, MESSAGE_ROW, BAND_COLUMNS);
        let caption = row(0, CAPTION_ROW, ROSTER_CELLS);
        let help = row(0, HELP_ROW, BAND_COLUMNS);
        let log = Rect::new(
            band_cell(LOG_COLUMN, FIRST_MEMBER_ROW).0,
            band_cell(LOG_COLUMN, FIRST_MEMBER_ROW).1,
            LOG_CELLS as u32 * CELL.0 as u32,
            LOG_ROWS as u32 * CELL.1 as u32,
        );
        let rows: Vec<Rect> = (0..MEMBER_ROWS)
            .map(|slot| {
                let (x, y) = NARROW.slot_origin(slot).unwrap();
                Rect::new(x, y, ROSTER_CELLS as u32 * CELL.0 as u32, CELL.1 as u32)
            })
            .collect();
        for r in [message, caption, help, log].iter().chain(&rows) {
            assert!(BAND.encloses(*r), "{r:?}");
        }
        assert!(message.bottom() <= caption.y && caption.bottom() <= rows[0].y);
        assert!(rows[MEMBER_ROWS - 1].bottom() <= help.y && log.bottom() <= help.y);
        for r in &rows {
            assert!(!r.overlaps(log), "{r:?}");
        }
        assert!(NARROW.slot_origin(MEMBER_ROWS).is_none());
        assert!(
            band_cell(0, 0).0 >= 5,
            "room for the marker before the roster"
        );
        assert!(band_cell(BAND_COLUMNS as i32, 0).0 <= BAND.right());
    }

    #[test]
    fn roster_rows_fit_sixty_one_cells_at_their_widest() {
        let wide = MemberRow {
            name: "Bartholomew Longname the Third".to_owned(),
            class: "Battle Chaplain of the Dawn".to_owned(),
            level: 200,
            hp: -999,
            hp_max: 9999,
            sp: 999,
            ac: -99,
            condition: Some("Unconscious and afraid".to_owned()),
        };
        let cells = wide.cells("front", row_style(false, false));
        assert_eq!(cells.len(), COLUMNS.len());
        for (i, (column, text, _)) in cells.iter().enumerate() {
            let end = column + text.chars().count() as i32;
            assert!(end <= ROSTER_CELLS as i32, "{text:?} ends at {end}");
            if let Some((next, _, _)) = cells.get(i + 1) {
                assert!(end < *next, "{text:?} runs into the next column");
            }
        }
        assert_eq!(cells[1].1, "Bartholomew Long");
        assert_eq!(cells[3].1, "99");
        assert_eq!(cells[4].1, "-99/999");
        assert_eq!(cells[7].1, "Unconsciou");
        let last = COLUMNS[COLUMNS.len() - 1];
        assert!(last.0 + last.1.len() as i32 <= ROSTER_CELLS as i32);
    }

    #[test]
    fn the_acting_row_differs_from_selected_and_plain_rows() {
        let members = [member("Brenna"), member("Durin"), member("Ilvara")];
        let message = Message::default();
        let mut frame = Frame::default();
        band(
            &mut frame,
            &NARROW,
            &view(&members, Some(1), Some(0), &[], &message),
        );
        let bar = |slot: usize| {
            let (x, y) = NARROW.slot_origin(slot).unwrap();
            // The gap cell between the group word and the name is bare on a plain row.
            rgb(&frame, x + 5 * CELL.0 + 2, y + 3)
        };
        assert_eq!(bar(0), Some(FRAME), "the acting row is barred");
        assert_eq!(bar(1), Some((0, 0, 0)), "the selected row is not");
        assert_eq!(bar(2), Some((0, 0, 0)));
        let name_ink = |slot: usize| {
            let (x, y) = NARROW.slot_origin(slot).unwrap();
            (0..16 * CELL.0)
                .flat_map(|dx| (0..CELL.1).map(move |dy| (dx, dy)))
                .find_map(|(dx, dy)| {
                    rgb(&frame, x + 6 * CELL.0 + dx, y + dy).filter(|c| *c == HI || *c == TEXT)
                })
        };
        assert_eq!(name_ink(1), Some(HI), "the selected name is highlighted");
        assert_eq!(name_ink(2), Some(TEXT));
        let (x, y) = NARROW.slot_origin(0).unwrap();
        assert_eq!(
            rgb(&frame, x - 4, y + 3),
            Some(HI),
            "the marker points at the acting row"
        );
        let (x, y) = NARROW.slot_origin(1).unwrap();
        assert_ne!(rgb(&frame, x - 4, y + 3), Some(HI));
        assert_eq!(frame.widgets.len(), 3);
        // Acting and selected on one row: barred, marked, and highlighted.
        let mut frame = Frame::default();
        band(
            &mut frame,
            &NARROW,
            &view(&members, Some(1), Some(1), &[], &message),
        );
        let (x, y) = NARROW.slot_origin(1).unwrap();
        assert_eq!(rgb(&frame, x + 5 * CELL.0 + 2, y + 3), Some(FRAME));
        assert_eq!(rgb(&frame, x - 4, y + 3), Some(HI));
        assert_eq!(name_ink(1), Some(HI));
        let _ = PANEL;
    }

    #[test]
    fn the_log_anchors_its_newest_line_at_the_bottom() {
        let members = [member("Brenna")];
        let message = Message::default();
        let lines: Vec<String> = (0..3).map(|i| format!("Line {i} HHHH")).collect();
        let mut frame = Frame::default();
        band(
            &mut frame,
            &NARROW,
            &view(&members, None, None, &lines, &message),
        );
        let last = FIRST_MEMBER_ROW + LOG_ROWS as i32 - 1;
        let ink = |row: i32| {
            let (x, y) = band_cell(LOG_COLUMN, row);
            (0..12 * CELL.0).find_map(|dx| rgb(&frame, x + dx, y + 3).filter(|c| *c != (0, 0, 0)))
        };
        assert_eq!(ink(last), Some(TEXT), "the newest line is bright");
        assert_eq!(ink(last - 1), Some(DIM), "older lines are dim");
        assert_eq!(ink(last - 3), None, "nothing above the tail");
        assert_eq!(ink(FIRST_MEMBER_ROW), None);
        // A line past the log's width is clipped at it.
        let long = vec!["H".repeat(LOG_CELLS + 20)];
        let mut frame = Frame::default();
        band(
            &mut frame,
            &NARROW,
            &view(&members, None, None, &long, &message),
        );
        let (x, y) = band_cell(LOG_COLUMN, last);
        assert_eq!(
            rgb(&frame, x + (LOG_CELLS as i32 - 1) * CELL.0, y + 3),
            Some(TEXT)
        );
        assert_ne!(
            rgb(&frame, x + LOG_CELLS as i32 * CELL.0, y + 3),
            Some(TEXT)
        );
        // The message and help lines are on their rows.
        let message = Message {
            text: "Hello".to_owned(),
            alert: true,
        };
        let mut frame = Frame::default();
        band(
            &mut frame,
            &NARROW,
            &view(&members, None, None, &[], &message),
        );
        let (x, y) = band_cell(0, MESSAGE_ROW);
        assert!((0..30).any(|dx| rgb(&frame, x + dx, y + 3) == Some(ALERT)));
        let (x, y) = band_cell(0, HELP_ROW);
        assert!((0..24).any(|dx| rgb(&frame, x + dx, y + 3) == Some(DIM)));
    }

    #[test]
    fn a_wide_band_seats_the_roster_in_the_wing_and_the_log_at_its_first_column() {
        let wide = Layout::for_width(2560);
        let wing = wide.wing.expect("2560 is wide");
        let members = [member("Ann"), member("Bob")];
        let message = Message {
            text: "hello".to_owned(),
            alert: false,
        };
        let long = vec!["X".repeat(400)];
        let mut frame = Frame::default();
        frame.reset(wide.width, wide.canvas().h);
        band(
            &mut frame,
            &wide,
            &view(&members, Some(1), Some(0), &long, &message),
        );
        for slot in 0..2 {
            let w = frame.widget(WidgetId::Member(slot)).expect("a row");
            assert!(wing.encloses(w.rect), "{:?} leaves the wing", w.rect);
            assert!(!wide.band.overlaps(w.rect), "the roster is out of the band");
        }
        let (x, y) = wide.slot_origin(0).unwrap();
        assert_eq!(rgb(&frame, x - 4, y + 3), Some(HI), "the acting marker");
        assert_eq!(
            rgb(&frame, x + 5 * CELL.0 + 2, y + 3),
            Some(FRAME),
            "the bar"
        );
        let (x, y) = wide.band_cell(0, CAPTION_ROW);
        assert_eq!(
            rgb(&frame, x, y),
            Some(DIM),
            "EVENTS at the band's first column"
        );
        let ink = |x, y| frame.raster.get(x, y).is_some_and(|p| p[3] != 0);
        let (nx, ny) = wide.band_cell(LOG_COLUMN, CAPTION_ROW);
        let old_caption = (0..6 * CELL.0).any(|dx| ink(nx + dx, ny));
        assert!(!old_caption, "nothing at the narrow caption column");
        let (rx, ry) = wide.band_cell(LOG_COLUMN - 1, CAPTION_ROW);
        let rule = (0..(HELP_ROW - CAPTION_ROW) * CELL.1)
            .any(|dy| rgb(&frame, rx + 2, ry + dy) == Some(FRAME));
        assert!(!rule, "no rule on a wide band");
        let cells = wide.log_cells();
        let (x, y) = wide.band_cell(0, FIRST_MEMBER_ROW + LOG_ROWS as i32 - 1);
        assert_eq!(rgb(&frame, x + (cells as i32 - 1) * CELL.0, y), Some(TEXT));
        assert!(!ink(x + cells as i32 * CELL.0, y), "cut at the log's width");
        assert!(x + cells as i32 * CELL.0 <= wide.band.right());
        let (x, y) = wide.band_cell(0, MESSAGE_ROW);
        assert_eq!(
            rgb(&frame, x, y),
            Some(TEXT),
            "the message at the band's first column"
        );
    }
}
