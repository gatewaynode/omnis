//! Widgets on the canvas: what the mouse can hit, how a click turns into the keys the menu
//! models already understand, and the composition of a whole frame from the models. Bevy-free.

use crate::layout::{PAD_BUTTONS, Rect, VIEWPORT};
use crate::menu::{Catalog, CreationForm, MenuKey, NewGameForm, Pause, ROW_SKILLS, Title};
use crate::panels::{self, Hud, MemberRow, Message};
use crate::raster::{Raster, Rgb};
use crate::screens;
use omnis_sim::omnis_core::{Direction, Rotation};
use omnis_sim::{Command, Settings};

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

/// The menu model a click lands on.
pub enum Target<'a> {
    /// The title screen.
    Title(&'a mut Title),
    /// The new game form.
    NewGame(&'a mut NewGameForm),
    /// The creation form.
    Creation(&'a mut CreationForm),
    /// The pause overlay.
    Pause(&'a mut Pause),
}

/// Turn a click into keys for the model: move its cursor to the clicked row, then the key
/// the part stands for. A text field click only takes the focus.
#[must_use]
pub fn click(target: Target<'_>, hit: Hit) -> Vec<MenuKey> {
    let row = match hit.id {
        WidgetId::Row(row) => row,
        WidgetId::Skill(index) => {
            if let Target::Creation(form) = target {
                form.cursor = ROW_SKILLS;
                form.skill_cursor = index;
                return vec![MenuKey::Enter];
            }
            return Vec::new();
        }
        WidgetId::Pad(_) | WidgetId::Member(_) => return Vec::new(),
    };
    match target {
        Target::Title(title) => title.cursor = row,
        Target::NewGame(form) => form.cursor = row,
        Target::Creation(form) => form.cursor = row,
        Target::Pause(pause) => pause.cursor = row,
    }
    match (hit.kind, hit.part) {
        (Kind::Choice, Part::Left) => vec![MenuKey::Left],
        (Kind::Choice, Part::Right) => vec![MenuKey::Right],
        (Kind::Button | Kind::Toggle, _) => vec![MenuKey::Enter],
        (Kind::Choice | Kind::TextField, Part::Body) => Vec::new(),
        (Kind::TextField, _) => Vec::new(),
    }
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

/// Which menu is up.
pub enum Menu<'a> {
    /// None: the world shows through.
    None,
    /// The title.
    Title(&'a Title),
    /// The new game form.
    NewGame(&'a NewGameForm),
    /// The creation form, with what it chooses from and how many members exist.
    Creation {
        /// The form.
        form: &'a CreationForm,
        /// The catalog.
        catalog: &'a Catalog,
        /// Members so far.
        members: usize,
    },
    /// The pause overlay with the settings it shows.
    Pause {
        /// The overlay.
        pause: &'a Pause,
        /// The game's settings.
        settings: Settings,
        /// The world seed.
        seed: u64,
    },
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

/// Everything a frame is composed from.
pub struct View<'a> {
    /// The menu, if any.
    pub menu: Menu<'a>,
    /// The location lines, when a world exists.
    pub hud: Option<&'a Hud>,
    /// The party, in marching order.
    pub members: &'a [MemberRow],
    /// How many members stand in front.
    pub front_row: usize,
    /// The member the mouse selected.
    pub selected: Option<usize>,
    /// Whether the band shows classes (creation) instead of points.
    pub creating: bool,
    /// The pad.
    pub pad: PadState,
    /// The message line.
    pub message: &'a Message,
    /// The help line.
    pub help: &'a str,
}

/// Compose a frame; `hover` outlines a widget and `pressed` shows a pad button held down.
#[must_use]
pub fn compose(view: &View<'_>, hover: Option<WidgetId>, pressed: Option<WidgetId>) -> Frame {
    let mut frame = Frame::default();
    panels::backdrop(&mut frame);
    match &view.menu {
        Menu::None => {}
        Menu::Title(title) => {
            frame.raster.fill(VIEWPORT, PANEL);
            screens::title(&mut frame, title);
        }
        Menu::NewGame(form) => {
            frame.raster.fill(VIEWPORT, PANEL);
            screens::new_game(&mut frame, form);
        }
        Menu::Creation {
            form,
            catalog,
            members,
        } => {
            frame.raster.fill(VIEWPORT, PANEL);
            screens::creation(&mut frame, form, catalog, *members);
        }
        Menu::Pause {
            pause,
            settings,
            seed,
        } => {
            frame.raster.fill(VIEWPORT, PANEL);
            screens::pause(&mut frame, pause, *settings, *seed);
        }
    }
    if let Some(hud) = view.hud {
        panels::hud(&mut frame, hud);
    }
    panels::pad(&mut frame, view.pad, pressed);
    panels::band(&mut frame, view);
    if let Some(id) = hover {
        frame.outline(id);
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::Catalog;
    use omnis_sim::omnis_data::load_packs;
    use std::path::PathBuf;

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
    fn clicks_move_the_cursor_and_press_the_matching_key() {
        let mut title = Title::default();
        let button = Hit {
            id: WidgetId::Row(2),
            part: Part::Body,
            kind: Kind::Button,
        };
        assert_eq!(
            click(Target::Title(&mut title), button),
            vec![MenuKey::Enter]
        );
        assert_eq!(title.cursor, 2);
        let mut form = NewGameForm::default();
        let arrow = Hit {
            id: WidgetId::Row(1),
            part: Part::Right,
            kind: Kind::Choice,
        };
        assert_eq!(
            click(Target::NewGame(&mut form), arrow),
            vec![MenuKey::Right]
        );
        assert_eq!(form.cursor, 1);
        let body = Hit {
            part: Part::Body,
            ..arrow
        };
        assert_eq!(click(Target::NewGame(&mut form), body), vec![]);
        let field = Hit {
            id: WidgetId::Row(0),
            part: Part::Body,
            kind: Kind::TextField,
        };
        let mut creation = CreationForm {
            cursor: 5,
            ..CreationForm::default()
        };
        assert_eq!(click(Target::Creation(&mut creation), field), vec![]);
        assert_eq!(creation.cursor, 0);
        let skill = Hit {
            id: WidgetId::Skill(3),
            part: Part::Body,
            kind: Kind::Toggle,
        };
        assert_eq!(
            click(Target::Creation(&mut creation), skill),
            vec![MenuKey::Enter]
        );
        assert_eq!((creation.cursor, creation.skill_cursor), (ROW_SKILLS, 3));
        let mut pause = Pause::default();
        assert_eq!(click(Target::Pause(&mut pause), skill), vec![]);
        let pad = Hit {
            id: WidgetId::Pad(PadButton::Use),
            part: Part::Body,
            kind: Kind::Button,
        };
        assert_eq!(click(Target::Pause(&mut pause), pad), vec![]);
        assert_eq!(pause.cursor, 0);
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

    /// `OMNIS_DUMP_SCREENS=<dir> cargo test -p omnis-app --lib dump_screens -- --ignored`
    /// writes every screen as a PPM for a look without a window (`sips -s format png` converts).
    #[test]
    #[ignore = "writes files; run by hand with OMNIS_DUMP_SCREENS set"]
    fn dump_screens() {
        let Ok(dir) = std::env::var("OMNIS_DUMP_SCREENS") else {
            return;
        };
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = load_packs(&[&repo.join("packs/base")]).unwrap_or_else(|r| panic!("{r}"));
        let catalog = Catalog::from_data(&data);
        let mut form = CreationForm::new(&catalog);
        form.name = "Oswin".into();
        form.cursor = crate::menu::ROW_ADD;
        form.skills = vec![omnis_sim::omnis_data::Skill::Religion];
        form.message = "This class picks 2 skills".into();
        let members = [
            ("Brenna", "Fighter", 12, 12, 0),
            ("Wren", "Cleric", 9, 9, 2),
            ("Ilvara", "Wizard", 2, 7, 4),
            ("Tam", "Rogue", 9, 9, 0),
        ]
        .map(|(name, class, hp, hp_max, sp)| MemberRow {
            name: name.into(),
            class: class.into(),
            hp,
            hp_max,
            sp,
            condition: None,
        });
        let hud = Hud::new("Test Dungeon", 3, 4, "south", 208);
        let message = Message {
            text: "The door opens.".into(),
            alert: false,
        };
        let creation_message = Message {
            text: form.message.clone(),
            alert: true,
        };
        let title = Title::default();
        let new_game = NewGameForm::default();
        let pause = Pause::default();
        let screens = [
            (
                "title",
                Menu::Title(&title),
                None,
                PadState::Hidden,
                &message,
                false,
            ),
            (
                "new_game",
                Menu::NewGame(&new_game),
                None,
                PadState::Hidden,
                &message,
                false,
            ),
            (
                "creation",
                Menu::Creation {
                    form: &form,
                    catalog: &catalog,
                    members: 4,
                },
                Some(&hud),
                PadState::Disabled,
                &creation_message,
                true,
            ),
            (
                "pause",
                Menu::Pause {
                    pause: &pause,
                    settings: Settings::default(),
                    seed: 42,
                },
                Some(&hud),
                PadState::Disabled,
                &message,
                false,
            ),
            (
                "explore",
                Menu::None,
                Some(&hud),
                PadState::Enabled,
                &message,
                false,
            ),
        ];
        for (name, menu, hud, pad, message, creating) in screens {
            let view = View {
                menu,
                hud,
                members: &members,
                front_row: 3,
                selected: Some(1),
                creating,
                pad,
                message,
                help: "Arrows or click  Enter ok  Esc back",
            };
            let frame = compose(
                &view,
                Some(WidgetId::Row(1)),
                Some(WidgetId::Pad(PadButton::Use)),
            );
            let mut ppm =
                format!("P6 {} {} 255\n", frame.raster.width, frame.raster.height).into_bytes();
            for px in frame.raster.rgba.chunks(4) {
                // Composite over the panel colour, as the canvas would.
                let a = u32::from(px[3]);
                for (c, panel) in px[..3].iter().zip([PANEL.0, PANEL.1, PANEL.2]) {
                    ppm.push(((u32::from(*c) * a + u32::from(panel) * (255 - a)) / 255) as u8);
                }
            }
            std::fs::write(format!("{dir}/{name}.ppm"), ppm).unwrap();
        }
    }
}
