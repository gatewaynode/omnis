//! Flag files (PRD D28 and §7.7, M8.5 7c): a pack declares whether a flag is the party's or the
//! world's; an undeclared flag is the world's. Declared flags are interned after the ones object
//! kinds name, so a declaration never moves a number a save already holds.

use crate::common::{bad_pack, base_pack, scratch, test_pack};
use omnis_data::{FlagScope, load_packs};
use std::path::PathBuf;

fn objects_pack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../omnis-sim/tests/packs/objects")
}

#[test]
fn a_pack_declares_each_flags_scope() {
    let data = load_packs(&[&base_pack(), &test_pack(), &objects_pack()])
        .unwrap_or_else(|r| panic!("{r}"));
    let flags = &data.registry.flags;
    let lever = flags.get("objects:flag:lever").expect("the lever's flag");
    let debt = flags
        .get("objects:flag:debt")
        .expect("declared, though no object names it");
    assert_eq!(data.flag_scope(debt), FlagScope::Party);
    assert_eq!(data.flag_scope(lever), FlagScope::World);
    assert_eq!(
        u32::from(lever),
        0,
        "the object's flag keeps its number: declarations come after"
    );
}

#[test]
fn an_undeclared_flag_and_a_flag_without_a_scope_are_the_worlds() {
    let root = scratch("flags-default");
    std::fs::create_dir_all(root.join("data/flags")).unwrap();
    std::fs::write(
        root.join("pack.ron"),
        r#"(schema: 1, id: "plain", version: "0.1.0", name: "plain", license: "MIT",
            attribution: [], depends: ["base"], entry: None)"#,
    )
    .unwrap();
    std::fs::write(
        root.join("data/flags/rumour.ron"),
        r#"(schema: 1, id: "plain:flag:rumour")"#,
    )
    .unwrap();
    let data = load_packs(&[&base_pack(), &root]).unwrap_or_else(|r| panic!("{r}"));
    let rumour = data.registry.flags.get("plain:flag:rumour").unwrap();
    assert_eq!(data.flag_scope(rumour), FlagScope::World);
    assert!(data.flags.contains_key(&rumour));
    assert_eq!(
        data.flag_scope(omnis_core::FlagId(4_000)),
        FlagScope::World,
        "a flag no file declares"
    );
}

#[test]
fn a_bad_flag_file_is_refused() {
    let report = load_packs(&[&base_pack(), &bad_pack("bad-flags")]).unwrap_err();
    let text = report.to_string();
    assert!(text.contains("must have type 'flag'"), "{text}");
    assert!(text.contains("schema 2 is not supported"), "{text}");
    assert!(text.contains("Unexpected variant named `Guild`"), "{text}");
}
