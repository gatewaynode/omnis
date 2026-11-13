//! Biospheres (presentation-PRD X18, X23; owner 2026-11-13): a palette for the world's lines in
//! pack data. A region names its biosphere, a map may name its own, and a map with neither has
//! none. A terrain may name the role its lines carry.

use crate::common::{base_pack, copy_pack, fixture_pack, scratch, test_pack};
use omnis_data::catalog::{self, Definition};
use omnis_data::{ColourRole, Data, load_packs};
use std::path::Path;

fn fixtures() -> Data {
    load_packs(&[
        &base_pack(),
        &test_pack(),
        &fixture_pack("objects"),
        &fixture_pack("levels"),
    ])
    .unwrap_or_else(|r| panic!("{r}"))
}

fn map(data: &Data, id: &str) -> omnis_core::MapId {
    data.registry.maps.get(id).unwrap_or_else(|| panic!("{id}"))
}

/// `file` under `root` with `old` replaced by `new`, once.
fn rewrite(root: &Path, file: &str, old: &str, new: &str) {
    let path = root.join(file);
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text.matches(old).count(), 1, "{old} in {file}");
    std::fs::write(&path, text.replace(old, new)).unwrap();
}

fn refused(packs: &[&Path]) -> String {
    match load_packs(packs) {
        Ok(_) => panic!("the pack loaded"),
        Err(report) => report.to_string(),
    }
}

#[test]
fn a_map_is_drawn_in_its_own_biosphere_else_its_regions_else_none() {
    let data = fixtures();
    let yard = data.biosphere_of(map(&data, "objects:map:yard"));
    assert_eq!(
        yard.map(|b| b.id.as_str()),
        Some("objects:biosphere:ember"),
        "the yard's region names it"
    );
    let tower = data.biosphere_of(map(&data, "levels:map:tower"));
    assert_eq!(
        tower.map(|b| b.id.as_str()),
        Some("levels:biosphere:deep"),
        "the tower names its own, over its region's frost"
    );
    let town = data.registry.maps.get("test:map:town").expect("the town");
    assert!(data.biosphere_of(town).is_none(), "nobody named one");
}

#[test]
fn a_palette_is_read_as_written_and_a_glow_left_out_is_plain() {
    let data = fixtures();
    let ember = &data.biospheres["objects:biosphere:ember"];
    assert_eq!(ember.palette.len(), ColourRole::ALL.len());
    let structure = ember.palette[&ColourRole::Structure];
    assert_eq!((structure.rgb, structure.glow), ((255, 120, 40), 2800));
    assert_eq!(ember.palette[&ColourRole::Interface].glow, 1000);
    let deep = &data.biospheres["levels:biosphere:deep"];
    assert_eq!(
        deep.palette.keys().copied().collect::<Vec<_>>(),
        [ColourRole::Structure, ColourRole::Passage],
        "a role left out is not invented"
    );
}

#[test]
fn a_role_s_number_is_its_place_in_the_list() {
    for (i, role) in ColourRole::ALL.iter().enumerate() {
        assert_eq!(role.index(), i);
    }
}

#[test]
fn a_terrain_names_its_role() {
    let data = fixtures();
    let yard = &data.maps[&map(&data, "objects:map:yard")].def;
    let role = |name: &str| yard.terrains.iter().find(|t| t.name == name).unwrap().role;
    assert_eq!(role("loft floor"), Some(ColourRole::Structure));
    assert_eq!(role("grass"), None);
}

#[test]
fn a_biosphere_no_pack_defines_is_refused_on_a_map_and_on_a_region() {
    let on_map = copy_pack(&fixture_pack("levels"), "biosphere-map");
    rewrite(
        &on_map,
        "data/maps/tower.ron",
        "levels:biosphere:deep",
        "levels:biosphere:nowhere",
    );
    omnis_data::seal::seal(&on_map, None).unwrap();
    let text = refused(&[&base_pack(), &test_pack(), &on_map]);
    assert!(
        text.contains("biosphere 'levels:biosphere:nowhere' is not defined"),
        "{text}"
    );
    assert!(text.contains("tower.ron"), "{text}");

    let on_region = copy_pack(&fixture_pack("levels"), "biosphere-region");
    rewrite(
        &on_region,
        "data/regions/tower.ron",
        "levels:biosphere:frost",
        "levels:biosphere:lost",
    );
    omnis_data::seal::seal(&on_region, None).unwrap();
    let text = refused(&[&base_pack(), &test_pack(), &on_region]);
    assert!(
        text.contains("biosphere 'levels:biosphere:lost' is not defined"),
        "{text}"
    );
    assert!(text.contains("regions"), "{text}");
}

#[test]
fn a_bad_biosphere_file_is_refused() {
    let root = scratch("biosphere-bad");
    std::fs::create_dir_all(root.join("data/biospheres")).unwrap();
    std::fs::write(
        root.join("pack.ron"),
        r#"(schema: 1, id: "plain", version: "0.1.0", name: "plain", license: "MIT",
            attribution: [], depends: ["base"], entry: None)"#,
    )
    .unwrap();
    let write = |name: &str, text: &str| {
        std::fs::write(root.join("data/biospheres").join(name), text).unwrap();
    };
    write(
        "bright.ron",
        r#"(schema: 1, id: "plain:biosphere:bright", name: "plain:text:none",
            palette: { Danger: (rgb: (255, 0, 0), glow: 4001), Ground: (rgb: (1, 1, 1), glow: 0) })"#,
    );
    write(
        "extra.ron",
        r#"(schema: 1, id: "plain:biosphere:extra", name: "x", relief: 3)"#,
    );
    write(
        "kind.ron",
        r#"(schema: 1, id: "plain:flag:kind", name: "x")"#,
    );
    write(
        "role.ron",
        r#"(schema: 1, id: "plain:biosphere:role", name: "x",
            palette: { Scenery: (rgb: (1, 2, 3)) })"#,
    );
    omnis_data::seal::seal(&root, None).unwrap();
    let text = refused(&[&base_pack(), &root]);
    assert!(text.contains("Danger glows 4001 per mille"), "{text}");
    assert!(text.contains("Ground glows 0 per mille"), "{text}");
    assert!(text.contains("relief"), "an unknown field: {text}");
    assert!(text.contains("must have type 'biosphere'"), "{text}");
    assert!(text.contains("Scenery"), "an unknown role: {text}");
    assert!(
        text.contains("'plain:text:none'"),
        "the name is a text key: {text}"
    );
}

#[test]
fn the_catalogue_lists_biospheres_and_finds_who_names_one() {
    let data = fixtures();
    let kind = catalog::kind("biosphere").expect("a kind as the others are");
    assert_eq!(
        kind.ids(&data),
        [
            "levels:biosphere:deep",
            "levels:biosphere:frost",
            "objects:biosphere:ember"
        ]
    );
    assert_eq!(
        kind.name(&data, "objects:biosphere:ember"),
        Some("objects:text:biosphere.ember.name")
    );
    let Some(Definition::Biosphere(ember)) = catalog::get(&data, "objects:biosphere:ember") else {
        panic!("the definition as written");
    };
    assert_eq!(ember, data.biospheres["objects:biosphere:ember"]);
    // A region and a map that name a biosphere refer to it, found by the same walk as any id.
    let region = catalog::get(&data, "objects:region:yard").unwrap();
    let links = catalog::links(&data, "objects:region:yard", &region);
    assert!(
        links.refers.contains("objects:biosphere:ember"),
        "{links:?}"
    );
    assert_eq!(
        catalog::referrers(&data, "levels:biosphere:deep"),
        ["levels:map:tower"]
    );
    assert_eq!(
        catalog::referrers(&data, "levels:biosphere:frost"),
        ["levels:region:tower"]
    );
}
