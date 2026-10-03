//! The tool bar on Feathers (M7 step 8a), driven as a person does: real pointer clicks through
//! layout and picking, the bar placed over the canvas's tool strip at both window sizes, each
//! word inside its button at every interface scale the slider allows and in every font, dim
//! buttons that send nothing however they are pressed, and the bar gone without a world.

mod common;

use bevy::prelude::*;
use common::feathers::{bar_faults, click_node, control, rect, resize, settle, ultrawide};
use common::{feathers_app, fighter_draft, play_state, seen, world};
use omnis_app::canvas::Layout;
use omnis_app::cursor::WindowSize;
use omnis_app::layout::{TOOLS, canvas_rect_to_window};
use omnis_app::sim::{PlayState, PlayerCommand, ShellCommand};
use omnis_app::tool_bar::{ToolButton, ToolPressed};
use omnis_app::ui_kit::{FontChoice, ScaleChoice, ToolBar};
use omnis_app::ui_model::{FONTS, scale_cap};
use omnis_sim::Command;
use omnis_sim::party::PartyCommand;

fn send(app: &mut App, command: Command) {
    app.world_mut()
        .resource_mut::<Messages<PlayerCommand>>()
        .write(PlayerCommand(command));
    settle(app);
}

/// A new game on the map with one fighter.
fn exploring(save: &str) -> App {
    let mut app = feathers_app(save, true);
    settle(&mut app);
    send(
        &mut app,
        Command::Party(PartyCommand::Create(fighter_draft())),
    );
    assert_eq!(play_state(&app), PlayState::Explore);
    assert!(!world(&app).party.members.is_empty());
    app
}

fn click_tool(app: &mut App, button: ToolButton) {
    let entity = control(app, button);
    click_node(app, entity);
}

fn bar(app: &mut App) -> Option<Entity> {
    let mut query = app.world_mut().query_filtered::<Entity, With<ToolBar>>();
    let found: Vec<_> = query.iter(app.world()).collect();
    assert!(found.len() <= 1, "one bar at most");
    found.first().copied()
}

/// Every word that does not sit on one line inside its button, with the sizes. One line is
/// the shortest caption's height ("Map" never wraps); a wrapped word is taller.
fn words_outside(app: &mut App) -> Vec<String> {
    let mut found = Vec::new();
    for button in ToolButton::ALL {
        let entity = control(app, button);
        let outer = rect(app, entity);
        let mut texts = Vec::new();
        let mut stack = vec![entity];
        while let Some(at) = stack.pop() {
            if let Some(children) = app.world().get::<Children>(at) {
                stack.extend(children.iter());
            }
            if at != entity && app.world().get::<Text>(at).is_some() {
                texts.push(at);
            }
        }
        assert_eq!(texts.len(), 1, "{button:?} has one caption");
        found.push((button, outer, rect(app, texts[0])));
    }
    let line = found
        .iter()
        .map(|(_, _, word)| word.height())
        .fold(f32::INFINITY, f32::min);
    found
        .into_iter()
        .filter(|(_, outer, word)| {
            let inside = outer.contains(word.min) && outer.contains(word.max);
            !(inside && word.height() < line * 1.5)
        })
        .map(|(button, outer, word)| format!("{button:?}: word {word:?} in button {outer:?}"))
        .collect()
}

#[test]
fn the_bar_sits_on_the_tool_strip_and_every_word_fits_at_both_sizes() {
    let mut app = exploring("tool-bar-layout.ron");
    for (size, canvas_scale) in [("1280 by 720", 1), ("5120 by 1440", 2)] {
        if canvas_scale == 2 {
            ultrawide(&mut app);
        }
        let entity = bar(&mut app).expect("a bar on the map");
        let layout = *app.world().resource::<Layout>();
        let window = *app.world().resource::<WindowSize>();
        let strip = (
            layout.core.0 + TOOLS.x,
            layout.core.1 + TOOLS.y,
            TOOLS.w,
            TOOLS.h,
        );
        let (x, y, w, h) =
            canvas_rect_to_window(strip, (window.width, window.height), window.scale_factor);
        let at = rect(&app, entity);
        let wanted = Rect::new(x, y, x + w, y + h);
        assert!(
            (at.min - wanted.min).length() < 1.0 && (at.max - wanted.max).length() < 1.0,
            "{size}: the bar {at:?} on the strip {wanted:?}"
        );
        for scale in [None, Some(scale_cap(canvas_scale))] {
            for (font, name) in FONTS.iter().enumerate() {
                app.world_mut().resource_mut::<ScaleChoice>().0 = scale;
                app.world_mut().resource_mut::<FontChoice>().0 = font;
                settle(&mut app);
                let what = format!("{size} at {scale:?} in {name}");
                assert_eq!(bar_faults(&mut app), Vec::new(), "{what}");
                assert_eq!(words_outside(&mut app), Vec::<String>::new(), "{what}");
            }
        }
        if let Ok(dir) = std::env::var("OMNIS_DUMP_SCREENS")
            && canvas_scale == 1
        {
            app.world_mut().resource_mut::<ScaleChoice>().0 = None;
            settle(&mut app);
            let path = std::path::Path::new(&dir).join("tool_bar.txt");
            let text = omnis_app::ui_text::panel_text(app.world(), entity);
            std::fs::write(&path, text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        }
    }
}

#[test]
fn every_live_button_opens_its_screen_by_pointer() {
    let mut app = exploring("tool-bar-clicks.ron");
    click_tool(&mut app, ToolButton::Items);
    assert_eq!(play_state(&app), PlayState::Inventory);
    app.world_mut()
        .resource_mut::<NextState<PlayState>>()
        .set(PlayState::Explore);
    settle(&mut app);

    click_tool(&mut app, ToolButton::Sheet);
    assert_eq!(play_state(&app), PlayState::Sheet);
    app.world_mut()
        .resource_mut::<NextState<PlayState>>()
        .set(PlayState::Explore);
    settle(&mut app);

    click_tool(&mut app, ToolButton::Map);
    assert_eq!(seen(&app).shell.last(), Some(&ShellCommand::ToggleAutomap));

    click_tool(&mut app, ToolButton::Menu);
    assert_eq!(play_state(&app), PlayState::Paused);
}

#[test]
fn a_dim_button_sends_nothing_by_pointer_or_by_a_forged_press() {
    let mut app = exploring("tool-bar-dim.ron");
    let sent = seen(&app).shell.len();
    // A fighter has no spell for the road; CAMP waits for its panel (step 8b).
    for button in [ToolButton::Spells, ToolButton::Camp] {
        click_tool(&mut app, button);
        app.world_mut()
            .resource_mut::<Messages<ToolPressed>>()
            .write(ToolPressed(button));
        settle(&mut app);
        assert_eq!(play_state(&app), PlayState::Explore, "{button:?}");
        assert_eq!(seen(&app).shell.len(), sent, "{button:?} sends nothing");
        let entity = control(&mut app, button);
        assert!(
            app.world()
                .get::<bevy::ui::InteractionDisabled>(entity)
                .is_some(),
            "{button:?} is drawn dim"
        );
    }
}

#[test]
fn the_bar_follows_the_screen_and_leaves_with_the_world() {
    let mut app = feathers_app("tool-bar-title.ron", false);
    settle(&mut app);
    assert_eq!(bar(&mut app), None, "no bar on the title");
    let mut app = exploring("tool-bar-follows.ron");
    assert!(bar(&mut app).is_some());
    click_tool(&mut app, ToolButton::Menu);
    assert_eq!(play_state(&app), PlayState::Paused);
    let menu = control(&mut app, ToolButton::Menu);
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(menu)
            .is_some(),
        "dim under the pause overlay"
    );
    resize(&mut app, 1280.0, 720.0);
    assert!(bar(&mut app).is_some(), "the bar stays while paused");
}
