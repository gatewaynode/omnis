//! The kit draws icons (the owner's drawing-GUI column, step 1, 2026-10-25): a `Kind::Icon`
//! control is a square box with the icon's image, drawn once per name, size and colour and kept
//! (`shell/icons.rs`); the text readers name it; the chosen one of a grid is framed; the
//! editor's buttons are narrower so two share a row; and the editor's buttons shade under the
//! pointer as a game's do. The first icons are the bar's view controls.

use crate::common;
use bevy::prelude::*;
use common::app::{set, ui_editor_app};
use common::editor::{editing, in_editor, press_edit};
use omnis_vector::editor::{EditAct, view};
use omnis_vector::screen::{Control, ControlId, Kind, Panel, Row, Screen, ScreenId, Span};
use omnis_vector::shell::icons::{IconImages, IconOf, NeedsIcon, tint};
use omnis_vector::shell::screen::EditorPress;
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Packs;
use omnis_vector::style::{Role, SHADES, scaled};

/// The bar's five view controls, in their order.
const VIEWS: [(EditAct, &str); 5] = [
    (EditAct::ZoomOut, "zoom-out"),
    (EditAct::ZoomIn, "zoom-in"),
    (EditAct::Centre, "focus"),
    (EditAct::FaceLeft, "rotate-ccw"),
    (EditAct::FaceRight, "rotate-cw"),
];

fn icon_screen(chosen: bool) -> Screen {
    Screen {
        id: ScreenId::EditorBar,
        title: Span::title("t"),
        panels: vec![Panel {
            rows: vec![Row::controls(vec![Control {
                kind: Kind::Icon {
                    name: "brush",
                    chosen,
                },
                ..Control::button(ControlId::Edit(EditAct::ZoomIn), "brush")
            }])],
        }],
        keys: Vec::new(),
        message: None,
    }
}

#[test]
fn an_icon_is_a_kind_whose_chosen_state_is_part_of_the_shape_and_whose_label_is_its_name() {
    let plain = icon_screen(false);
    let chosen = icon_screen(true);
    assert_ne!(plain.shape(), chosen.shape(), "a change of choice rebuilds");
    assert_eq!(plain.shape(), icon_screen(false).shape());
    let c = plain
        .control(ControlId::Edit(EditAct::ZoomIn))
        .expect("the icon");
    assert_eq!(c.label, "brush");
    assert!(c.why_not.is_none());
}

#[test]
fn the_bar_s_view_controls_are_icons_the_text_reader_names_and_a_press_uses() {
    let mut app = in_editor("test:map:town");
    let data = app.world().resource::<Packs>().data.clone();
    let bar = view::bar(editing(&app), &data);
    for (act, name) in VIEWS {
        let c = bar.control(ControlId::Edit(act)).expect("a view control");
        assert_eq!(
            c.kind,
            Kind::Icon {
                name,
                chosen: false
            },
            "{name}"
        );
        assert!(c.label.is_ascii());
    }
    // The column no longer holds them.
    let tools = view::tools(editing(&app), &data);
    for (act, _) in VIEWS {
        assert!(tools.control(ControlId::Edit(act)).is_none());
    }
    // Drawn: a node per icon, named for the readers, under a pressable box.
    let text = screen_text(app.world());
    for (_, name) in VIEWS {
        assert!(
            text.contains(&format!("[button] [icon] {name:?}")),
            "{name} in\n{text}"
        );
    }
    assert!(text.contains("px a cell"), "{text}");
    let world = app.world_mut();
    let named: Vec<&'static str> = world.query::<&IconOf>().iter(world).map(|i| i.0).collect();
    for (_, name) in VIEWS {
        assert!(named.contains(&name), "{name} among {named:?}");
    }
    // Headless, with no image assets, the nodes keep waiting and nothing panics.
    assert_eq!(world.query::<&NeedsIcon>().iter(world).count(), named.len());
    // A press on the icon's box is the control's press.
    let before = editing(&app).view.zoom;
    press_edit(&mut app, EditAct::ZoomIn);
    assert_eq!(editing(&app).view.zoom, before + 1);
    let facing = editing(&app).facing;
    press_edit(&mut app, EditAct::FaceRight);
    assert_eq!(editing(&app).facing, facing.right());
}

#[test]
fn an_editor_button_is_narrower_than_a_screen_s_and_shades_under_the_pointer() {
    let mut app = in_editor("test:map:town");
    // The icon's box is 44 at the reference height, a button at least 120: the ratio holds at
    // any height.
    let world = app.world_mut();
    let side = {
        let mut icons = world.query::<(&EditorPress, &Node)>();
        icons
            .iter(world)
            .find(|(p, _)| p.0 == ControlId::Edit(EditAct::ZoomIn))
            .map(|(_, n)| n.width)
            .expect("the zoom icon's box")
    };
    let least = {
        let mut buttons = world.query::<(&EditorPress, &Node)>();
        buttons
            .iter(world)
            .find(|(p, _)| p.0 == ControlId::Edit(EditAct::Menu))
            .map(|(_, n)| n.min_width)
            .expect("the menu button")
    };
    let (Val::Px(side), Val::Px(least)) = (side, least) else {
        panic!("{side:?} {least:?} are in pixels");
    };
    assert!(
        (least / side - 120.0 / 44.0).abs() < 0.01,
        "{least} / {side}"
    );
    assert!((scaled(120.0, 900.0) - 120.0).abs() < f32::EPSILON);
    // Hovering a live editor button lightens it; leaving darkens it again.
    let menu = EditorPress(ControlId::Edit(EditAct::Menu));
    let background = |app: &mut App| {
        let world = app.world_mut();
        let mut buttons = world.query::<(&EditorPress, &BackgroundColor)>();
        buttons
            .iter(world)
            .find(|(p, _)| **p == menu)
            .map(|(_, b)| b.0)
            .expect("the menu button")
    };
    let shade = |i: usize| Color::srgb(SHADES[i][0], SHADES[i][1], SHADES[i][2]);
    assert_eq!(background(&mut app), shade(0));
    set(&mut app, &menu, Interaction::Hovered);
    app.update();
    assert_eq!(background(&mut app), shade(1), "hovered");
    set(&mut app, &menu, Interaction::Pressed);
    app.update();
    assert_eq!(background(&mut app), shade(2), "pressed");
    set(&mut app, &menu, Interaction::None);
    app.update();
    assert_eq!(background(&mut app), shade(0));
}

#[test]
fn the_icon_images_are_drawn_once_in_the_role_s_colour_and_shared() {
    let (mut app, _window) = ui_editor_app();
    for _ in 0..3 {
        app.update();
    }
    let world = app.world_mut();
    assert_eq!(
        world.query::<&NeedsIcon>().iter(world).count(),
        0,
        "every icon has its image"
    );
    let drawn: Vec<(&'static str, Handle<Image>)> = world
        .query::<(&IconOf, &ImageNode)>()
        .iter(world)
        .map(|(i, n)| (i.0, n.image.clone()))
        .collect();
    for (_, name) in VIEWS {
        assert!(drawn.iter().any(|(n, _)| *n == name), "{name} drawn");
    }
    let cache = world.resource::<IconImages>();
    assert_eq!(cache.len(), drawn.len(), "one image per distinct icon");
    // The image: transparent where no stroke, the role's colour on a stroke.
    let (_, zoom_in) = drawn
        .iter()
        .find(|(n, _)| *n == "zoom-in")
        .expect("zoom-in")
        .clone();
    let images = world.resource::<Assets<Image>>();
    let image = images.get(&zoom_in).expect("the image is an asset");
    let data = image.data.as_deref().expect("the image has bytes");
    let side = image.width();
    assert!(side >= 8 && image.height() == side, "{side}");
    assert_eq!(&data[0..4], &[0, 0, 0, 0], "the corner is transparent");
    let colour = tint(Role::Interface);
    let strokes = data
        .chunks(4)
        .filter(|p| p[3] == 255 && p[..3] == colour[..3])
        .count();
    assert!(strokes > 0, "the strokes are the interface colour");
    assert!(
        data.chunks(4).all(|p| p[3] == 0 || p[..3] == colour[..3]),
        "nothing but the strokes"
    );
    // Asking again is the same handle, drawn no second time.
    let need = NeedsIcon {
        name: "zoom-in",
        px: side,
        colour,
    };
    let before = cache.len();
    let mut cache = world.resource_mut::<IconImages>();
    let mut cache = std::mem::take(&mut *cache);
    let mut images = world.resource_mut::<Assets<Image>>();
    let again = cache.image(&mut images, need);
    assert_eq!(again, zoom_in);
    assert_eq!(cache.len(), before);
    // A different colour is a different image.
    let white = cache.image(
        &mut images,
        NeedsIcon {
            colour: [255, 255, 255, 255],
            ..need
        },
    );
    assert_ne!(white, zoom_in);
    assert_eq!(cache.len(), before + 1);
}
