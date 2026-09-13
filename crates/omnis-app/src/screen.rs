//! The whole frame: how a click turns into the keys the menu models already understand, and
//! the composition of the menus, the location lines, the pad, and the band into one frame.
//! Bevy-free.

use crate::combat_menu::{CombatMenu, DefeatMenu, EncounterMenu, FightView};
use crate::combat_screen;
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
    /// The choice before a fight.
    Encounter(&'a mut EncounterMenu),
    /// The fight.
    Combat(&'a mut CombatMenu),
    /// The modal after a wipe.
    Defeat(&'a mut DefeatMenu),
}

/// Turn a click into keys for the model: move its cursor to the clicked row, then the key
/// the part stands for. A text field click only takes the focus; a stack row click only
/// picks the target.
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
        WidgetId::Stack(index) => {
            if let Target::Combat(menu) = target {
                menu.target = u8::try_from(index).unwrap_or(u8::MAX);
            }
            return Vec::new();
        }
        WidgetId::Action(index) => {
            match target {
                Target::Combat(menu) => menu.cursor = index,
                Target::Encounter(menu) => menu.cursor = index,
                _ => return Vec::new(),
            }
            return vec![MenuKey::Enter];
        }
        WidgetId::Pad(_) | WidgetId::Member(_) => return Vec::new(),
    };
    match target {
        Target::Title(title) => title.cursor = row,
        Target::NewGame(form) => form.cursor = row,
        Target::Creation(form) => form.cursor = row,
        Target::Pause(pause) => pause.cursor = row,
        Target::Defeat(menu) => menu.cursor = row,
        Target::Encounter(_) | Target::Combat(_) => return Vec::new(),
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
    /// The choice before a fight, over the scene.
    Encounter {
        /// The menu.
        menu: &'a EncounterMenu,
        /// The view.
        view: &'a FightView,
    },
    /// The fight, over the scene, with the roll log.
    Combat {
        /// The menu.
        menu: &'a CombatMenu,
        /// The view.
        view: &'a FightView,
        /// The roll log, oldest first.
        log: &'a [String],
    },
    /// The modal after a wipe, over the scene, with the roll log's tail.
    Defeat {
        /// The menu.
        menu: &'a DefeatMenu,
        /// The roll log, oldest first.
        log: &'a [String],
    },
}

impl Menu<'_> {
    /// Whether the menu hides the whole viewport; the fight screens and the defeat modal
    /// leave the scene visible.
    #[must_use]
    pub const fn covers_viewport(&self) -> bool {
        !matches!(
            self,
            Menu::None | Menu::Encounter { .. } | Menu::Combat { .. } | Menu::Defeat { .. }
        )
    }
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
    if view.menu.covers_viewport() {
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
        Menu::Encounter { menu, view } => combat_screen::encounter(&mut frame, view, menu),
        Menu::Combat { menu, view, log } => combat_screen::combat(&mut frame, view, menu, log),
        Menu::Defeat { menu, log } => combat_screen::defeat(&mut frame, menu, log),
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

    #[test]
    fn fight_clicks_pick_targets_press_actions_and_defeat_rows() {
        let stack = Hit {
            id: WidgetId::Stack(2),
            part: Part::Body,
            kind: Kind::Choice,
        };
        let action = Hit {
            id: WidgetId::Action(3),
            part: Part::Body,
            kind: Kind::Button,
        };
        let mut combat = CombatMenu::default();
        assert_eq!(click(Target::Combat(&mut combat), stack), vec![]);
        assert_eq!(combat.target, 2, "a stack click only picks the target");
        assert_eq!(
            click(Target::Combat(&mut combat), action),
            vec![MenuKey::Enter]
        );
        assert_eq!(combat.cursor, 3);
        let mut encounter = EncounterMenu::default();
        assert_eq!(click(Target::Encounter(&mut encounter), stack), vec![]);
        assert_eq!(
            click(Target::Encounter(&mut encounter), action),
            vec![MenuKey::Enter]
        );
        assert_eq!(encounter.cursor, 3);
        let row = Hit {
            id: WidgetId::Row(1),
            part: Part::Body,
            kind: Kind::Button,
        };
        assert_eq!(click(Target::Encounter(&mut encounter), row), vec![]);
        let mut defeat = DefeatMenu::default();
        assert_eq!(
            click(Target::Defeat(&mut defeat), row),
            vec![MenuKey::Enter]
        );
        assert_eq!(defeat.cursor, 1);
        assert_eq!(click(Target::Defeat(&mut defeat), action), vec![]);
        let mut pause = Pause::default();
        assert_eq!(click(Target::Pause(&mut pause), stack), vec![]);
        assert_eq!(click(Target::Pause(&mut pause), action), vec![]);
        assert_eq!(pause.cursor, 0);
    }

    #[test]
    fn the_fight_menus_leave_the_scene_visible() {
        let view = sample_fight();
        let members = sample_members();
        let log = vec!["Combat!".to_owned()];
        let combat = CombatMenu::default();
        let menu = Menu::Combat {
            menu: &combat,
            view: &view,
            log: &log,
        };
        assert!(!menu.covers_viewport());
        assert!(Menu::Title(&Title::default()).covers_viewport());
        let frame = compose(
            &View {
                menu,
                hud: None,
                members: &members,
                front_row: 3,
                selected: None,
                creating: false,
                pad: PadState::Disabled,
                message: &Message::default(),
                help: "",
            },
            None,
            None,
        );
        let mid = VIEWPORT.w as i32 / 2;
        let top = crate::combat_screen::TOP_PANEL;
        assert_eq!(
            frame.raster.get(mid, top.bottom() + 8),
            Some([0, 0, 0, 0]),
            "the window"
        );
        assert_eq!(
            frame.raster.get(top.right() - 2, top.y + 4),
            Some([PANEL.0, PANEL.1, PANEL.2, 255])
        );
        assert!(frame.widget(WidgetId::Stack(0)).is_some());
        assert!(frame.widget(WidgetId::Pad(PadButton::Use)).is_some());
    }

    /// Goblins and rats before a party of the sample members.
    fn sample_fight() -> FightView {
        use crate::combat_menu::StackRow;
        FightView {
            phase: omnis_sim::ModeKind::Combat,
            round: 2,
            own: Some(0),
            disposition: omnis_sim::omnis_data::Disposition::Hostile,
            stacks: [
                ("Goblin", 3, 2, true),
                ("Giant Rat", 2, 2, true),
                ("Skeleton", 2, 0, false),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (name, initial, count, alive))| StackRow {
                index: i as u8,
                name: name.to_owned(),
                count,
                initial,
                size: omnis_sim::omnis_data::Size::Small,
                front: alive && i < 2,
                alive,
                blocked: None,
            })
            .collect(),
            bribe: Some(200),
            gold: 60,
        }
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

    fn sample_creation(catalog: &Catalog) -> CreationForm {
        let mut form = CreationForm::new(catalog);
        form.name = "Oswin".into();
        form.cursor = crate::menu::ROW_ADD;
        form.skills = vec![omnis_sim::omnis_data::Skill::Religion];
        form.message = "This class picks 2 skills".into();
        form
    }

    fn sample_log() -> Vec<String> {
        [
            "Round 2",
            "Goblin 2 hits Brenna for 5",
            "Wren misses Goblin 1",
            "Ilvara hits Giant Rat 1 for 3",
        ]
        .map(str::to_owned)
        .to_vec()
    }

    /// Compose one screen with the sample party and write it as `<dir>/<name>.ppm`.
    fn dump(dir: &str, name: &str, menu: Menu<'_>, hud: Option<&Hud>, message: &Message) {
        let members = sample_members();
        let pad = match (&menu, hud) {
            (Menu::None, _) => PadState::Enabled,
            (_, None) => PadState::Hidden,
            (_, Some(_)) => PadState::Disabled,
        };
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
        let form = sample_creation(&catalog);
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
        let fight = sample_fight();
        let encounter = EncounterMenu::default();
        let combat = CombatMenu {
            target: 1,
            ..CombatMenu::default()
        };
        let defeat = DefeatMenu::default();
        let log = sample_log();
        let in_fight = Menu::Combat {
            menu: &combat,
            view: &fight,
            log: &log,
        };
        let before = Menu::Encounter {
            menu: &encounter,
            view: &fight,
        };
        dump(&dir, "title", Menu::Title(&title), None, &event);
        dump(&dir, "new_game", Menu::NewGame(&new_game), None, &event);
        dump(&dir, "creation", creation, Some(&hud), &rejection);
        dump(&dir, "pause", paused, Some(&hud), &event);
        dump(&dir, "explore", Menu::None, Some(&hud), &event);
        dump(&dir, "encounter", before, Some(&hud), &event);
        dump(&dir, "combat", in_fight, Some(&hud), &event);
        let fallen = Menu::Defeat {
            menu: &defeat,
            log: &log,
        };
        dump(&dir, "defeat", fallen, Some(&hud), &event);
    }
}
