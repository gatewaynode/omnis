//! The client protocol (ARCHITECTURE.md §9): one [`Op`] in, one [`Reply`] or [`OpError`] out.
//! The dev socket, the headless driver, and the MCP bridge all speak it, so the bridge is a
//! pure translator. Ops that touch the host (files, packs, the window) are declared here so
//! the wire format is one enum, but [`dispatch`] refuses them; the host handles those itself
//! and calls `dispatch` for the rest. Everything arriving is untrusted (§6.2): strings and
//! scripts are bounded before they are looked at.

use crate::LOG_CAPACITY;
use crate::apply::apply;
use crate::beings::BeingView;
use crate::cast_view::{CastView, cast_view};
use crate::command::{Command, Rejection};
use crate::creation_view::{CreationView, creation_view};
use crate::data_ops::{DefinitionRow, DefinitionView, KindCount, Words};
use crate::dev::DevCommand;
use crate::event::Event;
use crate::names::Place;
use crate::objects_view::{ItemCount, ObjectView, ObjectsView, objects_view};
use crate::party::PartyCommand;
pub use crate::party_view::{ItemView, MemberView, PartyView, party_view};
use crate::query::{self, ViewportModel};
use crate::rest_view::{RestView, rest_view};
pub use crate::rules_ops::{
    RuleOrigin, RulesView, SlotView, rule_reply, rules_set, rules_view, slot_view,
};
use crate::save::{SaveCheck, examine_running, restore};
use crate::service_view::{ServiceView, service_view};
use crate::time_view::{DateView, TimeView, time_view};
use crate::view::{CombatView, combat_view};
use crate::world::{Known, ModeKind, Settings, World};
use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use omnis_core::{EraId, Level, MapId};
use omnis_data::edit::{Brush, Edit, EditorStatus, MapEntry, NewMap, Place as EditPlace};
use omnis_data::limits::{MAX_FILE_BYTES, check_asset_path, string_fits};
use omnis_data::{Data, PackFingerprint};
use omnis_rules::Draft;
use serde::{Deserialize, Serialize};

/// The version of the engine's API, both tiers (ARCHITECTURE.md §4.9). Additions leave it alone;
/// a rename, a removal, or a change of meaning or units moves it, with a line in the changelog
/// of `docs/api.md`. `game.status` reports it so a client can check it first.
pub const PROTOCOL: u32 = 2;

/// Most commands one `sim.script` may carry.
pub const MAX_SCRIPT: usize = 10_000;

/// What a screenshot shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShotTarget {
    /// The game canvas at its internal resolution.
    #[default]
    Canvas,
    /// The canvas as the window scales it, with the window-space interface over it.
    Window,
}

/// A request. On the wire it is `{"op": "<name>", "args": {...}}`, `args` omitted for ops
/// that take none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", content = "args")]
pub enum Op {
    /// Mode, turn, clock, position, packs, fingerprint.
    #[serde(rename = "game.status")]
    GameStatus,
    /// Read one value by dotted path (`query::path`).
    #[serde(rename = "world.query")]
    WorldQuery {
        /// The path.
        path: String,
    },
    /// Apply one command.
    #[serde(rename = "sim.command")]
    SimCommand {
        /// The command.
        command: Command,
    },
    /// Apply commands in order, stopping at the first rejection.
    #[serde(rename = "sim.script")]
    SimScript {
        /// The commands.
        commands: Vec<Command>,
    },
    /// The last `count` events.
    #[serde(rename = "events.tail")]
    EventsTail {
        /// How many, at most `LOG_CAPACITY`.
        #[serde(default = "default_tail")]
        count: usize,
    },
    /// The viewport model.
    #[serde(rename = "viewport.get")]
    ViewportGet,
    /// A level of a map as text with the party marker; the party's map when `map` is absent,
    /// the party's level of it (else the ground) when `level` is.
    #[serde(rename = "map.text")]
    MapText {
        /// Map id such as `test:map:dungeon`.
        #[serde(default)]
        map: Option<String>,
        /// Level (M8.5 step 1b).
        #[serde(default)]
        level: Option<Level>,
    },
    /// Known tiles of a level of a map; the party's map and level when absent, as `map.text`.
    #[serde(rename = "automap.get")]
    AutomapGet {
        /// Map id.
        #[serde(default)]
        map: Option<String>,
        /// Level (M8.5 step 1b).
        #[serde(default)]
        level: Option<Level>,
    },
    /// Host: write the save to a path.
    #[serde(rename = "save.write")]
    SaveWrite {
        /// A relative `.ron` path.
        path: String,
    },
    /// Host: replace the world with a save.
    #[serde(rename = "save.read")]
    SaveRead {
        /// A relative `.ron` path.
        path: String,
        /// Skip the pack fingerprint check.
        #[serde(default)]
        force: bool,
        /// Apply the packs' migrations when they change the save (M8.5 7h6).
        #[serde(default)]
        migrate: bool,
    },
    /// Host: what the loaded packs make of a save, changing nothing (M8.5 7h4).
    #[serde(rename = "save.check")]
    SaveCheck {
        /// A relative `.ron` path.
        path: String,
    },
    /// Host: reload the packs from disk, keeping the world.
    #[serde(rename = "pack.reload")]
    PackReload {
        /// Apply the packs' migrations when they change the world (M8.5 7h6).
        #[serde(default)]
        migrate: bool,
    },
    /// Host, game only: save a PNG of the canvas, or of everything the window shows.
    #[serde(rename = "screenshot")]
    Screenshot {
        /// Where, or a default under `.omnis/`.
        #[serde(default)]
        path: Option<String>,
        /// What to capture; the canvas when absent.
        #[serde(default)]
        target: ShotTarget,
    },
    /// Host, game only: the text of every open interface panel, one line per control, label
    /// or text with its rectangle, so a panel can be read without a picture.
    #[serde(rename = "screen.text")]
    ScreenText,
    /// The party: members with their derived numbers, purse, and food.
    #[serde(rename = "party.get")]
    PartyGet,
    /// Create a character from a draft and add it to the party (`Command::Party(Create)`).
    #[serde(rename = "party.create")]
    PartyCreate {
        /// The draft.
        character: Draft,
    },
    /// The encounter or fight in progress: stacks, order, whose turn, round.
    #[serde(rename = "combat.get")]
    CombatGet,
    /// The service the party is inside: what is on offer, its price, and why not.
    #[serde(rename = "service.get")]
    ServiceGet,
    /// Every rule slot with its inputs and source, plus the values and tables.
    #[serde(rename = "rules.list")]
    RulesList,
    /// One slot's inputs and source.
    #[serde(rename = "rules.get")]
    RulesGet {
        /// Slot name such as `spell_points.pool`.
        slot: String,
    },
    /// The kinds of definition the loaded packs hold, each with its count, and the languages
    /// of their text. Needs no game, as every `data.*` op.
    #[serde(rename = "data.kinds")]
    DataKinds,
    /// The loaded definitions that pass every filter given, in kind then id order, a page at a
    /// time: each with its name, the name's text, and the pack that supplied it.
    #[serde(rename = "data.list")]
    DataList {
        /// Only this kind (`item`, `monster`, ...); a kind this engine lacks lists nothing.
        #[serde(default)]
        kind: Option<String>,
        /// Only ids starting with this.
        #[serde(default)]
        prefix: Option<String>,
        /// Only definitions whose id or name holds this, whatever the case.
        #[serde(default)]
        matching: Option<String>,
        /// Only definitions that name this id anywhere in them.
        #[serde(default)]
        refers_to: Option<String>,
        /// The language of the words; `en` when left out.
        #[serde(default)]
        lang: Option<String>,
        /// The `cursor` of the page before, to go on from it.
        #[serde(default)]
        cursor: Option<String>,
        /// How many at most: 100 when left out, never over 500.
        #[serde(default)]
        limit: Option<usize>,
    },
    /// One definition as its pack wrote it, with the text of every key in it, the ids it
    /// names, and the pack that supplied it.
    #[serde(rename = "data.get")]
    DataGet {
        /// `pack:kind:name`.
        id: String,
        /// The language of the words; `en` when left out.
        #[serde(default)]
        lang: Option<String>,
    },
    /// The text of text keys, as any view carries them.
    #[serde(rename = "data.text")]
    DataText {
        /// The keys, at most 256.
        keys: Vec<String>,
        /// The language; `en` when left out.
        #[serde(default)]
        lang: Option<String>,
    },
    /// Every holder's clock and contact, and the party's age, shared time and date (M8).
    #[serde(rename = "time.clocks")]
    TimeClocks,
    /// Dev: the party meets a region as on entering it; `sim.command` with
    /// `DevCommand::Reconcile`, so a replay holds it.
    #[serde(rename = "time.reconcile")]
    TimeReconcile {
        /// `pack:region:name`.
        region: String,
    },
    /// The camp: each member's hit dice and why a rest would be refused (M8 step 8).
    #[serde(rename = "rest.get")]
    RestGet,
    /// The spells each member may cast outside a fight, with cost and refusal (M8 step 8).
    #[serde(rename = "cast.get")]
    CastGet,
    /// What a party member's draft may be: point buy, races, classes, backgrounds (parity P3).
    #[serde(rename = "creation.get")]
    CreationGet,
    /// The objects in reach with every verb they offer and why each would be refused, and the
    /// pile underfoot (M8.5 step 2).
    #[serde(rename = "objects.get")]
    ObjectsGet,
    /// Host: replace one slot's formula in the loaded rules (hot swap; packs on disk are
    /// untouched).
    #[serde(rename = "rules.set")]
    RulesSet {
        /// Slot name.
        slot: String,
        /// The new expression.
        source: String,
    },
    // ---- The editor (M5.3; `omnis_data::edit`). Host ops: they touch the pack's files. ----
    /// Host: the editor's state: the open map's shape, dirtiness, and the loader's verdict.
    #[serde(rename = "editor.status")]
    EditorStatus,
    /// Host: the edited pack's map files, with the id each declares.
    #[serde(rename = "editor.maps")]
    EditorMaps,
    /// Host: open a map of the edited pack by id.
    #[serde(rename = "editor.open")]
    EditorOpen {
        /// `pack:map:name`.
        map: String,
    },
    /// Host: start a new map in the edited pack, open and unsaved.
    #[serde(rename = "editor.new")]
    EditorNew {
        /// Its shape.
        map: NewMap,
    },
    /// Host: paint one tile or edge of the open map.
    #[serde(rename = "editor.paint")]
    EditorPaint {
        /// The level; 0 when left out.
        #[serde(default)]
        level: Level,
        /// Column.
        x: u16,
        /// Row.
        y: u16,
        /// With what.
        brush: Brush,
        /// Join the step the last paint opened (a drag undoes as one); a new step when left out.
        #[serde(default)]
        stroke: bool,
    },
    /// Host: put a placement on the open map, replacing one with the same key, or remove one.
    #[serde(rename = "editor.place")]
    EditorPlace {
        /// Which.
        place: EditPlace,
    },
    /// Host: change the open map's structure: size, fields, a level, a terrain, a stair's rise.
    #[serde(rename = "editor.edit")]
    EditorEdit {
        /// Which.
        edit: Edit,
    },
    /// Host: back one step.
    #[serde(rename = "editor.undo")]
    EditorUndo,
    /// Host: forward one step.
    #[serde(rename = "editor.redo")]
    EditorRedo,
    /// Host: write the open map to its file and reseal the pack at its next patch version.
    #[serde(rename = "editor.save")]
    EditorSave,
    /// Host: every file of the edited pack the loader reads.
    #[serde(rename = "editor.files")]
    EditorFiles,
    /// Host: a file's text.
    #[serde(rename = "editor.read")]
    EditorRead {
        /// Root-relative `.ron` path.
        path: String,
    },
    /// Host: what the loader would say of `text` as the file at `path`; nothing is written.
    #[serde(rename = "editor.check")]
    EditorCheck {
        /// Root-relative `.ron` path.
        path: String,
        /// The text, at most `MAX_FILE_BYTES`.
        text: String,
    },
    /// Host: write `text` as the file at `path`, as typed, once the loader's check passes, and
    /// reseal the pack unless the file is a note.
    #[serde(rename = "editor.write")]
    EditorWrite {
        /// Root-relative `.ron` path.
        path: String,
        /// The text, at most `MAX_FILE_BYTES`.
        text: String,
    },
    /// Host: the author's note on a key.
    #[serde(rename = "editor.note")]
    EditorNote {
        /// A content id, or one followed by `/` and a path inside it.
        key: String,
    },
    /// Host: write the author's note on a key, or take it away.
    #[serde(rename = "editor.note_set")]
    EditorNoteSet {
        /// The key.
        key: String,
        /// The text, or none to take the note away.
        #[serde(default)]
        text: Option<String>,
    },
    /// Host: the keys that carry notes, and the orphans among all notes.
    #[serde(rename = "editor.notes")]
    EditorNotes {
        /// Only keys starting with this, when given.
        #[serde(default)]
        prefix: Option<String>,
    },
}

fn default_tail() -> usize {
    32
}

impl Op {
    /// Whether the host must handle this op; `dispatch` refuses it.
    #[must_use]
    pub const fn is_host(&self) -> bool {
        matches!(
            self,
            Op::SaveWrite { .. }
                | Op::SaveRead { .. }
                | Op::SaveCheck { .. }
                | Op::PackReload { .. }
                | Op::Screenshot { .. }
                | Op::ScreenText
                | Op::RulesSet { .. }
                | Op::EditorStatus
                | Op::EditorMaps
                | Op::EditorOpen { .. }
                | Op::EditorNew { .. }
                | Op::EditorPaint { .. }
                | Op::EditorPlace { .. }
                | Op::EditorEdit { .. }
                | Op::EditorUndo
                | Op::EditorRedo
                | Op::EditorSave
                | Op::EditorFiles
                | Op::EditorRead { .. }
                | Op::EditorCheck { .. }
                | Op::EditorWrite { .. }
                | Op::EditorNote { .. }
                | Op::EditorNoteSet { .. }
                | Op::EditorNotes { .. }
        )
    }
}

/// The party's age, broken down; the date it believes is `Status::date`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockView {
    /// Minutes since the party's origin.
    pub elapsed: i64,
    /// Whole days.
    pub day: i64,
    /// Minute of the day.
    pub minute: i64,
    /// The era.
    pub era: EraId,
}

/// `game.status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// The API's version, [`PROTOCOL`].
    pub protocol: u32,
    /// What the party is doing.
    pub mode: ModeKind,
    /// Commands applied.
    pub turn: u64,
    /// Where the party is.
    pub position: Place,
    /// The party's age.
    pub clock: ClockView,
    /// The date the party believes, on the pack's calendar (M8).
    pub date: DateView,
    /// The packs the world runs on.
    pub packs: Vec<PackFingerprint>,
    /// The world fingerprint as sixteen hex digits.
    pub fingerprint: String,
    /// The id of the service the party is inside, if it is inside one.
    #[serde(default)]
    pub service: Option<String>,
    /// The groups placed `once` on the party's map that are cleared, and how many there are.
    #[serde(default)]
    pub groups_cleared: (u16, u16),
    /// Whether the save rule allows a save here (M8 step 8).
    #[serde(default)]
    pub may_save: bool,
    /// The world seed (M8 step 8).
    #[serde(default)]
    pub seed: u64,
    /// The difficulty options the game started with (M8 step 8).
    #[serde(default)]
    pub settings: Settings,
}

/// One known tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownTile {
    /// Column.
    pub x: u16,
    /// Row.
    pub y: u16,
    /// What is known.
    pub known: Known,
}

/// A successful result. Tagged inside: the wire carries the plain object with `"reply"` naming
/// the variant in snake case, so a client can decode a reply without knowing the op it sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reply", rename_all = "snake_case")]
pub enum Reply {
    /// `game.status`, and `save.read` once the world is replaced.
    Status(Status),
    /// `viewport.get`.
    Viewport(ViewportModel),
    /// `sim.script`.
    Script {
        /// How many commands were applied.
        applied: usize,
        /// Their events, in order.
        events: Vec<Event>,
        /// Why the script stopped early, if it did.
        rejected: Option<Rejection>,
    },
    /// `sim.command` and `events.tail`.
    Events {
        /// The events.
        events: Vec<Event>,
    },
    /// `automap.get`.
    Automap {
        /// The map id.
        map: String,
        /// The level (M8.5 step 1b); unwritten when it is the ground.
        #[serde(default, skip_serializing_if = "Level::is_ground")]
        level: Level,
        /// Known tiles, column by column (`x`, then `y`).
        tiles: Vec<KnownTile>,
    },
    /// `map.text`.
    Text {
        /// The text.
        text: String,
    },
    /// `save.write`, `screenshot`, `editor.save` and `editor.write`.
    Written {
        /// The file written.
        path: String,
        /// The version the pack was resealed at, when the write resealed one (M5.3).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
    },
    /// `party.get`.
    Party {
        /// The party.
        party: PartyView,
    },
    /// `rules.list`.
    Rules {
        /// The rules.
        rules: RulesView,
    },
    /// `rules.get` and `rules.set`.
    Rule {
        /// The slot.
        rule: SlotView,
        /// Where its definition came from and what it beat; null for a slot no file wrote.
        origin: Option<RuleOrigin>,
    },
    /// `combat.get`.
    Combat {
        /// The encounter or fight.
        combat: CombatView,
    },
    /// `service.get`.
    Service {
        /// The service's view.
        view: ServiceView,
    },
    /// `time.clocks`.
    Time {
        /// Clocks, contacts and the party's time.
        time: TimeView,
    },
    /// `rest.get`.
    Rest {
        /// The camp.
        rest: RestView,
    },
    /// `cast.get`.
    Casts {
        /// Every member's spells castable outside a fight.
        casts: Vec<CastView>,
    },
    /// `objects.get`.
    Objects {
        /// The objects in reach, the party's cell first.
        objects: Vec<ObjectView>,
        /// The pile on the party's cell.
        ground: Vec<ItemCount>,
        /// The groups standing in reach (M8.5 step 2c); left out when none.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        beings: Vec<BeingView>,
    },
    /// `creation.get`.
    Creation {
        /// What a draft may be.
        creation: CreationView,
    },
    /// `world.query`: `None` when the path does not exist.
    Value {
        /// The value as text.
        value: Option<String>,
    },
    /// `pack.reload`: what the fresh packs made of the running world (M8.5 7h5).
    Done {
        /// The report: how the world was fitted to them, and each place changed.
        #[serde(default)]
        check: SaveCheck,
    },
    /// `save.check`: what the loaded packs make of a save (M8.5 7h4).
    Check {
        /// The report.
        check: SaveCheck,
    },
    /// The editor's state: `editor.status`, and every editor op that changes the open map.
    Editor {
        /// The state.
        editor: EditorStatus,
    },
    /// `editor.maps`.
    Maps {
        /// The edited pack's map files.
        maps: Vec<MapEntry>,
    },
    /// `editor.files`.
    Files {
        /// Root-relative paths.
        files: Vec<String>,
    },
    /// `editor.read`.
    File {
        /// Root-relative.
        path: String,
        /// The text.
        text: String,
    },
    /// `editor.check`.
    Checked {
        /// The loader's words, one per problem; empty when the text loads.
        errors: Vec<String>,
    },
    /// `editor.note` and `editor.note_set`.
    Note {
        /// The key.
        key: String,
        /// The text, if there is one.
        text: Option<String>,
    },
    /// `editor.notes`.
    Notes {
        /// The keys with notes, sorted.
        keys: Vec<String>,
        /// The keys whose target is nowhere, sorted.
        orphans: Vec<String>,
    },
    /// `data.kinds`.
    Kinds {
        /// Every kind of definition this engine reads, with how many are loaded, by word.
        kinds: Vec<KindCount>,
        /// The languages the loaded text is in.
        languages: Vec<String>,
    },
    /// `data.list`.
    Definitions {
        /// One page, in kind then id order.
        definitions: Vec<DefinitionRow>,
        /// What to pass as `cursor` for the next page; null when this page is the last.
        cursor: Option<String>,
    },
    /// `data.get`.
    Definition {
        /// The definition.
        definition: Box<DefinitionView>,
    },
    /// `data.text`.
    Words {
        /// The language answered in.
        lang: String,
        /// Each key some language file has, with its text; null where `lang` has none.
        words: Words,
        /// The keys no language file has, sorted.
        unknown: Vec<String>,
    },
}

/// A failed op. The world is unchanged unless the message says otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum OpError {
    /// The rules refused the command.
    Rejected {
        /// Why.
        rejection: Rejection,
    },
    /// No such map is loaded.
    UnknownMap {
        /// The id given.
        map: String,
    },
    /// No rule slot has that name.
    UnknownSlot {
        /// The name given.
        slot: String,
    },
    /// No loaded pack defines that id, or its kind is one this engine does not read.
    UnknownId {
        /// The id given.
        id: String,
    },
    /// A string argument is longer than `limits::MAX_STRING_BYTES`.
    TooLong {
        /// The limit in bytes.
        limit: usize,
    },
    /// A script has more than `MAX_SCRIPT` commands, or a `data.text` more than its 256 keys.
    TooMany {
        /// The limit.
        limit: usize,
    },
    /// The op needs the host; the simulation alone cannot do it.
    HostOnly,
    /// No encounter or fight is in progress.
    NoEncounter,
    /// The party is not inside a service.
    NoService,
    /// The request itself was malformed: not JSON, not an op, or too long.
    BadRequest {
        /// What was wrong.
        message: String,
    },
    /// The host or the data layer failed; the message says how.
    Failed {
        /// What went wrong.
        message: String,
    },
    /// The editor refused: the loader's words for a map or file that would not load (nothing
    /// was written), or the one rule the edit broke (M5.3).
    Invalid {
        /// The problems, one per line.
        errors: Vec<String>,
    },
}

impl fmt::Display for OpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpError::Rejected { rejection } => write!(f, "{rejection}"),
            OpError::UnknownMap { map } => write!(f, "no map '{map}' is loaded"),
            OpError::UnknownSlot { slot } => write!(f, "no rule slot '{slot}'"),
            OpError::UnknownId { id } => write!(f, "no loaded pack defines '{id}'"),
            OpError::TooLong { limit } => write!(f, "string longer than {limit} bytes"),
            OpError::TooMany { limit } => write!(f, "more than {limit} in one request"),
            OpError::HostOnly => f.write_str("this op needs the host, not the simulation"),
            OpError::NoEncounter => f.write_str("no encounter or fight is in progress"),
            OpError::NoService => f.write_str("the party is not inside a service"),
            OpError::BadRequest { message } => write!(f, "bad request: {message}"),
            OpError::Failed { message } => f.write_str(message),
            OpError::Invalid { errors } => {
                write!(f, "{} problem(s)", errors.len())?;
                for e in errors {
                    write!(f, "; {e}")?;
                }
                Ok(())
            }
        }
    }
}

impl core::error::Error for OpError {}

impl OpError {
    /// A host failure from anything that displays.
    pub fn failed<E: fmt::Display>(error: E) -> OpError {
        OpError::Failed {
            message: error.to_string(),
        }
    }

    /// A malformed request from anything that displays.
    pub fn bad_request<E: fmt::Display>(error: E) -> OpError {
        OpError::BadRequest {
            message: error.to_string(),
        }
    }
}

/// A file path from a client, allowed only when relative, free of `.` and `..` components,
/// and ending in one of `extensions` (§6.2). Returns the path unchanged.
pub fn client_path<'a>(path: &'a str, extensions: &[&str]) -> Result<&'a str, OpError> {
    bounded(path)?;
    check_asset_path(path, extensions)
        .map(|()| path)
        .map_err(|fault| OpError::failed(format!("path '{path}': {}", fault.reason())))
}

/// Answer a simulation op. Host ops return `OpError::HostOnly`.
pub fn dispatch(world: &mut World, data: &Data, op: &Op) -> Result<Reply, OpError> {
    match op {
        Op::GameStatus => status(world, data).map(Reply::Status),
        Op::WorldQuery { path } => {
            bounded(path)?;
            Ok(Reply::Value {
                value: query::path(world, path),
            })
        }
        Op::SimCommand { command } => apply(world, data, command.clone())
            .map(|events| Reply::Events { events })
            .map_err(|rejection| OpError::Rejected { rejection }),
        Op::SimScript { commands } => script(world, data, commands),
        Op::EventsTail { count } => {
            let keep = (*count).min(LOG_CAPACITY).min(world.log.len());
            Ok(Reply::Events {
                events: world.log[world.log.len() - keep..]
                    .iter()
                    .map(|l| l.event.clone())
                    .collect(),
            })
        }
        Op::ViewportGet => query::viewport(world, data)
            .map(Reply::Viewport)
            .ok_or_else(|| unknown(world.position.map, data)),
        Op::MapText { map, level } => {
            let id = resolve_map(world, data, map.as_deref())?;
            let level = level_of(world, id, *level);
            query::map_text(world, data, id, level)
                .map(|text| Reply::Text { text })
                .ok_or_else(|| unknown(id, data))
        }
        Op::AutomapGet { map, level } => {
            let id = resolve_map(world, data, map.as_deref())?;
            let level = level_of(world, id, *level);
            let tiles = world
                .automap
                .tiles(id, level)
                .map(|known| {
                    known
                        .iter()
                        .map(|(&(x, y), &known)| KnownTile { x, y, known })
                        .collect()
                })
                .unwrap_or_default();
            Ok(Reply::Automap {
                map: map_name(id, data),
                level,
                tiles,
            })
        }
        Op::PartyGet => Ok(Reply::Party {
            party: party_view(world, data),
        }),
        Op::PartyCreate { character } => apply(
            world,
            data,
            Command::Party(PartyCommand::Create(character.clone())),
        )
        .map(|events| Reply::Events { events })
        .map_err(|rejection| OpError::Rejected { rejection }),
        Op::CombatGet => combat_view(world, data)
            .map(|combat| Reply::Combat { combat })
            .ok_or(OpError::NoEncounter),
        Op::ServiceGet => service_view(world, data)
            .map(|view| Reply::Service { view })
            .ok_or(OpError::NoService),
        Op::RestGet => Ok(Reply::Rest {
            rest: rest_view(world, data),
        }),
        Op::CastGet => Ok(Reply::Casts {
            casts: cast_view(world, data),
        }),
        Op::ObjectsGet => {
            let ObjectsView {
                objects,
                ground,
                beings,
            } = objects_view(world, data);
            Ok(Reply::Objects {
                objects,
                ground,
                beings,
            })
        }
        Op::CreationGet => Ok(Reply::Creation {
            creation: creation_view(data),
        }),
        Op::TimeClocks => Ok(Reply::Time {
            time: time_view(world, data),
        }),
        Op::TimeReconcile { region } => apply(
            world,
            data,
            Command::Dev(DevCommand::Reconcile {
                region: region.clone(),
            }),
        )
        .map(|events| Reply::Events { events })
        .map_err(|rejection| OpError::Rejected { rejection }),
        Op::RulesList => Ok(Reply::Rules {
            rules: rules_view(data),
        }),
        Op::RulesGet { slot } => rule_reply(data, slot),
        Op::DataKinds | Op::DataList { .. } | Op::DataGet { .. } | Op::DataText { .. } => {
            op.data(data).unwrap_or(Err(OpError::HostOnly))
        }
        Op::SaveWrite { .. }
        | Op::SaveRead { .. }
        | Op::SaveCheck { .. }
        | Op::PackReload { .. }
        | Op::Screenshot { .. }
        | Op::ScreenText
        | Op::RulesSet { .. }
        | Op::EditorStatus
        | Op::EditorMaps
        | Op::EditorOpen { .. }
        | Op::EditorNew { .. }
        | Op::EditorPaint { .. }
        | Op::EditorPlace { .. }
        | Op::EditorEdit { .. }
        | Op::EditorUndo
        | Op::EditorRedo
        | Op::EditorSave
        | Op::EditorFiles
        | Op::EditorRead { .. }
        | Op::EditorCheck { .. }
        | Op::EditorWrite { .. }
        | Op::EditorNote { .. }
        | Op::EditorNoteSet { .. }
        | Op::EditorNotes { .. } => Err(OpError::HostOnly),
    }
}

/// `game.status`.
pub fn status(world: &World, data: &Data) -> Result<Status, OpError> {
    let clock = world.party_clock();
    let day_length = i64::from(data.calendar().minutes_per_day.max(1));
    let here = query::here(world, data);
    Ok(Status {
        protocol: PROTOCOL,
        mode: here.mode,
        turn: here.turn,
        position: here.position,
        clock: ClockView {
            elapsed: clock.elapsed,
            day: clock.elapsed.div_euclid(day_length),
            minute: clock.elapsed.rem_euclid(day_length),
            era: clock.era,
        },
        date: here.date,
        packs: world.packs.clone(),
        fingerprint: format!("{:016x}", world.fingerprint().map_err(OpError::failed)?),
        service: here.service,
        groups_cleared: crate::encounter::groups_cleared(world, data, world.position.map),
        may_save: here.may_save,
        seed: here.seed,
        settings: here.settings,
    })
}

// The host ops' rules (ARCHITECTURE.md §4.9): every host writes only the file and window code
// around these, so the socket and the headless driver cannot disagree.

// `save.write` and `save.read` are `save::save_files` and `save::load_save` (M8.5 7d): a
// character save and the world folder's files, which the host writes and reads.

/// `pack.reload` by name (M8.5 7h5): the running world is fitted to the freshly loaded packs
/// as a save made on `old`, the packs it runs on, would be on load (`save::examine_running`,
/// `save::restore`): the same packs change nothing; other packs are recorded; moved names
/// renumber it and fold it, its journal held for `old`'s folder; places the maps no longer
/// allow are fixed and reported. Migrations leading on from the running packs' versions apply
/// as a load's do (M8.5 7h6): when they change the world, only with `migrate`. Refused, nothing
/// changes; the host swaps its data only on `Ok`.
///
/// # Errors
/// A name the fresh packs lack, or a world they cannot hold, with the report.
pub fn reload(
    world: &mut World,
    old: &Data,
    fresh: &Data,
    migrate: bool,
) -> Result<SaveCheck, ReloadRefused> {
    let examined = examine_running(world, old, fresh);
    let check = examined.report().clone();
    match restore(examined, fresh, false, migrate) {
        Ok((next, _)) => {
            *world = next;
            Ok(check)
        }
        Err(e) => Err(ReloadRefused {
            error: OpError::failed(format!("{e}; nothing changed")),
            check: Box::new(check),
        }),
    }
}

/// A refused `pack.reload`: why, as the op answers it, and what the fresh packs made of the
/// world, for a host that shows the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadRefused {
    /// The op's error.
    pub error: OpError,
    /// The report.
    pub check: Box<SaveCheck>,
}

impl From<ReloadRefused> for OpError {
    fn from(refused: ReloadRefused) -> OpError {
        refused.error
    }
}

fn script(world: &mut World, data: &Data, commands: &[Command]) -> Result<Reply, OpError> {
    if commands.len() > MAX_SCRIPT {
        return Err(OpError::TooMany { limit: MAX_SCRIPT });
    }
    let mut events = Vec::new();
    let mut applied = 0;
    let mut rejected = None;
    for command in commands {
        match apply(world, data, command.clone()) {
            Ok(more) => {
                events.extend(more);
                applied += 1;
            }
            Err(rejection) => {
                rejected = Some(rejection);
                break;
            }
        }
    }
    Ok(Reply::Script {
        applied,
        events,
        rejected,
    })
}

/// Refuse a string argument longer than the limit.
pub fn bounded(s: &str) -> Result<(), OpError> {
    if string_fits(s) {
        Ok(())
    } else {
        Err(OpError::TooLong {
            limit: omnis_data::limits::MAX_STRING_BYTES,
        })
    }
}

/// Refuse a file's text longer than the loader would read (`MAX_FILE_BYTES`).
pub fn bounded_file(text: &str) -> Result<(), OpError> {
    let limit = usize::try_from(MAX_FILE_BYTES).unwrap_or(usize::MAX);
    if text.len() <= limit {
        Ok(())
    } else {
        Err(OpError::TooLong { limit })
    }
}

/// The level asked for, else the party's level when it is on `map`, else the ground.
fn level_of(world: &World, map: MapId, asked: Option<Level>) -> Level {
    asked.unwrap_or(if world.position.map == map {
        world.position.level
    } else {
        Level::GROUND
    })
}

fn resolve_map(world: &World, data: &Data, name: Option<&str>) -> Result<MapId, OpError> {
    match name {
        None => Ok(world.position.map),
        Some(name) => {
            bounded(name)?;
            data.registry
                .maps
                .get(name)
                .ok_or_else(|| OpError::UnknownMap {
                    map: name.to_string(),
                })
        }
    }
}

use query::map_name;

fn unknown(id: MapId, data: &Data) -> OpError {
    OpError::UnknownMap {
        map: map_name(id, data),
    }
}
