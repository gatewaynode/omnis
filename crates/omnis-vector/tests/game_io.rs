//! Saves and packs read with the loader's limits: a save file over the size cap or a link is
//! refused with the reason, and a pack directory that is not there stops the start with its name.

use crate::common::{data, session};
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
    let (world, _) = session(&data);
    game_io::save(&world, &root, "real.ron").unwrap();
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
