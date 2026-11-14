//! The interface's colours as a theme the nodes follow (owner, 2026-11-13;
//! `tasks/plans/fight-on-the-palette.md`). A node says what its text, border and background
//! mean ([`Ink`], [`Edge`], [`Fill`]) and [`apply`] paints them from the [`Theme`], when the
//! node is made, when its role changes and when the theme does. The theme follows the palette of
//! the map a fight is fought on; outside a fight it is the default until the HUD's plan.
//! Headless-safe.

use super::ViewState;
use super::session::Session;
use crate::style::{self, Role};
use bevy::prelude::*;

/// The interface's colours now.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct Theme(pub style::Theme);

/// An sRGB triple as a colour.
fn srgb([r, g, b]: [f32; 3]) -> Color {
    Color::srgb(r, g, b)
}

impl Theme {
    /// `role`'s colour.
    #[must_use]
    pub fn colour(&self, role: Role) -> Color {
        srgb(self.0.rgb(role))
    }

    /// A control's background: at rest, hovered or pressed.
    #[must_use]
    pub fn shade(&self, i: usize) -> Color {
        srgb(self.0.shade(i))
    }

    /// What `fill` is painted.
    #[must_use]
    pub fn fill(&self, fill: Fill) -> Color {
        match fill {
            Fill::Shade(i) => self.shade(i),
            Fill::Role(role) => self.colour(role),
        }
    }
}

/// The role a node's text is drawn in.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ink(pub Role);

/// The role a node's border is drawn in.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge(pub Role);

/// What a node's background is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    /// A control's background, by the pointer's state: at rest, hovered, pressed.
    Shade(usize),
    /// A role's own colour, solid: a slider's thumb, a scroll bar's.
    Role(Role),
}

/// Text in `role`: its colour in the default theme until [`apply`] paints it.
#[must_use]
pub fn ink(role: Role) -> (TextColor, Ink) {
    (TextColor(Theme::default().colour(role)), Ink(role))
}

/// A border in `role`.
#[must_use]
pub fn edge(role: Role) -> (BorderColor, Edge) {
    (BorderColor::all(Theme::default().colour(role)), Edge(role))
}

/// A background of shade `i`.
#[must_use]
pub fn fill(i: usize) -> (BackgroundColor, Fill) {
    let fill = Fill::Shade(i);
    (BackgroundColor(Theme::default().fill(fill)), fill)
}

/// A background of `role`'s own colour.
#[must_use]
pub fn solid(role: Role) -> (BackgroundColor, Fill) {
    let fill = Fill::Role(role);
    (BackgroundColor(Theme::default().fill(fill)), fill)
}

/// The theme of the fight's map while a fight is on, the default otherwise.
fn follow(
    session: Option<Res<Session>>,
    view: Option<Res<State<ViewState>>>,
    mut theme: ResMut<Theme>,
) {
    let fight = view.is_some_and(|v| *v.get() == ViewState::Fight);
    let want = match session.as_deref() {
        Some(session) if fight => Theme(style::Theme::of(&super::combat::palette(session))),
        _ => Theme::default(),
    };
    theme.set_if_neq(want);
}

/// Paint each node's text, border and background from the theme: all of them when the theme
/// changed, otherwise those made or given another role this frame. A fading node keeps its
/// alpha.
fn apply(
    theme: Res<Theme>,
    mut texts: Query<(Ref<Ink>, &mut TextColor)>,
    mut borders: Query<(Ref<Edge>, &mut BorderColor)>,
    mut fills: Query<(Ref<Fill>, &mut BackgroundColor)>,
) {
    let all = theme.is_changed();
    for (ink, mut text) in &mut texts {
        if all || ink.is_changed() {
            let colour = theme.colour(ink.0).with_alpha(text.0.alpha());
            if text.0 != colour {
                text.0 = colour;
            }
        }
    }
    for (edge, mut border) in &mut borders {
        if all || edge.is_changed() {
            let colour = BorderColor::all(theme.colour(edge.0).with_alpha(border.top.alpha()));
            if *border != colour {
                *border = colour;
            }
        }
    }
    for (fill, mut back) in &mut fills {
        if all || fill.is_changed() {
            let colour = theme.fill(*fill).with_alpha(back.0.alpha());
            if back.0 != colour {
                back.0 = colour;
            }
        }
    }
}

/// The theme, following the fight's map, and the painting of the nodes that name their roles.
/// Added by every plugin whose nodes name roles; set up once.
pub fn plugin(app: &mut App) {
    if app.world().contains_resource::<Theme>() {
        return;
    }
    app.init_resource::<Theme>()
        .add_systems(Update, follow.before(super::VectorSet::Grab))
        .add_systems(PostUpdate, apply.before(bevy::ui::UiSystems::Prepare));
}
