//! The seal (M8.5 7h1): every pack records its version and the hash of its content in
//! `pack.ron`, and the loader refuses a pack whose content no longer matches, so a pack never
//! changes under the same version. `seal` records the hash and, when the content changed, raises
//! the version.

use crate::common::{base_pack, scratch, test_pack};
use omnis_data::load_packs;
use omnis_data::seal::{seal, status};
use std::path::{Path, PathBuf};

const MANIFEST: &str = r#"// A pack written by the seal tests.
(
    schema: 1,
    id: "plain",
    version: "0.1.0",
    name: "plain",
    license: "MIT",
    depends: ["base"],
)
"#;

fn plain(name: &str) -> PathBuf {
    let root = scratch(name);
    std::fs::create_dir_all(root.join("data/flags")).unwrap();
    std::fs::write(root.join("pack.ron"), MANIFEST).unwrap();
    rumour(&root, "World");
    root
}

fn rumour(root: &Path, scope: &str) {
    std::fs::write(
        root.join("data/flags/rumour.ron"),
        format!("(schema: 1, id: \"plain:flag:rumour\", scope: {scope})"),
    )
    .unwrap();
}

fn refusal(root: &Path) -> String {
    load_packs(&[&base_pack(), root])
        .map(|_| ())
        .unwrap_err()
        .to_string()
}

#[test]
fn an_unsealed_pack_is_refused_and_sealing_it_lets_it_load() {
    let root = plain("seal-first");
    assert!(refusal(&root).contains("not sealed"), "{}", refusal(&root));
    let sealed = seal(&root, None).unwrap_or_else(|e| panic!("{e}"));
    assert!(sealed.changed);
    assert_eq!(sealed.version, "0.1.0", "a first seal keeps the version");
    let text = std::fs::read_to_string(root.join("pack.ron")).unwrap();
    assert!(
        text.starts_with("// A pack written"),
        "comments kept: {text}"
    );
    assert!(text.contains(&format!("hash: {},", sealed.hash)), "{text}");
    let data = load_packs(&[&base_pack(), &root]).unwrap_or_else(|r| panic!("{r}"));
    let fingerprint = data.fingerprints.iter().find(|f| f.id == "plain").unwrap();
    assert_eq!(
        fingerprint.hash, sealed.hash,
        "the recorded hash is the fingerprint's"
    );
    let again = seal(&root, None).unwrap();
    assert!(!again.changed, "nothing changed, nothing written");
    assert_eq!(
        std::fs::read_to_string(root.join("pack.ron")).unwrap(),
        text
    );
}

#[test]
fn a_changed_pack_is_refused_until_resealed_at_a_new_version() {
    let root = plain("seal-changed");
    let first = seal(&root, None).unwrap();
    rumour(&root, "Party");
    let refused = refusal(&root);
    assert!(refused.contains("reseal"), "{refused}");
    let second = seal(&root, None).unwrap();
    assert!(second.changed);
    assert_eq!(second.version, "0.1.1", "the patch number goes up");
    assert_ne!(second.hash, first.hash);
    load_packs(&[&base_pack(), &root]).unwrap_or_else(|r| panic!("{r}"));
    // A version given is the version written, and the hash follows it.
    let third = seal(&root, Some("0.2.0")).unwrap();
    assert_eq!(third.version, "0.2.0");
    assert_ne!(third.hash, second.hash, "the version is content");
    load_packs(&[&base_pack(), &root]).unwrap_or_else(|r| panic!("{r}"));
}

#[test]
fn the_hash_ignores_the_manifest_s_layout_and_its_own_field() {
    let root = plain("seal-layout");
    let sealed = seal(&root, None).unwrap();
    // Comments, spacing and the order of lines in pack.ron are not content.
    let text = std::fs::read_to_string(root.join("pack.ron")).unwrap();
    let respaced = text.replace("// A pack written by the seal tests.\n", "") + "\n\n";
    std::fs::write(root.join("pack.ron"), respaced).unwrap();
    let data = load_packs(&[&base_pack(), &root]).unwrap_or_else(|r| panic!("{r}"));
    assert!(data.fingerprints.iter().any(|f| f.hash == sealed.hash));
}

#[test]
fn the_shipped_packs_are_sealed() {
    for root in [base_pack(), test_pack()] {
        let status = status(&root).unwrap();
        assert_eq!(
            status.recorded,
            Some(status.hash),
            "{} needs `omnis-cli pack seal`",
            root.display()
        );
    }
}
