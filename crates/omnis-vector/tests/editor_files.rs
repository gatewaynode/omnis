//! The RON text editor (M5.9): Files mode lists the pack's files, opens one as text in the
//! centre, checks it with the loader's own words, writes it exactly as typed and reseals, and
//! reopens the map when its own file was written.

use crate::common;

use bevy::prelude::*;
use common::app::press_ui;
use common::editor::{
    arrow, editing, field_entity, in_editor, in_editor_scratch, press_edit, type_into,
};
use omnis_vector::editor::EditAct;
use omnis_vector::editor::files::Mode;
use omnis_vector::editor::view;
use omnis_vector::screen::{ControlId, ScreenId};
use omnis_vector::shell::editor::{Column, Editing};
use omnis_vector::shell::screen::{Pressable, Screens};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Packs;

/// The screen on top of the editor, if any.
fn top(app: &App) -> Option<ScreenId> {
    app.world().resource::<Screens>().nav.top(None)
}

fn reason(app: &App, act: EditAct) -> Option<String> {
    let data = app.world().resource::<Packs>().data.clone();
    view::tools(editing(app), &data)
        .control(ControlId::Edit(act))
        .unwrap_or_else(|| panic!("{act:?} in the tools"))
        .why_not
        .clone()
}

fn file_index(app: &App, rel: &str) -> u16 {
    let i = editing(app)
        .files
        .iter()
        .position(|f| f == rel)
        .unwrap_or_else(|| panic!("{rel} listed: {:?}", editing(app).files));
    u16::try_from(i).unwrap()
}

/// Open the file `rel` as text through the picker: Open file... lists the pack's files over the
/// editor, the file's button opens it and the picker closes.
fn open_file(app: &mut App, rel: &str) {
    press_edit(app, EditAct::OpenFile);
    assert_eq!(top(app), Some(ScreenId::EditorFiles), "the picker is up");
    let i = file_index(app, rel);
    press_ui(app, Pressable(ControlId::Edit(EditAct::File(i))));
    assert_eq!(top(app), None, "the picker closed");
    assert_eq!(editing(app).mode, Mode::Files);
}

/// The text the open file's field shows.
fn field_text(app: &mut App) -> String {
    let entity = field_entity(app, EditAct::FileText).expect("the file's field");
    app.world()
        .get::<bevy::text::EditableText>(entity)
        .unwrap()
        .value()
        .to_string()
}

fn display(app: &mut App, column: Column) -> Display {
    let world = app.world_mut();
    let mut q = world.query::<(&Column, &Node)>();
    q.iter(world)
        .find(|(c, _)| **c == column)
        .map(|(_, n)| n.display)
        .expect("the column")
}

#[test]
fn the_picker_lists_the_pack_s_files_by_folder_and_opens_one_as_text_in_the_centre() {
    let mut app = in_editor("test:map:town");
    assert_eq!(editing(&app).mode, Mode::Plan);
    assert_eq!(display(&mut app, Column::Plan), Display::Flex);
    assert_eq!(display(&mut app, Column::Text), Display::None);
    // No file open: Text is dim and says so (a dim control has no press).
    assert_eq!(
        reason(&app, EditAct::Files).as_deref(),
        Some("open a file first")
    );
    // Open file... lists the pack's own files over the editor, grouped by folder.
    press_edit(&mut app, EditAct::OpenFile);
    assert_eq!(top(&app), Some(ScreenId::EditorFiles));
    let files = editing(&app).files.clone();
    assert_eq!(files[0], "pack.ron");
    assert!(
        files.contains(&"data/maps/town.ron".to_owned()),
        "{files:?}"
    );
    assert!(
        files.contains(&"text/en/spells.ron".to_owned()),
        "{files:?}"
    );
    assert!(files.iter().all(|f| f.ends_with(".ron")));
    let text = screen_text(app.world());
    assert!(
        text.contains("OPEN A FILE")
            && text.contains("pack.ron")
            && text.contains("text/en/")
            && text.contains("\"spells.ron\"")
            && text.contains("data/maps/")
            && text.contains("\"town.ron\"")
            && text.contains("Cancel"),
        "{text}"
    );
    let shown = |text: &str, rel: &str| text.contains(&format!("\"{rel}\""));
    assert!(
        !shown(&text, "text/en/spells.ron"),
        "named under its folder: {text}"
    );
    // Cancel closes it with nothing opened; so does Esc.
    press_ui(&mut app, Pressable(ControlId::Resume));
    assert_eq!(top(&app), None);
    assert!(editing(&app).file.is_none());
    press_edit(&mut app, EditAct::OpenFile);
    common::app::tap(&mut app, KeyCode::Escape);
    assert_eq!(top(&app), None, "Esc closes the picker");
    assert_eq!(editing(&app).mode, Mode::Plan);
    // Open the spells' text: the model holds the file, the centre shows it, the plan is hidden.
    open_file(&mut app, "text/en/spells.ron");
    assert_eq!(display(&mut app, Column::Plan), Display::None);
    assert_eq!(display(&mut app, Column::Text), Display::Flex);
    let file = editing(&app).file.clone().expect("the file");
    assert_eq!(file.rel, "text/en/spells.ron");
    assert_eq!(file.text, file.disk);
    assert!(file.text.contains("ward"), "{}", file.text);
    let text = screen_text(app.world());
    assert!(text.contains("text/en/spells.ron"), "{text}");
    assert!(text.contains("ward"), "the text in the centre: {text}");
    assert_eq!(
        reason(&app, EditAct::WriteFile).as_deref(),
        Some("no changes")
    );
    assert_eq!(
        reason(&app, EditAct::RevertFile).as_deref(),
        Some("no changes")
    );
    // Plan returns to the map; the file stays open and Text shows it again.
    press_edit(&mut app, EditAct::Plan);
    assert_eq!(editing(&app).mode, Mode::Plan);
    assert_eq!(display(&mut app, Column::Plan), Display::Flex);
    assert!(editing(&app).file.is_some());
    assert_eq!(reason(&app, EditAct::Files), None);
    press_edit(&mut app, EditAct::Files);
    assert_eq!(editing(&app).mode, Mode::Files);
    assert_eq!(display(&mut app, Column::Text), Display::Flex);
}

/// Open file... over unsaved text asks first (the owner's flow, 2026-10-24): Save and continue,
/// Discard changes or Cancel, no file list until the file is closed; a clean file is closed at
/// once. So a pick always reads its file fresh and never drops typed work.
#[test]
fn open_file_asks_about_unsaved_text_and_clears_the_editor() {
    let (mut app, scratch) = in_editor_scratch("editor-files-unsaved", "test:map:town");
    let version = |app: &App| {
        app.world()
            .resource::<Packs>()
            .data
            .packs
            .iter()
            .find(|p| p.id == "test")
            .map(|p| p.version.clone())
            .unwrap()
    };
    open_file(&mut app, "text/en/spells.ron");
    let disk = editing(&app).file.clone().unwrap().disk;
    let typed = format!("// Typed.\n{disk}");
    type_into(&mut app, EditAct::FileText, &typed);
    press_edit(&mut app, EditAct::OpenFile);
    assert_eq!(top(&app), Some(ScreenId::EditorFiles));
    let text = screen_text(app.world());
    assert!(
        text.contains("text/en/spells.ron has unsaved changes")
            && text.contains("Save and continue")
            && text.contains("Discard changes")
            && text.contains("Cancel")
            && !text.contains("\"maps.ron\""),
        "the question, not the list: {text}"
    );
    // The model refuses another file meanwhile (the guard behind the question).
    let maps = file_index(&app, "text/en/maps.ron");
    let asked = app
        .world_mut()
        .resource_scope(|world, mut editing: Mut<Editing>| {
            let mut packs = world.resource_mut::<Packs>();
            omnis_vector::shell::editor::act(&mut editing, &mut packs, EditAct::File(maps), 0)
        });
    assert_eq!(asked, omnis_vector::editor::Asked::Nothing);
    assert_eq!(
        editing(&app).file.as_ref().unwrap().rel,
        "text/en/spells.ron"
    );
    // Cancel keeps the file as it stands.
    press_ui(&mut app, Pressable(ControlId::Resume));
    assert_eq!(top(&app), None);
    let file = editing(&app).file.clone().unwrap();
    assert_eq!(file.text, typed, "kept");
    assert_eq!(field_text(&mut app), typed);
    // Discard changes closes it; the list follows in the same picker.
    press_edit(&mut app, EditAct::OpenFile);
    press_ui(&mut app, Pressable(ControlId::Edit(EditAct::FileDiscard)));
    assert_eq!(top(&app), Some(ScreenId::EditorFiles), "the picker stays");
    assert!(editing(&app).file.is_none(), "closed");
    let text = screen_text(app.world());
    assert!(text.contains("\"maps.ron\""), "the list: {text}");
    assert_eq!(
        reason(&app, EditAct::Files).as_deref(),
        Some("open a file first")
    );
    press_ui(&mut app, Pressable(ControlId::Edit(EditAct::File(maps))));
    assert_eq!(top(&app), None);
    let file = editing(&app).file.clone().unwrap();
    assert_eq!(file.rel, "text/en/maps.ron");
    assert_eq!(file.text, file.disk, "read fresh");
    // Save and continue with a broken text: the loader's words, the question stands.
    let disk = file.disk;
    let broken = disk.replacen('"', "", 1);
    type_into(&mut app, EditAct::FileText, &broken);
    press_edit(&mut app, EditAct::OpenFile);
    press_ui(&mut app, Pressable(ControlId::Edit(EditAct::FileSaveClose)));
    assert_eq!(top(&app), Some(ScreenId::EditorFiles));
    let file = editing(&app).file.clone().expect("still open");
    assert!(file.dirty());
    assert!(
        editing(&app)
            .message
            .as_deref()
            .is_some_and(|m| m.contains("error")),
        "{:?}",
        editing(&app).message
    );
    let text = screen_text(app.world());
    assert!(text.contains("has unsaved changes"), "{text}");
    // Valid again: written, resealed, closed, the list shown.
    let before = version(&app);
    press_ui(&mut app, Pressable(ControlId::Resume));
    let kept = format!("// Kept.\n{disk}");
    type_into(&mut app, EditAct::FileText, &kept);
    press_edit(&mut app, EditAct::OpenFile);
    press_ui(&mut app, Pressable(ControlId::Edit(EditAct::FileSaveClose)));
    assert_eq!(top(&app), Some(ScreenId::EditorFiles));
    assert!(editing(&app).file.is_none(), "closed after the write");
    assert_ne!(version(&app), before, "resealed");
    assert_eq!(
        std::fs::read_to_string(scratch.join("text/en/maps.ron")).unwrap(),
        kept
    );
    let text = screen_text(app.world());
    assert!(text.contains("\"spells.ron\""), "the list: {text}");
    press_ui(&mut app, Pressable(ControlId::Resume));
    assert!(editing(&app).file.is_none());
    // A clean file is closed at Open file... itself.
    open_file(&mut app, "text/en/spells.ron");
    press_edit(&mut app, EditAct::OpenFile);
    assert!(editing(&app).file.is_none(), "cleared");
    assert_eq!(top(&app), Some(ScreenId::EditorFiles));
    press_ui(&mut app, Pressable(ControlId::Resume));
}

/// A second file opened shows its own text, not the first's (the owner's report, 2026-10-24:
/// "any subsequently chosen file just reloads the first file in the editor again"). The field's
/// shape is the same, so the words alone were rewritten and the field kept the first text.
#[test]
fn a_second_file_opened_shows_its_own_text_in_the_field() {
    let mut app = in_editor("test:map:town");
    open_file(&mut app, "text/en/spells.ron");
    let spells = editing(&app).file.clone().unwrap().text;
    assert_eq!(field_text(&mut app), spells);
    open_file(&mut app, "text/en/maps.ron");
    let maps = editing(&app).file.clone().unwrap();
    assert_eq!(maps.rel, "text/en/maps.ron");
    assert_ne!(maps.text, spells);
    assert_eq!(
        field_text(&mut app),
        maps.text,
        "the field shows the second file"
    );
    let text = screen_text(app.world());
    assert!(text.contains("text/en/maps.ron"), "{text}");
    // Typing then carries the second file's text on, never the first's.
    type_into(&mut app, EditAct::FileText, &format!("{} ", maps.text));
    assert_eq!(
        editing(&app).file.as_ref().unwrap().text,
        format!("{} ", maps.text)
    );
}

/// A column with more rows than its height scrolls; its rows never shrink into one another (the
/// owner, 2026-10-25: "button overlap ... in the left column"). The document's row is the one
/// that gives up its height; every other row keeps its content's.
#[test]
fn a_crowded_column_s_rows_keep_their_height_and_the_panel_scrolls() {
    use common::app::ui_editor_app;
    use common::editor::panel;
    let (mut app, _window) = ui_editor_app();
    // The grids made the column fit the test window (2026-10-25): a palette of fifty more
    // terrains, a swatch each, crowds it again.
    let data = app.world().resource::<Packs>().data.clone();
    {
        let mut e = app.world_mut().resource_mut::<Editing>();
        let def = e.def().expect("open");
        let base = def.terrains[0].clone();
        for glyph in ('A'..='Z').chain('a'..='z').chain('0'..='9') {
            if def.terrains.iter().any(|t| t.glyph == glyph) {
                continue;
            }
            let terrain = omnis_sim::omnis_data::Terrain {
                glyph,
                name: format!("terrain {glyph}"),
                ..base.clone()
            };
            e.session
                .document_mut()
                .and_then(|d| d.edit(omnis_sim::omnis_data::edit::Edit::TerrainAdd(terrain)))
                .expect("a terrain adds");
        }
        e.changed(&data);
    }
    for _ in 0..3 {
        app.update();
    }
    let (tools, _) = panel(&mut app, Column::Tools);
    let world = app.world_mut();
    let area = world.get::<ComputedNode>(tools).unwrap();
    assert!(
        area.content_size().y > area.size().y + 0.5,
        "the tools run past the column: {} in {}",
        area.content_size().y,
        area.size().y
    );
    let rows: Vec<Entity> = world.get::<Children>(tools).unwrap().iter().collect();
    assert!(rows.len() > 5, "{} rows", rows.len());
    for row in rows {
        let tallest = world
            .get::<Children>(row)
            .map(|children| {
                children
                    .iter()
                    .filter_map(|c| world.get::<ComputedNode>(c))
                    .map(|n| n.size().y)
                    .fold(0.0, f32::max)
            })
            .unwrap_or(0.0);
        let height = world.get::<ComputedNode>(row).unwrap().size().y;
        assert!(
            height + 0.5 >= tallest,
            "a row of {height} px holds a control of {tallest} px"
        );
    }
}

/// Opening a map from the tools shows its plan in the centre, whatever was shown before (the
/// owner, 2026-10-25: "when I switch to opening a map with the map selector to edit the old .ron
/// file editor remains and I don't get the map editing view in the central column"). The open
/// text file stays open behind the plan, a press on Text away.
#[test]
fn opening_a_map_from_the_tools_shows_its_plan_over_the_open_file() {
    let mut app = in_editor("test:map:town");
    open_file(&mut app, "text/en/spells.ron");
    assert_eq!(display(&mut app, Column::Text), Display::Flex);
    let first = editing(&app).chosen;
    arrow(&mut app, EditAct::Map, true);
    let at = editing(&app).chosen;
    assert_ne!(at, first, "the choice stepped");
    let chosen = editing(&app).maps[at].id.clone();
    assert_ne!(Some(chosen.clone()), editing(&app).map_id());
    press_edit(&mut app, EditAct::Open);
    assert_eq!(editing(&app).map_id(), Some(chosen));
    assert_eq!(
        editing(&app).mode,
        Mode::Plan,
        "the map opened: its plan shows"
    );
    assert_eq!(display(&mut app, Column::Plan), Display::Flex);
    assert_eq!(display(&mut app, Column::Text), Display::None);
    // The file is still open: Text brings it back as it was.
    assert_eq!(reason(&app, EditAct::Files), None);
    press_edit(&mut app, EditAct::Files);
    assert_eq!(editing(&app).mode, Mode::Files);
    assert_eq!(display(&mut app, Column::Text), Display::Flex);
    assert_eq!(
        editing(&app).file.as_ref().map(|f| f.rel.as_str()),
        Some("text/en/spells.ron")
    );
}

#[test]
fn a_broken_edit_is_checked_in_the_loader_s_words_and_reverted() {
    let mut app = in_editor("test:map:town");
    open_file(&mut app, "text/en/spells.ron");
    let disk = editing(&app).file.clone().unwrap().disk;
    let broken = disk.replacen('"', "", 1);
    type_into(&mut app, EditAct::FileText, &broken);
    assert_eq!(editing(&app).file.as_ref().unwrap().text, broken);
    assert!(editing(&app).file.as_ref().unwrap().dirty());
    assert_eq!(
        reason(&app, EditAct::WriteFile).as_deref(),
        Some("check first")
    );
    let text = screen_text(app.world());
    assert!(text.contains("not checked since the last change"), "{text}");
    press_edit(&mut app, EditAct::Check);
    let errors = editing(&app)
        .file
        .as_ref()
        .unwrap()
        .errors
        .clone()
        .expect("checked");
    assert!(!errors.is_empty(), "the loader refuses a lost quote");
    assert!(reason(&app, EditAct::WriteFile).is_some_and(|r| r.contains("error")));
    let text = screen_text(app.world());
    assert!(
        text.contains(&errors[0]),
        "the words in the inspector: {text}"
    );
    // Typing again clears the verdict; Revert file restores the disk text.
    type_into(&mut app, EditAct::FileText, &format!("{broken} "));
    assert_eq!(editing(&app).file.as_ref().unwrap().errors, None);
    press_edit(&mut app, EditAct::RevertFile);
    let file = editing(&app).file.clone().unwrap();
    assert_eq!(file.text, disk);
    assert!(!file.dirty() && file.errors.is_none());
    assert_eq!(
        field_text(&mut app),
        disk,
        "the field shows the file's text again"
    );
}

#[test]
fn a_checked_edit_is_written_as_typed_and_resealed_and_the_map_s_own_file_reopens_it() {
    let (mut app, scratch) = in_editor_scratch("editor-files-write", "test:map:town");
    let version = |app: &App| {
        app.world()
            .resource::<Packs>()
            .data
            .packs
            .iter()
            .find(|p| p.id == "test")
            .map(|p| p.version.clone())
            .unwrap()
    };
    let before = version(&app);
    open_file(&mut app, "text/en/spells.ron");
    let disk = editing(&app).file.clone().unwrap().disk;
    // A comment at the top: the text differs, the file still loads, and the comment survives.
    let typed = format!("// Edited in the RON editor.\n{disk}");
    type_into(&mut app, EditAct::FileText, &typed);
    press_edit(&mut app, EditAct::Check);
    assert_eq!(
        editing(&app).file.as_ref().unwrap().errors,
        Some(Vec::new())
    );
    assert_eq!(reason(&app, EditAct::WriteFile), None);
    press_edit(&mut app, EditAct::WriteFile);
    let written = std::fs::read_to_string(scratch.join("text/en/spells.ron")).unwrap();
    assert_eq!(written, typed, "exactly as typed");
    assert!(!editing(&app).file.as_ref().unwrap().dirty());
    assert_ne!(version(&app), before, "resealed and the packs reloaded");
    assert!(
        editing(&app)
            .message
            .as_deref()
            .is_some_and(|m| m.contains("sealed at")),
        "{:?}",
        editing(&app).message
    );
    // The open map's own file written: the document reopens as the file, the cursor kept, a
    // paint made before (and not in the text) gone with the dirty mark.
    let data = app.world().resource::<Packs>().data.clone();
    {
        let mut e = app.world_mut().resource_mut::<Editing>();
        e.point((3, 3));
        e.tool = omnis_vector::editor::brush::Tool::Brush;
        e.entry =
            omnis_vector::editor::brush::Entry::Edge(omnis_sim::omnis_data::edit::EdgeKind::Wall);
        let cell = e.cursor;
        assert!(e.paint(
            &data,
            omnis_vector::editor::brush::Target {
                cell,
                edge: Some(omnis_sim::omnis_core::Facing::East)
            },
            false
        ));
    }
    app.update();
    assert!(editing(&app).status.dirty, "a paint not in the file's text");
    open_file(&mut app, "data/maps/town.ron");
    let disk = editing(&app).file.clone().unwrap().disk;
    let typed = format!("// The town, with a comment.\n{disk}");
    type_into(&mut app, EditAct::FileText, &typed);
    press_edit(&mut app, EditAct::Check);
    press_edit(&mut app, EditAct::WriteFile);
    let e = editing(&app);
    assert_eq!(e.cursor, (3, 3), "the cursor kept");
    assert_eq!(e.mode, Mode::Files, "the text stays in the centre");
    assert!(!e.status.dirty, "the document is the file's again");
    assert_eq!(
        e.grid()
            .unwrap()
            .edge(3, 3, omnis_sim::omnis_core::Facing::East),
        Some(omnis_sim::omnis_data::edit::EdgeKind::Open),
        "the paint not written is gone"
    );
    assert!(e.resolved.is_some());
    assert!(
        std::fs::read_to_string(scratch.join("data/maps/town.ron"))
            .unwrap()
            .starts_with("// The town")
    );
    // A note file is written without a reseal.
    let sealed = version(&app);
    std::fs::create_dir_all(scratch.join("notes")).unwrap();
    std::fs::write(
        scratch.join("notes/town.ron"),
        "(\n    schema: 1,\n    entries: {},\n)\n",
    )
    .unwrap();
    open_file(&mut app, "notes/town.ron");
    let typed = "(\n    schema: 1,\n    entries: {\n        \"test:map:town/0/3,3\": \"A note.\",\n    },\n)\n";
    type_into(&mut app, EditAct::FileText, typed);
    press_edit(&mut app, EditAct::Check);
    press_edit(&mut app, EditAct::WriteFile);
    assert_eq!(version(&app), sealed, "a note is not sealed");
    assert_eq!(
        editing(&app).note(omnis_vector::editor::NoteOf::Cell),
        Some("A note."),
        "the note reads back"
    );
}

/// The owner's path (2026-10-24, second round), under the window's condition: Bevy's layout marks
/// every field changed every frame, so in the frame of the pick the old field, still showing the
/// first file, read as typed into the second ("pressing revert removes the first loaded file and
/// reveals the second selected file"). A field drawn for an older document is stale: its text
/// never flows into a fresh one.
#[test]
fn a_second_file_through_the_picker_under_the_window_s_churn_keeps_its_own_text() {
    use common::app::{churn_fields, keyboard, keyboard_on};
    let (mut app, window) = keyboard_on(in_editor("test:map:town"));
    churn_fields(&mut app);
    app.update();
    open_file(&mut app, "text/en/spells.ron");
    let spells = editing(&app).file.clone().unwrap().text;
    assert_eq!(field_text(&mut app), spells);
    open_file(&mut app, "text/en/maps.ron");
    for _ in 0..3 {
        app.update();
    }
    let maps = editing(&app).file.clone().unwrap();
    assert_eq!(maps.rel, "text/en/maps.ron");
    assert_eq!(
        maps.text, maps.disk,
        "the second file's own text, not the first's"
    );
    assert!(!maps.dirty());
    assert_eq!(
        field_text(&mut app),
        maps.disk,
        "the field shows the second file"
    );
    // The field has the keyboard: a character typed is the file's text now.
    keyboard(&mut app, window, "w");
    app.update();
    let file = editing(&app).file.clone().unwrap();
    assert!(
        file.dirty(),
        "a key typed edits the file: {:?}",
        file.text.len()
    );
    assert_eq!(file.text.len(), maps.disk.len() + 1);
}

/// The document control under Bevy's layout (M5.9c): the open file is one field with every line
/// shown and no wrapping, its line numbers beside it in the same scrolling area, a scrollbar on
/// each axis; typing a line adds a number and a line.
#[test]
fn the_document_is_numbered_unwrapped_and_scrolls_on_both_axes() {
    use bevy::text::{EditableText, LineBreak, TextLayout};
    use bevy::ui_widgets::{ControlOrientation, Scrollbar};
    use common::app::ui_editor_app;
    use omnis_vector::shell::document::{DocumentField, Gutter, lines, longest_line, numbers};
    use omnis_vector::shell::screen::Scrolls;
    let (mut app, _window) = ui_editor_app();
    // Under Bevy's picking a faked press is reset before the route reads it: the model is
    // driven directly here, the picker's own path being the other tests'.
    let open = |app: &mut App, act: EditAct| {
        app.world_mut()
            .resource_scope(|world, mut editing: Mut<Editing>| {
                let mut packs = world.resource_mut::<Packs>();
                omnis_vector::shell::editor::act(&mut editing, &mut packs, act, 0)
            })
    };
    open(&mut app, EditAct::OpenFile);
    let i = file_index(&app, "text/en/spells.ron");
    open(&mut app, EditAct::File(i));
    assert_eq!(editing(&app).mode, Mode::Files);
    for _ in 0..3 {
        app.update();
    }
    let field = field_entity(&mut app, EditAct::FileText).expect("the file's field");
    let text = editing(&app).file.clone().unwrap().text;
    let world = app.world_mut();
    assert!(world.get::<DocumentField>(field).is_some());
    let editable = world.get::<EditableText>(field).unwrap();
    assert_eq!(editable.visible_lines, Some(lines(&text) as f32));
    assert_eq!(
        editable.visible_width,
        Some(longest_line(&text) as f32 + 1.0)
    );
    assert!(editable.allow_newlines);
    assert_eq!(
        world.get::<TextLayout>(field).unwrap().linebreak,
        LineBreak::NoWrap
    );
    // The gutter numbers the field's lines, in the same scrolling area.
    let area = world.get::<ChildOf>(field).unwrap().parent();
    assert!(world.get::<Scrolls>(area).is_some(), "the area scrolls");
    assert!(world.get::<ScrollPosition>(area).is_some());
    assert_eq!(
        world.get::<Node>(area).unwrap().overflow,
        Overflow::scroll()
    );
    let mut gutters = world.query::<(&Gutter, &Text, &ChildOf)>();
    let (_, words, of) = gutters
        .iter(world)
        .find(|(g, _, _)| g.0 == field)
        .expect("the gutter");
    assert_eq!(words.0, numbers(lines(&text)));
    assert_eq!(of.parent(), area, "beside the field");
    // A scrollbar on each axis, both for the area.
    let mut bars = world.query::<&Scrollbar>();
    let along: Vec<ControlOrientation> = bars
        .iter(world)
        .filter(|b| b.target == area)
        .map(|b| b.orientation)
        .collect();
    assert_eq!(along.len(), 2, "{along:?}");
    assert!(along.contains(&ControlOrientation::Vertical));
    assert!(along.contains(&ControlOrientation::Horizontal));
    // Laid out: the area has a size, and the field is as tall as its lines.
    let area_size = world.get::<ComputedNode>(area).unwrap().size();
    assert!(area_size.x > 0.0 && area_size.y > 0.0, "{area_size}");
    // A line typed: one more number, one more visible line, a wider field for a longer line.
    let longer =
        format!("{text}\n// A new last line that is rather longer than the others in this file.");
    type_into(&mut app, EditAct::FileText, &longer);
    app.update();
    let world = app.world_mut();
    let editable = world.get::<EditableText>(field).unwrap();
    assert_eq!(editable.visible_lines, Some(lines(&longer) as f32));
    assert_eq!(
        editable.visible_width,
        Some(longest_line(&longer) as f32 + 1.0)
    );
    let mut gutters = world.query::<(&Gutter, &Text)>();
    let (_, words) = gutters.iter(world).find(|(g, _)| g.0 == field).unwrap();
    assert_eq!(words.0, numbers(lines(&longer)));
    assert_eq!(editing(&app).file.as_ref().unwrap().text, longer);
}
