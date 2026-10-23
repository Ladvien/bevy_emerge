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
    /// **What came back**: `applied` or `refused`. An undo is an `applied` line whose `undoes`
    /// names the line it took back.
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
    /// **The parameters that put this line's change back**, for the same operation.
    ///
    /// **Recorded with the change, because only the change knows what it replaced.** An undo
    /// read from the log afterwards cannot reconstruct the old values from the new ones. Lines
    /// written before this field existed have none, and [`undo`] refuses them by name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inverse: Option<serde_json::Value>,
    /// **The state this line left behind**, for the guard [`undo`] runs. See [`Outcome::after`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<serde_json::Value>,
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
    /// **The parameters that put this change back**, for the same operation — what
    /// [`undo`] dispatches. `None` for an operation that declares no inverse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inverse: Option<serde_json::Value>,
    /// **The part of the editor's state this change left behind**, as [`Operation::state`] reads
    /// it. [`undo`] refuses when the state is no longer this, because something outside the log
    /// changed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<serde_json::Value>,
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

    /// **The part of the editor's state this operation's inverse restores**, named so it can be
    /// compared: an undo is safe only while the state is still what the change left. `None` for an
    /// operation with no inverse.
    fn state(&self, _ctx: &Ctx) -> Option<serde_json::Value> {
        None
    }

    /// **The same part, as the change recorded it in `after`**, for an operation whose change
    /// touches one part of the map. An undo compares only that part, so an unrelated edit made
    /// by hand does not block it. Defaults to the whole [`Operation::state`].
    fn state_of(&self, ctx: &Ctx, _after: &serde_json::Value) -> Option<serde_json::Value> {
        self.state(ctx)
    }

    /// **Whether these parameters are a change only an undo may make**, so a dispatch that is not an
    /// undo refuses them. `false` for every operation but a removal's put-back (gap 2).
    fn only_as_undo(&self, _params: &serde_json::Value) -> bool {
        false
    }
}

/// **Every operation the editor knows, in one list.**
///
/// **The one list.** The MCP tool surface, the parity test and the log's vocabulary are all read
/// from it, so they cannot disagree about what exists.
pub fn registry() -> Vec<Box<dyn Operation>> {
    vec![
        Box::new(map_set_settings::MapSetSettings),
        Box::new(placement_set_pin::PlacementSetPin),
        Box::new(placement_set_yaw::PlacementSetYaw),
        Box::new(placement_remove::PlacementRemove),
        Box::new(stamp_remove::StampRemove),
        Box::new(map_rename::MapRename),
        Box::new(brush_arm::BrushArm),
    ]
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
    dispatch_as(ctx, name, params, None)
}

/// **The refusal for a change only an undo may make**, the same wherever it is found: [`dispatch`]
/// and [`preview`] both give it.
fn undo_only_refusal(name: &str) -> Refusal {
    Refusal::new(format!(
        "`{name}` with `put_back` is the undo of a logged removal, not a change. Take that removal \
         back with `emerge/operation_undo`, or Cmd+Z in the editor. Nothing was put back."
    ))
}

/// **What [`dispatch`] would do, without doing it.** A preview goes through this, so it refuses
/// what dispatch refuses: a preview never says "would change" for a change dispatch would not make.
pub fn preview(
    ctx: &Ctx,
    op: &dyn Operation,
    params: &serde_json::Value,
) -> Result<serde_json::Value, Refusal> {
    if op.only_as_undo(params) {
        return Err(undo_only_refusal(op.name()));
    }
    op.plan(ctx, params)
}

/// **[`dispatch`], with the line it undoes named** — the one path for a change and for its undo,
/// so an undo is validated, written and logged exactly as the change was.
fn dispatch_as(
    ctx: &mut Ctx,
    name: &str,
    params: &serde_json::Value,
    undoes: Option<&str>,
) -> Result<Applied, Refusal> {
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
    // **A change only an undo may make is refused from anywhere else** (gap 2). Only [`undo`] passes
    // `undoes`, and it dispatches the line's own recorded inverse, so this is the one door a put-back
    // comes through. Applied on its own, it would bring back a piece the person may have removed again.
    if undoes.is_none() && op.only_as_undo(params) {
        return Err(undo_only_refusal(name));
    }
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
        undoes: undoes.map(str::to_owned),
        inverse: outcome.inverse.clone(),
        after: outcome.after.clone(),
    };
    // **A log that could not be written is a refusal, and the change does not stay.**
    //
    // The log is the source of truth. An operation applied without a record of it is an operation
    // that happened and cannot be told about or undone, so a failed append takes the change back
    // through its recorded inverse and says so. The take-back itself is not logged: nothing the
    // log should describe happened. See [`take_back`].
    append(ctx, &entry).map_err(|e| take_back(ctx, &*op, &outcome, e))?;
    Ok(Applied::Done {
        outcome,
        operation_id: entry.operation_id.clone(),
        entry: Box::new(entry),
    })
}

/// **A change whose log line cannot be written is taken back, or the refusal says it stayed.**
///
/// The change is already in the map and the file when the append fails. Its recorded inverse is
/// the put-back, so it is committed through the same operation, with no log line. The refusal
/// then says the change was taken back. When there is no inverse, or the take-back itself fails,
/// the refusal says the change is still in place rather than claiming it was undone.
fn take_back(ctx: &mut Ctx, op: &dyn Operation, outcome: &Outcome, log_error: String) -> Refusal {
    let Some(inverse) = outcome.inverse.as_ref() else {
        // **An operation that changed nothing has nothing to keep in place.** Saying "still in
        // place" would tell the author a change exists on the map when none does.
        if !outcome.saved && outcome.affected == 0 {
            return Refusal::new(format!(
                "the operation could not be logged, so nothing was changed: {log_error}"
            ));
        }
        return Refusal::new(format!(
            "the operation was applied but could not be logged, and it records no inverse to take \
             it back, so the change is still in place: {log_error}"
        ));
    };
    match op.commit(ctx, inverse) {
        Ok(_) => Refusal::new(format!(
            "the operation could not be logged, so it was taken back and nothing was changed: \
             {log_error}"
        )),
        Err(Refusal(why)) => Refusal::new(format!(
            "the operation was applied but could not be logged, and taking it back failed, so the \
             change is still in place: {log_error}. Taking it back said: {why}"
        )),
    }
}

/// **What an undo resolved to**: the change being taken back or put back, the log line whose
/// inverse does it, and that inverse.
#[derive(Debug, Clone)]
pub struct UndoPlan {
    /// **The change**, by the id of the line that first made it.
    pub change: String,
    /// **The line whose inverse is dispatched**: the change itself, or the newest undo or redo of
    /// it. The new line's `undoes` names this one.
    pub line: Entry,
    /// **The parameters dispatched.**
    pub inverse: serde_json::Value,
    /// **Whether this puts the change back into effect** (a redo) rather than taking it back. The
    /// editor's history records a put-back as its newest change, exactly as it records a new one.
    pub restores: bool,
}

/// **Which change an undo takes back, and the parameters that do it.**
///
/// **Pure: it reads the log and changes nothing**, so a preview and an undo resolve the same way.
///
/// **A change is a chain**: the line that made it, then any undo of it, then any undo of that (a
/// redo), and so on. Each chain is in effect or not, and its newest line is the one whose inverse
/// flips it. With no `target`, the newest change in effect is taken back. A `target` names any line
/// of a chain: in effect, the chain is taken back; out of effect, naming the chain's newest line
/// puts it back (a redo), and naming an older line is refused, pointing at the newest.
///
/// **Two guards, because a newer change must never be overwritten:**
/// - a change still in effect that is newer than this chain's newest line blocks it, named;
/// - the state must still be what the chain's newest line left ([`Operation::state`]), or
///   something changed it outside the log (a hand-edited file the watcher adopted, the menu, or a
///   different map open), and the undo is refused, naming what differs.
pub fn undo_target(ctx: &Ctx, target: Option<&str>) -> Result<UndoPlan, Refusal> {
    use std::collections::{HashMap, HashSet};
    let Some(project) = ctx.project.as_ref() else {
        return Err(Refusal::new(
            "no project is open, so there is no log to undo from.",
        ));
    };
    let path = oplog_path(project);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines = Vec::new();
    for (n, raw) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let entry: Entry = serde_json::from_str(raw).map_err(|e| {
            Refusal::new(format!(
                "line {} of {} is not a log entry ({e}), so the log cannot say what the newest \
                 change is. Nothing was changed.",
                n + 1,
                path.display()
            ))
        })?;
        if entry.outcome == "applied" && find(&entry.op).is_none_or(|op| op.undo() != Undo::None) {
            lines.push(entry);
        }
    }
    // **Each line's chain, whether each chain is in effect, and each chain's newest line.**
    let mut chain_of: HashMap<&str, &str> = HashMap::new();
    let mut in_effect: HashSet<&str> = HashSet::new();
    let mut newest: HashMap<&str, usize> = HashMap::new();
    for (i, line) in lines.iter().enumerate() {
        let chain = match line.undoes.as_deref() {
            None => line.operation_id.as_str(),
            Some(undone) => chain_of.get(undone).copied().unwrap_or(undone),
        };
        chain_of.insert(line.operation_id.as_str(), chain);
        // A new change starts in effect; an undo line flips its chain.
        if line.undoes.is_none() || !in_effect.remove(chain) {
            in_effect.insert(chain);
        }
        newest.insert(chain, i);
    }
    // **The log is the project's; a change belongs to one map.** A line's map is the one its
    // recorded state names, and only changes to the open map are picked, or can block.
    let here = ctx.open.as_ref().map(|o| o.map.name.as_str());
    let map_of = |l: &Entry| {
        l.after
            .as_ref()
            .and_then(|a| a.get("map"))
            .and_then(|m| m.as_str())
            .map(str::to_owned)
    };
    let chain_map = |c: &str| map_of(&lines[newest[c]]);
    let chain: &str = match target {
        None => {
            let Some(chain) = in_effect
                .iter()
                .copied()
                .filter(|c| chain_map(c).as_deref() == here)
                .max_by_key(|c| newest[c])
            else {
                return Err(Refusal::new(
                    "nothing to undo: no change to the open map is still in effect.",
                ));
            };
            chain
        }
        Some(id) => {
            let Some(chain) = chain_of.get(id).copied() else {
                return Err(Refusal::new(format!(
                    "no operation `{id}` is in the log. emerge_read_oplog lists the ids."
                )));
            };
            let head = &lines[newest[chain]].operation_id;
            if !in_effect.contains(chain) && head != id {
                return Err(Refusal::new(format!(
                    "`{id}` is not in effect: `{head}` took it back. Undo `{head}` to put it back."
                )));
            }
            chain
        }
    };
    let line = &lines[newest[chain]];
    let line_map = map_of(line);
    if let Some(m) = line_map.as_deref()
        && Some(m) != here
    {
        return Err(Refusal::new(format!(
            "`{}` changed map `{m}`, and the open map is `{}`. Open `{m}` to take it back.",
            line.operation_id,
            here.unwrap_or("none")
        )));
    }
    // **Guard 1: a newer change to the same map, logged, still in effect.**
    if let Some(blocker) = in_effect
        .iter()
        .filter(|c| **c != chain && newest[*c] > newest[chain] && chain_map(c) == line_map)
        .map(|c| &lines[newest[c]])
        .max_by_key(|l| l.seq)
    {
        return Err(Refusal::new(format!(
            "`{}` ({} by {:?}) changed the map after `{}`, so putting the old values back would \
             overwrite it. Undo `{}` first, or leave `{}` as it is.",
            blocker.operation_id,
            blocker.op,
            blocker.source,
            line.operation_id,
            blocker.operation_id,
            line.operation_id
        )));
    }
    let Some(op) = find(&line.op) else {
        return Err(Refusal::new(format!(
            "`{}` is not an operation this editor knows.",
            line.op
        )));
    };
    // **Guard 2: the state is what the line left, or something changed it since.**
    if let (Some(left), Some(now)) = (
        line.after.as_ref(),
        line.after.as_ref().and_then(|a| op.state_of(ctx, a)),
    ) && *left != now
    {
        // **A logged change is named first**: an undo or redo of another change to this map can
        // be what moved the state, and blaming an edit outside the log would be false.
        if let Some(later) = lines[newest[chain] + 1..]
            .iter()
            .rev()
            .find(|l| map_of(l) == line_map)
        {
            return Err(Refusal::new(format!(
                "`{}` ({} by {:?}) changed the map after `{}`, so putting the old values back \
                 would overwrite it. Take `{}` back, or put it back, first.",
                later.operation_id, later.op, later.source, line.operation_id, later.operation_id
            )));
        }
        let differs: Vec<String> = match (left.as_object(), now.as_object()) {
            (Some(l), Some(n)) => l
                .keys()
                .filter(|k| l.get(*k) != n.get(*k))
                .map(|k| {
                    format!(
                        "{k} is {} and `{}` left {}",
                        n.get(k.as_str()).unwrap_or(&serde_json::Value::Null),
                        line.operation_id,
                        l[k.as_str()]
                    )
                })
                .collect(),
            _ => vec![format!(
                "the state is {now} and `{}` left {left}",
                line.operation_id
            )],
        };
        return Err(Refusal::new(format!(
            "the map has changed outside the operation log since `{}` ({}): a file edited by hand, \
             the menu, or a different map open. Putting the old values back would overwrite that, \
             so nothing was changed.",
            line.operation_id,
            differs.join("; ")
        )));
    }
    let Some(inverse) = line.inverse.clone() else {
        return Err(Refusal::new(format!(
            "`{}` recorded no inverse (it was logged before undo existed), so it cannot be taken \
             back from the log.",
            line.operation_id
        )));
    };
    Ok(UndoPlan {
        change: chain.to_owned(),
        line: line.clone(),
        inverse,
        restores: !in_effect.contains(chain),
    })
}

/// **Take one change back, or put it back, by dispatching the inverse its newest line recorded.**
///
/// **The undo is an operation like any other**: the inverse goes through [`dispatch`]'s own path
/// (plan, commit, the file written, a new log line) and the new line's `undoes` names the line it
/// inverted. **Nothing is deleted from the log.**
pub fn undo(ctx: &mut Ctx, target: Option<&str>) -> Result<(UndoPlan, Applied), Refusal> {
    let plan = undo_target(ctx, target)?;
    let applied = dispatch_as(
        ctx,
        &plan.line.op,
        &plan.inverse,
        Some(&plan.line.operation_id),
    )?;
    Ok((plan, applied))
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
        ///
        /// **Absent leaves it alone; `null` clears it.** See [`present`].
        #[serde(default, deserialize_with = "present")]
        pub bash: Option<Option<String>>,
        /// **The map's note** — the header prose, as data. Absent leaves it; `null` clears it.
        #[serde(default, deserialize_with = "present")]
        pub note: Option<Option<String>>,
    }

    /// **A field that is present is `Some`, even when it is `null`.**
    ///
    /// serde reads an explicit `null` into `Option<Option<T>>` as the outer `None` — the same as an
    /// absent field — so `"note": null` meant "leave the note alone", the operation answered
    /// `applied`, and nothing changed, while the schema says null clears it. **Called only when the
    /// field is present** (`#[serde(default)]` covers absence), so wrapping in `Some` is exact:
    /// absent is `None`, `null` is `Some(None)`, a value is `Some(Some(v))`.
    fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Option::<T>::deserialize(deserializer).map(Some)
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

        fn state(&self, ctx: &Ctx) -> Option<serde_json::Value> {
            ctx.open.as_ref().map(settings_of)
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
            // **The inverse is the settings this change replaced, all four of them.** Naming every
            // field, rather than only the ones that changed, keeps the undo exact when a field was
            // absent (`null` clears it) and keeps it one call.
            let inverse = serde_json::json!({
                "origin": [old.origin.0, old.origin.1, old.origin.2],
                "bounds": [old.bounds.0, old.bounds.1, old.bounds.2],
                "bash": old.bash,
                "note": old.note,
            });
            let after = ctx.open.as_ref().map(settings_of);
            Ok(Outcome {
                summary,
                affected: 1,
                saved,
                inverse: Some(inverse),
                after,
            })
        }
    }

    /// **Which map, and its four settings** — what an undo of a settings change compares before it
    /// puts old values back. **The map's name is part of it**, because the log is the project's and
    /// the operation acts on whichever map is open.
    pub fn settings_of(open: &OpenMap) -> serde_json::Value {
        json!({
            "map": open.map.name,
            "origin": [open.map.origin.0, open.map.origin.1, open.map.origin.2],
            "bounds": [open.map.bounds.0, open.map.bounds.1, open.map.bounds.2],
            "bash": open.map.bash,
            "note": open.map.note,
        })
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

/// **One placement's pin, set or cleared** — the operation behind the O key and the pin-reason field.
///
/// **Addressed by the placement's id, not its row.** A row is where the piece sits in the list
/// this minute; the id is what the piece is called, and it does not change when another piece is
/// removed. The undo names the id for the same reason.
pub mod placement_set_pin {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Params {
        /// The placement's id, as the map file and `emerge/state` name it.
        pub id: String,
        /// `true` pins the piece; `false` unpins it.
        pub pinned: bool,
        /// **Why the piece is held.** Required when `pinned` is true, and refused when it is not:
        /// an unpin takes no reason, and sending one is two answers to one question.
        #[serde(default)]
        pub because: Option<String>,
    }

    impl Params {
        /// **The JSON schema**, which is what an agent reads.
        pub fn schema() -> serde_json::Value {
            json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["id", "pinned"],
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "The placement's id, as the map file names it."
                    },
                    "pinned": {
                        "type": "boolean",
                        "description": "true pins the piece so the solver routes around it; false unpins it."
                    },
                    "because": {
                        "type": ["string", "null"],
                        "description": "Why the piece is held. Required when pinned is true; refused otherwise."
                    }
                }
            })
        }

        /// **The parameters, parsed.** `deny_unknown_fields`, so a typo is a refusal.
        pub fn parse(raw: &serde_json::Value) -> Result<Params, Refusal> {
            serde_json::from_value(raw.clone()).map_err(|e| {
                Refusal::new(format!(
                    "these are not a placement's pin: {e}. The fields are `id`, `pinned` and \
                     `because`, and nothing else."
                ))
            })
        }
    }

    /// **The one rule for a pin's reason**, in one place: a pin has a reason, an unpin has none.
    /// Returns the reason as it will be stored, trimmed.
    fn reason_of(p: &Params) -> Result<Option<String>, Refusal> {
        match (p.pinned, p.because.as_deref().map(str::trim)) {
            (true, Some(r)) if !r.is_empty() => Ok(Some(r.to_owned())),
            (true, _) => Err(Refusal::new(
                "a pin needs a reason, and it is what the author will read when they ask why the \
                 piece is held. Nothing was pinned.",
            )),
            (false, None) => Ok(None),
            (false, Some(_)) => Err(Refusal::new(
                "an unpin takes no reason, so `because` has to be left out. Nothing was unpinned.",
            )),
        }
    }

    /// **The pins a map holds, as its id and reason**, which is the state an undo checks.
    fn pins_of(open: &OpenMap) -> serde_json::Value {
        let pins: Vec<serde_json::Value> = open
            .map
            .placements
            .iter()
            .filter(|p| p.owned)
            .map(|p| json!([p.id, p.owned_because]))
            .collect();
        json!({"map": open.map.name, "pins": pins})
    }

    /// **Set or clear one placement's pin.**
    pub struct PlacementSetPin;

    impl Operation for PlacementSetPin {
        fn name(&self) -> &'static str {
            "placement_set_pin"
        }

        fn undo(&self) -> Undo {
            Undo::Inverse
        }

        fn schema(&self) -> serde_json::Value {
            Params::schema()
        }

        fn state(&self, ctx: &Ctx) -> Option<serde_json::Value> {
            ctx.open.as_ref().map(pins_of)
        }

        fn plan(&self, ctx: &Ctx, params: &serde_json::Value) -> Result<serde_json::Value, Refusal> {
            let p = Params::parse(params)?;
            let Some(open) = ctx.open.as_ref() else {
                return Err(Refusal::new(
                    "no map is open, so there are no placements to pin.",
                ));
            };
            if !open.map.placements.iter().any(|q| q.id == p.id) {
                return Err(Refusal::new(format!(
                    "no placement is called `{}` on this map, so nothing was pinned.",
                    p.id
                )));
            }
            let because = reason_of(&p)?;
            Ok(json!({"id": p.id, "pinned": p.pinned, "because": because}))
        }

        fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
            let p = Params::parse(params)?;
            let because = reason_of(&p)?;
            let Some(mut open) = ctx.open.take() else {
                return Err(Refusal::new(
                    "no map is open, so there are no placements to pin.",
                ));
            };
            let Some(index) = open.map.placements.iter().position(|q| q.id == p.id) else {
                ctx.open = Some(open);
                return Err(Refusal::new(format!(
                    "no placement is called `{}` on this map, so nothing was pinned.",
                    p.id
                )));
            };
            // **The old pin, for the undo.** Its inverse is the same operation with these values.
            let old_owned = open.map.placements[index].owned;
            let old_because = open.map.placements[index].owned_because.clone();
            open.map.placements[index].owned = p.pinned;
            open.map.placements[index].owned_because = because.clone();
            let summary = match &because {
                Some(reason) => format!("pinned {}: {reason}", p.id),
                None => format!("unpinned {}", p.id),
            };
            // **And the file, so the pin is not only in memory.** The save is the map's own gate;
            // a refusal to save puts the old pin back, so the editor and the file agree.
            let Some(project) = ctx.project.as_ref() else {
                ctx.open = Some(open);
                return Err(Refusal::new("no project is open."));
            };
            let saved = match open.save(project) {
                Ok(()) => true,
                Err(e) => {
                    open.map.placements[index].owned = old_owned;
                    open.map.placements[index].owned_because = old_because;
                    ctx.open = Some(open);
                    return Err(Refusal::new(format!(
                        "the pin was changed but the file refused it, so nothing was written: {e}"
                    )));
                }
            };
            let inverse = json!({"id": p.id, "pinned": old_owned, "because": old_because});
            ctx.open = Some(open);
            let after = ctx.open.as_ref().map(pins_of);
            Ok(Outcome {
                summary,
                affected: 1,
                saved,
                inverse: Some(inverse),
                after,
            })
        }
    }
}

/// **Turn one placement to a facing**, which is what the turn keys do and what an agent asks for.
pub mod placement_set_yaw {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Params {
        /// The placement's id, as the map file and `emerge/state` name it.
        pub id: String,
        /// The facing in degrees. Wrapped into `[0, 360)` before it is stored, so `-15` is `345`.
        pub yaw: f32,
    }

    impl Params {
        /// **The JSON schema**, which is what an agent reads.
        pub fn schema() -> serde_json::Value {
            json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["id", "yaw"],
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "The placement's id, as the map file names it."
                    },
                    "yaw": {
                        "type": "number",
                        "description": "The facing in degrees. Wrapped into 0 to 360, so -15 is 345."
                    }
                }
            })
        }

        /// **The parameters, parsed.** `deny_unknown_fields`, so a typo is a refusal.
        pub fn parse(raw: &serde_json::Value) -> Result<Params, Refusal> {
            let p: Params = serde_json::from_value(raw.clone()).map_err(|e| {
                Refusal::new(format!(
                    "these are not a placement's facing: {e}. The fields are `id` and `yaw`, and \
                     nothing else."
                ))
            })?;
            if !p.yaw.is_finite() {
                return Err(Refusal::new(
                    "a facing has to be a finite number of degrees. Nothing was turned.",
                ));
            }
            Ok(p)
        }
    }

    /// **The facing a placement is stored with**: wrapped into `[0, 360)`, as the keys wrap it.
    fn wrapped(yaw: f32) -> f32 {
        yaw.rem_euclid(360.0)
    }

    /// **The facings a map holds, as id and degrees**, which is the state an undo checks.
    fn yaws_of(open: &OpenMap) -> serde_json::Value {
        let yaws: Vec<serde_json::Value> = open
            .map
            .placements
            .iter()
            .map(|p| json!([p.id, p.yaw]))
            .collect();
        json!({"map": open.map.name, "yaws": yaws})
    }

    /// **Turn one placement to a facing.**
    pub struct PlacementSetYaw;

    impl Operation for PlacementSetYaw {
        fn name(&self) -> &'static str {
            "placement_set_yaw"
        }

        fn undo(&self) -> Undo {
            Undo::Inverse
        }

        fn schema(&self) -> serde_json::Value {
            Params::schema()
        }

        fn state(&self, ctx: &Ctx) -> Option<serde_json::Value> {
            ctx.open.as_ref().map(yaws_of)
        }

        /// **The turned piece's facing now**, against the one its change recorded. A log line
        /// without a piece id (written before this was scoped) falls back to the whole map.
        fn state_of(&self, ctx: &Ctx, after: &serde_json::Value) -> Option<serde_json::Value> {
            let Some(id) = after.get("id").and_then(serde_json::Value::as_str) else {
                return self.state(ctx);
            };
            let open = ctx.open.as_ref()?;
            let p = open.map.placements.iter().find(|q| q.id == id)?;
            Some(json!({"id": p.id, "yaw": p.yaw}))
        }

        fn plan(
            &self,
            ctx: &Ctx,
            params: &serde_json::Value,
        ) -> Result<serde_json::Value, Refusal> {
            let p = Params::parse(params)?;
            let Some(open) = ctx.open.as_ref() else {
                return Err(Refusal::new(
                    "no map is open, so there are no placements to turn.",
                ));
            };
            if !open.map.placements.iter().any(|q| q.id == p.id) {
                return Err(Refusal::new(format!(
                    "no placement is called `{}` on this map, so nothing was turned.",
                    p.id
                )));
            }
            Ok(json!({"id": p.id, "yaw": wrapped(p.yaw)}))
        }

        fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
            let p = Params::parse(params)?;
            let yaw = wrapped(p.yaw);
            let Some(mut open) = ctx.open.take() else {
                return Err(Refusal::new(
                    "no map is open, so there are no placements to turn.",
                ));
            };
            let Some(index) = open.map.placements.iter().position(|q| q.id == p.id) else {
                ctx.open = Some(open);
                return Err(Refusal::new(format!(
                    "no placement is called `{}` on this map, so nothing was turned.",
                    p.id
                )));
            };
            // **The old facing, for the undo.** Its inverse is the same operation with this value.
            let old_yaw = open.map.placements[index].yaw;
            open.map.placements[index].yaw = yaw;
            let summary = format!("turned {} to {yaw:.0} deg", p.id);
            // **And the file, so the turn is not only in memory.** A refusal to save puts the old
            // facing back, so the editor and the file agree.
            let Some(project) = ctx.project.as_ref() else {
                ctx.open = Some(open);
                return Err(Refusal::new("no project is open."));
            };
            let saved = match open.save(project) {
                Ok(()) => true,
                Err(e) => {
                    open.map.placements[index].yaw = old_yaw;
                    ctx.open = Some(open);
                    return Err(Refusal::new(format!(
                        "the file refused the turn, so nothing was written and the facing is unchanged: {e}"
                    )));
                }
            };
            let inverse = json!({"id": p.id, "yaw": old_yaw});
            ctx.open = Some(open);
            // **Only the turned piece is recorded**: an undo checks that piece, not the whole map,
            // so an unrelated edit made by hand cannot block it.
            let after = Some(json!({"id": p.id, "yaw": yaw}));
            Ok(Outcome {
                summary,
                affected: 1,
                saved,
                inverse: Some(inverse),
                after,
            })
        }
    }
}

/// **Arm a piece, so the next placement puts it down.**
///
/// E-037: the palette click did this in the UI, so an agent could place pieces but could not choose
/// one. Arming changes no map, so it records no undo: its `after` is the piece the editor now holds,
/// and the editor applies it (`editor::arm_piece`, `editor::apply_armed`).
pub mod brush_arm {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Params {
        /// The library id of the piece to arm, as the palette names it, e.g. `site/wall`.
        pub descriptor: String,
    }

    /// How many ids a refusal lists. The closest first, so a typo's intended piece is among them.
    const LISTED_ON_REFUSAL: usize = 20;

    /// Levenshtein distance, two rows at a time. The ranking needs no more than this, so no crate.
    fn distance(a: &str, b: &str) -> usize {
        let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
        let mut prev: Vec<usize> = (0..=b.len()).collect();
        let mut cur = vec![0usize; b.len() + 1];
        for (i, ca) in a.iter().enumerate() {
            cur[0] = i + 1;
            for (j, cb) in b.iter().enumerate() {
                let sub = prev[j] + usize::from(ca != cb);
                cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
            }
            std::mem::swap(&mut prev, &mut cur);
        }
        prev[b.len()]
    }

    impl Params {
        /// **The JSON schema**, which is what an agent reads.
        pub fn schema() -> serde_json::Value {
            json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["descriptor"],
                "properties": {
                    "descriptor": {
                        "type": "string",
                        "description": "The library id of the piece to arm, as the palette names it, e.g. `site/wall`."
                    }
                }
            })
        }

        /// **The parameters, parsed.** `deny_unknown_fields`, so a typo is a refusal.
        pub fn parse(raw: &serde_json::Value) -> Result<Params, Refusal> {
            serde_json::from_value(raw.clone()).map_err(|e| {
                Refusal::new(format!(
                    "this is not a piece to arm: {e}. The one field is `descriptor`, and nothing else."
                ))
            })
        }
    }

    /// **Arm one piece from the library.**
    pub struct BrushArm;

    impl Operation for BrushArm {
        fn name(&self) -> &'static str {
            "brush_arm"
        }

        fn undo(&self) -> Undo {
            Undo::None
        }

        fn schema(&self) -> serde_json::Value {
            Params::schema()
        }

        /// **A piece the library holds, or a refusal.** Nothing is armed by a refused request.
        fn plan(
            &self,
            ctx: &Ctx,
            params: &serde_json::Value,
        ) -> Result<serde_json::Value, Refusal> {
            let p = Params::parse(params)?;
            let Some(project) = ctx.project.as_ref() else {
                return Err(Refusal::new(
                    "no project is open, so there is no library to arm a piece from.",
                ));
            };
            if !project.library.descriptors.iter().any(|d| d.id == p.descriptor) {
                // **The refusal names what exists** (gap 2): an agent cannot retry with an id it cannot
                // see, and "the palette lists the pieces" sent it to a palette it has no tool to read.
                let mut ids: Vec<&str> = project
                    .library
                    .descriptors
                    .iter()
                    .map(|d| d.id.as_str())
                    .collect();
                if ids.is_empty() {
                    return Err(Refusal::new(format!(
                        "`{}` is not in the library, and the library holds no pieces, so nothing was armed.",
                        p.descriptor
                    )));
                }
                ids.sort_by_key(|id| (distance(id, &p.descriptor), id.to_string()));
                let shown = ids
                    .iter()
                    .take(LISTED_ON_REFUSAL)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(Refusal::new(format!(
                    "`{}` is not in the library, so nothing was armed. The library holds {} pieces; \
                     the closest are: {shown}.",
                    p.descriptor,
                    ids.len()
                )));
            }
            Ok(json!({"armed": p.descriptor}))
        }

        fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
            let plan = self.plan(ctx, params)?;
            let id = plan["armed"].as_str().unwrap_or_default().to_owned();
            Ok(Outcome {
                summary: format!("armed {id}"),
                affected: 0,
                saved: false,
                inverse: None,
                after: Some(plan),
            })
        }
    }
}

/// **Remove one placement, or put back what a logged removal took.**
///
/// **A removal takes the piece and what stands on it**, because a stack goes whole
/// (`stack::group_of`): removing a host alone would leave its riders pointing at a placement that
/// no longer exists, and the map would refuse to load. So the undo is the group, put back at the
/// rows it held.
///
/// **A put-back is accepted only against the log.** The registry has no other way to add a
/// placement, so a put-back the log did not record would be one.
pub mod placement_remove {
    use super::*;
    use bevy_emerge_core::map::Placed;
    use serde_json::json;

    /// **One row a removal took**: the index it held and the placement as the file wrote it.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Row {
        pub index: usize,
        pub placed: serde_json::Value,
    }

    /// **What an undo puts back**: the piece the removal was asked for, and the rows it took.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct PutBack {
        pub head: String,
        pub rows: Vec<Row>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Params {
        /// The placement to remove. What stands on it goes too.
        #[serde(default)]
        pub id: Option<String>,
        /// **Only an undo sends this**: puts back the rows a logged removal took.
        #[serde(default)]
        pub put_back: Option<PutBack>,
    }

    impl Params {
        /// **The JSON schema**, which is what an agent reads.
        pub fn schema() -> serde_json::Value {
            json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "The placement to remove. A piece standing on it goes too."
                    },
                    "put_back": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["head", "rows"],
                        "description": "Only an undo sends this: it restores the pieces a logged removal took. It is not a way to add a piece.",
                        "properties": {
                            "head": {"type": "string"},
                            "rows": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "additionalProperties": false,
                                    "required": ["index", "placed"],
                                    "properties": {
                                        "index": {"type": "integer", "minimum": 0},
                                        "placed": {"type": "object"}
                                    }
                                }
                            }
                        }
                    }
                }
            })
        }

        /// **The parameters, parsed.** `deny_unknown_fields`, so a typo is a refusal; and exactly one
        /// of `id` and `put_back`, so a request cannot say both.
        pub fn parse(raw: &serde_json::Value) -> Result<Params, Refusal> {
            let p: Params = serde_json::from_value(raw.clone()).map_err(|e| {
                Refusal::new(format!(
                    "these are not a removal: {e}. The fields are `id`, or `put_back` for an undo, and \
                     nothing else."
                ))
            })?;
            match (&p.id, &p.put_back) {
                (Some(_), None) | (None, Some(_)) => Ok(p),
                _ => Err(Refusal::new(
                    "a removal names one placement by `id`, or puts back what a removal took with \
                     `put_back`, and not both. Nothing was removed.",
                )),
            }
        }
    }

    /// **The position of a placement by id**, or `None` when this map does not hold it.
    fn index_of(open: &OpenMap, id: &str) -> Option<usize> {
        open.map.placements.iter().position(|q| q.id == id)
    }

    /// **Whether the log holds a removal that took exactly these rows.** A put-back's parameters are
    /// the removal's recorded inverse, so equality with that inverse is the proof.
    fn logged_removal(project: &Project, pb: &PutBack) -> bool {
        let Ok(want) = serde_json::to_value(pb) else {
            return false;
        };
        let text = std::fs::read_to_string(oplog_path(project)).unwrap_or_default();
        text.lines()
            .filter_map(|l| serde_json::from_str::<Entry>(l).ok())
            .any(|e| {
                e.op == "placement_remove"
                    && e.outcome == "applied"
                    && e.inverse.as_ref().and_then(|i| i.get("put_back")) == Some(&want)
            })
    }

    /// **The rows a put-back restores, checked before anything is written.** Each must be a piece
    /// this map does not hold, the rows must be what a logged removal took, and each must land at an
    /// index the map can take once the earlier rows are in.
    fn restorable(
        project: &Project,
        open: &OpenMap,
        pb: &PutBack,
    ) -> Result<Vec<(usize, Placed)>, Refusal> {
        if pb.rows.is_empty() {
            return Err(Refusal::new(
                "a put-back has no rows, so nothing was put back.",
            ));
        }
        if !logged_removal(project, pb) {
            return Err(Refusal::new(
                "no logged removal took these pieces, so they cannot be put back. An undo restores \
                 only what a removal in the log took. Nothing was changed.",
            ));
        }
        let mut rows: Vec<(usize, Placed)> = Vec::with_capacity(pb.rows.len());
        for row in &pb.rows {
            let placed: Placed = serde_json::from_value(row.placed.clone()).map_err(|e| {
                Refusal::new(format!(
                    "a restored row is not a placement: {e}. Nothing was put back."
                ))
            })?;
            if index_of(open, &placed.id).is_some() || rows.iter().any(|(_, q)| q.id == placed.id) {
                return Err(Refusal::new(format!(
                    "`{}` is already on this map, so putting it back would make two. Nothing was put \
                     back.",
                    placed.id
                )));
            }
            rows.push((row.index, placed));
        }
        rows.sort_by_key(|(i, _)| *i);
        for (n, (i, _)) in rows.iter().enumerate() {
            if *i > open.map.placements.len() + n {
                return Err(Refusal::new(format!(
                    "row {i} is past the end of the map, so it cannot go back there. Nothing was put \
                     back."
                )));
            }
        }
        Ok(rows)
    }

    /// **Take a removal out**, and the group it took with it, then save. A refused save puts every
    /// row back, so the editor and the file agree.
    fn remove(project: &Project, open: &mut OpenMap, id: &str) -> Result<Outcome, Refusal> {
        let Some(index) = index_of(open, id) else {
            return Err(Refusal::new(format!(
                "no placement is called `{id}` on this map, so nothing was removed."
            )));
        };
        // **A stack goes whole**, by the same function the editor's delete has always used.
        let mut group = bevy_emerge_core::stack::group_of(&open.map, index);
        group.sort_unstable();
        let mut rows: Vec<Row> = Vec::with_capacity(group.len());
        for &i in &group {
            let placed = serde_json::to_value(&open.map.placements[i]).map_err(|e| {
                Refusal::new(format!(
                    "cannot record `{}` for the undo: {e}. Nothing was removed.",
                    open.map.placements[i].id
                ))
            })?;
            rows.push(Row { index: i, placed });
        }
        let ids: Vec<String> = group
            .iter()
            .map(|&i| open.map.placements[i].id.clone())
            .collect();
        // **Back to front**: removing an earlier row shifts every later one down.
        let mut taken: Vec<(usize, Placed)> = Vec::with_capacity(group.len());
        for &i in group.iter().rev() {
            taken.push((i, open.map.placements.remove(i)));
        }
        if let Err(e) = open.save(project) {
            // **Ascending again**, the order the rows went back in.
            for (i, placed) in taken.into_iter().rev() {
                open.map.placements.insert(i, placed);
            }
            return Err(Refusal::new(format!(
                "the file refused the removal, so nothing was written and the piece is still there: {e}"
            )));
        }
        let summary = match ids.len() {
            1 => format!("removed {id}"),
            n => format!("removed {id} and {} on it", n - 1),
        };
        let inverse = json!({"put_back": PutBack { head: id.to_owned(), rows }});
        Ok(Outcome {
            summary,
            affected: ids.len(),
            saved: true,
            inverse: Some(inverse),
            // **How many of the pieces are still on the map**: none, after a removal. An undo checks
            // that count, so a piece put back or taken away by hand blocks it.
            after: Some(json!({"map": open.map.name, "ids": ids, "on_map": 0})),
        })
    }

    /// **Put a logged removal's pieces back**, at the rows they held, then save.
    fn put_back(project: &Project, open: &mut OpenMap, pb: &PutBack) -> Result<Outcome, Refusal> {
        let rows = restorable(project, open, pb)?;
        let ids: Vec<String> = rows.iter().map(|(_, p)| p.id.clone()).collect();
        // **Ascending**: each row lands at its own index once the earlier rows are back.
        for (i, placed) in rows {
            open.map.placements.insert(i, placed);
        }
        if let Err(e) = open.save(project) {
            for id in &ids {
                if let Some(i) = index_of(open, id) {
                    open.map.placements.remove(i);
                }
            }
            return Err(Refusal::new(format!(
                "the file refused the put-back, so nothing was written and the pieces are not back: {e}"
            )));
        }
        let summary = match ids.len() {
            1 => format!("put back {}", ids[0]),
            n => format!("put back {} and {} on it", pb.head, n - 1),
        };
        Ok(Outcome {
            summary,
            affected: ids.len(),
            saved: true,
            // **Its inverse removes the piece it was put back for**: the same group, now the map's again.
            inverse: Some(json!({"id": pb.head})),
            after: Some(json!({"map": open.map.name, "ids": ids, "on_map": ids.len()})),
        })
    }

    /// **Remove one placement, and what stands on it.**
    pub struct PlacementRemove;

    impl Operation for PlacementRemove {
        fn name(&self) -> &'static str {
            "placement_remove"
        }

        fn undo(&self) -> Undo {
            Undo::Inverse
        }

        /// **A put-back is the undo of a logged removal and nothing else** (gap 2), so only [`undo`]
        /// may send one. `parse` refuses a request that names both `id` and `put_back`.
        fn only_as_undo(&self, params: &serde_json::Value) -> bool {
            Params::parse(params).is_ok_and(|p| p.put_back.is_some())
        }

        fn schema(&self) -> serde_json::Value {
            Params::schema()
        }

        /// **The pieces a change names, and how many the map still holds** — the part an undo checks.
        /// A removal's undo is safe only while none of them is back on the map; a put-back's only while
        /// all of them are.
        fn state_of(&self, ctx: &Ctx, after: &serde_json::Value) -> Option<serde_json::Value> {
            let open = ctx.open.as_ref()?;
            let ids: Vec<String> = after
                .get("ids")?
                .as_array()?
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect();
            let on_map = ids.iter().filter(|id| index_of(open, id).is_some()).count();
            Some(json!({"map": open.map.name, "ids": ids, "on_map": on_map}))
        }

        fn plan(
            &self,
            ctx: &Ctx,
            params: &serde_json::Value,
        ) -> Result<serde_json::Value, Refusal> {
            let p = Params::parse(params)?;
            let Some(open) = ctx.open.as_ref() else {
                return Err(Refusal::new(
                    "no map is open, so there is no placement to remove.",
                ));
            };
            if let Some(pb) = &p.put_back {
                let Some(project) = ctx.project.as_ref() else {
                    return Err(Refusal::new("no project is open."));
                };
                restorable(project, open, pb)?;
                return Ok(json!({"put_back": pb.head, "rows": pb.rows.len()}));
            }
            let Some(id) = p.id.as_deref() else {
                return Err(Refusal::new("a removal names one placement by `id`."));
            };
            let Some(index) = index_of(open, id) else {
                return Err(Refusal::new(format!(
                    "no placement is called `{id}` on this map, so nothing was removed."
                )));
            };
            let group = bevy_emerge_core::stack::group_of(&open.map, index);
            Ok(json!({"id": id, "removes": group.len()}))
        }

        fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
            let p = Params::parse(params)?;
            let Some(project) = ctx.project.as_ref() else {
                return Err(Refusal::new("no project is open."));
            };
            let Some(mut open) = ctx.open.take() else {
                return Err(Refusal::new(
                    "no map is open, so there is no placement to remove.",
                ));
            };
            let done = match (&p.put_back, p.id.as_deref()) {
                (Some(pb), _) => put_back(project, &mut open, pb),
                (None, Some(id)) => remove(project, &mut open, id),
                (None, None) => Err(Refusal::new("a removal names one placement by `id`.")),
            };
            ctx.open = Some(open);
            done
        }
    }
}

/// **Remove one stamp**: a composition placed on the map, taken off `map.stamps` by its id.
///
/// The shape of [`placement_remove`], one row wide: a stamp is one entry in the file, and what it
/// expands to is not written, so a removal takes exactly the one entry and its undo puts that entry
/// back at the index it held.
pub mod stamp_remove {
    use super::*;
    use bevy_emerge_core::composition::Stamped;
    use serde_json::json;

    /// **What an undo puts back**: the stamp the removal took, and the index it held.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct PutBack {
        pub index: usize,
        pub stamp: serde_json::Value,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Params {
        /// The stamp to remove, by its id.
        #[serde(default)]
        pub id: Option<String>,
        /// **Only an undo sends this**: puts back the stamp a logged removal took.
        #[serde(default)]
        pub put_back: Option<PutBack>,
    }

    impl Params {
        /// **The JSON schema**, which is what an agent reads.
        pub fn schema() -> serde_json::Value {
            json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "The stamp to remove, by its id, as the map's stamps list names it."
                    },
                    "put_back": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["index", "stamp"],
                        "description": "Only an undo sends this: it restores the stamp a logged removal took. It is not a way to add a stamp.",
                        "properties": {
                            "index": {"type": "integer", "minimum": 0},
                            "stamp": {"type": "object"}
                        }
                    }
                }
            })
        }

        /// **The parameters, parsed.** `deny_unknown_fields`, so a typo is a refusal; and exactly one
        /// of `id` and `put_back`, so a request cannot say both.
        pub fn parse(raw: &serde_json::Value) -> Result<Params, Refusal> {
            let p: Params = serde_json::from_value(raw.clone()).map_err(|e| {
                Refusal::new(format!(
                    "these are not a stamp removal: {e}. The fields are `id`, or `put_back` for an undo, \
                     and nothing else."
                ))
            })?;
            match (&p.id, &p.put_back) {
                (Some(_), None) | (None, Some(_)) => Ok(p),
                _ => Err(Refusal::new(
                    "a stamp removal names one stamp by `id`, or puts back what a removal took with \
                     `put_back`, and not both. Nothing was removed.",
                )),
            }
        }
    }

    /// **The position of a stamp by id**, or `None` when this map does not hold it.
    fn index_of(open: &OpenMap, id: &str) -> Option<usize> {
        open.map.stamps.iter().position(|s| s.id == id)
    }

    /// **Whether the log holds a removal that took exactly this stamp at this index.** A put-back's
    /// parameters are the removal's recorded inverse, so equality with that inverse is the proof.
    fn logged_removal(project: &Project, pb: &PutBack) -> bool {
        let Ok(want) = serde_json::to_value(pb) else {
            return false;
        };
        let text = std::fs::read_to_string(oplog_path(project)).unwrap_or_default();
        text.lines()
            .filter_map(|l| serde_json::from_str::<Entry>(l).ok())
            .any(|e| {
                e.op == "stamp_remove"
                    && e.outcome == "applied"
                    && e.inverse.as_ref().and_then(|i| i.get("put_back")) == Some(&want)
            })
    }

    /// **The stamp a put-back restores, checked before anything is written.** The log must hold the
    /// removal, the stamp must not already be on the map, and its index must be one the map can take.
    fn restorable(project: &Project, open: &OpenMap, pb: &PutBack) -> Result<Stamped, Refusal> {
        if !logged_removal(project, pb) {
            return Err(Refusal::new(
                "no logged removal took this stamp, so it cannot be put back. An undo restores only \
                 what a removal in the log took. Nothing was changed.",
            ));
        }
        let stamp: Stamped = serde_json::from_value(pb.stamp.clone()).map_err(|e| {
            Refusal::new(format!(
                "the restored stamp is not a stamp: {e}. Nothing was put back."
            ))
        })?;
        if index_of(open, &stamp.id).is_some() {
            return Err(Refusal::new(format!(
                "`{}` is already on this map, so putting it back would make two. Nothing was put back.",
                stamp.id
            )));
        }
        if pb.index > open.map.stamps.len() {
            return Err(Refusal::new(format!(
                "index {} is past the end of the map's stamps, so it cannot go back there. Nothing was \
                 put back.",
                pb.index
            )));
        }
        Ok(stamp)
    }

    /// **Take a stamp out**, then save. A refused save puts it back where it was, so the editor and
    /// the file agree.
    fn remove(project: &Project, open: &mut OpenMap, id: &str) -> Result<Outcome, Refusal> {
        let Some(index) = index_of(open, id) else {
            return Err(Refusal::new(format!(
                "no stamp is called `{id}` on this map, so nothing was removed."
            )));
        };
        let taken = open.map.stamps.remove(index);
        let stamp = match serde_json::to_value(&taken) {
            Ok(v) => v,
            Err(e) => {
                open.map.stamps.insert(index, taken);
                return Err(Refusal::new(format!(
                    "cannot record `{id}` for the undo: {e}. Nothing was removed."
                )));
            }
        };
        if let Err(e) = open.save(project) {
            open.map.stamps.insert(index, taken);
            return Err(Refusal::new(format!(
                "the file refused the removal, so nothing was written and the stamp is still there: {e}"
            )));
        }
        Ok(Outcome {
            summary: format!("removed stamp {id}"),
            affected: 1,
            saved: true,
            inverse: Some(json!({"put_back": PutBack { index, stamp }})),
            // **How many of the stamps are still on the map**: none, after a removal.
            after: Some(json!({"map": open.map.name, "ids": [id], "on_map": 0})),
        })
    }

    /// **Put a logged removal's stamp back**, at the index it held, then save.
    fn put_back(project: &Project, open: &mut OpenMap, pb: &PutBack) -> Result<Outcome, Refusal> {
        let stamp = restorable(project, open, pb)?;
        let id = stamp.id.clone();
        open.map.stamps.insert(pb.index, stamp);
        if let Err(e) = open.save(project) {
            open.map.stamps.remove(pb.index);
            return Err(Refusal::new(format!(
                "the file refused the put-back, so nothing was written and the stamp is not back: {e}"
            )));
        }
        Ok(Outcome {
            summary: format!("put back stamp {id}"),
            affected: 1,
            saved: true,
            // **Its inverse removes the stamp it was put back for**, as a fresh logged removal.
            inverse: Some(json!({"id": id})),
            after: Some(json!({"map": open.map.name, "ids": [id], "on_map": 1})),
        })
    }

    /// **Remove one stamp.**
    pub struct StampRemove;

    impl Operation for StampRemove {
        fn name(&self) -> &'static str {
            "stamp_remove"
        }

        fn undo(&self) -> Undo {
            Undo::Inverse
        }

        /// **A put-back is the undo of a logged removal and nothing else**, so only [`undo`] may send
        /// one. `parse` refuses a request that names both `id` and `put_back`.
        fn only_as_undo(&self, params: &serde_json::Value) -> bool {
            Params::parse(params).is_ok_and(|p| p.put_back.is_some())
        }

        fn schema(&self) -> serde_json::Value {
            Params::schema()
        }

        /// **The stamps a change names, and how many the map still holds**: the part an undo checks.
        fn state_of(&self, ctx: &Ctx, after: &serde_json::Value) -> Option<serde_json::Value> {
            let open = ctx.open.as_ref()?;
            let ids: Vec<String> = after
                .get("ids")?
                .as_array()?
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect();
            let on_map = ids.iter().filter(|id| index_of(open, id).is_some()).count();
            Some(json!({"map": open.map.name, "ids": ids, "on_map": on_map}))
        }

        fn plan(
            &self,
            ctx: &Ctx,
            params: &serde_json::Value,
        ) -> Result<serde_json::Value, Refusal> {
            let p = Params::parse(params)?;
            let Some(open) = ctx.open.as_ref() else {
                return Err(Refusal::new(
                    "no map is open, so there is no stamp to remove.",
                ));
            };
            if let Some(pb) = &p.put_back {
                let Some(project) = ctx.project.as_ref() else {
                    return Err(Refusal::new("no project is open."));
                };
                restorable(project, open, pb)?;
                return Ok(json!({"put_back": pb.stamp.get("id"), "index": pb.index}));
            }
            let Some(id) = p.id.as_deref() else {
                return Err(Refusal::new("a stamp removal names one stamp by `id`."));
            };
            if index_of(open, id).is_none() {
                return Err(Refusal::new(format!(
                    "no stamp is called `{id}` on this map, so nothing was removed."
                )));
            }
            Ok(json!({"id": id, "removes": 1}))
        }

        fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
            let p = Params::parse(params)?;
            let Some(project) = ctx.project.as_ref() else {
                return Err(Refusal::new("no project is open."));
            };
            let Some(mut open) = ctx.open.take() else {
                return Err(Refusal::new(
                    "no map is open, so there is no stamp to remove.",
                ));
            };
            let done = match (&p.put_back, p.id.as_deref()) {
                (Some(pb), _) => put_back(project, &mut open, pb),
                (None, Some(id)) => remove(project, &mut open, id),
                (None, None) => Err(Refusal::new("a stamp removal names one stamp by `id`.")),
            };
            ctx.open = Some(open);
            done
        }
    }
}

/// **A map's name, changed, and the file moves with it.**
///
/// **The name becomes the file**, so the name is forced the way every map name is, and a name
/// another map in the project has is refused before anything is written. The N key and an agent
/// both come here, so the rule has one answer.
pub mod map_rename {
    use serde_json::json;

    use super::map_set_settings::settings_of;
    use super::{Ctx, Operation, Outcome, Refusal, Undo};
    use bevy_emerge_core::naming::{map_file_name, to_snake_case};

    /// **What the operation takes.** One field, the new name.
    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Params {
        pub name: String,
    }

    impl Params {
        pub fn schema() -> serde_json::Value {
            json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["name"],
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "The map's new name. It is forced to snake_case, the way every map name is, because the name becomes the file: `Town Two` is written as `town_two.map.ron`. It must not be empty, and no other map in the project may have it."
                    }
                }
            })
        }

        pub fn parse(raw: &serde_json::Value) -> Result<Params, Refusal> {
            serde_json::from_value(raw.clone()).map_err(|e| {
                Refusal::new(format!(
                    "this is not a map's new name: {e}. The one field is `name`, and nothing else."
                ))
            })
        }
    }

    /// **The name, forced the way every map name is.** A name that is empty once forced is a
    /// refusal, because a map with no name has no file to be.
    fn checked_name(raw: &str) -> Result<String, Refusal> {
        let name = to_snake_case(raw.trim());
        if name.is_empty() {
            return Err(Refusal::new("a map needs a name; nothing was changed"));
        }
        Ok(name)
    }

    /// **The rules a rename rewrites**: every `example` that names `was` becomes `name`, in the
    /// file's own text, so its comments survive. `Ok(None)` when no rule names the map.
    ///
    /// **The rewrite is checked before anything is written.** The number of `example: "was"`
    /// strings must equal the rules that name it, and the new text must parse to the in-memory
    /// rules with that one name changed. A rewrite that misses a rule is the dangling example this
    /// operation exists to prevent, so a mismatch is a refusal, not a best effort.
    fn rules_renamed(
        project: &crate::project::Project,
        was: &str,
        name: &str,
    ) -> Result<Option<(String, String)>, Refusal> {
        let named = project.rules.blocks.iter().filter(|b| b.example == was).count();
        if named == 0 {
            return Ok(None);
        }
        let file = project.project_dir.join(bevy_emerge_core::rules::FILE);
        let refuse = |why: String| {
            Refusal::new(format!("the map was not renamed, and nothing was written: {why}"))
        };
        let old = std::fs::read_to_string(&file)
            .map_err(|e| refuse(format!("{}: {e}", file.display())))?;
        let needle = format!("example: \"{was}\"");
        if old.matches(&needle).count() != named {
            return Err(refuse(format!(
                "{} names `{was}` in a form this rename cannot rewrite; change its example by hand first",
                file.display()
            )));
        }
        let new = old.replace(&needle, &format!("example: \"{name}\""));
        let parsed = bevy_emerge_core::rules::Rules::parse(&new)
            .map_err(|e| refuse(format!("{}: {e}", file.display())))?;
        let expected = project
            .rules
            .blocks
            .iter()
            .map(|b| if b.example == was { name } else { b.example.as_str() });
        if !parsed.blocks.iter().map(|b| b.example.as_str()).eq(expected) {
            return Err(refuse(format!(
                "the rewrite of {} does not match the rules the editor holds",
                file.display()
            )));
        }
        Ok(Some((old, new)))
    }

    /// **The in-memory rules follow the rename**, so generate reads the example the file now names.
    fn set_example(project: &mut crate::project::Project, was: &str, name: &str) {
        for block in project.rules.blocks.iter_mut().filter(|b| b.example == was) {
            block.example = name.to_owned();
        }
    }

    /// **A map's name, changed.**
    pub struct MapRename;

    impl Operation for MapRename {
        fn name(&self) -> &'static str {
            "map_rename"
        }

        fn undo(&self) -> Undo {
            Undo::Inverse
        }

        fn schema(&self) -> serde_json::Value {
            Params::schema()
        }

        fn state(&self, ctx: &Ctx) -> Option<serde_json::Value> {
            ctx.open.as_ref().map(settings_of)
        }

        fn plan(
            &self,
            ctx: &Ctx,
            params: &serde_json::Value,
        ) -> Result<serde_json::Value, Refusal> {
            let p = Params::parse(params)?;
            let Some(open) = ctx.open.as_ref() else {
                return Err(Refusal::new(
                    "no map is open, so there is no name to change.",
                ));
            };
            let name = checked_name(&p.name)?;
            if name == open.map.name {
                return Err(Refusal::new(format!(
                    "the map is already `{name}`, so nothing changes."
                )));
            }
            let Some(project) = ctx.project.as_ref() else {
                return Err(Refusal::new("no project is open."));
            };
            // **A taken name is refused here**, before a byte is written. `OpenMap::save` would
            // refuse it too, but the refusal belongs where the author asked, with nothing moved.
            let target = project.maps_dir.join(map_file_name(&name));
            if target.exists() {
                return Err(Refusal::new(format!(
                    "`{name}` is already a map in this project, so this map cannot take that name. \
                     Rename this map to something else."
                )));
            }
            Ok(json!({"name": name, "was": open.map.name}))
        }

        fn commit(&self, ctx: &mut Ctx, params: &serde_json::Value) -> Result<Outcome, Refusal> {
            let name = checked_name(&Params::parse(params)?.name)?;
            let Some(mut open) = ctx.open.take() else {
                return Err(Refusal::new(
                    "no map is open, so there is no name to change.",
                ));
            };
            let was = open.map.name.clone();
            let Some(project) = ctx.project.as_mut() else {
                // **The map goes back before the refusal**, as the settings commit does: a
                // refusal that dropped it would leave the editor holding nothing.
                ctx.open = Some(open);
                return Err(Refusal::new("no project is open."));
            };
            // **The rules are part of the rename.** A rule names the map its grammar is learned
            // from, so a rename that moved the file and left that name behind would leave the block
            // unreadable. The rules file is written first; a refusal here changes nothing.
            let rules = match rules_renamed(project, &was, &name) {
                Ok(rules) => rules,
                Err(refusal) => {
                    ctx.open = Some(open);
                    return Err(refusal);
                }
            };
            let file = project.project_dir.join(bevy_emerge_core::rules::FILE);
            if let Some((_, new)) = &rules
                && let Err(e) = bevy_emerge_core::ron_surgery::save_atomic(&file, new)
            {
                ctx.open = Some(open);
                return Err(Refusal::new(format!(
                    "the map was not renamed, and nothing was written: {e}"
                )));
            }
            // **The save is the map's own gate.** It moves the file and removes the old one, and
            // it refuses a taken name rather than replacing it.
            open.map.name = name.clone();
            if let Err(e) = open.save(project) {
                // **The write landed; only the old file's removal failed.** Save has moved the map's
                // path to the new file, so the name stays with it, and the refusal tells the author
                // which file to delete. Putting the old name back here would leave a name no file holds.
                if open.map_path == project.maps_dir.join(map_file_name(&name)) {
                    set_example(project, &was, &name);
                    ctx.open = Some(open);
                    return Err(Refusal::new(e));
                }
                // **The map did not move, so the rules go back too.** If that restore itself fails,
                // the refusal says so: the rules file then names the new map, and the author must fix it.
                let mut message = format!("the map was not renamed, and nothing was written: {e}");
                if let Some((old, _)) = &rules
                    && let Err(restore) = bevy_emerge_core::ron_surgery::save_atomic(&file, old)
                {
                    message.push_str(&format!(
                        "; {} could not be put back ({restore}), and it now names `{name}`, so change its example by hand",
                        file.display()
                    ));
                }
                open.map.name = was;
                ctx.open = Some(open);
                return Err(Refusal::new(message));
            }
            set_example(project, &was, &name);
            let summary = if rules.is_some() {
                format!("renamed `{was}` to `{name}`, and the file moved with it; the rules' example follows")
            } else {
                format!("renamed `{was}` to `{name}`, and the file moved with it")
            };
            // **The inverse is the name it replaced**, and undo takes the same path: a rename back,
            // refused if that name is taken by now. The rules follow it back the same way.
            let inverse = json!({"name": was});
            ctx.open = Some(open);
            let after = ctx.open.as_ref().map(settings_of);
            Ok(Outcome {
                summary,
                affected: 1,
                saved: true,
                inverse: Some(inverse),
                after,
            })
        }
    }
}
