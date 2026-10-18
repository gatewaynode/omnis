//! Saves and packs read with the loader's limits: a save file over the size cap or a link is
//! refused with the reason, and a pack directory that is not there stops the start with its name.

use crate::common::{data, town_session};
use omnis_sim::omnis_data::limits::MAX_FILE_BYTES;
use omnis_vector::game_io;
use omnis_vector::shell::session::{Config, Packs};
use std::path::PathBuf;

#[test]
fn a_save_is_read_with_the_loader_s_limits() {
    let data = data();
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("vector-save-limits");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let (world, _) = town_session(&data);
    game_io::save(&world, &data, &root, "real.ron").unwrap();
    assert!(game_io::load(&data, &root, "real.ron", false).is_ok());

    let size = usize::try_from(MAX_FILE_BYTES).unwrap() + 1;
    std::fs::write(root.join("big.ron"), vec![b' '; size]).unwrap();
    let refused = game_io::load(&data, &root, "big.ron", false).unwrap_err();
    assert!(refused.to_string().contains("limit"), "{refused}");

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("real.ron"), root.join("link.ron")).unwrap();
        let refused = game_io::load(&data, &root, "link.ron", false).unwrap_err();
        assert!(refused.to_string().contains("symlink"), "{refused}");
    }
}

#[test]
fn a_missing_pack_stops_the_start_with_its_name() {
    let config = Config {
        packs: vec![PathBuf::from("no/such/pack")],
        ..Config::default()
    };
    let Err(why) = Packs::load(&config) else {
        panic!("a pack that is not there loads");
    };
    assert!(why.contains("no/such/pack"), "{why}");
}

/// Tidy (M8.5 7e) lists the world-folder files no save under `.omnis/` names and deletes them
/// only when asked; the files a save names, and anything not a world-folder file, stay.
#[test]
fn tidy_lists_what_no_save_names_and_deletes_only_when_asked() {
    let data = data();
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("vector-tidy");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let (world, _) = town_session(&data);
    game_io::save(&world, &data, &root, ".omnis/one.ron").unwrap();
    let worlds = root.join(".omnis/worlds");
    let id = std::fs::read_dir(&worlds)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .file_name()
        .into_string()
        .unwrap();
    let stray = format!(".omnis/worlds/{id}/world/0123456789abcdef.ron");
    std::fs::create_dir_all(root.join(format!(".omnis/worlds/{id}/world"))).unwrap();
    std::fs::write(root.join(&stray), "(flags: {})").unwrap();
    std::fs::write(root.join(".omnis/worlds/notes.ron"), "(mine)").unwrap();
    let listed = game_io::tidy(&root, false).unwrap();
    assert_eq!(listed.files, std::slice::from_ref(&stray));
    assert_eq!(listed.bytes, 11);
    assert!(root.join(&stray).is_file(), "listing deletes nothing");
    let deleted = game_io::tidy(&root, true).unwrap();
    assert_eq!(deleted.files, std::slice::from_ref(&stray));
    assert!(!root.join(&stray).exists());
    assert!(
        root.join(format!(".omnis/worlds/{id}/manifest.ron"))
            .is_file()
    );
    assert!(
        root.join(".omnis/worlds/notes.ron").is_file(),
        "not ours, never listed"
    );
    std::fs::remove_file(root.join(".omnis/one.ron")).unwrap();
    let orphan = game_io::tidy(&root, false).unwrap();
    assert_eq!(
        orphan.files,
        [format!(".omnis/worlds/{id}/manifest.ron")],
        "a world no save names goes whole"
    );
    std::fs::write(root.join(".omnis/broken.ron"), "(schema: 8, world: 1)").unwrap();
    assert!(
        game_io::tidy(&root, false).is_err(),
        "a save that does not read stops the tidy"
    );
}
