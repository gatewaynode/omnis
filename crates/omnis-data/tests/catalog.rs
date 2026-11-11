//! The catalogue (`omnis_data::catalog`, W1's `data.*` ops): every loaded definition by kind and
//! by id behind one table, what each names found by one walk over its strings, and the pack that
//! supplied it.

use crate::common::{base_pack, fixture_pack, scratch, test_pack};
use omnis_data::catalog::{self, Definition, KINDS};
use omnis_data::{Data, load_packs};
use std::collections::BTreeSet;

/// The base pack, the frozen fixture and the objects fixture: between them every kind is loaded.
fn loaded() -> Data {
    load_packs(&[&base_pack(), &test_pack(), &fixture_pack("objects")])
        .unwrap_or_else(|r| panic!("{r}"))
}

/// The kind words the loader gathers, read from its source: the string literal after each
/// `"data/..."` directory it names.
fn gathered_kinds() -> BTreeSet<&'static str> {
    let source = include_str!("../src/loader.rs");
    source
        .match_indices("\"data/")
        .filter_map(|(at, _)| {
            // The directory's own literal, then the text up to the next one, then its contents.
            let mut literals = source[at + 1..].split('"');
            literals.next()?;
            literals.next()?;
            literals.next()
        })
        .filter(|word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_lowercase()))
        .collect()
}

#[test]
fn the_table_has_a_row_for_every_kind_the_loader_gathers_but_the_rules() {
    let rows: BTreeSet<&str> = KINDS.iter().map(|k| k.word).collect();
    assert_eq!(rows.len(), KINDS.len(), "no kind twice");
    let mut gathered = gathered_kinds();
    assert!(gathered.remove("rules"), "the loader gathers rules files");
    assert_eq!(rows, gathered);
    let sorted: Vec<&str> = rows.iter().copied().collect();
    let listed: Vec<&str> = KINDS.iter().map(|k| k.word).collect();
    assert_eq!(listed, sorted, "a listing gives the kinds in order");
}

#[test]
fn every_kind_lists_its_ids_sorted_and_gets_each_one() {
    let data = loaded();
    for kind in &KINDS {
        let ids = kind.ids(&data);
        assert!(!ids.is_empty(), "the fixtures hold a {}", kind.word);
        assert!(ids.is_sorted(), "{} ids are sorted", kind.word);
        for id in ids {
            assert_eq!(catalog::kind_word(id), Some(kind.word));
            let got = catalog::get(&data, id).unwrap_or_else(|| panic!("{id} is loaded"));
            let text = omnis_data::ron_io::to_compact(&got).unwrap();
            assert!(
                text.starts_with(&format!("(kind:{},def:(", kind.word)),
                "{id} is tagged with its kind's word: {text}"
            );
            assert!(
                text.contains(&format!("id:\"{id}\"")),
                "{id} is written with its own id: {text}"
            );
        }
    }
    let count = |word: &str| catalog::kind(word).unwrap().ids(&data).len();
    assert_eq!(count("item"), data.items.len());
    assert_eq!(count("map"), data.maps.len());
    assert_eq!(count("flag"), data.flags.len(), "declared flags only");
}

#[test]
fn an_id_no_pack_defines_and_a_kind_the_engine_lacks_get_nothing() {
    let data = loaded();
    assert!(catalog::get(&data, "base:item:no_such_thing").is_none());
    assert!(catalog::get(&data, "base:quest:the_missing_cat").is_none());
    assert_eq!(
        catalog::kind_word("base:quest:the_missing_cat"),
        Some("quest")
    );
    assert!(catalog::kind("quest").is_none());
    assert!(catalog::get(&data, "not an id").is_none());
    // An id of one kind is not found under another kind's row.
    let items = catalog::kind("item").unwrap();
    assert!(items.get(&data, "base:spell:shield").is_none());
}

#[test]
fn a_definition_is_written_as_its_file_wrote_it() {
    let data = loaded();
    let Some(Definition::Item(mace)) = catalog::get(&data, "base:item:mace") else {
        panic!("the mace is an item");
    };
    assert_eq!(mace.name, "base:text:item.mace.name");
    // A region is loaded with registry numbers; the catalogue writes its ids as strings.
    let Some(Definition::Region(dungeon)) = catalog::get(&data, "test:region:dungeon") else {
        panic!("the dungeon is a region");
    };
    assert_eq!(dungeon.id, "test:region:dungeon");
    assert_eq!(dungeon.name, "test:text:region.dungeon.name");
    assert_eq!(dungeon.maps, ["test:map:dungeon", "test:map:depths"]);
    assert_eq!(dungeon.rule, "time.wild");
    let Some(Definition::Region(crossroads)) = catalog::get(&data, "test:region:crossroads") else {
        panic!("the crossroads is a region");
    };
    assert_eq!(crossroads.couplings, ["test:region:town"]);
    assert_eq!((crossroads.company, crossroads.stability), (700, 950));
}

#[test]
fn a_definition_s_name_is_its_text_key_and_some_kinds_have_none() {
    let data = loaded();
    let name = |id: &str| {
        let kind = catalog::kind(catalog::kind_word(id).unwrap()).unwrap();
        kind.name(&data, id).map(str::to_owned)
    };
    assert_eq!(
        name("objects:monster:pack_rat").as_deref(),
        Some("objects:text:monster.pack_rat.name")
    );
    assert_eq!(
        name("objects:object:gate").as_deref(),
        Some("objects:text:gate.name")
    );
    assert_eq!(name("objects:loot:hoard"), None);
    assert_eq!(name("objects:flag:lever"), None);
    assert_eq!(name("test:tileset:outdoor"), None);
    assert_eq!(
        name("test:map:town").as_deref(),
        catalog::get(&data, "test:map:town")
            .map(|d| match d {
                Definition::Map(map) => map.name,
                other => panic!("{other:?}"),
            })
            .as_deref()
    );
    assert_eq!(
        name("test:region:dungeon").as_deref(),
        Some("test:text:region.dungeon.name")
    );
    assert_eq!(name("base:item:no_such_thing"), None);
}

#[test]
fn what_a_definition_names_is_read_from_its_strings() {
    let data = loaded();
    let links = |id: &str| catalog::links(&data, id, &catalog::get(&data, id).unwrap());
    let set = |ids: &[&str]| -> BTreeSet<String> { ids.iter().map(|s| (*s).to_owned()).collect() };

    let rat = links("objects:monster:pack_rat");
    assert_eq!(rat.refers, set(&["objects:loot:hoard"]));
    assert_eq!(
        rat.keys,
        set(&[
            "objects:text:monster.pack_rat.bite",
            "objects:text:monster.pack_rat.name"
        ])
    );

    let bob = links("test:monster:bob_the_rat_king");
    assert_eq!(
        bob.refers,
        set(&[
            "base:spell:fire_bolt",
            "base:spell:magic_missile",
            "base:spell:shield",
            "base:spell:thunderwave"
        ])
    );

    let hoard = links("objects:loot:hoard");
    assert_eq!(hoard.refers, set(&["base:item:dagger", "base:item:gem"]));
    assert!(hoard.keys.is_empty());

    // A flag an object names is a reference, declared or not; its messages are keys.
    let gate = links("objects:object:gate");
    assert_eq!(gate.refers, set(&["objects:flag:lever"]));
    assert_eq!(
        gate.keys,
        set(&[
            "objects:text:gate.locked",
            "objects:text:gate.name",
            "objects:text:gate.opens"
        ])
    );

    let town = links("test:map:town");
    assert!(town.refers.contains("test:tileset:outdoor"), "{town:?}");
    assert!(!town.refers.contains("test:map:town"), "never itself");

    let region = links("test:region:dungeon");
    assert_eq!(region.refers, set(&["test:map:depths", "test:map:dungeon"]));
    assert_eq!(region.keys, set(&["test:text:region.dungeon.name"]));
}

#[test]
fn what_names_a_definition_is_asked_of_every_kind() {
    let data = loaded();
    let dagger = catalog::referrers(&data, "base:item:dagger");
    for expected in [
        "base:class:rogue",
        "base:class:wizard",
        "base:service:smith",
        "objects:loot:hoard",
    ] {
        assert!(
            dagger.contains(&expected),
            "{expected} names the dagger: {dagger:?}"
        );
    }
    assert!(!dagger.contains(&"base:item:dagger"), "never itself");
    // In the table's order: classes before loot before services.
    let at = |id: &str| dagger.iter().position(|d| *d == id).unwrap();
    assert!(at("base:class:wizard") < at("objects:loot:hoard"));
    assert!(at("objects:loot:hoard") < at("base:service:smith"));

    assert_eq!(
        catalog::referrers(&data, "objects:loot:hoard"),
        ["objects:monster:pack_rat"]
    );
    assert!(catalog::referrers(&data, "base:item:no_such_thing").is_empty());
    // Every reference `links` finds is one `referrers` finds from the other side.
    for kind in &KINDS {
        for id in kind.ids(&data) {
            let found = catalog::links(&data, id, &kind.get(&data, id).unwrap());
            for named in found.refers.iter().filter(|n| !n.contains(":flag:")) {
                assert!(
                    catalog::referrers(&data, named).contains(&id),
                    "{id} names {named}"
                );
            }
        }
    }
}

/// A pack `over` after the base pack that writes the base pack's mace again, dearer, and an item
/// of its own, built under a scratch directory of the test's own (tests run side by side).
fn overriding(name: &str) -> Data {
    let root = scratch(name);
    std::fs::create_dir_all(root.join("data/items")).unwrap();
    std::fs::write(
        root.join("pack.ron"),
        r#"(schema: 1, id: "over", version: "0.1.0", name: "over", license: "MIT",
            attribution: [], depends: ["base"], entry: None)"#,
    )
    .unwrap();
    std::fs::write(
        root.join("data/items/mace.ron"),
        r#"(schema: 1, id: "base:item:mace", name: "base:text:item.mace.name", kind: Weapon(kind: Simple, damage: (count: 1, sides: 6, modifier: 0), damage_type: Bludgeoning), cost_cp: 900, weight_tenths: 40)"#,
    )
    .unwrap();
    std::fs::write(
        root.join("data/items/club.ron"),
        r#"(schema: 1, id: "over:item:cudgel", name: "base:text:item.mace.name", kind: Weapon(kind: Simple, damage: (count: 1, sides: 4, modifier: 0), damage_type: Bludgeoning), cost_cp: 10, weight_tenths: 20)"#,
    )
    .unwrap();
    // A tag shaped like an id that no pack defines, beside one that names a loaded item.
    std::fs::create_dir_all(root.join("data/objects")).unwrap();
    std::fs::write(
        root.join("data/objects/idol.ron"),
        r#"(schema: 1, id: "over:object:idol", name: "base:text:item.mace.name",
            look: (shape: Heap, width: 2, depth: 2, height: 1),
            attributes: (material: Some(Stone), tags: ["over:item:ghost", "base:item:dagger", "heavy"]),
            interactions: [])"#,
    )
    .unwrap();
    omnis_data::seal::seal(&root, None).unwrap();
    load_packs(&[&base_pack(), &root]).unwrap_or_else(|r| panic!("{r}"))
}

#[test]
fn a_string_shaped_like_an_id_is_a_reference_only_when_it_is_loaded() {
    let data = overriding("catalog-tags");
    let idol = catalog::get(&data, "over:object:idol").unwrap();
    let links = catalog::links(&data, "over:object:idol", &idol);
    let refers: Vec<&str> = links.refers.iter().map(String::as_str).collect();
    assert_eq!(
        refers,
        ["base:item:dagger"],
        "a tag may name a thing; a ghost is no reference"
    );
    let keys: Vec<&str> = links.keys.iter().map(String::as_str).collect();
    assert_eq!(keys, ["base:text:item.mace.name"]);
    // An author's tag is found from the other side too: what is tagged with the dagger.
    assert!(catalog::referrers(&data, "base:item:dagger").contains(&"over:object:idol"));
}

#[test]
fn the_pack_that_supplied_a_definition_is_the_last_that_wrote_it() {
    let data = overriding("catalog-override");
    let Some(Definition::Item(mace)) = catalog::get(&data, "base:item:mace") else {
        panic!("the mace is an item");
    };
    assert_eq!(mace.cost_cp, 900, "the later pack's mace is the one loaded");
    // The id's first segment is a namespace, not the supplier.
    assert_eq!(catalog::supplier(&data, "base:item:mace"), Some("over"));
    assert_eq!(catalog::replaced(&data, "base:item:mace"), ["base"]);
    assert_eq!(catalog::supplier(&data, "over:item:cudgel"), Some("over"));
    assert!(catalog::replaced(&data, "over:item:cudgel").is_empty());
    assert_eq!(catalog::supplier(&data, "base:item:dagger"), Some("base"));
    assert!(catalog::replaced(&data, "base:item:dagger").is_empty());
    assert_eq!(catalog::supplier(&data, "base:item:no_such_thing"), None);
    assert!(catalog::replaced(&data, "base:item:no_such_thing").is_empty());
}

#[test]
fn every_loaded_definition_has_a_supplier_among_the_loaded_packs() {
    let data = loaded();
    let packs: BTreeSet<&str> = data.packs.iter().map(|p| p.id.as_str()).collect();
    for kind in &KINDS {
        for id in kind.ids(&data) {
            let pack =
                catalog::supplier(&data, id).unwrap_or_else(|| panic!("{id} has a supplier"));
            assert!(packs.contains(pack), "{id} from {pack}");
        }
    }
    assert_eq!(catalog::supplier(&data, "test:map:town"), Some("test"));
    assert_eq!(
        catalog::supplier(&data, "test:tileset:outdoor"),
        Some("test")
    );
}
