//! Known-bad packs are refused with the complete expected error list, never a panic.

mod common;

use omnis_data::load_packs;
use std::path::Path;

fn messages(name: &str) -> Vec<String> {
    match load_packs(&[&common::bad_pack(name)]) {
        Ok(_) => panic!("{name} loaded"),
        Err(report) => report.messages(),
    }
}

#[test]
fn missing_manifest() {
    let m = messages("missing-manifest");
    assert_eq!(m.len(), 1);
    assert!(m[0].starts_with("cannot read:"), "{m:?}");
}

#[test]
fn unsupported_schema() {
    assert_eq!(
        messages("bad-schema"),
        ["schema 2 is not supported; this build reads schema 1"]
    );
}

#[test]
fn syntax_error_names_the_line() {
    let report = load_packs(&[&common::bad_pack("syntax-error")]).unwrap_err();
    assert_eq!(report.errors.len(), 1);
    let e = &report.errors[0];
    assert_eq!(
        e.line,
        Some(5),
        "the missing comma is noticed at the next field"
    );
    assert!(e.message.starts_with("parse error:"), "{}", e.message);
    assert!(e.file.ends_with(Path::new("syntax-error/pack.ron")));
}

#[test]
fn every_error_in_a_broken_pack_is_reported() {
    let expected = [
        // pack.ron
        "pack id 'Broken' is not [a-z0-9_-]+",
        "depends on 'base', which is not loaded before this pack",
        // data/tiles/bad.ron
        "surface 'floor' declares (0, 0) twice",
        "surface 'floor' slot depth 5 is beyond detail_depth",
        "surface 'floor' slot offset 3 is beyond width",
        "surface 'floor' slot path '../../etc/passwd.png': path must not contain '.' or '..' components",
        "surface 'floor' slot path 'assets/floor.exe': file type is not allowed",
        // data/maps/glyph.ron
        "terrain glyph '.' is used twice",
        "terrain glyph '-' is reserved for edges",
        "terrain 'floor' visibility_depth must be 1..=32",
        "start (2, 0) is outside the map",
        "portal at (5, 5) is outside the map",
        "tile (1, 0) has unknown glyph '?'",
        // data/maps/refs.ron: nothing structural
        // data/maps/rows.ron
        "layout row 1 has 7 characters; a 2-tile-wide map needs 5",
        // text/en/strings.ron
        "id 'not-an-id' is not of the form pack:text:name",
        // resolution, maps in id order: glyph, refs, rows
        "wall front surface 'nowhere' is not in tileset 'broken:tileset:bad'",
        "wall left surface 'floor' is a Floor surface, not WallLeft",
        "terrain floor surface 'sky' is a Ceiling surface, not Floor",
        "terrain ceiling surface 'floor' is a Floor surface, not Ceiling",
        "name text key 'broken:text:map.refs.name' is not defined in any language",
        "portal at (0, 0) leads to unknown map 'broken:map:nowhere'",
        "portal at (0, 0) lands outside map 'broken:map:glyph'",
        "tileset 'broken:tileset:missing' is not defined by any loaded pack",
    ];
    let got = messages("broken");
    let missing: Vec<&&str> = expected
        .iter()
        .filter(|e| !got.contains(&(**e).to_owned()))
        .collect();
    let extra: Vec<&String> = got
        .iter()
        .filter(|g| !expected.contains(&g.as_str()))
        .collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "missing: {missing:#?}\nextra: {extra:#?}\nall: {got:#?}"
    );
    assert_eq!(got.len(), expected.len(), "each reported once");
}

#[test]
fn declared_size_is_checked_before_the_layout_is_trusted() {
    let m = messages("too-big");
    assert!(
        m.contains(&"size 300x300 must be within 1x1 and 256x256".to_owned()),
        "{m:?}"
    );
    assert!(
        m.contains(&"layout has 1 rows; a 300-tile-high map needs 601".to_owned()),
        "{m:?}"
    );
}

#[cfg(unix)]
#[test]
fn symlinked_files_are_refused() {
    let dir = common::scratch("symlink-pack");
    std::os::unix::fs::symlink(common::test_pack().join("pack.ron"), dir.join("pack.ron")).unwrap();
    let m = match load_packs(&[&dir]) {
        Ok(_) => panic!("loaded through a symlink"),
        Err(report) => report.messages(),
    };
    assert_eq!(m, ["symlinks are not allowed in packs"]);
}

#[test]
fn dependencies_must_load_first() {
    let dir = common::scratch("dep-pack");
    std::fs::write(
        dir.join("pack.ron"),
        r#"(schema: 1, id: "mod", version: "0.1.0", name: "a mod", license: "MIT", depends: ["test"])"#,
    )
    .unwrap();
    assert!(load_packs(&[&dir]).is_err());
    let data = load_packs(&[&common::test_pack(), &dir]).unwrap_or_else(|r| panic!("{r}"));
    assert_eq!(
        data.packs.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
        ["test", "mod"]
    );
    assert_eq!(
        data.maps.len(),
        2,
        "the mod adds nothing and removes nothing"
    );
    assert_eq!(
        data.entry,
        data.registry.maps.get("test:map:meadow"),
        "the mod sets no entry, so the base's stands"
    );

    let dir = common::scratch("entry-pack");
    std::fs::write(
        dir.join("pack.ron"),
        r#"(schema: 1, id: "mod", version: "0.1.0", name: "a mod", license: "MIT", depends: ["test"], entry: Some("mod:map:none"))"#,
    )
    .unwrap();
    let report = load_packs(&[&common::test_pack(), &dir]).unwrap_err();
    assert_eq!(
        report.messages(),
        ["entry map 'mod:map:none' is not defined by any loaded pack"]
    );
}
