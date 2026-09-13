//! The whole frame: how a click turns into the keys the menu models already understand, and
//! the composition of the menus, the location lines, the pad, and the band into one frame.
//! Bevy-free.

use crate::layout::VIEWPORT;
use crate::menu::{Catalog, CreationForm, MenuKey, NewGameForm, Pause, ROW_SKILLS, Title};
use crate::panels::{self, Band, Hud, MemberRow, Message};
use crate::screens;
use crate::widget::{Frame, Hit, Kind, PANEL, PadState, Part, WidgetId};
use omnis_sim::Settings;

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
    if !matches!(view.menu, Menu::None) {
        frame.raster.fill(VIEWPORT, PANEL);
    }
    match &view.menu {
        Menu::None => {}
        Menu::Title(title) => screens::title(&mut frame, title),
        Menu::NewGame(form) => screens::new_game(&mut frame, form),
        Menu::Creation {
            form,
            catalog,
            members,
        } => screens::creation(&mut frame, form, catalog, *members),
        Menu::Pause {
            pause,
            settings,
            seed,
        } => screens::pause(&mut frame, pause, *settings, *seed),
    }
    if let Some(hud) = view.hud {
        panels::hud(&mut frame, hud);
    }
    panels::pad(&mut frame, view.pad, pressed);
    panels::band(
        &mut frame,
        &Band {
            message: view.message,
            members: view.members,
            front_row: view.front_row,
            selected: view.selected,
            creating: view.creating,
            help: view.help,
        },
    );
    if let Some(id) = hover {
        frame.outline(id);
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::PadButton;
    use omnis_sim::omnis_data::load_packs;
    use std::path::PathBuf;

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

    fn sample_members() -> [MemberRow; 4] {
        [
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
        })
    }

    /// The frame as a binary PPM, composited over the panel colour as the canvas would.
    fn ppm(frame: &Frame) -> Vec<u8> {
        let mut out =
            format!("P6 {} {} 255\n", frame.raster.width, frame.raster.height).into_bytes();
        for px in frame.raster.rgba.chunks(4) {
            let a = u32::from(px[3]);
            for (c, panel) in px[..3].iter().zip([PANEL.0, PANEL.1, PANEL.2]) {
                out.push(((u32::from(*c) * a + u32::from(panel) * (255 - a)) / 255) as u8);
            }
        }
        out
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
        let members = sample_members();
        let hud = Hud::new("Test Dungeon", 3, 4, "south", 208);
        let event = Message {
            text: "The door opens.".into(),
            alert: false,
        };
        let rejection = Message {
            text: form.message.clone(),
            alert: true,
        };
        let (title, new_game, pause) = (Title::default(), NewGameForm::default(), Pause::default());
        let creation = Menu::Creation {
            form: &form,
            catalog: &catalog,
            members: 4,
        };
        let paused = Menu::Pause {
            pause: &pause,
            settings: Settings::default(),
            seed: 42,
        };
        let screens = [
            ("title", Menu::Title(&title), None, PadState::Hidden, &event),
            (
                "new_game",
                Menu::NewGame(&new_game),
                None,
                PadState::Hidden,
                &event,
            ),
            (
                "creation",
                creation,
                Some(&hud),
                PadState::Disabled,
                &rejection,
            ),
            ("pause", paused, Some(&hud), PadState::Disabled, &event),
            ("explore", Menu::None, Some(&hud), PadState::Enabled, &event),
        ];
        for (name, menu, hud, pad, message) in screens {
            let view = View {
                menu,
                hud,
                members: &members,
                front_row: 3,
                selected: Some(1),
                creating: name == "creation",
                pad,
                message,
                help: "Arrows or click  Enter ok  Esc back",
            };
            let frame = compose(
                &view,
                Some(WidgetId::Row(1)),
                Some(WidgetId::Pad(PadButton::Use)),
            );
            std::fs::write(format!("{dir}/{name}.ppm"), ppm(&frame)).unwrap();
        }
    }
}
