//! **The agent's door into the editor** — the same operations, over BRP.
//!
//! # Why this module exists
//!
//! The editor's verbs lived in **key handlers**: a `KeyboardInput` message, a match on the key,
//! a write to some state. **An agent has no keyboard.** It could reach the editor over BRP —
//! `bevy_debugger/input` injects into `ButtonInput`, so a key *can* be synthesised — but that is a
//! simulation of a person, and the brief's rule is explicit: *"Never drive the UI by clicking."*
//!
//! So this module registers **typed commands** with the debugger, and each one dispatches an
//! operation through [`crate::ops::dispatch`] — **the same function the key handlers call**.
//! There is no second implementation of anything.
//!
//! # What a command looks like
//!
//! ```text
//! emerge/operation_list     — every operation, its schema, and how it is undone
//! emerge/operation_apply    — run one, by name, with its parameters
//! emerge/operation_preview  — what one would change, and nothing changed
//! emerge/oplog_read         — the log, as lines
//! ```
//!
//! **The names are `emerge/`-prefixed because the debugger reserves its own.** A command that
//! took a name the debugger used would be refused at registration — and it would be refused by a
//! **panic**, so the name is checked by `tests/the_operations_have_two_front_ends.rs`.
//!
//! # The shape of an apply
//!
//! **A preview is the default for an operation that changes the map's own shape**, because
//! resizing a map is wide in scope: it moves the ground every piece stands on. **An agent that
//! means it passes `dry_run: false`.**
//!
//! **Every apply returns the operation's id**, so the agent can cite it when it presents the
//! change and the human can revert by it.

use bevy::prelude::*;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::ops::{self, Ctx, Source};

/// **The parameters of `emerge/operation_apply`.**
///
/// **Everything in one struct**, and `deny_unknown_fields`, so a typo is a refusal rather than a
/// field silently ignored. An agent that misspelled `parameters` would otherwise be told it had
/// applied an operation with no parameters, and it would believe it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyParams {
    /// **Which operation.**
    pub operation: String,
    /// **Its parameters**, as the operation's own schema describes them.
    #[serde(default)]
    pub parameters: Value,
    /// **Whether to actually do it.** `true` previews and changes nothing.
    ///
    /// **The default is `true` here and `false` on the MCP tool**, and the difference is
    /// deliberate. **The BRP surface is a raw door**, and a raw door that changed the editor by
    /// default would be a door an agent could fall into; **the MCP tool is the door with a
    /// description in front of it**, and an agent that reads it knows what it is asking for.
    #[serde(default = "yes")]
    pub dry_run: bool,
}

/// **`true`, the default for the fields that mean "yes".**
fn yes() -> bool {
    true
}

/// **The parameters of `emerge/operation_preview`.**
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewParams {
    /// **Which operation.**
    pub operation: String,
    /// **Its parameters.**
    #[serde(default)]
    pub parameters: Value,
}

/// **The parameters of `emerge/oplog_read`.**
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogParams {
    /// **Read from this sequence number on.** `0` is the beginning.
    #[serde(default)]
    pub since_seq: u64,
    /// **How many entries at most.**
    #[serde(default = "fifty")]
    pub limit: usize,
}

/// **Fifty entries, which is a screenful.**
fn fifty() -> usize {
    50
}

/// **Build a context from the editor's own resources.**
///
/// **The two `Option`s, and why they are options.** A map door has an open map and a kit door
/// does not, and a project is present in both — but the resources are `Option` in the signature
/// because a command can be called in a state where one is missing, and **a missing resource must
/// be a refusal and not a panic**. `CLAUDE.md` says it: *"A missing `Res<T>` panics its system."*
fn context(
    project: Option<ResMut<crate::project::Project>>,
    open: Option<ResMut<crate::project::OpenMap>>,
) -> Ctx {
    Ctx {
        // **Cloned out, because the context owns what it carries.** The reason is the registry:
        // a trait with a lifetime on its methods cannot be a `dyn`, and the registry has to be a
        // `dyn` because a list of operations is looked up by name. One copy of a map's settings
        // is a price worth paying for that.
        project: project.map(|p| p.into_inner().clone()),
        open: open.map(|o| o.into_inner().clone()),
        source: Source::Agent,
        session: crate::ops::session_id(),
    }
}

/// **The context, built from the editor's own resources, and the resources back afterwards.**
///
/// **The change has to land in the RUNNING editor, and not only in the file.** A context that
/// holds a *copy* means the operation edits the copy, writes the file, and the editor's own map
/// is untouched — so the screen shows the old settings until the watcher notices the file, up to
/// a quarter of a second later, and any edit the author makes in that window is thrown away when
/// the watcher swaps the map in.
///
/// **So the context is built from clones and the result is written back**, which is the same
/// thing the UI path does with the map it owns: the operation's answer *is* the editor's new
/// state.
///
/// **`ResMut::into_inner` hands back a borrow, not the value** — so the value is cloned out and
/// the changed one is cloned back. **The clone is the price of the registry's `dyn`**, and it is
/// paid once per operation rather than once per frame.
fn take(
    project: &Option<ResMut<crate::project::Project>>,
    open: &Option<ResMut<crate::project::OpenMap>>,
) -> Ctx {
    Ctx {
        project: project.as_deref().cloned(),
        open: open.as_deref().cloned(),
        source: Source::Agent,
        session: crate::ops::session_id(),
    }
}

/// **Put the operation's map back into the editor's own resource.**
///
/// **Only the map.** The project is the editor's, and the operation does not change it — the
/// operation reads the library and writes the file.
fn put_back(open: &mut Option<ResMut<crate::project::OpenMap>>, ctx: Ctx) {
    if let Some(fresh) = ctx.open
        && let Some(handle) = open.as_deref_mut()
    {
        *handle = fresh;
    }
}

/// **`emerge/project_root` — which project the editor has open.**
///
/// **A read, and the one fact the MCP server cannot know for itself.** The server is told a
/// project on the command line and the editor knows which one it has open, and **the two have to
/// be the same** or a change lands in the wrong place.
fn project_root(
    _: In<serde_json::Value>,
    project: Option<Res<crate::project::Project>>,
) -> Result<Value, String> {
    let Some(project) = project else {
        return Err("no project is open.".to_owned());
    };
    Ok(json!({
        "project_root": project.root.to_string_lossy(),
        "map": project.open_map_name,
    }))
}

/// **`emerge/operation_list` — every operation, its schema, and how it is undone.**
///
/// **A discovery tool, not a page of tools.** The MCP server exposes one tool per operation, but
/// the *BRP* surface is one list, because a BRP caller can ask. **The always-loaded MCP toolset
/// stays small** and the rare ones are found here.
fn operation_list(_: In<serde_json::Value>) -> Result<Value, String> {
    let ops = ops::registry();
    let list: Vec<Value> = ops
        .iter()
        .map(|op| {
            json!({
                "operation": op.name(),
                "undo": match op.undo() {
                    ops::Undo::None => "none",
                    ops::Undo::Inverse => "inverse",
                    ops::Undo::Snapshot => "snapshot",
                },
                "parameters": op.schema(),
            })
        })
        .collect();
    Ok(json!({ "operations": list }))
}

/// **`emerge/operation_preview` — what an operation would change, and nothing changed.**
///
/// **A pure call.** The operation's `plan` runs and the answer is returned; `commit` never runs,
/// so nothing is written to the map, to the file, or to the log.
fn operation_preview(
    In(p): In<PreviewParams>,
    project: Option<ResMut<crate::project::Project>>,
    open: Option<ResMut<crate::project::OpenMap>>,
) -> Result<Value, String> {
    let Some(op) = ops::find(&p.operation) else {
        return Err(format!(
            "`{}` is not an operation this editor knows.",
            p.operation
        ));
    };
    let ctx = context(project, open);
    match op.plan(&ctx, &p.parameters) {
        Ok(plan) => Ok(json!({
            "operation": p.operation,
            "outcome": "previewed",
            "would_change": plan,
        })),
        Err(e) => Ok(json!({
            "operation": p.operation,
            "outcome": "refused",
            "refusal": e.0,
        })),
    }
}

/// **`emerge/operation_apply` — run one operation.**
///
/// **The one way an agent changes the editor.** It dispatches through
/// [`crate::ops::dispatch`], which is the same function a key handler calls — so the two front
/// ends cannot drift, and the log records the agent's work in the same file the human's work is
/// recorded in.
fn operation_apply(
    In(p): In<ApplyParams>,
    project: Option<ResMut<crate::project::Project>>,
    mut open: Option<ResMut<crate::project::OpenMap>>,
) -> Result<Value, String> {
    let Some(op) = ops::find(&p.operation) else {
        return Err(format!(
            "`{}` is not an operation this editor knows. The known ones are: {}.",
            p.operation,
            ops::registry()
                .iter()
                .map(|o| o.name())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    };
    // **A preview is the plan, and nothing else.**
    //
    // **And a preview takes the resources by *reference*, not by ownership** — a preview that
    // took them mutably would mark them changed, and every system gated on a change would
    // rebuild the interface for a call that changed nothing.
    if p.dry_run {
        let read_project = project.as_deref();
        let read_open = open.as_deref();
        let ctx = Ctx {
            project: read_project.cloned(),
            open: read_open.cloned(),
            source: Source::Agent,
            session: crate::ops::session_id(),
        };
        return match op.plan(&ctx, &p.parameters) {
            Ok(plan) => Ok(json!({
                "operation": p.operation,
                "outcome": "previewed",
                "would_change": plan,
            })),
            Err(e) => Ok(json!({
                "operation": p.operation,
                "outcome": "refused",
                "refusal": e.0,
            })),
        };
    }
    // **And the real thing**, which takes the editor's own map and project and gives them back.
    let mut ctx = take(&project, &open);
    let applied = ops::dispatch(&mut ctx, &p.operation, &p.parameters);
    // **The map goes back either way** — changed, refused, or not — so the editor's own state is
    // never left holding a half-applied edit.
    put_back(&mut open, ctx);
    match applied {
        Ok(ops::Applied::Done {
            outcome,
            operation_id,
            entry,
        }) => Ok(json!({
            "operation": p.operation,
            "outcome": "applied",
            "summary": outcome.summary,
            "affected": outcome.affected,
            "saved": outcome.saved,
            // **The id the agent cites and the human reverts by.** The brief asks for it on every
            // mutating tool, and an operation whose id nobody can name is an operation nobody can
            // present.
            "operation_id": operation_id,
            "log_entry": entry,
        })),
        Ok(ops::Applied::Refused(e)) => Ok(json!({
            "operation": p.operation,
            "outcome": "refused",
            "refusal": e.0,
        })),
        // **A refusal from the dispatch itself** — the plan refused, or the log could not be
        // written. It is a refusal like any other, and it says why.
        Err(e) => Ok(json!({
            "operation": p.operation,
            "outcome": "refused",
            "refusal": e.0,
        })),
    }
}

/// **`emerge/oplog_read` — the log, as lines.**
///
/// **Capped and paged, and it says how to narrow.** A log is append-only and a busy session makes
/// a long one, so the answer carries the next sequence number to ask from and a sentence saying
/// so — the brief's rule for a result that hits its cap.
fn oplog_read(
    In(p): In<LogParams>,
    project: Option<ResMut<crate::project::Project>>,
) -> Result<Value, String> {
    let Some(project) = project else {
        return Err("no project is open, so there is no log to read.".to_owned());
    };
    let path = ops::oplog_path(&project);
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let total = lines.len() as u64;
    let from = (p.since_seq as usize).min(lines.len());
    let take = p.limit.clamp(1, 200);
    let end = (from + take).min(lines.len());
    let entries: Vec<Value> = lines[from..end]
        .iter()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect();
    let next = if end < lines.len() {
        Some(end as u64)
    } else {
        None
    };
    Ok(json!({
        "entries": entries,
        "total": total,
        "next_seq": next,
        "how_to_narrow": next.map(|n| format!("pass since_seq={n} to read the rest")),
    }))
}

/// **Register the agent's commands.
///
/// **Under the `debugger` feature**, because the transport is what carries them and a build
/// without it has no way to reach them.
pub struct OpsBridgePlugin;

impl Plugin for OpsBridgePlugin {
    fn build(&self, app: &mut App) {
        use bevy_debugger_bevy::{DebugCommandInfo, DebugCommandsAppExt};

        app.add_debug_command(
            DebugCommandInfo::new(
                "emerge/project_root",
                "Which project the editor has open, and which map. A read.",
            )
            .example(json!({})),
            project_root,
        );

        app.add_debug_command(
            DebugCommandInfo::new(
                "emerge/operation_list",
                "Every operation this editor knows, with its parameters' JSON schema and how it is \
                 undone. Read this before calling `emerge/operation_apply`.",
            )
            .example(json!({})),
            operation_list,
        );

        app.add_debug_command(
            DebugCommandInfo::new(
                "emerge/operation_preview",
                "What an operation would change, and nothing changed. The map, the file and the log \
                 are all left alone. Use this to check a proposal before applying it.",
            )
            .params("operation: the operation's name, as `emerge/operation_list` gives it. \
                     parameters: the operation's own parameters, as its schema describes them.")
            .example(json!({
                "operation": "map_set_settings",
                "parameters": {"bounds": [64.0, 8.0, 64.0]}
            })),
            operation_preview,
        );

        app.add_debug_command(
            DebugCommandInfo::new(
                "emerge/operation_apply",
                "Run one operation and record it in the operation log. `dry_run` is false by \
                 default for a small edit and true for one that changes the map's shape; set it \
                 explicitly. A refusal changes nothing.",
            )
            .params(
                "operation: the operation's name. parameters: the operation's own parameters. \
                     dry_run: true previews and writes nothing; false applies.",
            )
            .example(json!({
                "operation": "map_set_settings",
                "parameters": {"bounds": [64.0, 8.0, 64.0]},
                "dry_run": true
            })),
            operation_apply,
        );

        app.add_debug_command(
            DebugCommandInfo::new(
                "emerge/oplog_read",
                "The operation log, newest last, as parsed entries. Every operation writes one \
                 line, from a person or an agent, and undo is a new line rather than a deletion.",
            )
            .params(
                "since_seq: read from this sequence number on; 0 is the beginning. limit: how \
                     many entries at most, 1 to 200.",
            )
            .example(json!({"since_seq": 0, "limit": 50})),
            oplog_read,
        );
    }
}
