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

/// **The parameters of `emerge/operation_undo`.**
///
/// **`deny_unknown_fields`, and `dry_run` defaults to `true`**, for the reason [`ApplyParams`]
/// gives: this is the raw door, and the MCP tool is the one with a description in front of it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UndoParams {
    /// **The change to take back**, by its operation id. `None`: the newest one not yet taken back.
    #[serde(default)]
    pub operation_id: Option<String>,
    /// **Whether to preview.** `true` changes nothing.
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

/// The project and the open map, both optional: `editor_state` answers for an editor with neither.
#[derive(bevy::ecs::system::SystemParam)]
struct Loaded<'w> {
    project: Option<Res<'w, crate::project::Project>>,
    open: Option<Res<'w, crate::project::OpenMap>>,
}

/// **`emerge/state` — what the editor is holding right now.**
///
/// **A read, and the answer to "what does the editor look like?"** — the question an agent asks
/// before it changes anything, and the question it asks afterwards to check that the change landed.
///
/// **It is a summary, not a dump.** The editor holds a great deal: a library of hundreds of
/// descriptors, a map of thousands of placements, a grid of cells. **None of that is what an agent
/// needs to know**; what it needs is the *shape* of the current state — which map, how big, how
/// many pieces, what is armed, what is selected, what the editor last said, and what has gone
/// wrong. **So the state is the handful of numbers a person would read off the screen**, and the
/// picture is a separate tool.
///
/// **And it is cheap.** Everything here is already in memory, so the whole command is a read and
/// a `json!` — no scanning, no measuring, no rendering.
fn editor_state(
    _: In<serde_json::Value>,
    loaded: Loaded,
    state: Option<Res<crate::editor::EditorState>>,
    tiles: Option<Res<crate::tiles::ImportState>>,
    armed: Option<Res<crate::compose::Armed>>,
    door: Option<Res<crate::tiles::Door>>,
    mode: Option<Res<crate::tiles::Mode>>,
) -> Result<Value, String> {
    let Loaded { project, open } = loaded;
    // **The map, and what is in it.**
    let map = open.as_ref().map(|o| {
        json!({
            "name": o.map.name,
            "path": o.map_path.to_string_lossy(),
            "bounds": [o.map.bounds.0, o.map.bounds.1, o.map.bounds.2],
            "origin": [o.map.origin.0, o.map.origin.1, o.map.origin.2],
            // **The rows a consumer will spawn** -- the map's own placements plus every stamp
            // expanded -- because that is the number the game will see and the number an agent
            // should reason about.
            "placements": o.map.placements.len(),
            "stamps": o.map.stamps.len(),
            "bash": o.map.bash,
            "note": o.map.note,
            // **Whether the file and the editor agree.** An agent that changed something needs to
            // know it reached the disk.
            "dirty": o.dirty(),
        })
    });
    // **The project, and what it holds.**
    let project_summary = project.as_ref().map(|p| {
        json!({
            "root": p.root.to_string_lossy(),
            "kit": p.namespace,
            "descriptors": p.library.descriptors.len(),
            "compositions": p.compositions.compositions.len(),
            "kits": p.kits.len(),
        })
    });
    // **What the editor is holding and what it last said.**
    //
    // **The armed piece's id, not its index.** An index means nothing to a reader; the id is what
    // the palette shows and what an agent would name. And the index is into the *library*, so the
    // name is looked up there.
    let armed_piece = match (state.as_ref(), project.as_ref()) {
        (Some(s), Some(p)) => s
            .brush
            .and_then(|i| p.library.descriptors.get(i))
            .map(|d| d.id.clone()),
        _ => None,
    };
    // **What Cmd+Z and Cmd+Shift+Z would do**, named the way the INSPECTOR's UNDO and REDO rows name
    // them, so an agent and a person read the same words.
    let undo_map = open.as_ref().map(|o| &o.map);
    let editor = state.as_ref().map(|s| {
        json!({
            "armed_piece": armed_piece,
            "brush_yaw_deg": s.brush_yaw,
            "tool": format!("{:?}", s.tool),
            "status": s.status.note_text(),
            "problems": s.status.problems().iter().map(|p| p.line()).collect::<Vec<_>>(),
            "journal_entries": s.status.raised(),
            "undo_next": undo_map.and_then(|m| s.undo_next(m)),
            "undo_depth": s.undo_depth(),
            "redo_next": undo_map.and_then(|m| s.redo_next(m)),
            "redo_depth": s.redo_depth(),
        })
    });
    Ok(json!({
        "map": map,
        "project": project_summary,
        "editor": editor,
        "armed_composition": armed.and_then(|a| a.id().map(|s| s.to_owned())),
        "selected": tiles.as_ref().map(|t| t.selected_library_id.clone()),
        "door": door.map(|d| d.label()),
        "mode": mode.map(|m| m.label()),
    }))
}

/// **`emerge/capture_sequence_start` — begin a capture sequence.**
///
/// **A refusal, and not a note** — the sequence is an agent's request, and a request that cannot
/// be honoured has to say so where the agent reads it.
fn capture_sequence_start(
    In(p): In<crate::sequencer::SequenceParams>,
    time: Res<Time>,
    mut sequencer: ResMut<crate::sequencer::Sequencer>,
) -> Result<Value, String> {
    let id = sequencer.start(&p, time.elapsed_secs())?;
    Ok(json!({ "id": id }))
}

/// **`emerge/capture_sequence_status` — what the sequence is doing.**
fn capture_sequence_status(
    In(p): In<crate::sequencer::StatusParams>,
    sequencer: Res<crate::sequencer::Sequencer>,
) -> Result<Value, String> {
    Ok(sequencer.status(p.id))
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

/// **`emerge/operation_undo` — take one change back, by its id.**
///
/// **A front end and nothing more.** [`ops::undo`] resolves the line, dispatches its recorded
/// inverse through the registry and appends the new line; this builds the context from the
/// editor's own resources and puts the map back, exactly as `emerge/operation_apply` does.
fn operation_undo(
    In(p): In<UndoParams>,
    project: Option<ResMut<crate::project::Project>>,
    mut open: Option<ResMut<crate::project::OpenMap>>,
) -> Result<Value, String> {
    if p.dry_run {
        // **A preview reads the log and plans the inverse, and writes nothing.**
        let ctx = Ctx {
            project: project.as_deref().cloned(),
            open: open.as_deref().cloned(),
            source: Source::Agent,
            session: crate::ops::session_id(),
        };
        let resolved = ops::undo_target(&ctx, p.operation_id.as_deref()).and_then(|plan| {
            let op = ops::find(&plan.line.op).ok_or_else(|| {
                ops::Refusal::new(format!(
                    "`{}` is not an operation this editor knows.",
                    plan.line.op
                ))
            })?;
            op.plan(&ctx, &plan.inverse).map(|_| plan)
        });
        return Ok(match resolved {
            Ok(plan) => json!({
                "operation": plan.line.op,
                "outcome": "previewed",
                "change": plan.change,
                "undoes": plan.line.operation_id,
                "would_set": plan.inverse,
            }),
            Err(e) => json!({"operation": "undo", "outcome": "refused", "refusal": e.0}),
        });
    }
    let mut ctx = take(&project, &open);
    let undone = ops::undo(&mut ctx, p.operation_id.as_deref());
    // **The map goes back either way**, so a refused undo leaves the editor holding what it held.
    put_back(&mut open, ctx);
    Ok(match undone {
        Ok((
            plan,
            ops::Applied::Done {
                outcome,
                operation_id,
                entry,
            },
        )) => json!({
            "operation": entry.op,
            "outcome": "applied",
            // **The change taken back or put back**, by the id of the line that made it, and the
            // line whose inverse did it.
            "change": plan.change,
            "undoes": entry.undoes,
            "summary": outcome.summary,
            "saved": outcome.saved,
            "set": entry.params,
            // **The undo's own id**, which is what a redo names.
            "operation_id": operation_id,
            "log_entry": entry,
        }),
        Ok((_, ops::Applied::Refused(e))) | Err(e) => {
            json!({"operation": "undo", "outcome": "refused", "refusal": e.0})
        }
    })
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

/// **The target of `emerge/frame_view`, as the caller writes it.** A bare word names a target (`"map"`
/// or `"selection"`); an object names placements or a rectangle.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FrameTargetParam {
    Named(String),
    Placements(PlacementsParam),
    Rect(RectParam),
}

/// `{"placements": [ids]}` — the ids as the map lists them.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementsParam {
    pub placements: Vec<String>,
}

/// `{"rect": [x0, z0, x1, z1]}` — in map metres.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RectParam {
    pub rect: [f32; 4],
}

/// **The parameters of `emerge/frame_view`.** `deny_unknown_fields`, so a typo is a refusal.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameParams {
    pub target: FrameTargetParam,
}

/// **`emerge/frame_view` — frame the Map camera on a box.**
///
/// **Camera state only.** It moves the view, not the map: it is not an operation, it is not in the
/// registry or the operation log, and it is not undoable. It resolves and applies through
/// [`crate::editor::resolve_frame`] and [`crate::editor::apply_frame`], the one place a frame is computed.
///
/// **The person sees why the view moved**: a framed call leaves a receipt naming what was framed and
/// that an agent asked, so a camera that jumps under the author's eyes is never unexplained.
///
/// **A refusal moves nothing.** Every name is resolved before the camera is touched.
fn frame_view(
    In(p): In<FrameParams>,
    project: Option<Res<crate::project::Project>>,
    open: Option<Res<crate::project::OpenMap>>,
    lock: Option<Res<crate::editor::TargetLock>>,
    rig: Option<ResMut<crate::view::Rig>>,
    cameras: Query<&Camera, With<crate::view::MainCamera>>,
    state: Option<ResMut<crate::editor::EditorState>>,
) -> Result<Value, String> {
    fn refused(why: impl Into<String>) -> Value {
        json!({"outcome": "refused", "why": why.into()})
    }
    let (Some(project), Some(open), Some(mut rig)) = (project, open, rig) else {
        return Ok(refused("no map is open, so there is nothing to frame"));
    };
    let target = match p.target {
        FrameTargetParam::Named(name) => match name.as_str() {
            "map" => crate::editor::FrameTarget::Map,
            "selection" => crate::editor::FrameTarget::Selection,
            other => {
                return Ok(refused(format!(
                    "`{other}` is not a frame target: say \"map\", \"selection\", {{\"placements\": [ids]}} or {{\"rect\": [x0, z0, x1, z1]}}"
                )));
            }
        },
        FrameTargetParam::Placements(list) => {
            crate::editor::FrameTarget::Placements(list.placements)
        }
        FrameTargetParam::Rect(r) => crate::editor::FrameTarget::Rect(r.rect),
    };
    let locked = lock
        .as_deref()
        .and_then(|l| l.placement_id().map(str::to_owned));
    let framing = match crate::editor::resolve_frame(&target, &project, &open, locked.as_deref()) {
        Ok(framing) => framing,
        Err(why) => return Ok(refused(why)),
    };
    let Some(aspect) = cameras.single().ok().and_then(crate::editor::aspect_of) else {
        return Ok(refused(
            "the Map camera has no viewport yet, so there is nothing to frame against",
        ));
    };
    let (focus, height) = crate::editor::apply_frame(&mut rig, &framing, aspect, &open);
    if let Some(mut state) = state {
        state.status.note(format!(
            "an agent framed {} ({:.0} x {:.0} m)",
            framing.what, framing.size.x, framing.size.z,
        ));
    }
    Ok(json!({
        "outcome": "framed",
        "what": framing.what,
        "focus": [focus.x, focus.y, focus.z],
        "height_m": height,
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
                "emerge/state",
                "What the editor is holding right now: the open map and its bounds, how many \
                 pieces it has, what is armed, what is selected, which door and tab are open, and \
                 what the editor last said. A read.",
            )
            .example(json!({})),
            editor_state,
        );

        app.add_debug_command(
            DebugCommandInfo::new(
                "emerge/capture_sequence_start",
                "Begin a capture sequence: the editor takes the frames, so frame 0 is the state \
                 before the operation and the operation is applied after it. Returns the run's id; \
                 poll emerge/capture_sequence_status with it.",
            )
            .params("label: a snake_case name for the sequence. out_dir: the absolute directory the \
                     frames are written to. seconds: how long the sequence runs. fps: 4, or 2 for a \
                     long slow operation. operation: an optional operation to apply once frame 0 is \
                     in.")
            .example(json!({
                "label": "fill_preview",
                "out_dir": "/tmp/seq",
                "seconds": 2.0,
                "fps": 4,
            })),
            capture_sequence_start,
        );

        app.add_debug_command(
            DebugCommandInfo::new(
                "emerge/capture_sequence_status",
                "What a capture sequence is doing: running, done, or failed, and the frames it \
                 wrote. A read.",
            )
            .params("id: the run's id, as emerge/capture_sequence_start returned it.")
            .example(json!({"id": 1})),
            capture_sequence_status,
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
                "emerge/operation_undo",
                "Take one change back by its operation id: the editor dispatches the inverse the \
                 change recorded and appends a new log line naming the line it undoes. Refuses \
                 when a newer change exists, naming it. Previews unless dry_run is false.",
            )
            .params(
                "operation_id: the id to take back; omitted means the newest change not yet \
                     taken back. dry_run: true (the default) previews and changes nothing.",
            )
            .example(json!({"operation_id": "0003-ab12", "dry_run": false})),
            operation_undo,
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

        app.add_debug_command(
            DebugCommandInfo::new(
                "emerge/frame_view",
                "Frame the Map camera on the whole map, on the locked placement, on named placements, or \
                 on a rectangle in map space. Moves only the camera: the map does not change, and the \
                 change is not logged or undoable. A refusal moves nothing.",
            )
            .params(
                "target: \"map\"; \"selection\" (the locked placement, refused when nothing is locked); \
                     {\"placements\": [ids]}; or {\"rect\": [x0, z0, x1, z1]} in map metres.",
            )
            .example(json!({"target": {"rect": [200.0, 100.0, 400.0, 300.0]}})),
            frame_view,
        );
    }
}
