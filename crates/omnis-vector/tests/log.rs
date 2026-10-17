//! The HUD's running log (owner, 2026-10-17): its own column under the status lines, wrapped no
//! wider than the gap left of the outcome notice so it never runs under it, in the interface's
//! dim role, each older line fainter than the one after it, so the notice stands out.

use crate::common;

use bevy::prelude::*;
use omnis_vector::shell::hud::{
    HudPlugin, LOG_LINES, LogColumn, LogLine, StatusText, log_alpha, log_width,
};
use omnis_vector::shell::notice::{NOTICE_WIDTH, NoticePanel};
use omnis_vector::shell::screen::colour;
use omnis_vector::shell::session::Session;
use omnis_vector::style::Role;

#[test]
fn the_newest_line_is_whole_and_each_older_one_fainter_to_a_floor() {
    assert!((log_alpha(0) - 1.0).abs() < 1e-6);
    for age in 1..LOG_LINES {
        assert!(log_alpha(age) < log_alpha(age - 1), "age {age}");
    }
    assert!(log_alpha(LOG_LINES - 1) >= 0.3, "the oldest still reads");
}

#[test]
fn the_log_s_column_ends_before_the_widest_notice_begins() {
    // The notice is centred at most NOTICE_WIDTH percent wide; the log starts 16 px in.
    let notice_left = (100.0 - NOTICE_WIDTH) / 2.0;
    assert!(log_width() > 10.0, "room to read");
    // 16 px is at most 2 vw from 800 px wide up.
    assert!(log_width() + 2.0 <= notice_left);
}

/// The log's lines as drawn, top first: their words and colours, the shown ones only.
fn drawn(app: &mut App) -> Vec<(String, Color)> {
    let world = app.world_mut();
    let mut q = world.query::<(&LogLine, &Text, &TextColor, &Visibility)>();
    let mut lines: Vec<_> = q
        .iter(world)
        .filter(|(.., v)| **v != Visibility::Hidden)
        .map(|(l, t, c, _)| (l.0, t.0.clone(), c.0))
        .collect();
    lines.sort_by_key(|l| l.0);
    lines.into_iter().map(|(_, t, c)| (t, c)).collect()
}

#[test]
fn in_the_window_the_log_stands_apart_from_the_status_dim_and_fading() {
    let mut app = common::app::shell(common::app::config(), true);
    app.add_plugins(HudPlugin);
    app.update();
    {
        let mut session = app.world_mut().resource_mut::<Session>();
        session.lines.clear();
        for line in ["first", "second", "third"] {
            session.say(line.to_owned());
        }
    }
    app.update();
    let lines = drawn(&mut app);
    let words: Vec<&str> = lines.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(words, ["first", "second", "third"]);
    let dim = colour(Role::Dim);
    for (i, (_, c)) in lines.iter().enumerate() {
        let age = lines.len() - 1 - i;
        assert_eq!(*c, dim.with_alpha(log_alpha(age)), "line {i}");
    }
    // The status lines no longer carry the log.
    let world = app.world_mut();
    let mut status = world.query_filtered::<&Text, With<StatusText>>();
    let text = status.single(world).expect("the status").0.clone();
    assert!(!text.contains("third"), "{text}");
    // The column's width and the notice's come from the one measure.
    let mut column = world.query_filtered::<&Node, With<LogColumn>>();
    assert_eq!(
        column.single(world).unwrap().max_width,
        Val::Vw(log_width())
    );
    let mut notice = world.query_filtered::<&Node, With<NoticePanel>>();
    assert_eq!(
        notice.single(world).unwrap().max_width,
        Val::Percent(NOTICE_WIDTH)
    );
}
