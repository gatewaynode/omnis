//! Screens as descriptions (presentation-ARCHITECTURE §6, VA10). Every screen outside the 3D view
//! and the fight's line drawing is plain data built here from the views, the packs' labels and
//! the screen's own state; one shell renderer draws it (`shell/screen.rs`) and one router turns a
//! press or a key into a [`Payload`] for [`apply`], which answers with an [`Effect`] the shell
//! carries out. Bevy-free, so every screen is tested without a window.
//!
//! A control is enabled, or says why not: [`Control::why_not`]. [`apply`] refuses a control that is
//! not on the screen, one that is dim, and a payload its kind does not take, before any screen
//! sees it, so the renderer and a test reach the same answer.

pub mod actions;
pub mod defeat;
pub mod nav;
pub mod service;

use crate::sim::{SimWorld, Views};
use crate::style::{Role, Size};
use core::hash::{Hash, Hasher};
use omnis_sim::Command;
use omnis_sim::omnis_data::Data;

/// Which screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScreenId {
    /// The square's actions (Space).
    Actions,
    /// Inside a town service (the stopgap until P4).
    Service,
    /// The party has fallen.
    Defeat,
}

/// A control, the same on every screen that has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlId {
    /// The square's nth action.
    Action(u8),
    /// Close the overlay.
    Close,
    /// Leave the service.
    Leave,
    /// Start the same game again.
    StartAgain,
}

/// A key a screen offers as a shortcut for one of its controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// A number key, 1 to 9.
    Digit(u8),
}

/// What a piece of text is: its words, what it means, and its size.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    /// The words.
    pub text: String,
    /// What it means, and so its colour.
    pub role: Role,
    /// Its size on the type scale.
    pub size: Size,
}

impl Span {
    /// Body text in the interface role.
    #[must_use]
    pub fn body(text: impl Into<String>) -> Span {
        Span {
            text: text.into(),
            role: Role::Interface,
            size: Size::Body,
        }
    }

    /// A heading.
    #[must_use]
    pub fn title(text: impl Into<String>) -> Span {
        Span {
            text: text.into(),
            role: Role::Title,
            size: Size::Title,
        }
    }

    /// A refusal or failure.
    #[must_use]
    pub fn alert(text: impl Into<String>) -> Span {
        Span {
            text: text.into(),
            role: Role::Alert,
            size: Size::Body,
        }
    }
}

/// How a control is used.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Pressed.
    Button,
}

/// A control: what it is, what it says, and why it cannot be used now, if it cannot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Control {
    /// Which control.
    pub id: ControlId,
    /// Its label.
    pub label: String,
    /// How it is used.
    pub kind: Kind,
    /// Why it cannot be used now; it is drawn dim with this reason and takes no input.
    pub why_not: Option<String>,
}

impl Control {
    /// A live button.
    #[must_use]
    pub fn button(id: ControlId, label: impl Into<String>) -> Control {
        Control {
            id,
            label: label.into(),
            kind: Kind::Button,
            why_not: None,
        }
    }
}

/// One line of a panel: text, then controls, side by side.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Row {
    /// The text.
    pub spans: Vec<Span>,
    /// The controls.
    pub controls: Vec<Control>,
}

impl Row {
    /// A row of one span.
    #[must_use]
    pub fn text(span: Span) -> Row {
        Row {
            spans: vec![span],
            controls: Vec::new(),
        }
    }

    /// A row of controls.
    #[must_use]
    pub fn controls(controls: Vec<Control>) -> Row {
        Row {
            spans: Vec::new(),
            controls,
        }
    }
}

/// A block of rows.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Panel {
    /// Top to bottom.
    pub rows: Vec<Row>,
}

/// A screen: everything the renderer draws and the router may route to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Screen {
    /// Which screen.
    pub id: ScreenId,
    /// The heading.
    pub title: Span,
    /// The panels, top to bottom.
    pub panels: Vec<Panel>,
    /// Shortcuts: each names a control on this screen.
    pub keys: Vec<(Key, ControlId)>,
}

impl Screen {
    /// Every control, in screen order.
    pub fn controls(&self) -> impl Iterator<Item = &Control> {
        self.panels
            .iter()
            .flat_map(|p| &p.rows)
            .flat_map(|r| &r.controls)
    }

    /// The control `id`, if the screen has it.
    #[must_use]
    pub fn control(&self, id: ControlId) -> Option<&Control> {
        self.controls().find(|c| c.id == id)
    }

    /// The control a key stands for, if the screen has that key.
    #[must_use]
    pub fn key(&self, key: Key) -> Option<ControlId> {
        self.keys.iter().find(|(k, _)| *k == key).map(|(_, c)| *c)
    }

    /// A hash of the screen's shape: its panels, rows, spans' count and controls' ids, kinds and
    /// whether they are live, not their words. The renderer rebuilds its nodes only when the
    /// shape changes, and rewrites the words in place otherwise.
    #[must_use]
    pub fn shape(&self) -> u64 {
        let mut h = Fnv(0xcbf2_9ce4_8422_2325);
        self.id.hash(&mut h);
        for panel in &self.panels {
            panel.rows.len().hash(&mut h);
            for row in &panel.rows {
                row.spans.len().hash(&mut h);
                for c in &row.controls {
                    (c.id, core::mem::discriminant(&c.kind), c.why_not.is_some()).hash(&mut h);
                }
            }
        }
        h.0
    }
}

/// A deterministic hasher for [`Screen::shape`] (std's is seeded per process).
struct Fnv(u64);

impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
        }
    }
}

/// What a control reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {
    /// Pressed, or its key.
    Press,
}

/// The client's own work a screen asks for: what is not a command to the simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostOp {
    /// Start the same game again.
    Restart,
}

/// What the shell does after a control was used.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Nothing more.
    None,
    /// Send a command to the simulation, through the binder, so it is logged; then close the
    /// screen when `close` (the square's actions close, an inventory stays open).
    Command {
        /// The command.
        command: Command,
        /// Close the screen afterwards.
        close: bool,
    },
    /// The client's own work.
    Host(HostOp),
    /// Close the top screen.
    Back,
}

/// Why [`apply`] did nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// The screen has no such control.
    NotHere,
    /// The control is dim; its reason.
    Dim(String),
    /// The control does not take that payload.
    WrongPayload,
}

/// The game a screen looks at, when one is running.
#[derive(Clone, Copy)]
pub struct Game<'a> {
    /// The world, for trials.
    pub world: &'a SimWorld,
    /// The views, as of the last change.
    pub views: &'a Views,
}

/// What every screen may read.
#[derive(Clone, Copy)]
pub struct Ctx<'a> {
    /// The packs, for labels and catalogues only (VA9).
    pub data: &'a Data,
    /// The running game.
    pub game: Game<'a>,
}

/// The screen `id` as it stands.
#[must_use]
pub fn content(id: ScreenId, ctx: &Ctx) -> Screen {
    match id {
        ScreenId::Actions => actions::content(ctx),
        ScreenId::Service => service::content(ctx),
        ScreenId::Defeat => defeat::content(),
    }
}

/// Use control `control` of screen `id` with `payload`.
///
/// # Errors
/// [`Refused`] when the screen has no such control, the control is dim, or it does not take the
/// payload; nothing is done.
pub fn apply(
    id: ScreenId,
    ctx: &Ctx,
    control: ControlId,
    payload: &Payload,
) -> Result<Effect, Refused> {
    let screen = content(id, ctx);
    let found = screen.control(control).ok_or(Refused::NotHere)?;
    if let Some(why) = &found.why_not {
        return Err(Refused::Dim(why.clone()));
    }
    match (&found.kind, payload) {
        (Kind::Button, Payload::Press) => {}
    }
    Ok(match id {
        ScreenId::Actions => actions::apply(ctx, control),
        ScreenId::Service => service::apply(control),
        ScreenId::Defeat => defeat::apply(control),
    })
}
