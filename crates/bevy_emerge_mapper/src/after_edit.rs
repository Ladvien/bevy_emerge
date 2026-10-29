//! **When the project's after-edit checks are due: once per operation an agent applied, never for a
//! person's own edit (#111).**
//!
//! A person editing is not interrupted by a model call, and a finding about their own placement would
//! be a comment nobody asked for. An agent's operation is different: nobody was watching it land, so
//! the editor looks for it. The trigger reads the operation log, the same file the journal's steps are
//! read from, and so sees an agent over BRP and over MCP alike without either side calling it.
//!
//! This module decides *when*; it takes no picture and calls no model. [`AfterEditQueue::pending`] is
//! what a runner drains: the sequence and name of each agent operation whose checks are owed.
//!
//! What triggers: a line whose `source` is `agent`, that was `applied` (it has a `result`), and that is
//! not an undo (an undo puts a state back that was already looked at). What does not: a person's line,
//! a refusal, anything already in the log when the editor opened the project.

use bevy::prelude::*;

use crate::ops::{Entry, Source};

/// One agent operation whose after-edit checks are owed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentEdit {
    /// The operation log line it is, so a finding hangs on the step that made it.
    pub seq: u64,
    /// The operation's name (`placement_add`).
    pub op: String,
    /// The id the agent cites, which a finding names.
    pub operation_id: String,
}

/// **The checks owed, oldest first.**
#[derive(Resource, Default, Debug)]
pub struct AfterEditQueue {
    /// Drained by whatever runs the checks.
    pub pending: Vec<AgentEdit>,
    /// The last log sequence looked at. `None` until the first read, which takes the log as it stands
    /// as already seen: history is not re-checked each time the editor opens.
    seen: Option<u64>,
}

/// **The agent operations in a log's text with a sequence above `after`**, oldest first.
pub fn agent_edits(text: &str, after: Option<u64>) -> Vec<AgentEdit> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Entry>(l).ok())
        .filter(|e| after.is_none_or(|a| e.seq > a))
        .filter(|e| e.source == Source::Agent && e.result.is_some() && e.undoes.is_none())
        .map(|e| AgentEdit {
            seq: e.seq,
            op: e.op,
            operation_id: e.operation_id,
        })
        .collect()
}

/// The highest sequence in a log's text.
fn last_seq(text: &str) -> Option<u64> {
    text.lines()
        .filter_map(|l| serde_json::from_str::<Entry>(l).ok())
        .map(|e| e.seq)
        .max()
}

/// **Queue the checks owed for agent operations that landed since the last look.**
fn queue_after_edit(
    project: Option<Res<crate::project::Project>>,
    mut queue: ResMut<AfterEditQueue>,
) {
    let Some(project) = project else {
        return;
    };
    let Ok(text) = std::fs::read_to_string(crate::ops::oplog_path(project.as_ref())) else {
        return;
    };
    let Some(newest) = last_seq(&text) else {
        return;
    };
    match queue.seen {
        // First look: what is there already was checked, or was not asked about, in its own session.
        None => queue.seen = Some(newest),
        Some(seen) if newest > seen => {
            let fresh = agent_edits(&text, Some(seen));
            queue.seen = Some(newest);
            queue.pending.extend(fresh);
        }
        Some(_) => {}
    }
}

pub struct AfterEditPlugin;

impl Plugin for AfterEditPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AfterEditQueue>()
            .add_systems(Update, queue_after_edit);
    }
}
