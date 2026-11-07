//! Shared paths for the data crate's integration tests.
#![allow(dead_code)]

use std::path::PathBuf;

/// The repository's test fixture pack.
pub fn test_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/omnis-sim/tests/packs/test")
}

/// The base content pack, which the test pack depends on.
pub fn base_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/base")
}

/// The test pack after the base pack it depends on, the order every load of it uses.
pub fn test_packs() -> [PathBuf; 2] {
    [base_pack(), test_pack()]
}

/// Load the base and test packs.
pub fn load_test_packs() -> omnis_data::Data {
    let [base, test] = test_packs();
    omnis_data::load_packs(&[&base, &test]).unwrap_or_else(|report| panic!("{report}"))
}

/// A fixture pack of the simulation's tests (`levels`, `heights`, `objects`, ...), which depends
/// on the base and test packs.
pub fn fixture_pack(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../omnis-sim/tests/packs")
        .join(name)
}

/// A known-bad pack from the corpus under `tests/packs-bad/`.
pub fn bad_pack(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/packs-bad")
        .join(name)
}

/// A fresh scratch directory under the build tree for tests that write files.
pub fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A copy of the pack at `src` under a fresh scratch directory `name`: `pack.ron` and the
/// directories the loader reads, so tests may write and reseal without touching the repository.
pub fn copy_pack(src: &std::path::Path, name: &str) -> PathBuf {
    let dst = scratch(name);
    std::fs::copy(src.join("pack.ron"), dst.join("pack.ron")).expect("pack.ron");
    for dir in ["data", "text", "notes", "migrations"] {
        copy_dir(&src.join(dir), &dst.join(dir));
    }
    dst
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(src) else {
        return;
    };
    std::fs::create_dir_all(dst).expect("copy dir");
    for entry in entries {
        let entry = entry.expect("entry");
        let to = dst.join(entry.file_name());
        if entry.file_type().expect("type").is_dir() {
            copy_dir(&entry.path(), &to);
        } else {
            std::fs::copy(entry.path(), to).expect("copy file");
        }
    }
}
