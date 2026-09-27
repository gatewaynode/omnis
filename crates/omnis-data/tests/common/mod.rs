//! Shared paths for the data crate's integration tests.
#![allow(dead_code)]

use std::path::PathBuf;

/// The repository's test fixture pack.
pub fn test_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/test")
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
