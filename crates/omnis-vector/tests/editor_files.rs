//! The RON text editor (M5.9): Files mode lists the pack's files, opens one as text in the
//! centre, checks it with the loader's own words, writes it exactly as typed and reseals, and
//! reopens the map when its own file was written.

use crate::common;

use bevy::prelude::*;
use common::editor::{editing, in_editor, in_editor_scratch, press_edit, type_into};
use omnis_vector::editor::EditAct;
use omnis_vector::editor::files::Mode;
use omnis_vector::editor::view;
use omnis_vector::screen::ControlId;
use omnis_vector::shell::editor::{Column, Editing};
use omnis_vector::shell::screen_text::screen_text;
use omnis_vector::shell::session::Packs;

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

/// Open the listed file `rel` as text.
fn open_file(app: &mut App, rel: &str) {
    let i = file_index(app, rel);
    press_edit(app, EditAct::File(i));
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
fn files_mode_lists_the_pack_s_files_and_opens_one_as_text_in_the_centre() {
    let mut app = in_editor("test:map:town");
    assert_eq!(editing(&app).mode, Mode::Plan);
    assert_eq!(display(&mut app, Column::Plan), Display::Flex);
    assert_eq!(display(&mut app, Column::Text), Display::None);
    press_edit(&mut app, EditAct::Files);
    assert_eq!(editing(&app).mode, Mode::Files);
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
        text.contains("text/en/spells.ron") && text.contains("Choose a file"),
        "{text}"
    );
    assert_eq!(display(&mut app, Column::Plan), Display::None);
    assert_eq!(display(&mut app, Column::Text), Display::Flex);
    assert_eq!(
        reason(&app, EditAct::Check).as_deref(),
        Some("choose a file")
    );
    // Open the spells' text: the model holds the file, the centre shows it.
    open_file(&mut app, "text/en/spells.ron");
    let file = editing(&app).file.clone().expect("the file");
    assert_eq!(file.rel, "text/en/spells.ron");
    assert_eq!(file.text, file.disk);
    assert!(file.text.contains("ward"), "{}", file.text);
    let text = screen_text(app.world());
    assert!(text.contains("> text/en/spells.ron"), "{text}");
    assert!(text.contains("ward"), "the text in the centre: {text}");
    assert_eq!(
        reason(&app, EditAct::WriteFile).as_deref(),
        Some("no changes")
    );
    assert_eq!(
        reason(&app, EditAct::RevertFile).as_deref(),
        Some("no changes")
    );
    // Plan returns to the map; the file stays open for next time.
    press_edit(&mut app, EditAct::Plan);
    assert_eq!(editing(&app).mode, Mode::Plan);
    assert_eq!(display(&mut app, Column::Plan), Display::Flex);
    assert!(editing(&app).file.is_some());
}

#[test]
fn a_broken_edit_is_checked_in_the_loader_s_words_and_reverted() {
    let mut app = in_editor("test:map:town");
    press_edit(&mut app, EditAct::Files);
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
    press_edit(&mut app, EditAct::Files);
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
        e.tool =
            omnis_vector::editor::brush::Tool::Edge(omnis_sim::omnis_data::edit::EdgeKind::Wall);
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
    press_edit(&mut app, EditAct::Files);
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
