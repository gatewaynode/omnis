//! The character sheet headless: P opens it, Left/Right move the member and the band's
//! selection with it, a tab click turns the page, a band click steers the sheet, Escape
//! closes it where the world is, and the pause menu's item and the SHEET button open it.

mod common;

use bevy::input::keyboard::Key;
use bevy::prelude::*;
use common::{click, key, play_state, ui_app_saving_to, widget, world};
use omnis_app::menus::Screens;
use omnis_app::sheet_menu::SheetPage;
use omnis_app::sheet_screen::ROW_MEMBER;
use omnis_app::sim::{PlayState, PlayerCommand};
use omnis_app::ui::Selected;
use omnis_app::widget::{Part, ToolButton, WidgetId};
use omnis_sim::omnis_data::{Alignment, Skill};
use omnis_sim::omnis_rules::Draft;
use omnis_sim::{Command, PartyCommand};

fn send(app: &mut App, command: Command) {
    app.world_mut()
        .resource_mut::<Messages<PlayerCommand>>()
        .write(PlayerCommand(command));
    app.update();
    app.update();
}

fn draft(name: &str, class: &str, skills: [Skill; 2]) -> Draft {
    Draft {
        name: name.to_owned(),
        race: "base:race:human".to_owned(),
        class: format!("base:class:{class}"),
        background: "base:background:acolyte".to_owned(),
        alignment: Alignment::NeutralGood,
        scores: [15, 14, 13, 12, 10, 8],
        skills: skills.to_vec(),
    }
}

fn press(app: &mut App, code: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(code);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
}

fn sheet(app: &App) -> (usize, SheetPage) {
    let menu = &app.world().resource::<Screens>().sheet;
    (menu.member, menu.page)
}

#[test]
fn the_sheet_opens_pages_and_follows_the_band() {
    let mut app = ui_app_saving_to("sheet.ron", true);
    app.update();
    app.update();
    assert!(
        !widget(&app, WidgetId::Tool(ToolButton::Sheet)).enabled,
        "no member yet"
    );
    send(
        &mut app,
        Command::Party(PartyCommand::Create(draft(
            "Brenna",
            "fighter",
            [Skill::Athletics, Skill::Perception],
        ))),
    );
    send(
        &mut app,
        Command::Party(PartyCommand::Create(draft(
            "Wren",
            "wizard",
            [Skill::Arcana, Skill::History],
        ))),
    );
    assert_eq!(world(&app).party.members.len(), 2);
    assert!(widget(&app, WidgetId::Tool(ToolButton::Sheet)).enabled);

    press(&mut app, KeyCode::KeyP);
    assert_eq!(play_state(&app), PlayState::Sheet, "P opens the sheet");
    assert_eq!(sheet(&app), (0, SheetPage::Stats));
    assert_eq!(*app.world().resource::<Selected>(), Selected(Some(0)));
    assert!(widget(&app, WidgetId::Row(ROW_MEMBER)).enabled);

    key(&mut app, Key::ArrowRight);
    assert_eq!(sheet(&app).0, 1, "Right moves to the wizard");
    assert_eq!(
        *app.world().resource::<Selected>(),
        Selected(Some(1)),
        "the band follows"
    );
    click(&mut app, WidgetId::Row(1), Part::Body);
    assert_eq!(
        sheet(&app).1,
        SheetPage::Magic,
        "the tab click turns the page"
    );
    click(&mut app, WidgetId::Row(ROW_MEMBER), Part::Right);
    assert_eq!(sheet(&app).0, 0, "the arrow wraps to the fighter");
    key(&mut app, Key::Tab);
    assert_eq!(sheet(&app).1, SheetPage::Gear);
    click(&mut app, WidgetId::Member(1), Part::Body);
    assert_eq!(sheet(&app).0, 1, "a band click steers the sheet");

    key(&mut app, Key::Escape);
    assert_eq!(play_state(&app), PlayState::Explore, "Escape closes it");
    assert_eq!(
        *app.world().resource::<Selected>(),
        Selected(Some(1)),
        "the selection stays"
    );

    click(&mut app, WidgetId::Tool(ToolButton::Sheet), Part::Body);
    assert_eq!(
        play_state(&app),
        PlayState::Sheet,
        "the SHEET button opens it"
    );
    assert_eq!(sheet(&app).0, 1, "on the selected member");
    key(&mut app, Key::Character("p".into()));
    assert_eq!(play_state(&app), PlayState::Explore, "P closes it too");

    press(&mut app, KeyCode::Escape);
    assert_eq!(play_state(&app), PlayState::Paused);
    click(&mut app, WidgetId::Row(3), Part::Body);
    assert_eq!(
        play_state(&app),
        PlayState::Sheet,
        "the pause menu's item opens it"
    );
    key(&mut app, Key::Escape);
    assert_eq!(play_state(&app), PlayState::Explore);
}
