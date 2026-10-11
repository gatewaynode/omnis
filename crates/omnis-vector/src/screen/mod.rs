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
pub mod create_party;
pub mod defeat;
pub mod nav;
pub mod new_game;
pub mod pause;
pub mod roster;
pub mod service;
pub mod title;

use crate::sim::{SimWorld, Views};
use crate::style::{Role, Size};
use core::hash::{Hash, Hasher};
use create_party::CreateForm;
use new_game::NewGameForm;
use omnis_sim::omnis_data::Data;
use omnis_sim::{Command, Settings};

/// Which screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScreenId {
    /// The title: no game yet.
    Title,
    /// The new game's seed and settings.
    NewGame,
    /// A new game's party, created member by member.
    CreateParty,
    /// The game paused: save, load, leave.
    Pause,
    /// The square's actions (Space).
    Actions,
    /// Inside a town service.
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
    /// Start a new game: open the form, or start it from the form.
    NewGame,
    /// Start the game the form describes.
    Start,
    /// The seed field.
    Seed,
    /// The save rule.
    Rule,
    /// Permadeath on or off.
    Permadeath,
    /// Back to the screen before.
    Back,
    /// Close the pause.
    Resume,
    /// Save to the quick save.
    Save,
    /// Load the quick save.
    Load,
    /// Write the command log as a replay.
    SaveLog,
    /// End the game and go back to the title.
    QuitToTitle,
    /// Leave the program.
    Quit,
    /// The draft's name.
    Name,
    /// The draft's race.
    Race,
    /// The draft's class.
    Class,
    /// The draft's background.
    Background,
    /// The draft's alignment.
    Alignment,
    /// One bought score, by ability index (SRD order).
    Score(u8),
    /// One skill pick, by index into `Skill::ALL`.
    Skill(u8),
    /// Add the draft to the party.
    AddMember,
    /// The party is made: play.
    Begin,
    /// A member's entry in the party column: selects or clears them.
    Member(omnis_sim::omnis_core::CharacterId),
    /// Open one of the selected member's screens.
    Open(roster::Tool),
    /// A service's offer, by index into its view's offers.
    Offer(u16),
    /// The bank's amount, in whole gold.
    Amount,
    /// Pay the amount into the bank.
    Deposit,
    /// Draw the amount from the bank.
    Withdraw,
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
    /// One of `options`, stepped left and right (a press steps right); `at` is the current one.
    Choice {
        /// The options' words.
        options: Vec<String>,
        /// The current option.
        at: usize,
    },
    /// A whole number from `min` to `max`, stepped left and right (a press steps right) or set
    /// outright (a slider).
    Number {
        /// The number.
        value: i64,
        /// The lowest it may be.
        min: i64,
        /// The highest it may be.
        max: i64,
    },
    /// Typed text, at most `max` bytes.
    Text {
        /// The text.
        value: String,
        /// The most bytes it takes.
        max: usize,
    },
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

/// One line of a panel: text, then controls, then text after them, side by side.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Row {
    /// The text.
    pub spans: Vec<Span>,
    /// The controls.
    pub controls: Vec<Control>,
    /// Text after the controls (a score's bonus and cost).
    pub after: Vec<Span>,
}

impl Row {
    /// A row of one span.
    #[must_use]
    pub fn text(span: Span) -> Row {
        Row {
            spans: vec![span],
            ..Row::default()
        }
    }

    /// Span `i` of the row, counting the spans after the controls on from the ones before.
    #[must_use]
    pub fn span(&self, i: usize) -> Option<&Span> {
        self.spans
            .get(i)
            .or_else(|| self.after.get(i.checked_sub(self.spans.len())?))
    }

    /// A row of controls.
    #[must_use]
    pub fn controls(controls: Vec<Control>) -> Row {
        Row {
            controls,
            ..Row::default()
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
    /// The last outcome to report (saved, failed to load), under the panels.
    pub message: Option<Span>,
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
                row.after.len().hash(&mut h);
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
    /// A choice stepped left.
    Left,
    /// A choice stepped right.
    Right,
    /// A text field's new text.
    Text(String),
    /// A number set outright.
    Set(i64),
}

/// The number a `Kind::Number` holds after `payload`: one step either way, held inside
/// `min..=max`, or the number set. `apply` has refused a set outside the range already.
#[must_use]
pub fn stepped(value: i64, min: i64, max: i64, payload: &Payload) -> i64 {
    match payload {
        Payload::Left => value.saturating_sub(1).max(min),
        Payload::Press | Payload::Right => value.saturating_add(1).min(max),
        Payload::Set(n) => (*n).clamp(min, max),
        Payload::Text(_) => value,
    }
}

/// The client's own work a screen asks for: what is not a command to the simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostOp {
    /// Start the same game again.
    Restart,
    /// Start a new game.
    NewGame {
        /// Its seed.
        seed: u64,
        /// Its settings.
        settings: Settings,
    },
    /// Save to the quick save.
    Save,
    /// Load the quick save.
    Load,
    /// Write the command log as a replay.
    SaveLog,
    /// The party is made: play begins.
    Begin,
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
    /// Open a screen over this one.
    Open(ScreenId),
    /// End the game and go back to the title.
    ToTitle,
    /// End a game not yet begun and reopen the new game's form, as it was left.
    ToNewGame,
    /// Leave the program.
    Quit,
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
    /// The text is longer than the field takes; its limit in bytes.
    TooLong(usize),
    /// The number is outside the control's range.
    OutOfRange,
    /// The field takes only digits.
    NotDigits,
}

/// The game a screen looks at, when one is running.
#[derive(Clone, Copy)]
pub struct Game<'a> {
    /// The world, for trials.
    pub world: &'a SimWorld,
    /// The views, as of the last change.
    pub views: &'a Views,
    /// The current or last fight's roll log.
    pub log: &'a [String],
    /// Why the command log no longer replays, if it does not.
    pub unreplayable: Option<&'a str>,
}

/// The quick save: where it is, and whether there is one to load.
#[derive(Debug, Clone, Copy)]
pub struct Slot<'a> {
    /// Its path, relative to the working directory.
    pub path: &'a str,
    /// Whether the file is there.
    pub exists: bool,
}

impl Slot<'_> {
    /// Why Load cannot be used, when there is nothing to load.
    #[must_use]
    pub fn missing(&self) -> Option<String> {
        (!self.exists).then(|| format!("no save at {}", self.path))
    }
}

/// What every screen may read.
#[derive(Clone, Copy)]
pub struct Ctx<'a> {
    /// The packs, for labels and catalogues only (VA9).
    pub data: &'a Data,
    /// The running game, if one is.
    pub game: Option<Game<'a>>,
    /// The quick save.
    pub save: Slot<'a>,
}

/// What screens keep between frames: the forms being filled, and the last outcome to report.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Forms {
    /// The new game's form; kept across Back, so it reopens as it was left.
    pub new_game: NewGameForm,
    /// The next member's draft.
    pub create: CreateForm,
    /// The last outcome to report (saved, failed to load); cleared when the screen changes.
    pub message: Option<Span>,
    /// The bank's amount in whole gold, the digits as typed; cleared when a service is entered.
    pub bank: String,
    /// The member selected in the party column: who receives a give, a potion or a spell.
    pub selected: Option<omnis_sim::omnis_core::CharacterId>,
    /// The top screen and the base as last shown, for [`Forms::shown`].
    seen: (Option<ScreenId>, Option<ScreenId>),
}

impl Forms {
    /// A command was accepted: a member was created, so the draft starts over for the next.
    pub fn accepted(&mut self, command: &Command) {
        if matches!(command, Command::Party(omnis_sim::PartyCommand::Create(_))) {
            self.create = CreateForm::default();
        }
    }

    /// The screens now shown: a new top screen clears the last outcome, and entering a service
    /// clears the bank's amount.
    pub fn shown(&mut self, top: Option<ScreenId>, base: Option<ScreenId>) {
        if top != self.seen.0 {
            self.message = None;
        }
        if base == Some(ScreenId::Service) && self.seen.1 != base {
            self.bank.clear();
        }
        self.seen = (top, base);
    }
}

/// A screen with nothing on it, for one asked for without the game it needs.
pub(crate) fn empty(id: ScreenId) -> Screen {
    Screen {
        id,
        title: Span::title(""),
        panels: Vec::new(),
        keys: Vec::new(),
        message: None,
    }
}

/// The screen `id` as it stands.
#[must_use]
pub fn content(id: ScreenId, ctx: &Ctx, forms: &Forms) -> Screen {
    let mut screen = match (id, ctx.game) {
        (ScreenId::Title, _) => title::content(ctx),
        (ScreenId::NewGame, _) => new_game::content(&forms.new_game),
        (ScreenId::Pause, Some(game)) => pause::content(ctx, game),
        (ScreenId::Actions, Some(game)) => actions::content(ctx.data, game),
        (ScreenId::Service, Some(game)) => service::content(ctx.data, game, &forms.bank),
        (ScreenId::Defeat, Some(game)) => defeat::content(ctx, game),
        (ScreenId::CreateParty, Some(game)) => create_party::content(ctx.data, game, &forms.create),
        (id, None) => empty(id),
    };
    screen.message.clone_from(&forms.message);
    screen
}

/// Use control `control` of screen `id` with `payload`. `entropy` seeds a game whose seed was
/// left blank; the shell passes the clock's, a test a fixed number.
///
/// # Errors
/// [`Refused`] when the screen has no such control, the control is dim, it does not take the
/// payload, the text is too long, or the number is outside its range, or the screen refuses
/// what was typed; nothing is done.
pub fn apply(
    id: ScreenId,
    ctx: &Ctx,
    forms: &mut Forms,
    control: ControlId,
    payload: &Payload,
    entropy: u64,
) -> Result<Effect, Refused> {
    let screen = content(id, ctx, forms);
    let found = screen.control(control).ok_or(Refused::NotHere)?;
    if let Some(why) = &found.why_not {
        return Err(Refused::Dim(why.clone()));
    }
    match (&found.kind, payload) {
        (Kind::Button, Payload::Press)
        | (Kind::Choice { .. }, Payload::Press | Payload::Left | Payload::Right) => {}
        (Kind::Number { .. }, Payload::Press | Payload::Left | Payload::Right) => {}
        (Kind::Number { min, max, .. }, Payload::Set(n)) => {
            if n < min || n > max {
                return Err(Refused::OutOfRange);
            }
        }
        (Kind::Text { max, .. }, Payload::Text(text)) => {
            if text.len() > *max {
                return Err(Refused::TooLong(*max));
            }
        }
        _ => return Err(Refused::WrongPayload),
    }
    Ok(match (id, ctx.game) {
        (ScreenId::Title, _) => title::apply(control),
        (ScreenId::NewGame, _) => new_game::apply(&mut forms.new_game, control, payload, entropy),
        (ScreenId::Pause, _) => pause::apply(control),
        (ScreenId::Actions, Some(game)) => actions::apply(ctx.data, game, control),
        (ScreenId::Service, Some(game)) => service::apply(game, &mut forms.bank, control, payload)?,
        (ScreenId::Defeat, _) => defeat::apply(control),
        (ScreenId::CreateParty, Some(game)) => {
            create_party::apply(&mut forms.create, game, control, payload)
        }
        (ScreenId::Actions | ScreenId::CreateParty | ScreenId::Service, None) => Effect::None,
    })
}
