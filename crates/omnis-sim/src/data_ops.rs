//! The `data.*` ops (W1): what the loaded packs define, over Tier 2. A client turns an id or a
//! text key from any view into words and a definition, lists what there is by kind, and asks
//! what names a thing. Every answer is read from [`omnis_data::catalog`]'s one table of kinds,
//! so a kind or a field the packs gain later is answered with no op of its own.
//!
//! The author's tier: these answer from everything loaded, whatever the party has met. They
//! need no world, so a host answers them with no game running ([`Op::data`]).

use crate::ops::{Op, OpError, Reply, bounded};
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use omnis_data::Data;
use omnis_data::catalog::{self, Definition, KINDS, Kind};
use serde::{Deserialize, Serialize};

/// The language a `data.*` op answers in when none is asked for.
pub const DEFAULT_LANG: &str = "en";
/// How many definitions `data.list` gives when no `limit` is asked for.
pub const LIST_DEFAULT: usize = 100;
/// The most definitions one `data.list` gives.
pub const LIST_MAX: usize = 500;
/// The most keys one `data.text` takes.
pub const MAX_KEYS: usize = 256;

/// Text keys with their text in the language asked for; null where that language has none.
pub type Words = BTreeMap<String, Option<String>>;

/// One kind of definition and how many are loaded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KindCount {
    /// The kind as an id writes it (`pack:item:name`).
    pub kind: String,
    /// How many definitions of it are loaded.
    pub count: usize,
}

/// One definition in a listing: what it is called and where it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionRow {
    /// `pack:kind:name`.
    pub id: String,
    /// The kind, the id's second segment.
    pub kind: String,
    /// Text key of its name; null for a kind that has none (a tileset, a loot table, a flag).
    pub name: Option<String>,
    /// The name's text, by its key.
    pub words: Words,
    /// The pack that supplied it: the last loaded that wrote this id.
    pub pack: String,
    /// The packs whose definition under this id it replaced, in load order.
    pub replaces: Vec<String>,
}

/// One definition in full.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionView {
    /// `pack:kind:name`.
    pub id: String,
    /// `kind`, the id's second segment, and `def`, the definition as its pack wrote it. `def`
    /// follows the pack format (`omnis_data::SCHEMA`), not the protocol.
    #[serde(flatten)]
    pub definition: Definition,
    /// Text key of its name; null for a kind that has none.
    pub name: Option<String>,
    /// Every text key in the definition, with its text.
    pub words: Words,
    /// Every loaded definition and every flag it names, by id, sorted.
    pub refers: Vec<String>,
    /// The pack that supplied it: the last loaded that wrote this id.
    pub pack: String,
    /// The packs whose definition under this id it replaced, in load order.
    pub replaces: Vec<String>,
}

/// What `data.list` is asked: every filter given must hold.
#[derive(Debug, Clone, Copy, Default)]
pub struct Listing<'a> {
    /// Only this kind.
    pub kind: Option<&'a str>,
    /// Only ids starting with this.
    pub prefix: Option<&'a str>,
    /// Only definitions whose id or name holds this, whatever the case.
    pub matching: Option<&'a str>,
    /// Only definitions that name this id.
    pub refers_to: Option<&'a str>,
    /// The language of the words.
    pub lang: Option<&'a str>,
    /// The `cursor` of the page before.
    pub cursor: Option<&'a str>,
    /// How many at most.
    pub limit: Option<usize>,
}

impl Op {
    /// The answer to a `data.*` op from `data` alone; `None` for every other op. A host calls
    /// this before it looks for a game: these ops need none.
    #[must_use]
    pub fn data(&self, data: &Data) -> Option<Result<Reply, OpError>> {
        Some(match self {
            Op::DataKinds => Ok(kinds(data)),
            Op::DataList {
                kind,
                prefix,
                matching,
                refers_to,
                lang,
                cursor,
                limit,
            } => list(
                data,
                &Listing {
                    kind: kind.as_deref(),
                    prefix: prefix.as_deref(),
                    matching: matching.as_deref(),
                    refers_to: refers_to.as_deref(),
                    lang: lang.as_deref(),
                    cursor: cursor.as_deref(),
                    limit: *limit,
                },
            ),
            Op::DataGet { id, lang } => {
                get(data, id, lang.as_deref()).map(|definition| Reply::Definition {
                    definition: Box::new(definition),
                })
            }
            Op::DataText { keys, lang } => text(data, keys, lang.as_deref()),
            _ => return None,
        })
    }
}

/// `data.kinds`.
#[must_use]
pub fn kinds(data: &Data) -> Reply {
    Reply::Kinds {
        kinds: KINDS
            .iter()
            .map(|kind| KindCount {
                kind: kind.word.to_owned(),
                count: kind.ids(data).len(),
            })
            .collect(),
        languages: data.text.keys().cloned().collect(),
    }
}

/// The text of `key` in `lang`, when that language has it.
fn text_of<'a>(data: &'a Data, lang: &str, key: &str) -> Option<&'a str> {
    let key = data.registry.text.get(key)?;
    data.text.get(lang)?.get(&key).map(String::as_str)
}

/// `keys` with their text in `lang`.
fn words<'a>(data: &Data, lang: &str, keys: impl IntoIterator<Item = &'a str>) -> Words {
    keys.into_iter()
        .map(|key| (key.to_owned(), text_of(data, lang, key).map(str::to_owned)))
        .collect()
}

/// Where `id` stands in a listing: its kind, then itself.
fn place(id: &str) -> (&str, &str) {
    (catalog::kind_word(id).unwrap_or_default(), id)
}

/// Whether the definition `id` of `kind` passes every filter of `asked` but the cursor.
fn passes(data: &Data, kind: &Kind, id: &str, asked: &Listing<'_>, lang: &str) -> bool {
    if asked.prefix.is_some_and(|prefix| !id.starts_with(prefix)) {
        return false;
    }
    asked.matching.is_none_or(|wanted| {
        let wanted = wanted.to_lowercase();
        let name = kind.name(data, id).and_then(|key| text_of(data, lang, key));
        id.contains(&wanted) || name.is_some_and(|name| name.to_lowercase().contains(&wanted))
    })
}

/// `data.list`: the definitions that pass every filter given, in kind then id order, at most
/// `limit` of them after `cursor`. A kind this engine does not have lists nothing.
pub fn list(data: &Data, asked: &Listing<'_>) -> Result<Reply, OpError> {
    for given in [
        asked.kind,
        asked.prefix,
        asked.matching,
        asked.refers_to,
        asked.lang,
        asked.cursor,
    ]
    .into_iter()
    .flatten()
    {
        bounded(given)?;
    }
    let lang = asked.lang.unwrap_or(DEFAULT_LANG);
    let limit = asked.limit.unwrap_or(LIST_DEFAULT).clamp(1, LIST_MAX);
    let naming: Option<BTreeSet<&str>> = asked
        .refers_to
        .map(|id| catalog::referrers(data, id).into_iter().collect());
    let mut definitions = Vec::new();
    let mut cursor = None;
    'kinds: for kind in KINDS
        .iter()
        .filter(|kind| asked.kind.is_none_or(|word| word == kind.word))
    {
        for id in kind.ids(data) {
            let listed = asked.cursor.is_none_or(|last| place(id) > place(last))
                && naming.as_ref().is_none_or(|naming| naming.contains(id))
                && passes(data, kind, id, asked, lang);
            if !listed {
                continue;
            }
            if definitions.len() == limit {
                cursor = definitions.last().map(|row: &DefinitionRow| row.id.clone());
                break 'kinds;
            }
            let name = kind.name(data, id);
            definitions.push(DefinitionRow {
                id: id.to_owned(),
                kind: kind.word.to_owned(),
                name: name.map(str::to_owned),
                words: words(data, lang, name),
                pack: catalog::supplier(data, id).unwrap_or_default().to_owned(),
                replaces: catalog::replaced(data, id).to_vec(),
            });
        }
    }
    Ok(Reply::Definitions {
        definitions,
        cursor,
    })
}

/// `data.get`.
pub fn get(data: &Data, id: &str, lang: Option<&str>) -> Result<DefinitionView, OpError> {
    bounded(id)?;
    if let Some(lang) = lang {
        bounded(lang)?;
    }
    let unknown = || OpError::UnknownId { id: id.to_owned() };
    let kind = catalog::kind_word(id)
        .and_then(catalog::kind)
        .ok_or_else(unknown)?;
    let definition = kind.get(data, id).ok_or_else(unknown)?;
    let links = catalog::links(data, id, &definition);
    Ok(DefinitionView {
        id: id.to_owned(),
        name: kind.name(data, id).map(str::to_owned),
        words: words(
            data,
            lang.unwrap_or(DEFAULT_LANG),
            links.keys.iter().map(String::as_str),
        ),
        refers: links.refers.into_iter().collect(),
        pack: catalog::supplier(data, id).unwrap_or_default().to_owned(),
        replaces: catalog::replaced(data, id).to_vec(),
        definition,
    })
}

/// `data.text`: each key some language file has, with its text in `lang`; the others apart.
pub fn text(data: &Data, keys: &[String], lang: Option<&str>) -> Result<Reply, OpError> {
    if keys.len() > MAX_KEYS {
        return Err(OpError::TooMany { limit: MAX_KEYS });
    }
    let lang = lang.unwrap_or(DEFAULT_LANG);
    bounded(lang)?;
    for key in keys {
        bounded(key)?;
    }
    let (known, unknown): (Vec<&String>, Vec<&String>) = keys
        .iter()
        .partition(|key| data.registry.text.get(key).is_some());
    let mut unknown: Vec<String> = unknown.into_iter().cloned().collect();
    unknown.sort_unstable();
    unknown.dedup();
    Ok(Reply::Words {
        lang: lang.to_owned(),
        words: words(data, lang, known.into_iter().map(String::as_str)),
        unknown,
    })
}
