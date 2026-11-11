//! The `data.*` ops (W1): what the loaded packs define, by kind and by id, with their words, what
//! each names and what names it, a page at a time. The author's tier; no world is consulted.

use crate::common;

use common::{data, world};
use omnis_data::catalog::KINDS;
use omnis_data::limits::MAX_STRING_BYTES;
use omnis_sim::data_ops::{
    DefinitionRow, DefinitionView, LIST_DEFAULT, LIST_MAX, Listing, MAX_KEYS, get, list, text,
};
use omnis_sim::{Op, OpError, Reply, dispatch};

fn page(reply: Reply) -> (Vec<DefinitionRow>, Option<String>) {
    match reply {
        Reply::Definitions {
            definitions,
            cursor,
        } => (definitions, cursor),
        other => panic!("not a page of definitions: {other:?}"),
    }
}

fn ids(rows: &[DefinitionRow]) -> Vec<&str> {
    rows.iter().map(|row| row.id.as_str()).collect()
}

/// Every loaded id, in the order a listing gives them.
fn every_id(data: &omnis_data::Data) -> Vec<String> {
    KINDS
        .iter()
        .flat_map(|kind| kind.ids(data))
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_kinds_are_the_table_s_with_what_is_loaded_and_the_languages() {
    let data = data();
    let Reply::Kinds { kinds, languages } = omnis_sim::data_ops::kinds(&data) else {
        panic!("not kinds")
    };
    let words: Vec<&str> = kinds.iter().map(|k| k.kind.as_str()).collect();
    let table: Vec<&str> = KINDS.iter().map(|k| k.word).collect();
    assert_eq!(words, table);
    let count = |word: &str| kinds.iter().find(|k| k.kind == word).unwrap().count;
    assert_eq!(count("item"), data.items.len());
    assert_eq!(count("monster"), data.monsters.len());
    assert_eq!(count("map"), data.maps.len());
    assert_eq!(
        count("object"),
        0,
        "a kind with nothing loaded is still named"
    );
    assert_eq!(languages, ["en"]);
}

#[test]
fn a_listing_walks_every_definition_once_whatever_the_page() {
    // With the objects fixture the packs hold more than a default page.
    let objects = common::test_pack().with_file_name("objects");
    let data = omnis_data::load_packs(&[&common::base_pack(), &common::test_pack(), &objects])
        .unwrap_or_else(|r| panic!("{r}"));
    let all = every_id(&data);
    assert!(
        all.len() > LIST_DEFAULT,
        "the fixtures fill more than a page: {}",
        all.len()
    );
    for limit in [1, 7, LIST_DEFAULT, LIST_MAX] {
        let mut walked = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let asked = Listing {
                cursor: cursor.as_deref(),
                limit: Some(limit),
                ..Listing::default()
            };
            let (rows, next) = page(list(&data, &asked).unwrap());
            assert!(rows.len() <= limit);
            assert_eq!(
                next.as_deref(),
                rows.last()
                    .map(|r| r.id.as_str())
                    .filter(|_| next.is_some()),
                "the cursor is the last row's id"
            );
            walked.extend(rows.into_iter().map(|row| row.id));
            assert!(walked.len() <= all.len(), "a page gave a row again");
            match next {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        assert_eq!(walked, all, "at {limit} a page");
    }
    // No limit is the default page; a limit out of range is brought inside it.
    let (rows, next) = page(list(&data, &Listing::default()).unwrap());
    assert_eq!(rows.len(), LIST_DEFAULT);
    assert!(next.is_some());
    let none = Listing {
        limit: Some(0),
        ..Listing::default()
    };
    assert_eq!(page(list(&data, &none).unwrap()).0.len(), 1);
    let huge = Listing {
        limit: Some(usize::MAX),
        ..Listing::default()
    };
    let (rows, next) = page(list(&data, &huge).unwrap());
    assert_eq!(rows.len(), all.len().min(LIST_MAX));
    assert_eq!(next.is_some(), all.len() > LIST_MAX);
    // A page that ends on the last definition says so: no cursor to an empty page.
    let exact = Listing {
        kind: Some("monster"),
        limit: Some(data.monsters.len()),
        ..Listing::default()
    };
    assert_eq!(page(list(&data, &exact).unwrap()).1, None);
}

/// A reply is bounded however much is loaded: more flags than the largest page, put into the
/// loaded data in memory (no simulation crate's test writes files).
#[test]
fn no_page_holds_more_than_the_most_whatever_is_asked() {
    let mut data = data();
    for n in 0..LIST_MAX + 20 {
        let id = format!("many:flag:f{n:04}");
        let flag = data.registry.flags.intern(&id);
        data.flags.insert(
            flag,
            omnis_data::FlagDef {
                schema: omnis_data::SCHEMA,
                id: id.clone(),
                scope: omnis_data::FlagScope::World,
            },
        );
        data.sources.insert(id, vec!["many".to_owned()]);
    }
    let asked = Listing {
        kind: Some("flag"),
        limit: Some(usize::MAX),
        ..Listing::default()
    };
    let (rows, next) = page(list(&data, &asked).unwrap());
    assert_eq!(rows.len(), LIST_MAX);
    assert_eq!(next.as_deref(), Some("many:flag:f0499"));
    let rest = Listing {
        cursor: next.as_deref(),
        ..asked
    };
    let (rows, next) = page(list(&data, &rest).unwrap());
    assert_eq!((rows.len(), next), (20, None));
    assert_eq!(rows[0].id, "many:flag:f0500");
    assert_eq!(rows[0].pack, "many");
}

#[test]
fn a_row_carries_its_name_its_words_and_the_pack_that_supplied_it() {
    let data = data();
    let asked = Listing {
        kind: Some("monster"),
        ..Listing::default()
    };
    let (rows, next) = page(list(&data, &asked).unwrap());
    assert_eq!(next, None);
    assert_eq!(rows.len(), data.monsters.len());
    assert!(rows.iter().all(|row| row.kind == "monster"));
    let goblin = rows.iter().find(|r| r.id == "base:monster:goblin").unwrap();
    assert_eq!(
        goblin.name.as_deref(),
        Some("base:text:monster.goblin.name")
    );
    assert_eq!(
        goblin.words.get("base:text:monster.goblin.name"),
        Some(&Some("Goblin".to_owned()))
    );
    assert_eq!(goblin.words.len(), 1, "a row carries its name's text alone");
    assert_eq!((goblin.pack.as_str(), goblin.replaces.len()), ("base", 0));
    let bob = rows
        .iter()
        .find(|r| r.id == "test:monster:bob_the_rat_king")
        .unwrap();
    assert_eq!(bob.pack, "test");
    // A kind with no name has none, and no words.
    let tiles = Listing {
        kind: Some("tileset"),
        ..Listing::default()
    };
    let (rows, _) = page(list(&data, &tiles).unwrap());
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|r| r.name.is_none() && r.words.is_empty()));
}

#[test]
fn every_filter_given_must_hold() {
    let data = data();
    let asked = |listing: Listing<'_>| page(list(&data, &listing).unwrap()).0;
    // A kind this engine does not read lists nothing: a client may ask ahead of the engine.
    let quests = asked(Listing {
        kind: Some("quest"),
        ..Listing::default()
    });
    assert!(quests.is_empty());
    // A prefix is of the id.
    let test_maps = asked(Listing {
        prefix: Some("test:map:"),
        ..Listing::default()
    });
    assert_eq!(test_maps.len(), data.maps.len());
    assert!(test_maps.iter().all(|r| r.kind == "map"));
    // `matching` reads the id and the name's text, whatever the case.
    let rats = asked(Listing {
        matching: Some("RAT"),
        kind: Some("monster"),
        ..Listing::default()
    });
    assert!(ids(&rats).contains(&"base:monster:giant_rat"), "by its id");
    let by_name = asked(Listing {
        matching: Some("giant rat"),
        ..Listing::default()
    });
    assert_eq!(
        ids(&by_name),
        ["base:monster:giant_rat", "test:monster:giant_rat"],
        "by its name's text: the id has no space"
    );
    let smithy = asked(Listing {
        matching: Some("Smithy"),
        ..Listing::default()
    });
    assert_eq!(ids(&smithy), ["base:service:smith"], "the id says smith");
    // In a language no pack has, only the id can match.
    let nowhere = asked(Listing {
        matching: Some("giant rat"),
        lang: Some("de"),
        ..Listing::default()
    });
    assert!(nowhere.is_empty());
    // What names the dagger, of every kind, then of one.
    let named = asked(Listing {
        refers_to: Some("base:item:dagger"),
        ..Listing::default()
    });
    for id in [
        "base:class:rogue",
        "base:class:wizard",
        "base:service:smith",
    ] {
        assert!(
            ids(&named).contains(&id),
            "{id} names the dagger: {:?}",
            ids(&named)
        );
    }
    assert!(!ids(&named).contains(&"base:item:dagger"));
    let classes = asked(Listing {
        refers_to: Some("base:item:dagger"),
        kind: Some("class"),
        ..Listing::default()
    });
    assert!(!classes.is_empty() && classes.iter().all(|r| r.kind == "class"));
    assert!(classes.len() < named.len());
    // Nothing names a thing that is not there.
    let ghost = asked(Listing {
        refers_to: Some("base:item:no_such_thing"),
        ..Listing::default()
    });
    assert!(ghost.is_empty());
}

fn view(data: &omnis_data::Data, id: &str) -> DefinitionView {
    get(data, id, None).unwrap_or_else(|e| panic!("{id}: {e}"))
}

#[test]
fn a_definition_comes_with_its_words_what_it_names_and_its_pack() {
    let data = data();
    let bob = view(&data, "test:monster:bob_the_rat_king");
    assert_eq!(bob.id, "test:monster:bob_the_rat_king");
    assert_eq!(
        bob.name.as_deref(),
        Some("test:text:monster.bob_the_rat_king.name")
    );
    let omnis_data::catalog::Definition::Monster(def) = &bob.definition else {
        panic!("a monster: {:?}", bob.definition)
    };
    assert_eq!(def.xp, 200, "the definition as its file wrote it");
    assert_eq!(
        bob.refers,
        [
            "base:spell:fire_bolt",
            "base:spell:magic_missile",
            "base:spell:shield",
            "base:spell:thunderwave"
        ]
    );
    // Every key in it, each with its text: the name and the attack's.
    let keys: Vec<&str> = bob.words.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "test:text:monster.bob_the_rat_king.name",
            "test:text:monster.bob_the_rat_king.quarterstaff"
        ]
    );
    assert!(
        bob.words
            .values()
            .all(|text| text.as_deref().is_some_and(|t| !t.is_empty()))
    );
    assert_eq!((bob.pack.as_str(), bob.replaces.len()), ("test", 0));
    // Every loaded definition answers, and what it names is loaded.
    for id in every_id(&data) {
        let got = view(&data, &id);
        assert_eq!(got.id, id);
        for named in &got.refers {
            assert!(
                get(&data, named, None).is_ok() || named.contains(":flag:"),
                "{id} names {named}"
            );
        }
    }
}

#[test]
fn words_are_text_or_null_never_the_key() {
    let data = data();
    let german = get(&data, "base:monster:goblin", Some("de")).unwrap();
    assert_eq!(
        german.words.get("base:text:monster.goblin.name"),
        Some(&None),
        "no German text: null, not the key"
    );
    let Reply::Words {
        lang,
        words,
        unknown,
    } = text(
        &data,
        &[
            "base:text:monster.goblin.name".to_owned(),
            "base:text:no.such.key".to_owned(),
            "not a key".to_owned(),
            "base:text:no.such.key".to_owned(),
            "base:text:service.smith.name".to_owned(),
        ],
        None,
    )
    .unwrap()
    else {
        panic!("not words")
    };
    assert_eq!(lang, "en");
    assert_eq!(
        words.into_iter().collect::<Vec<_>>(),
        [
            (
                "base:text:monster.goblin.name".to_owned(),
                Some("Goblin".to_owned())
            ),
            (
                "base:text:service.smith.name".to_owned(),
                Some("Smithy".to_owned())
            ),
        ]
    );
    assert_eq!(
        unknown,
        ["base:text:no.such.key", "not a key"],
        "sorted, once each"
    );
    let Reply::Words { words, unknown, .. } = text(
        &data,
        &["base:text:monster.goblin.name".to_owned()],
        Some("de"),
    )
    .unwrap() else {
        panic!("not words")
    };
    assert_eq!(words.get("base:text:monster.goblin.name"), Some(&None));
    assert!(unknown.is_empty(), "the key is known; German lacks it");
}

#[test]
fn what_is_not_there_and_what_is_too_much_are_refused() {
    let data = data();
    for id in [
        "base:item:no_such_thing",
        "base:quest:the_missing_cat",
        "not an id",
        "",
        "base:text:monster.goblin.name",
    ] {
        assert_eq!(
            get(&data, id, None).unwrap_err(),
            OpError::UnknownId { id: id.to_owned() },
            "{id}"
        );
    }
    let long = "x".repeat(MAX_STRING_BYTES + 1);
    let too_long = OpError::TooLong {
        limit: MAX_STRING_BYTES,
    };
    assert_eq!(get(&data, &long, None).unwrap_err(), too_long);
    assert_eq!(
        get(&data, "base:item:mace", Some(&long)).unwrap_err(),
        too_long
    );
    assert_eq!(
        text(&data, std::slice::from_ref(&long), None).unwrap_err(),
        too_long
    );
    assert_eq!(text(&data, &[], Some(&long)).unwrap_err(), too_long);
    for listing in [
        Listing {
            kind: Some(&long),
            ..Listing::default()
        },
        Listing {
            prefix: Some(&long),
            ..Listing::default()
        },
        Listing {
            matching: Some(&long),
            ..Listing::default()
        },
        Listing {
            refers_to: Some(&long),
            ..Listing::default()
        },
        Listing {
            lang: Some(&long),
            ..Listing::default()
        },
        Listing {
            cursor: Some(&long),
            ..Listing::default()
        },
    ] {
        assert_eq!(list(&data, &listing).unwrap_err(), too_long);
    }
    let keys = vec!["base:text:monster.goblin.name".to_owned(); MAX_KEYS];
    assert!(
        text(&data, &keys, None).is_ok(),
        "{MAX_KEYS} keys are taken"
    );
    let keys = vec!["base:text:monster.goblin.name".to_owned(); MAX_KEYS + 1];
    assert_eq!(
        text(&data, &keys, None).unwrap_err(),
        OpError::TooMany { limit: MAX_KEYS }
    );
}

#[test]
fn the_ops_answer_through_dispatch_and_without_a_world() {
    let data = data();
    let mut world = world(&data);
    let before = world.clone();
    let ops = [
        Op::DataKinds,
        Op::DataList {
            kind: Some("spell".into()),
            prefix: None,
            matching: None,
            refers_to: None,
            lang: None,
            cursor: None,
            limit: Some(3),
        },
        Op::DataGet {
            id: "base:item:mace".into(),
            lang: None,
        },
        Op::DataText {
            keys: vec!["base:text:item.mace.name".into()],
            lang: None,
        },
    ];
    for op in &ops {
        assert!(!op.is_host(), "{op:?}");
        let through = dispatch(&mut world, &data, op).unwrap();
        let alone = op.data(&data).expect("a data op").unwrap();
        assert_eq!(through, alone, "{op:?}");
    }
    assert!(world == before, "a data op changes nothing");
    assert!(Op::GameStatus.data(&data).is_none());
    assert!(Op::RulesList.data(&data).is_none());
    let missing = Op::DataGet {
        id: "base:item:no_such_thing".into(),
        lang: None,
    };
    assert_eq!(
        dispatch(&mut world, &data, &missing).unwrap_err(),
        OpError::UnknownId {
            id: "base:item:no_such_thing".into()
        }
    );
}
