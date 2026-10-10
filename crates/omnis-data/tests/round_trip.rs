//! What the writer produces, the loader reads back unchanged: the editor and the game share
//! one code path (PRD §10).

use crate::common;

use omnis_data::ron_io::{from_str, read_ron, to_string, write_ron};
use omnis_data::{MapDef, TextFile, Tileset, load_packs};
use std::path::Path;

#[test]
fn structs_survive_text_and_back() {
    let data = common::load_test_packs();
    for map in data.maps.values() {
        let text = to_string(&map.def).unwrap();
        let back: MapDef = from_str(&text, Path::new("memory")).unwrap();
        assert_eq!(back, map.def, "{}", map.def.id);
    }
    for tileset in data.tilesets.values() {
        let text = to_string(tileset).unwrap();
        let back: Tileset = from_str(&text, Path::new("memory")).unwrap();
        assert_eq!(&back, tileset, "{}", tileset.id);
    }
}

#[test]
fn a_rewritten_pack_loads_identically() {
    let data = common::load_test_packs();
    let dir = common::scratch("rewritten-pack");
    write_ron(&dir.join("pack.ron"), &data.packs[1]).unwrap();
    for (i, map) in data.maps.values().enumerate() {
        write_ron(&dir.join(format!("data/maps/m{i}.ron")), &map.def).unwrap();
    }
    for (i, tileset) in data.tilesets.values().enumerate() {
        write_ron(&dir.join(format!("data/tiles/t{i}.ron")), tileset).unwrap();
    }
    for (i, monster) in data.monsters.values().enumerate() {
        write_ron(&dir.join(format!("data/monsters/m{i}.ron")), monster).unwrap();
    }
    for (i, spell) in data.spells.values().enumerate() {
        write_ron(&dir.join(format!("data/spells/s{i}.ron")), spell).unwrap();
    }
    // Regions keep only their resolved form; the files go across as written.
    let regions = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/test/data/regions");
    for entry in std::fs::read_dir(&regions).unwrap() {
        let path = entry.unwrap().path();
        let to = dir.join("data/regions").join(path.file_name().unwrap());
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(&path, to).unwrap();
    }
    for (lang, table) in &data.text {
        let entries = table
            .iter()
            .map(|(k, v)| (data.registry.text.name(*k).unwrap().to_owned(), v.clone()))
            .collect();
        write_ron(
            &dir.join(format!("text/{lang}/all.ron")),
            &TextFile { schema: 1, entries },
        )
        .unwrap();
    }
    let again = load_packs(&[&common::base_pack(), &dir]).unwrap_or_else(|r| panic!("{r}"));
    assert_eq!(again.maps, data.maps);
    assert_eq!(again.tilesets, data.tilesets);
    assert_eq!(again.monsters, data.monsters);
    assert_eq!(again.spells, data.spells);
    assert_eq!(again.text, data.text);
    assert_eq!(again.regions, data.regions);
    assert_eq!(again.registry, data.registry);
    assert_ne!(
        again.fingerprints, data.fingerprints,
        "formatting differs, so the content hash does"
    );

    let read: MapDef = read_ron(&dir.join("data/maps/m0.ron"), Path::new("m0")).unwrap();
    assert_eq!(read, data.maps.values().next().unwrap().def);
}
