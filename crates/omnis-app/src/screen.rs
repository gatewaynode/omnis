//! The whole frame: how a click turns into the keys the menu models already understand, and
//! the composition of the menus, the location lines, the pad, and the band into one frame.
//! Bevy-free.

use crate::band::{self, Band, MemberRow};
use crate::canvas::{Layout, NARROW};
use crate::combat_menu::{CombatMenu, DefeatMenu, EncounterMenu, FightView};
use crate::combat_screen;
use crate::debug_menu::{DebugMenu, DebugView};
use crate::debug_screen;
use crate::layout::{CANVAS_HEIGHT, MENU_BOX, VIEWPORT};
use crate::menu::{Catalog, CreationForm, MenuKey, NewGameForm, Pause, ROW_SKILLS, Title};
use crate::panels::{self, Hud, Message};
use crate::screens;
use crate::spell_menu::{CastMenu, CastRow, cast_screen};
use crate::widget::{FRAME, Frame, Hit, Kind, PANEL, PadState, Part, WidgetId};
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
    /// The debug menu.
    Debug(&'a mut DebugMenu),
    /// The cast menu while exploring.
    Cast(&'a mut CastMenu),
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
        WidgetId::Spell(index) => {
            if let Target::Combat(menu) = target
                && menu.picker.is_some()
            {
                menu.picker = Some(index);
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
        Target::Defeat(menu) => menu.cursor = row,
        Target::Debug(menu) => {
            if menu.row != row {
                menu.row = row;
                menu.field = 0;
            }
        }
        Target::Cast(menu) => menu.cursor = row,
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
    /// The fight, over the scene.
    Combat {
        /// The menu.
        menu: &'a CombatMenu,
        /// The view.
        view: &'a FightView,
    },
    /// The modal after a wipe, over the scene, with the roll log's tail.
    Defeat {
        /// The menu.
        menu: &'a DefeatMenu,
        /// The roll log, oldest first.
        log: &'a [String],
    },
    /// The debug menu.
    Debug {
        /// The menu.
        menu: &'a DebugMenu,
        /// What it edits.
        view: &'a DebugView,
    },
    /// The cast menu while exploring.
    Cast {
        /// The menu.
        menu: &'a CastMenu,
        /// The spells it lists.
        rows: &'a [CastRow],
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
    /// The member whose turn it is.
    pub acting: Option<usize>,
    /// The event log, oldest first.
    pub log: &'a [String],
    /// The pad.
    pub pad: PadState,
    /// The message line.
    pub message: &'a Message,
    /// The help line.
    pub help: &'a str,
}

/// Compose a frame on the narrow canvas; `hover` outlines a widget and `pressed` shows a pad
/// button held down.
#[must_use]
pub fn compose(view: &View<'_>, hover: Option<WidgetId>, pressed: Option<WidgetId>) -> Frame {
    compose_in(&NARROW, view, hover, pressed)
}

/// Compose a frame on a canvas of the layout's width.
#[must_use]
pub fn compose_in(
    layout: &Layout,
    view: &View<'_>,
    hover: Option<WidgetId>,
    pressed: Option<WidgetId>,
) -> Frame {
    let mut frame = Frame::default();
    compose_into(&mut frame, layout, view, hover, pressed);
    frame
}

/// Compose into an existing frame, reusing its buffer when the size is unchanged: the
/// backdrop, then the core (the menus or the fight overlay, the location lines, the pad)
/// through its origin, then the band, then the hover outline.
pub fn compose_into(
    frame: &mut Frame,
    layout: &Layout,
    view: &View<'_>,
    hover: Option<WidgetId>,
    pressed: Option<WidgetId>,
) {
    frame.reset(layout.width, CANVAS_HEIGHT);
    panels::backdrop(frame, layout);
    frame.within(layout.core, |frame| core(frame, view, pressed));
    band::band(
        frame,
        layout,
        &Band {
            message: view.message,
            members: view.members,
            front_row: view.front_row,
            selected: view.selected,
            acting: view.acting,
            log: view.log,
            help: view.help,
        },
    );
    if let Some(id) = hover {
        frame.outline(id);
    }
}

/// The core in its own coordinates: the menu box and the menus, or the fight overlay; the
/// location lines; the pad.
fn core(frame: &mut Frame, view: &View<'_>, pressed: Option<WidgetId>) {
    if view.menu.covers_viewport() {
        frame.raster.fill(VIEWPORT, PANEL);
        frame.raster.stroke(MENU_BOX, FRAME);
    }
    match &view.menu {
        Menu::None => {}
        Menu::Title(title) => screens::title(frame, title),
        Menu::NewGame(form) => screens::new_game(frame, form),
        Menu::Creation {
            form,
            catalog,
            members,
        } => screens::creation(frame, form, catalog, *members),
        Menu::Pause {
            pause,
            settings,
            seed,
        } => screens::pause(frame, pause, *settings, *seed),
        Menu::Encounter { menu, view } => combat_screen::encounter(frame, view, menu),
        Menu::Combat { menu, view } => combat_screen::combat(frame, view, menu),
        Menu::Defeat { menu, log } => combat_screen::defeat(frame, menu, log),
        Menu::Debug { menu, view } => debug_screen::debug(frame, menu, view),
        Menu::Cast { menu, rows } => cast_screen(frame, menu, rows),
    }
    if let Some(hud) = view.hud {
        panels::hud(frame, hud);
    }
    panels::pad(frame, view.pad, pressed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{HI, PadButton, hit};
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
    fn the_menu_box_is_framed_over_the_covered_viewport() {
        let view = View {
            menu: Menu::Title(&Title::default()),
            hud: None,
            members: &[],
            front_row: 3,
            selected: None,
            acting: None,
            log: &[],
            pad: PadState::Hidden,
            message: &Message::default(),
            help: "",
        };
        let frame = compose(&view, None, None);
        let rgb = |x, y| frame.raster.get(x, y).map(|p| (p[0], p[1], p[2]));
        assert_eq!(
            rgb(MENU_BOX.x, MENU_BOX.y),
            Some(FRAME),
            "the frame's corner"
        );
        assert_eq!(
            rgb(MENU_BOX.right() - 1, MENU_BOX.bottom() - 1),
            Some(FRAME)
        );
        assert_eq!(
            rgb(MENU_BOX.x - 1, MENU_BOX.y - 1),
            Some(PANEL),
            "panel outside"
        );
        assert_eq!(
            rgb(MENU_BOX.x + 1, MENU_BOX.y + 1),
            Some(PANEL),
            "panel inside"
        );
        let title = frame.widget(WidgetId::Row(0)).unwrap();
        assert!(MENU_BOX.encloses(title.rect), "{:?}", title.rect);
    }

    #[test]
    fn composing_into_a_used_frame_matches_a_fresh_one() {
        let view = View {
            menu: Menu::Title(&Title::default()),
            hud: None,
            members: &[],
            front_row: 3,
            selected: None,
            acting: None,
            log: &[],
            pad: PadState::Hidden,
            message: &Message::default(),
            help: "help",
        };
        let fresh = compose(&view, Some(WidgetId::Row(0)), None);
        let mut reused = compose(&view, Some(WidgetId::Row(1)), None);
        assert_ne!(reused, fresh, "a different hover outlines a different row");
        compose_into(&mut reused, &NARROW, &view, Some(WidgetId::Row(0)), None);
        assert_eq!(reused, fresh, "nothing of the earlier paint survives");
        reused.clear();
        assert!(reused.widgets.is_empty());
        assert!(reused.raster.rgba.iter().all(|b| *b == 0));
        assert_eq!(
            reused.raster.width, fresh.raster.width,
            "the buffer is kept"
        );
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
                acting: Some(0),
                log: &log,
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
            spells: vec![
                crate::combat_menu::SpellRow {
                    index: 0,
                    name: "Fire Bolt".to_owned(),
                    cost: 0,
                    targets_members: false,
                    auto: None,
                    active: false,
                    blocked: None,
                },
                crate::combat_menu::SpellRow {
                    index: 1,
                    name: "Magic Missile".to_owned(),
                    cost: 1,
                    targets_members: false,
                    auto: None,
                    active: false,
                    blocked: Some("need 1 pt".to_owned()),
                },
            ],
            points: (0, 4),
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
            level: 1,
            hp,
            hp_max,
            sp,
            ac: 14,
            condition: (hp < hp_max).then(|| "Wounded".to_owned()),
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

    /// Compose one screen with the sample party and write it as `<dir>/<name>.ppm`, and the
    /// same on a 2560-wide canvas as `<dir>/<name>-wide.ppm`.
    fn dump(dir: &str, name: &str, menu: Menu<'_>, hud: Option<&Hud>, message: &Message) {
        let log = sample_log();
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
            acting: (name == "combat").then_some(0),
            log: if hud.is_some() { &log } else { &[] },
            pad,
            message,
            help: "Arrows or click  Enter ok  Esc back",
        };
        let hover = Some(WidgetId::Row(1));
        let pressed = Some(WidgetId::Pad(PadButton::Use));
        let frame = compose(&view, hover, pressed);
        std::fs::write(format!("{dir}/{name}.ppm"), ppm(&frame)).unwrap();
        let wide = compose_in(&Layout::for_width(2560), &view, hover, pressed);
        std::fs::write(format!("{dir}/{name}-wide.ppm"), ppm(&wide)).unwrap();
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
        let picking = CombatMenu {
            target: 1,
            picker: Some(0),
            ..CombatMenu::default()
        };
        let casting = Menu::Combat {
            menu: &picking,
            view: &fight,
        };
        dump(&dir, "combat_cast", casting, Some(&hud), &event);
        let debug_view = crate::debug_menu::DebugView {
            fighting: true,
            devtools: true,
            members: vec![crate::debug_menu::MemberDebug {
                name: "Brenna".to_owned(),
                hp: (12, 12),
                sp: (0, 0),
                xp: 25,
                scores: [15, 14, 13, 12, 10, 8],
                conditions: vec![],
            }],
            gold: 90,
            food: 60,
            items: vec![("base:item:spyglass".to_owned(), "Spyglass".to_owned())],
            conditions: vec![("base:condition:poisoned".to_owned(), "Poisoned".to_owned())],
            flags: vec![],
            maps: vec![("test:map:dungeon".to_owned(), 24, 24)],
            position: (0, 3, 8, omnis_sim::omnis_core::Facing::South),
            stacks: vec![crate::debug_menu::StackDebug {
                index: 0,
                name: "Goblin".to_owned(),
                count: (3, 3),
                lead_hp: 7,
            }],
        };
        let mut debug_menu = DebugMenu::default();
        debug_menu.open(&debug_view);
        let debugging = Menu::Debug {
            menu: &debug_menu,
            view: &debug_view,
        };
        dump(&dir, "debug", debugging, Some(&hud), &event);
        let fallen = Menu::Defeat {
            menu: &defeat,
            log: &log,
        };
        dump(&dir, "defeat", fallen, Some(&hud), &event);
    }

    /// The sample explore view: no menu, the location lines, the pad enabled.
    fn explore<'a>(
        members: &'a [MemberRow],
        hud: &'a Hud,
        log: &'a [String],
        message: &'a Message,
    ) -> View<'a> {
        View {
            menu: Menu::None,
            hud: Some(hud),
            members,
            front_row: 3,
            selected: Some(1),
            acting: None,
            log,
            pad: PadState::Enabled,
            message,
            help: "help",
        }
    }

    #[test]
    fn a_wide_canvas_shifts_the_core_widgets_by_its_origin_and_seats_the_roster_in_the_wing() {
        let wide = Layout::for_width(2560);
        let wing = wide.wing.expect("wide");
        let members = sample_members();
        let hud = Hud::new("Test Dungeon", 3, 4, "south", 208);
        let log = sample_log();
        let message = Message::default();
        let fight = sample_fight();
        let combat = CombatMenu::default();
        let views = [
            explore(&members, &hud, &log, &message),
            View {
                menu: Menu::Title(&Title::default()),
                ..explore(&members, &hud, &log, &message)
            },
            View {
                menu: Menu::Combat {
                    menu: &combat,
                    view: &fight,
                },
                acting: Some(0),
                ..explore(&members, &hud, &log, &message)
            },
        ];
        for view in &views {
            let narrow = compose(view, None, None);
            let frame = compose_in(&wide, view, None, None);
            assert_eq!(frame.raster.width, 2560);
            assert_eq!(frame.widgets.len(), narrow.widgets.len());
            for (a, b) in narrow.widgets.iter().zip(&frame.widgets) {
                assert_eq!(a.id, b.id);
                if let WidgetId::Member(_) = a.id {
                    assert!(wing.encloses(b.rect), "{:?} leaves the wing", b.rect);
                    continue;
                }
                let (dx, dy) = wide.core;
                assert_eq!(b.rect, a.rect.shifted(dx, dy), "{:?}", a.id);
                assert_eq!(b.left, a.left.map(|r| r.shifted(dx, dy)), "{:?}", a.id);
                assert_eq!(b.right, a.right.map(|r| r.shifted(dx, dy)), "{:?}", a.id);
            }
        }
    }

    #[test]
    fn a_wide_canvas_keeps_the_scene_and_the_minimap_clear_and_frames_the_menu_where_it_sits() {
        let wide = Layout::for_width(2560);
        let members = sample_members();
        let hud = Hud::new("Test Dungeon", 3, 4, "south", 208);
        let log = sample_log();
        let message = Message::default();
        let view = explore(&members, &hud, &log, &message);
        let frame = compose_in(&wide, &view, Some(WidgetId::Member(0)), None);
        let rgba = |x, y| frame.raster.get(x, y);
        let panel = Some([PANEL.0, PANEL.1, PANEL.2, 255]);
        let clear = Some([0, 0, 0, 0]);
        let viewport = wide.viewport();
        assert_eq!(rgba(viewport.x + 480, viewport.y + 270), clear, "the scene");
        assert_eq!(rgba(viewport.x - 1, 270), panel, "the wing beside it");
        assert_eq!(rgba(viewport.right(), 270), panel, "the column beside it");
        let (mx, my, mw, mh) = wide.minimap();
        assert_eq!(
            rgba(mx + mw as i32 / 2, my + mh as i32 / 2),
            clear,
            "the minimap"
        );
        assert_eq!(rgba(mx - 1, my), panel);
        assert_eq!(rgba(10, 300), panel, "the margin");
        assert_eq!(rgba(2559, 719), panel, "the far corner");
        assert_eq!(
            rgba(480, 270),
            panel,
            "the narrow scene's centre is panel now"
        );
        let member = frame.widget(WidgetId::Member(0)).unwrap();
        let hi = Some([HI.0, HI.1, HI.2, 255]);
        assert_eq!(
            rgba(member.rect.x - 1, member.rect.y - 1),
            hi,
            "the outline"
        );
        assert_eq!(hit(&frame.widgets, 7, 568).map(|h| h.id), None);
        let title = View {
            menu: Menu::Title(&Title::default()),
            ..explore(&members, &hud, &log, &message)
        };
        let frame = compose_in(&wide, &title, None, None);
        let boxed = MENU_BOX.shifted(wide.core.0, wide.core.1);
        let rgb = |x, y| frame.raster.get(x, y).map(|p| (p[0], p[1], p[2]));
        assert_eq!(rgb(boxed.x, boxed.y), Some(FRAME));
        assert_eq!(rgb(boxed.right() - 1, boxed.bottom() - 1), Some(FRAME));
        assert_eq!(
            rgb(MENU_BOX.x, MENU_BOX.y),
            Some(PANEL),
            "not at the narrow corner"
        );
        assert!(boxed.encloses(frame.widget(WidgetId::Row(0)).unwrap().rect));
    }
}
