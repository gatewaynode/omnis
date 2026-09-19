//! Headless mouse tests (ARCHITECTURE.md §11, tier 6): the pointer and window messages a
//! window would send, without a window, and the whole menu flow driven by clicks on the
//! composed frame's widgets.

mod common;

use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::{
    CursorLeft, WindowCreated, WindowResized, WindowResolution, WindowScaleFactorChanged,
};
use common::{
    button, click, click_at, draft_fighter_by_mouse, escape, frame, move_to, play_state, point_at,
    pointer, seen, spot, start_new_game_by_mouse, ui_app, widget, world,
};
use omnis_app::canvas::Layout;
use omnis_app::cursor::{Pointer, WindowSize};
use omnis_app::layout::{CANVAS_WIDTH, MENU_BOX, canvas_rect_to_window};
use omnis_app::menu::ROW_BEGIN;
use omnis_app::menus::Screens;
use omnis_app::sim::{AppState, PlayState, ShellCommand, SimWorld};
use omnis_app::ui::{MessageLine, Selected};
use omnis_app::widget::{ALERT, PadButton, Part, ToolButton, WidgetId};
use omnis_sim::SaveRule;
use omnis_sim::omnis_core::Facing;

#[test]
fn cursor_messages_map_through_the_letterbox_and_track_the_button() {
    let mut app = ui_app(false);
    app.update();
    assert_eq!(
        *app.world().resource::<WindowSize>(),
        size(1280.0, 720.0, 1.0)
    );
    let (x, y, _, _) = canvas_rect_to_window((250, 100, 1, 1), (1280.0, 720.0), 1.0);
    move_to(&mut app, x, y);
    app.update();
    assert_eq!(pointer(&app).canvas, Some((250, 100)));

    // A 1920x1200 window letterboxes the 720 rows and widens the canvas to 1920: the
    // corner is in the bar above, and the layout composed is the wide one.
    app.insert_resource(size(1920.0, 1200.0, 1.0));
    move_to(&mut app, 5.0, 5.0);
    app.update();
    assert_eq!(pointer(&app).canvas, None);
    let layout = *app.world().resource::<Layout>();
    assert_eq!(layout, Layout::for_width(1920));
    assert!(layout.is_wide());
    let composed = &frame(&app).frame;
    assert_eq!(composed.raster.width, 1920);
    let title = composed
        .widget(WidgetId::Row(0))
        .expect("the title's first row");
    let boxed = MENU_BOX.shifted(layout.core.0, layout.core.1);
    assert!(
        boxed.encloses(title.rect),
        "{:?} outside {boxed:?}",
        title.rect
    );
    assert!(!MENU_BOX.encloses(title.rect), "moved with the core");
    let (x, y, _, _) = canvas_rect_to_window((100, 100, 1, 1), (1920.0, 1200.0), 1.0);
    move_to(&mut app, x, y);
    app.update();
    assert_eq!(pointer(&app).canvas, Some((100, 100)));

    button(&mut app, ButtonState::Pressed);
    app.update();
    assert_eq!(
        pointer(&app),
        Pointer {
            canvas: Some((100, 100)),
            held: true,
            clicked: true
        }
    );
    app.update();
    assert!(
        pointer(&app).held && !pointer(&app).clicked,
        "a click lasts one frame"
    );
    button(&mut app, ButtonState::Released);
    app.update();
    assert!(!pointer(&app).held);

    app.world_mut()
        .resource_mut::<Messages<CursorLeft>>()
        .write(CursorLeft {
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert_eq!(
        pointer(&app).canvas,
        None,
        "leaving the window clears the pointer"
    );
}

fn size(width: f32, height: f32, scale_factor: f32) -> WindowSize {
    WindowSize {
        width,
        height,
        scale_factor,
    }
}

#[test]
fn a_hidpi_window_fits_in_physical_pixels() {
    // A 4K panel at 2x logical: the window is 1920x1080 logical, 3840x2160 physical.
    let mut app = ui_app(false);
    app.update();
    let mut resolution = WindowResolution::new(3840, 2160);
    resolution.set_scale_factor(2.0);
    let window = app
        .world_mut()
        .spawn(Window {
            resolution,
            ..default()
        })
        .id();
    app.world_mut()
        .resource_mut::<Messages<WindowCreated>>()
        .write(WindowCreated { window });
    app.update();
    assert_eq!(
        *app.world().resource::<WindowSize>(),
        size(1920.0, 1080.0, 2.0)
    );
    assert_eq!(app.world().resource::<WindowSize>().fit().scale, 3);
    assert!(
        !app.world().resource::<Layout>().is_wide(),
        "3x is the narrow canvas"
    );
    move_to(&mut app, 960.0, 540.0);
    app.update();
    assert_eq!(pointer(&app).canvas, Some((640, 360)));
    move_to(&mut app, 319.0, 179.0);
    app.update();
    assert_eq!(pointer(&app).canvas, Some((212, 119)), "inside at 3x");
    // Moved to a 1x monitor of the same logical size: 1x, the rows letterboxed and the
    // canvas as wide as the window.
    app.world_mut()
        .resource_mut::<Messages<WindowScaleFactorChanged>>()
        .write(WindowScaleFactorChanged {
            window,
            scale_factor: 1.0,
        });
    app.update();
    assert_eq!(
        *app.world().resource::<WindowSize>(),
        size(1920.0, 1080.0, 1.0)
    );
    assert_eq!(app.world().resource::<WindowSize>().fit().scale, 1);
    assert_eq!(*app.world().resource::<Layout>(), Layout::for_width(1920));
    move_to(&mut app, 319.0, 179.0);
    app.update();
    assert_eq!(pointer(&app).canvas, None, "in the bar above at 1x");
    move_to(&mut app, 960.0, 540.0);
    app.update();
    assert_eq!(
        pointer(&app).canvas,
        Some((960, 360)),
        "the wide canvas's centre"
    );
}

#[test]
fn the_window_size_follows_creation_and_resizes() {
    let mut app = ui_app(false);
    app.update();
    let window = app
        .world_mut()
        .spawn(Window {
            resolution: WindowResolution::new(1000, 600),
            ..default()
        })
        .id();
    app.world_mut()
        .resource_mut::<Messages<WindowCreated>>()
        .write(WindowCreated { window });
    app.update();
    assert_eq!(
        *app.world().resource::<WindowSize>(),
        size(1000.0, 600.0, 1.0)
    );
    app.world_mut()
        .resource_mut::<Messages<WindowResized>>()
        .write(WindowResized {
            window,
            width: 640.0,
            height: 360.0,
        });
    app.update();
    assert_eq!(
        *app.world().resource::<WindowSize>(),
        size(640.0, 360.0, 1.0)
    );
    // The pointer maps through the new size.
    let (x, y, _, _) = canvas_rect_to_window((50, 25, 1, 1), (640.0, 360.0), 1.0);
    move_to(&mut app, x, y);
    app.update();
    assert_eq!(pointer(&app).canvas, Some((50, 25)));
}

#[test]
fn the_mouse_starts_a_game_and_builds_a_party() {
    let mut app = ui_app(false);
    app.update();
    app.update();
    let w = widget(&app, WidgetId::Row(1));
    point_at(&mut app, spot(&w, Part::Body));
    app.update();
    assert_eq!(frame(&app).hover, Some(WidgetId::Row(1)));

    start_new_game_by_mouse(&mut app);
    assert_eq!(world(&app).seed, 42);
    assert_eq!(world(&app).settings.save_rule, SaveRule::Relief);
    assert_eq!(
        seen(&app).replaced,
        1,
        "a new game redraws the world before its first step"
    );

    draft_fighter_by_mouse(&mut app);
    let members = &world(&app).party.members;
    assert_eq!(members.len(), 1, "the draft became a member");
    assert_eq!(members[0].name, "Brenna");
    assert_eq!(members[0].hp_max, 12);
    assert_eq!(members[0].scores, [16, 15, 14, 13, 11, 9]);
    assert!(app.world().resource::<Screens>().creation.name.is_empty());
    assert!(
        frame(&app).frame.widget(WidgetId::Member(0)).is_some(),
        "the band shows the member"
    );

    click(&mut app, WidgetId::Row(ROW_BEGIN), Part::Body);
    assert_eq!(play_state(&app), PlayState::Explore);
    assert!(frame(&app).frame.widget(WidgetId::Row(0)).is_none());
}

#[test]
fn pad_clicks_step_and_turn_the_party_while_exploring() {
    let mut app = ui_app(true);
    app.update();
    app.update();
    let start = world(&app).position;
    assert_eq!((start.x, start.y, start.facing), (16, 16, Facing::North));
    click(&mut app, WidgetId::Pad(PadButton::Forward), Part::Body);
    assert_eq!(world(&app).position.y, 15);
    click(&mut app, WidgetId::Pad(PadButton::TurnLeft), Part::Body);
    assert_eq!(world(&app).position.facing, Facing::West);

    // The gap between buttons hits nothing.
    let forward = widget(&app, WidgetId::Pad(PadButton::Forward));
    click_at(&mut app, (forward.rect.x - 1, forward.rect.y));
    assert_eq!(
        (world(&app).position.y, world(&app).position.facing),
        (15, Facing::West)
    );

    // Held shows pressed; released clears it.
    point_at(&mut app, spot(&forward, Part::Body));
    button(&mut app, ButtonState::Pressed);
    app.update();
    assert_eq!(frame(&app).pressed, Some(WidgetId::Pad(PadButton::Forward)));
    button(&mut app, ButtonState::Released);
    app.update();
    assert_eq!(frame(&app).pressed, None);
    let before = world(&app).position;

    // Paused: the pad is drawn dim and inert.
    escape(&mut app);
    assert_eq!(play_state(&app), PlayState::Paused);
    assert!(!widget(&app, WidgetId::Pad(PadButton::Forward)).enabled);
    click_at(&mut app, spot(&forward, Part::Body));
    assert_eq!(world(&app).position, before);
    assert_eq!(frame(&app).hover, None);
}

#[test]
fn party_rows_select_and_the_pause_menu_works_by_mouse() {
    let mut app = ui_app(false);
    start_new_game_by_mouse(&mut app);
    draft_fighter_by_mouse(&mut app);
    click(&mut app, WidgetId::Row(ROW_BEGIN), Part::Body);

    click(&mut app, WidgetId::Member(0), Part::Body);
    assert_eq!(*app.world().resource::<Selected>(), Selected(Some(0)));
    click(&mut app, WidgetId::Member(0), Part::Body);
    assert_eq!(*app.world().resource::<Selected>(), Selected(None));
    assert!(frame(&app).frame.widget(WidgetId::Member(1)).is_none());

    escape(&mut app);
    assert_eq!(play_state(&app), PlayState::Paused);
    click(&mut app, WidgetId::Row(0), Part::Body);
    assert_eq!(play_state(&app), PlayState::Explore);
    escape(&mut app);
    click(&mut app, WidgetId::Row(3), Part::Body);
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::MainMenu
    );
    assert!(app.world().get_resource::<SimWorld>().is_none());
    assert!(
        frame(&app)
            .frame
            .widget(WidgetId::Pad(PadButton::Use))
            .is_none()
    );
}

/// The tool pad by mouse: MENU pauses (and the pad goes dim under the overlay), MAP sends
/// the automap toggle, a dim button does nothing, and SPELLS lights up with a caster and
/// opens the cast menu.
#[test]
fn the_tool_pad_opens_the_menu_the_map_and_the_spells_by_mouse() {
    let mut app = common::ui_app_saving_to("tool-pad.ron", true);
    app.update();
    app.update();
    for button in [ToolButton::Items, ToolButton::Sheet, ToolButton::Look] {
        assert!(!widget(&app, WidgetId::Tool(button)).enabled, "{button:?}");
    }
    assert!(
        !widget(&app, WidgetId::Tool(ToolButton::Spells)).enabled,
        "no caster yet"
    );
    assert!(widget(&app, WidgetId::Tool(ToolButton::Map)).enabled);

    click(&mut app, WidgetId::Tool(ToolButton::Menu), Part::Body);
    assert_eq!(play_state(&app), PlayState::Paused);
    assert!(
        !widget(&app, WidgetId::Tool(ToolButton::Menu)).enabled,
        "dim under the overlay"
    );
    click(&mut app, WidgetId::Row(0), Part::Body);
    assert_eq!(play_state(&app), PlayState::Explore);

    click(&mut app, WidgetId::Tool(ToolButton::Map), Part::Body);
    assert_eq!(seen(&app).shell.last(), Some(&ShellCommand::ToggleAutomap));

    let items = widget(&app, WidgetId::Tool(ToolButton::Items));
    let sent = seen(&app).shell.len();
    click_at(&mut app, spot(&items, Part::Body));
    assert_eq!(play_state(&app), PlayState::Explore);
    assert_eq!(seen(&app).shell.len(), sent, "a dim button sends nothing");
    assert_eq!(frame(&app).hover, None);

    let draft = omnis_sim::omnis_rules::Draft {
        name: "Ilvara".to_owned(),
        race: "base:race:elf".to_owned(),
        class: "base:class:wizard".to_owned(),
        background: "base:background:acolyte".to_owned(),
        alignment: omnis_sim::omnis_data::Alignment::ChaoticGood,
        scores: [8, 14, 13, 15, 12, 10],
        skills: vec![
            omnis_sim::omnis_data::Skill::Arcana,
            omnis_sim::omnis_data::Skill::History,
        ],
    };
    app.world_mut()
        .resource_mut::<Messages<omnis_app::sim::PlayerCommand>>()
        .write(omnis_app::sim::PlayerCommand(omnis_sim::Command::Party(
            omnis_sim::PartyCommand::Create(draft),
        )));
    app.update();
    app.update();
    assert!(widget(&app, WidgetId::Tool(ToolButton::Spells)).enabled);
    click(&mut app, WidgetId::Tool(ToolButton::Spells), Part::Body);
    assert_eq!(play_state(&app), PlayState::Cast);
    assert!(
        !widget(&app, WidgetId::Tool(ToolButton::Spells)).enabled,
        "dim under the cast menu"
    );
    common::key(&mut app, bevy::input::keyboard::Key::Escape);
    assert_eq!(play_state(&app), PlayState::Explore);
}

/// The pause menu's Save and Load by mouse: the quick save is written, read back, and the
/// overlay stays up with the notice on the band; Resume then goes where the loaded world is.
#[test]
fn the_pause_menu_saves_and_loads_by_mouse() {
    let mut app = common::ui_app_saving_to("pause-menu-save.ron", true);
    app.update();
    app.update();
    let save_path = app
        .world()
        .resource::<omnis_app::AppConfig>()
        .save_path
        .clone();
    let _ = std::fs::remove_file(&save_path);
    click(&mut app, WidgetId::Pad(PadButton::Forward), Part::Body);
    let saved_at = world(&app).position;
    assert_eq!(saved_at.y, 15);

    escape(&mut app);
    assert_eq!(play_state(&app), PlayState::Paused);
    assert!(widget(&app, WidgetId::Row(4)).enabled, "five items");
    click(&mut app, WidgetId::Row(1), Part::Body);
    app.update();
    assert_eq!(
        play_state(&app),
        PlayState::Paused,
        "Save stays on the overlay"
    );
    assert!(save_path.is_file(), "Save wrote {}", save_path.display());
    let line = app.world().resource::<MessageLine>();
    assert!(line.0.text.starts_with("Saved to "), "{}", line.0.text);
    assert!(!line.0.alert);

    // Walk on, then Load: the party is back where it was saved, still paused.
    click(&mut app, WidgetId::Row(0), Part::Body);
    assert_eq!(play_state(&app), PlayState::Explore);
    click(&mut app, WidgetId::Pad(PadButton::Forward), Part::Body);
    assert_eq!(world(&app).position.y, 14);
    let replaced_before = seen(&app).replaced;
    escape(&mut app);
    click(&mut app, WidgetId::Row(2), Part::Body);
    app.update();
    assert_eq!(
        play_state(&app),
        PlayState::Paused,
        "Load stays on the overlay"
    );
    assert_eq!(
        seen(&app).replaced,
        replaced_before + 1,
        "the world was replaced"
    );
    assert_eq!(world(&app).position, saved_at);
    let line = app.world().resource::<MessageLine>();
    assert!(line.0.text.starts_with("Loaded "), "{}", line.0.text);
    click(&mut app, WidgetId::Row(0), Part::Body);
    assert_eq!(play_state(&app), PlayState::Explore);
    assert_eq!(world(&app).position, saved_at);
    let _ = std::fs::remove_file(&save_path);
}

#[test]
fn the_message_line_shows_rejections_and_notices() {
    let mut app = ui_app(false);
    start_new_game_by_mouse(&mut app);
    click(&mut app, WidgetId::Row(ROW_BEGIN), Part::Body);
    assert_eq!(play_state(&app), PlayState::CreateParty);
    assert_eq!(
        app.world().resource::<Screens>().creation.message,
        "Add at least one member"
    );
    let raster = &frame(&app).frame.raster;
    let alert = (0..CANVAS_WIDTH as i32).any(|x| {
        raster
            .get(
                x,
                omnis_app::band::band_cell(0, omnis_app::band::MESSAGE_ROW).1 + 3,
            )
            .is_some_and(|p| (p[0], p[1], p[2]) == ALERT)
    });
    assert!(alert, "the rejection is painted in the alert colour");

    draft_fighter_by_mouse(&mut app);
    click(&mut app, WidgetId::Row(ROW_BEGIN), Part::Body);
    app.world_mut()
        .resource_mut::<Messages<ShellCommand>>()
        .write(ShellCommand::Save);
    app.update();
    app.update();
    let line = app.world().resource::<MessageLine>();
    assert!(line.0.text.contains("save rule"), "{}", line.0.text);
    assert!(line.0.alert);

    // A notice is shown once; the next event takes the line back.
    app.world_mut()
        .resource_mut::<Messages<omnis_app::sim::PlayerCommand>>()
        .write(omnis_app::sim::PlayerCommand(omnis_sim::Command::Interact));
    app.update();
    app.update();
    let line = app.world().resource::<MessageLine>();
    assert_eq!(line.0.text, "sim:message:nothing_here");
    assert!(!line.0.alert);
}
