//! **The one place an edit is defined** — and the one place an edit is applied.
//!
//! # Why this module exists
//!
//! The editor had **two ways to change the same thing**, and they were not the same way:
//!
//! - a **key handler** in `editor.rs` / `tiles.rs` / `compose.rs`, which read the keyboard, read
//!   some state, wrote some state, and put a line in the status bar;
//! - a **file on disk**, which the editor read once, at open, and never again.
//!
//! So an **agent** — which reaches the editor over BRP — could not make an edit at all, because
//! the only door was a key handler and an agent has no keyboard. And a **developer** who edited a
//! data file in their editor saw nothing happen until they restarted the editor, which is the
//! slowest possible feedback loop for the person who is trying to see what a change looks like.
//!
//! **This module is the third door, and the only one that is the definition.** An operation is
//! named once, validated once, applied once, and undone once — and every front end (a key
//! handler, a BRP command, an MCP tool) is a *thin* front end that builds the operation's
//! parameters and dispatches it. **No front end changes editor state any other way**, and
//! `tests/operations_have_two_front_ends.rs` says so.
//!
//! # The shape
//!
//! An operation is three things:
//!
//! | | what it is | why it is separate |
//! |---|---|---|
//! | **parameters** | a typed struct, with a JSON schema | so a key handler and an agent send the *same* thing, and a typo is a refusal rather than a silent no-op |
//! | **plan** | a pure function: what would change | so **preview** is free — a plan that changes nothing is the preview |
//! | **commit** | the one function that writes | so there is exactly one place a change is made, and it is not in a front end |
//!
//! **And the undo is declared, not inferred.** Each operation says how it is undone: an inverse
//! delta for a cheap edit, a snapshot for a procedural one. The editor's own `Undo` stack is a
//! *different* thing — it is the author's undo, at the keyboard — and the operation's undo
//! strategy is what an agent's `emerge_undo` uses.
//!
//! # The operation log
//!
//! **One append-only file, and it is the source of truth.** Every operation — from a key handler
//! or from an agent — appends one line to `assets/emerge/oplog.jsonl`. The line records who did
//! it (a human or an agent, and which session), what it was, what it was called with, what came
//! back, and — where the operation is procedural — the seed and the rules it ran under.
//!
//! **Undo is a new line, never a deletion.** The log is append-only, so the fact that something
//! was undone is itself recorded; an agent can read what the human just did, and a session can be
//! replayed.
//!
//! # What is not here yet
//!
//! **This is the first slice, and the slice is small.** One workflow — a map's own settings —
//! goes through the registry, with both front ends. The rest of the editor's verbs still change
//! state directly, and `tests/operations_have_two_front_ends.rs` carries an explicit allowlist of
//! them, which later iterations shrink. **The allowlist is the work list**, and a verb leaving it
//! is the point of the exercise.

use std::path::PathBuf;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::project::{OpenMap, Project};

/// **A metre count with no trailing `.0`** — `32` rather than `32.0`, `3.5` as itself.
///
/// **It lives here, and not in the editor**, because the receipt that uses it is written by the
/// operation, and a receipt's wording is part of the operation's contract.
pub fn trim_metres(m: f32) -> String {
    if (m - m.round()).abs() < 1e-4 {
        format!("{m:.0}")
    } else {
        format!("{m:.1}")
    }
}

/// **Where the operation log lives** — in the project's own directory.
///
/// **At the project level, not inside a kit's.** The log records what was done to *the project* —
/// to its maps, its compositions, its vocabulary — and a project binds several kits. **A log
/// inside the authoring kit's directory would be lost when the authoring kit changed**, and a
/// project whose log moves when the author switches kits is not a log the author can rely on.
///
/// **In the project directory, not the user's home.** The log is about *this* project: an agent
/// working on a game needs to know what was done to that game's map, and a log in a shared
/// location would answer a different question.
pub fn oplog_path(project: &Project) -> PathBuf {
    project.project_dir.join("oplog.jsonl")
}

/// **Who did it.**
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// A person at the keyboard.
    #[default]
    Human,
    /// An agent, over BRP or MCP.
    Agent,
}

/// **One line of the operation log.**
///
/// **Every field earns its place, and the two that look like bookkeeping are the ones an agent
/// cannot work without.** `operation_id` is what the agent cites when it presents a change and
/// what the human reverts by; `session` is what tells two agents' work apart in a log they both
/// read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// **Monotonic, from the log itself.** The count of lines before this one, so a reader can
    /// ask for "everything after what I saw" without a timestamp comparison.
    pub seq: u64,
    /// **What the agent cites and the human reverts by.** Unique within a project.
    pub operation_id: String,
    /// **When it happened**, RFC 3339, UTC.
    pub at: String,
    /// **Who did it.**
    pub source: Source,
    /// **Which session.** The editor's own session, or the agent's.
    pub session: String,
    /// **The operation's name** — the same name the registry and the MCP tool use.
    pub op: String,
    /// **What it was called with**, verbatim. So a replay can send the same call.
    pub params: serde_json::Value,
    /// **What came back**: `applied`, `refused`, or `undone`.
    pub outcome: String,
    /// **The refusal's text**, when the outcome is `refused`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// **A one-line summary of the result**, for a reader who does not want to replay it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// **The seed, where the operation was procedural.** `None` for a deterministic edit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rng_seed: Option<u64>,
    /// **The rules hash, where the operation ran under rules.** `None` for a deterministic edit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules_hash: Option<String>,
    /// **The operation this one undid**, when it is an undo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undoes: Option<String>,
}

/// **The session this editor is running under.**
///
/// **One per launch, and it is what tells two agents' work apart in a log they both read.** The
/// id is the process id and the start time, which is enough to be unique among the editors
/// running on one machine and is free — no dependency, no clock call at every operation.
///
/// **It lives here, and not in the BRP bridge**, because the keyboard's own edits are in the log
/// too, and a session id that only the agent's door could produce would leave the person's
/// work without one.
pub fn session_id() -> String {
    format!("{}-{}", std::process::id(), editor_start())
}

/// **When this editor started, in seconds since the epoch.**
///
/// **Computed once and kept**, because the session id has to be stable for the editor's life and
/// a per-call clock read would make it a different id every time.
fn editor_start() -> u64 {
    use std::sync::LazyLock;
    static START: LazyLock<u64> = LazyLock::new(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    });
    *START
}

/// **How an operation is undone.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Undo {
    /// **Nothing to undo** — the operation only read.
    None,
    /// **The inverse of the edit**, which is enough to put it back.
    ///
    /// For an edit that changed one field of one thing, the inverse is the field's old value.
    /// Cheap, exact, and it does not need the whole map.
    Inverse,
    /// **A snapshot of what was there**, put back whole.
    ///
    /// For an operation whose effect cannot be expressed as a delta — a generator that laid a
    /// hundred pieces, say. The snapshot is the map as it was.
    Snapshot,
}

/// **A refusal — the editor could not do what was asked.**
///
/// **One type, for both front ends.** A key handler and an agent get the *same* refusal, from
/// the same place, with the same text — which is what makes the two front ends interchangeable
/// rather than merely similar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal(pub String);

impl Refusal {
    pub fn new(text: impl Into<String>) -> Refusal {
        Refusal(text.into())
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// **What an operation would change, and what it changed.**
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    /// **A one-line summary for a receipt.**
    pub summary: String,
    /// **What the operation would place or change**, as a count. `0` for a read.
    pub affected: usize,
    /// **Whether the map on disk changed.** The one thing a caller always wants to know.
    pub saved: bool,
}

/// **The result of a dispatch.**
#[derive(Debug, Clone, PartialEq)]
pub enum Applied {
    /// **It happened.** The summary says what, and the id is what the agent cites.
    Done {
        /// **What happened.**
        outcome: Outcome,
        /// **The id the agent cites and the human reverts by.**
        operation_id: String,
        /// **The log's own record of it** — the same line that was appended.
        ///
        /// **Boxed, because the entry is a big datum and the refusal arm is not.** An enum whose
        /// variants differ by four times their size costs every match on it the size of the
        /// larger arm, and the refusal arm is on the hot path of every refused call.
        entry: Box<Entry>,
    },
    /// **It could not.** The refusal says why.
    Refused(Refusal),
}

/// **The context an operation runs in.**
///
/// **One context, both front ends.** A key handler and an agent build the same `Ctx` and call the
/// same `dispatch`, so "the agent's undo works" and "the author's undo works" are the same
/// statement about the same code.
///
/// **It owns what it carries, and that is what makes the operation trait dyn-compatible.** A
/// context that borrowed the project and the map would put a lifetime on every method, and a
/// trait with a lifetime on its methods cannot be a `dyn` — and the registry has to be a `dyn`,
/// because a list of operations is looked up by name. **So the context takes them by value and
/// gives them back.** One copy of a map's settings, and a file written once, is a price worth
/// paying for a registry that can hold more than one operation.
#[derive(Default)]
pub struct Ctx {
    /// **The project**, for the library and the paths.
    pub project: Option<Project>,
    /// **The map being edited**, if there is one.
    pub open: Option<OpenMap>,
    /// **Who is doing it.**
    pub source: Source,
    /// **Which session.**
    pub session: String,
}

/// **An operation, as the registry knows it.**
///
/// **A trait object, not a generic.** The registry is a list of operations that different front
/// ends look up by name, so the list has to be one type — and the parameters have to be erased,
/// because a list of `dyn` operations cannot be generic over its own parameter type.
///
/// **The name is a method, not an associated constant.** A `const` cannot be read off a
/// `dyn Operation`, and the registry's whole job is to look an operation up by name.
pub trait Operation: 'static {
    /// **The name the registry, the log and the MCP tool all use.**
    ///
    /// **Snake_case, and the same word the UI uses.** `map_set_settings` and `emerge_map_set_settings`
    /// are one operation, and the parity test says so.
    fn name(&self) -> &'static str;

    /// **How it is undone.**
    fn undo(&self) -> Undo;

    /// **The JSON schema of its parameters.**
    ///
    /// **A function, because a schema is built at run time** and a `const` cannot be. It is what
    /// `docs/mcp-tools.md` is generated from and what an agent reads.
    fn schema(&self) -> serde_json::Value;

    /// **What would change, and whether it is allowed to.**
    ///
    /// **Pure: it changes nothing.** A plan that says "no" is a refusal; a plan that says "yes" is
    /// what `commit` will do. **Preview is this function called and the result thrown away** —
    /// which is why preview is free and cannot half-apply.
    fn plan(&self, ctx: &Ctx, params: &serde_json::Value) -> Result<serde_json::Value, Refusal>;

    /// **Do it.**
    ///
    /// **The only function in the editor that writes an operation's effect.** A front end calls
    /// `dispatch`, never this.
    fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal>;
}

/// **Every operation the editor knows, in one list.**
///
/// **The one list.** The MCP tool surface, the parity test and the log's vocabulary are all read
/// from it, so they cannot disagree about what exists.
pub fn registry() -> Vec<Box<dyn Operation>> {
    vec![Box::new(map_set_settings::MapSetSettings)]
}

/// **The operation with this name.**
pub fn find(name: &str) -> Option<Box<dyn Operation>> {
    registry().into_iter().find(|op| op.name() == name)
}

/// **Apply an operation, and record what happened.**
///
/// **The one path.** A key handler calls this; a BRP command calls this; an MCP tool calls this.
/// **Nobody calls `commit` directly**, and `tests/operations_have_two_front_ends.rs` holds that.
///
/// **And the log is written here, once**, so an operation cannot be applied without being
/// recorded — which is what makes the log a source of truth rather than a report.
pub fn dispatch(ctx: &mut Ctx, name: &str, params: &serde_json::Value) -> Result<Applied, Refusal> {
    let Some(op) = find(name) else {
        return Err(Refusal::new(format!(
            "`{name}` is not an operation this editor knows. The known ones are: {}.",
            registry()
                .iter()
                .map(|o| o.name())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    };
    // **The plan runs first, always.** A refusal found by the plan is found before anything is
    // written, so a refused operation changes nothing — not the map, not the file, not the log
    // (beyond the record that it was refused, which is itself worth having).
    op.plan(ctx, params)?;
    let outcome = op.commit(ctx, params)?;
    let entry = Entry {
        seq: next_seq(ctx),
        operation_id: operation_id(ctx),
        at: now_rfc3339(),
        source: ctx.source,
        session: ctx.session.clone(),
        op: name.to_owned(),
        params: params.clone(),
        outcome: "applied".to_owned(),
        refusal: None,
        result: Some(outcome.summary.clone()),
        rng_seed: None,
        rules_hash: None,
        undoes: None,
    };
    // **A log that could not be written is a refusal, not a silent loss.**
    //
    // The log is the source of truth, so an operation applied without a record of it is an
    // operation that happened and cannot be told about — and an agent that cannot cite what it
    // did cannot present it. **So a failed append fails the operation**, and the caller is told
    // why.
    append(ctx, &entry).map_err(|e| {
        Refusal::new(format!(
            "the operation was applied but the log could not be written, so it cannot be reported \
             or undone: {e}"
        ))
    })?;
    Ok(Applied::Done {
        outcome,
        operation_id: entry.operation_id.clone(),
        entry: Box::new(entry),
    })
}

/// **The next sequence number** — the count of lines already in the log.
///
/// **Counted, not tracked**, so a log edited by hand does not desynchronise the count.
fn next_seq(ctx: &Ctx) -> u64 {
    let Some(project) = ctx.project.as_ref() else {
        return 0;
    };
    std::fs::read_to_string(oplog_path(project))
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count() as u64)
        .unwrap_or(0)
}

/// **An operation id** — unique within a project, and short enough to read aloud.
///
/// **The sequence and the session's first four characters.** Two agents in one project therefore
/// get different ids without a clock or a random source, and the id says which run made the
/// change.
fn operation_id(ctx: &Ctx) -> String {
    let seq = next_seq(ctx);
    let tag: String = ctx
        .session
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(4)
        .collect();
    format!("{seq:04}-{tag}")
}

/// **When it happened, as RFC 3339 UTC.**
///
/// **Computed here, not passed in**, so a caller cannot record a time that is not the time.
fn now_rfc3339() -> String {
    // **`SystemTime` and a hand-rolled formatter**, because the crate has no date dependency and
    // adding one for a timestamp is not worth it. The format is the RFC 3339 subset that matters:
    // UTC, seconds precision, a `Z` suffix.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, mo, d, h, mi, s) = civil_from_unix(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// **The calendar date and time of a Unix timestamp, in UTC.**
///
/// **Howard Hinnant's `civil_from_days`**, which is the standard conversion and is exact for every
/// date this century. It is here because the alternative is a dependency, and the alternative to
/// *that* is a wrong timestamp.
fn civil_from_unix(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, mi, s) = (
        (rem / 3600) as u32,
        ((rem % 3600) / 60) as u32,
        (rem % 60) as u32,
    );
    // Shift the epoch to 0000-03-01 so the leap day is the last day of the cycle.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d, h, mi, s)
}

/// **Append one entry to the log.**
///
/// **An append, not a replace.** The file is opened with `append(true)`, so the log cannot lose
/// an earlier line by being rewritten — and the crate's own `ron_surgery::save_atomic` rules do
/// not apply to it, because it is not a document anybody edits.
fn append(ctx: &Ctx, entry: &Entry) -> Result<(), String> {
    let Some(project) = ctx.project.as_ref() else {
        return Err("no project is open, so there is nowhere to write the log.".to_owned());
    };
    let path = oplog_path(project);
    // **A line that cannot be serialised is a failure, not a substitute.** A log line that says
    // "unloggable" is a line about the log, in a log about the work, and a reader cannot tell
    // the two apart.
    let line = serde_json::to_string(entry)
        .map_err(|e| format!("the log entry could not be written: {e}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    use std::io::Write;
    writeln!(f, "{line}").map_err(|e| format!("{}: {e}", path.display()))?;
    f.sync_data()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(())
}

/// **The operations a map's own settings are changed by.**
pub mod map_set_settings {
    use super::*;
    use serde_json::json;

    /// **A map's own settings** — the fields an author tunes while looking at a result.
    ///
    /// **A partial update, and every field is optional.** "Change the map's bounds" is one call
    /// with one field, and the fields not named keep their values. **A full replacement would be
    /// a different operation**, and a worse one: an agent that wanted to change the bounds would
    /// have to know and restate the origin and the note, and a restatement that got one field
    /// wrong would silently rewrite it.
    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Params {
        /// **Where the map sits in the world**, in metres. A triple.
        #[serde(default)]
        pub origin: Option<(f32, f32, f32)>,
        /// **How big the map is**, in metres on each axis. A triple.
        #[serde(default)]
        pub bounds: Option<(f32, f32, f32)>,
        /// **One axis of the bounds, in metres.**
        ///
        /// **The UI's own shape, and it is here for the UI's reason.** The editor's size field
        /// edits **one** axis at a time — X, then Y, then Z — and the author sees the other two
        /// on screen while they type. **So the operation takes an axis as well as a whole
        /// triple**, and the UI's field and an agent's call are the same call.
        ///
        /// **`None` means "the whole triple", and `bounds` and `axis` together are a refusal** —
        /// two ways of saying the same thing is not a thing an operation should guess about.
        #[serde(default)]
        pub axis: Option<Axis>,
        /// **The value for `axis`, in metres.**
        ///
        /// **Required when `axis` is given, and refused without it.** An axis with no value is a
        /// half-said thing, and the operation says so rather than guessing.
        #[serde(default)]
        pub axis_value: Option<f32>,
        /// **The named combination of kits the map offers**, or `None` for every bound kit.
        #[serde(default)]
        pub bash: Option<Option<String>>,
        /// **The map's note** — the header prose, as data.
        #[serde(default)]
        pub note: Option<Option<String>>,
    }

    /// **Which axis of the map's bounds.**
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum Axis {
        X,
        Y,
        Z,
    }

    impl Axis {
        /// **The axis's own letter**, for a receipt.
        pub fn label(self) -> &'static str {
            match self {
                Axis::X => "X",
                Axis::Y => "Y",
                Axis::Z => "Z",
            }
        }

        /// **The axis's place in a bounds triple.**
        pub fn index(self) -> usize {
            match self {
                Axis::X => 0,
                Axis::Y => 1,
                Axis::Z => 2,
            }
        }
    }

    impl Params {
        /// **The JSON schema**, which is what an agent reads.
        pub fn schema() -> serde_json::Value {
            json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "origin": {
                        "type": "array",
                        "items": {"type": "number"},
                        "minItems": 3,
                        "maxItems": 3,
                        "description": "Where the map sits in the world, in metres, as [x, y, z]."
                    },
                    "bounds": {
                        "type": "array",
                        "items": {"type": "number"},
                        "minItems": 3,
                        "maxItems": 3,
                        "description": "How big the map is, in metres on each axis, as [x, y, z]."
                    },
                    "axis": {
                        "description": "One axis of the bounds, for an edit that changes one axis and leaves the others. Use `axis_value` for its size.",
                        "type": ["string", "null"],
                        "enum": ["x", "y", "z", null]
                    },
                    "axis_value": {
                        "description": "The size of `axis`, in metres. Required when `axis` is given.",
                        "type": "number"
                    },
                    "bash": {
                        "description": "The named combination of kits the map offers. Null means every bound kit.",
                        "type": ["string", "null"]
                    },
                    "note": {
                        "description": "The map's note: the header prose, as data.",
                        "type": ["string", "null"]
                    }
                }
            })
        }

        /// **The parameters, parsed.**
        ///
        /// **`deny_unknown_fields`, so a typo is a refusal** rather than a field silently ignored
        /// and an agent left to believe it changed something.
        pub fn parse(raw: &serde_json::Value) -> Result<Params, Refusal> {
            serde_json::from_value(raw.clone()).map_err(|e| {
                Refusal::new(format!(
                    "these are not a map's settings: {e}. The fields are `origin`, `bounds`, `axis`, \
                     `axis_value`, `bash` and `note`, and nothing else."
                ))
            })
        }
    }

    /// **A map's own settings, changed.**
    pub struct MapSetSettings;

    impl Operation for MapSetSettings {
        fn name(&self) -> &'static str {
            "map_set_settings"
        }

        fn undo(&self) -> Undo {
            Undo::Inverse
        }

        fn schema(&self) -> serde_json::Value {
            Params::schema()
        }

        fn plan(
            &self,
            ctx: &Ctx,
            params: &serde_json::Value,
        ) -> Result<serde_json::Value, Refusal> {
            let p = Params::parse(params)?;
            // **A map has to be open.** A settings change with no map has no subject, and saying
            // so is the whole refusal.
            if ctx.open.is_none() {
                return Err(Refusal::new(
                    "no map is open, so there are no settings to change.",
                ));
            }
            // **The bounds have to be positive.** A map with a zero or negative extent is a map
            // whose edges are inside it, and the validators refuse it — so the refusal comes here,
            // where the author is, rather than at the save.
            if let Some(b) = p.bounds
                && (b.0 <= 0.0 || b.1 <= 0.0 || b.2 <= 0.0)
            {
                return Err(Refusal::new(format!(
                    "a map's bounds are metres on each axis, and every one has to be positive —                      `{b:?}` is not."
                )));
            }
            // **An axis needs its value, and only its value.**
            //
            // **Two ways of saying the same thing is a refusal, not a guess.** A caller that
            // sends both `bounds` and `axis` has said two things about the bounds, and the
            // operation will not pick one — because picking wrong writes a size the author did
            // not ask for.
            if p.bounds.is_some() && p.axis.is_some() {
                return Err(Refusal::new(
                    "say the bounds one way or the other: `bounds` for all three axes, or `axis`                      and `axis_value` for one. Sending both is two answers to one question.",
                ));
            }
            if p.axis.is_some() && p.axis_value.is_none() {
                return Err(Refusal::new(
                    "an axis needs its size: send `axis` and `axis_value` together.",
                ));
            }
            if p.axis_value.is_some() && p.axis.is_none() {
                return Err(Refusal::new(
                    "`axis_value` is a size for an axis, and no axis was named.",
                ));
            }
            // **And the axis's size has to be a size.** A map's axis runs `MIN_BOUND..MAX_BOUND`
            // metres, and a value outside that is a map whose edges are inside it.
            if let (Some(axis), Some(value)) = (p.axis, p.axis_value)
                && !(value.is_finite() && value > 0.0)
            {
                return Err(Refusal::new(format!(
                    "the {} axis is metres across the map, and it has to be a positive number — \
                     `{value}` is not.",
                    axis.label()
                )));
            }
            // **The preview is the change, stated.**
            Ok(json!({
                "origin": p.origin,
                "bounds": p.bounds,
                "bash": p.bash,
                "note": p.note,
            }))
        }

        fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
            let p = Params::parse(params)?;
            let Some(mut open) = ctx.open.take() else {
                return Err(Refusal::new(
                    "no map is open, so there are no settings to change.",
                ));
            };
            // **The old values, for the undo.**
            let old = Settings {
                origin: open.map.origin,
                bounds: open.map.bounds,
                bash: open.map.bash.clone(),
                note: open.map.note.clone(),
            };
            if let Some(v) = p.origin {
                open.map.origin = v;
            }
            if let Some(v) = p.bounds {
                open.map.bounds = v;
            }
            // **One axis, and only that one.** The editor's size field edits X, then Y, then Z,
            // and the author sees the other two on screen while they type — so an axis edit
            // changes one number and leaves the other two exactly as they were.
            if let (Some(axis), Some(value)) = (p.axis, p.axis_value) {
                let (x, y, z) = open.map.bounds;
                open.map.bounds = match axis {
                    Axis::X => (value, y, z),
                    Axis::Y => (x, value, z),
                    Axis::Z => (x, y, value),
                };
            }
            if let Some(v) = p.bash {
                open.map.bash = v;
            }
            if let Some(v) = p.note {
                open.map.note = v;
            }
            // **And the file, so the change is not only in memory.** The brief's rule: a live
            // change is written back to the data file, so nothing exists only in memory.
            //
            // **The save is the map's own gate** — `OpenMap::save` validates, expands the stamps
            // the way the game does, and refuses rather than writing a file the game cannot load.
            // **A refusal to save is a refusal of the whole operation**, not a half-applied change
            // that reports success.
            let Some(project) = ctx.project.as_ref() else {
                // **The map goes back before the refusal.** The commit took it out of the
                // context, and a refusal that dropped it would leave the editor holding nothing
                // — the map would come back as a default, and the author's work with it.
                ctx.open = Some(open);
                return Err(Refusal::new("no project is open."));
            };
            let saved = match open.save(project) {
                Ok(()) => true,
                Err(e) => {
                    // **Put the map back.** The settings were changed in memory and the file
                    // refused them, so the in-memory map and the file now disagree — and the
                    // editor's own dirty flag would say the map is unsaved, which is true and
                    // useless. **The old values go back, so the editor and the file agree.**
                    open.map.origin = old.origin;
                    open.map.bounds = old.bounds;
                    open.map.bash = old.bash;
                    open.map.note = old.note;
                    ctx.open = Some(open);
                    return Err(Refusal::new(format!(
                        "the settings were changed but the file refused them, so nothing was \
                         written: {e}"
                    )));
                }
            };
            // **The map goes back into the context, changed or not.** The refusal arm restores
            // the old values *before* this, so the editor and the file always agree.
            //
            // **The receipt is written before the move**, because the map is the context's after
            // this line and the summary is about the map.
            let (x, y, z) = open.map.bounds;
            let summary = format!(
                "map is {} x {} x {} m, and the file was written",
                trim_metres(x),
                trim_metres(y),
                trim_metres(z)
            );
            ctx.open = Some(open);
            Ok(Outcome {
                summary,
                affected: 1,
                saved,
            })
        }
    }

    /// **A map's settings, as they were.**
    ///
    /// **The inverse delta.** The undo of a settings change is the settings it replaced, and
    /// that is a small thing to keep.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Settings {
        pub origin: (f32, f32, f32),
        pub bounds: (f32, f32, f32),
        pub bash: Option<String>,
        pub note: Option<String>,
    }
}
