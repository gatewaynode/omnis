//! Widgets on the canvas: the palette, what the mouse can hit, and the composed frame the
//! screens and panels paint into. Bevy-free, and below `screens` and `panels` in the module
//! graph so the painters share these types without a cycle.

use crate::layout::{PAD_BUTTONS, Rect};
use crate::raster::{Raster, Rgb};
use omnis_sim::Command;
use omnis_sim::omnis_core::{Direction, Rotation};

/// Panel background.
pub const PANEL: Rgb = crate::layout::PANEL_COLOR;
/// Ordinary text.
pub const TEXT: Rgb = (236, 236, 228);
/// Selected text, hover outlines, pressed buttons.
pub const HI: Rgb = (255, 214, 90);
/// Hints and disabled controls.
pub const DIM: Rgb = (120, 124, 140);
/// Button frames.
pub const FRAME: Rgb = (72, 76, 96);
/// Rejections and low hit points.
pub const ALERT: Rgb = (232, 88, 72);
/// Spell points.
pub const SP: Rgb = (120, 170, 255);

/// A movement pad button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PadButton {
    /// Turn left.
    TurnLeft,
    /// Step forward.
    Forward,
    /// Turn right.
    TurnRight,
    /// Sidestep left.
    StepLeft,
    /// Step back.
    Back,
    /// Sidestep right.
    StepRight,
    /// Interact with the facing edge.
    Use,
}

impl PadButton {
    /// Every button, in the pad's reading order.
    pub const ALL: [PadButton; 7] = [
        PadButton::TurnLeft,
        PadButton::Forward,
        PadButton::TurnRight,
        PadButton::StepLeft,
        PadButton::Back,
        PadButton::StepRight,
        PadButton::Use,
    ];

    /// The command the button sends.
    #[must_use]
    pub fn command(self) -> Command {
        match self {
            PadButton::TurnLeft => Command::Turn(Rotation::Left),
            PadButton::Forward => Command::Step(Direction::Forward),
            PadButton::TurnRight => Command::Turn(Rotation::Right),
            PadButton::StepLeft => Command::Step(Direction::Left),
            PadButton::Back => Command::Step(Direction::Back),
            PadButton::StepRight => Command::Step(Direction::Right),
            PadButton::Use => Command::Interact,
        }
    }

    /// The glyphs on the button.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            PadButton::TurnLeft => "<",
            PadButton::Forward => "^",
            PadButton::TurnRight => ">",
            PadButton::StepLeft => "<-",
            PadButton::Back => "v",
            PadButton::StepRight => "->",
            PadButton::Use => "USE",
        }
    }

    /// Where the button sits.
    #[must_use]
    pub fn rect(self) -> Rect {
        PAD_BUTTONS[self as usize]
    }
}

/// Whether the pad is drawn and live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadState {
    /// No world: nothing drawn.
    Hidden,
    /// A world, but not exploring: drawn dim, inert.
    Disabled,
    /// Exploring.
    Enabled,
}

/// What a widget stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetId {
    /// A row of the active menu, by the model's row index.
    Row(usize),
    /// A skill pick on the creation screen.
    Skill(usize),
    /// A pad button.
    Pad(PadButton),
    /// A party slot in the band.
    Member(usize),
    /// A monster stack row in a fight, by its index in the encounter.
    Stack(usize),
    /// A button on the combat or encounter action row, by the menu's action index.
    Action(usize),
}

/// Which part of a widget was hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The widget itself.
    Body,
    /// The `<` arrow.
    Left,
    /// The `>` arrow.
    Right,
}

/// How a widget answers a click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Activates: Enter.
    Button,
    /// Cycles by its arrows: Left or Right.
    Choice,
    /// Takes the keyboard focus only.
    TextField,
    /// Toggles: Enter.
    Toggle,
}

/// A hit region on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Widget {
    /// What it stands for.
    pub id: WidgetId,
    /// Where it is.
    pub rect: Rect,
    /// How it answers a click.
    pub kind: Kind,
    /// Whether it reacts at all.
    pub enabled: bool,
    /// Whether it draws its own frame, so the hover outline replaces it instead of wrapping.
    pub framed: bool,
    /// The `<` arrow, for a choice.
    pub left: Option<Rect>,
    /// The `>` arrow, for a choice.
    pub right: Option<Rect>,
}

impl Widget {
    /// An enabled, unframed widget without arrows.
    #[must_use]
    pub const fn new(id: WidgetId, rect: Rect, kind: Kind) -> Widget {
        Widget {
            id,
            rect,
            kind,
            enabled: true,
            framed: false,
            left: None,
            right: None,
        }
    }
}

/// A hit: the widget under the pointer and which part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    /// The widget.
    pub id: WidgetId,
    /// The part.
    pub part: Part,
    /// How the widget answers.
    pub kind: Kind,
}

/// The topmost enabled widget under a canvas pixel.
#[must_use]
pub fn hit(widgets: &[Widget], x: i32, y: i32) -> Option<Hit> {
    let widget = widgets
        .iter()
        .rev()
        .find(|w| w.enabled && w.rect.contains(x, y))?;
    let part = if widget.left.is_some_and(|r| r.contains(x, y)) {
        Part::Left
    } else if widget.right.is_some_and(|r| r.contains(x, y)) {
        Part::Right
    } else {
        Part::Body
    };
    Some(Hit {
        id: widget.id,
        part,
        kind: widget.kind,
    })
}

/// One composed frame: the pixels and the widgets that were painted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Frame {
    /// The pixels.
    pub raster: Raster,
    /// The hit regions, in paint order.
    pub widgets: Vec<Widget>,
}

impl Frame {
    /// The widget with this id, if painted.
    #[must_use]
    pub fn widget(&self, id: WidgetId) -> Option<&Widget> {
        self.widgets.iter().find(|w| w.id == id)
    }

    /// Outline the hovered widget.
    pub fn outline(&mut self, id: WidgetId) {
        let Some(w) = self.widget(id).copied() else {
            return;
        };
        if !w.enabled {
            return;
        }
        let rect = if w.framed {
            w.rect
        } else {
            Rect::new(w.rect.x - 1, w.rect.y - 1, w.rect.w + 2, w.rect.h + 2)
        };
        self.raster.stroke(rect, HI);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choice(row: usize, x: i32) -> Widget {
        let mut w = Widget::new(WidgetId::Row(row), Rect::new(x, 16, 100, 8), Kind::Choice);
        w.left = Some(Rect::new(x + 12, 16, 6, 8));
        w.right = Some(Rect::new(x + 40, 16, 6, 8));
        w
    }

    #[test]
    fn hits_find_the_topmost_enabled_widget_and_its_part() {
        let mut disabled = Widget::new(WidgetId::Row(9), Rect::new(0, 0, 50, 50), Kind::Button);
        disabled.enabled = false;
        let widgets = [
            Widget::new(WidgetId::Row(0), Rect::new(0, 0, 50, 50), Kind::Button),
            choice(1, 7),
            disabled,
        ];
        assert_eq!(
            hit(&widgets, 20, 17).map(|h| (h.id, h.part)),
            Some((WidgetId::Row(1), Part::Left))
        );
        assert_eq!(hit(&widgets, 50, 17).map(|h| h.part), Some(Part::Right));
        assert_eq!(hit(&widgets, 30, 17).map(|h| h.part), Some(Part::Body));
        assert_eq!(hit(&widgets, 3, 3).map(|h| h.id), Some(WidgetId::Row(0)));
        assert_eq!(hit(&widgets, 200, 200), None);
    }

    #[test]
    fn pad_buttons_map_to_the_key_commands() {
        assert_eq!(
            PadButton::Forward.command(),
            Command::Step(Direction::Forward)
        );
        assert_eq!(PadButton::TurnLeft.command(), Command::Turn(Rotation::Left));
        assert_eq!(PadButton::Use.command(), Command::Interact);
        assert_eq!(PadButton::Use.rect(), PAD_BUTTONS[6]);
        for (i, b) in PadButton::ALL.iter().enumerate() {
            assert_eq!(b.rect(), PAD_BUTTONS[i]);
            assert!(b.label().len() <= 3);
        }
    }

    #[test]
    fn hover_outlines_wrap_text_rows_and_replace_button_frames() {
        let mut frame = Frame::default();
        frame.widgets.push(Widget::new(
            WidgetId::Row(0),
            Rect::new(7, 8, 12, 8),
            Kind::Button,
        ));
        let mut framed = Widget::new(
            WidgetId::Pad(PadButton::Use),
            Rect::new(100, 100, 10, 10),
            Kind::Button,
        );
        framed.framed = true;
        frame.widgets.push(framed);
        frame.outline(WidgetId::Row(0));
        assert_eq!(frame.raster.get(6, 7), Some([HI.0, HI.1, HI.2, 255]));
        assert_eq!(frame.raster.get(7, 8), Some([0, 0, 0, 0]));
        frame.outline(WidgetId::Pad(PadButton::Use));
        assert_eq!(frame.raster.get(100, 100), Some([HI.0, HI.1, HI.2, 255]));
        assert_eq!(frame.raster.get(99, 99), Some([0, 0, 0, 0]));
        frame.outline(WidgetId::Row(5));
    }
}
