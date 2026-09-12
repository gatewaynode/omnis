//! What the writer produces, the loader reads back unchanged: the editor and the game share
//! one code path (PRD §10).

mod common;

use omnis_data::ron_io::{from_str, read_ron, to_string, write_ron};
use omnis_data::{MapDef, TextFile, Tileset, load_packs};
use std::path::Path;

#[test]
fn structs_survive_text_and_back() {
    let data = load_packs(&[&common::test_pack()]).unwrap_or_else(|r| panic!("{r}"));
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
    let data = load_packs(&[&common::test_pack()]).unwrap_or_else(|r| panic!("{r}"));
    let dir = common::scratch("rewritten-pack");
    write_ron(&dir.join("pack.ron"), &data.packs[0]).unwrap();
    for (i, map) in data.maps.values().enumerate() {
        write_ron(&dir.join(format!("data/maps/m{i}.ron")), &map.def).unwrap();
    }
    for (i, tileset) in data.tilesets.values().enumerate() {
        write_ron(&dir.join(format!("data/tiles/t{i}.ron")), tileset).unwrap();
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
    let again = load_packs(&[&dir]).unwrap_or_else(|r| panic!("{r}"));
    assert_eq!(again.maps, data.maps);
    assert_eq!(again.tilesets, data.tilesets);
    assert_eq!(again.text, data.text);
    assert_eq!(again.registry, data.registry);
    assert_ne!(
        again.fingerprints, data.fingerprints,
        "formatting differs, so the content hash does"
    );

    let read: MapDef = read_ron(&dir.join("data/maps/m0.ron"), Path::new("m0")).unwrap();
    assert_eq!(read, data.maps.values().next().unwrap().def);
}
