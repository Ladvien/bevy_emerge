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

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};

use bevy::prelude::*;
use bevy_emerge_decide::{Checks, Config, Error as DecideError, Evidence, Finding, Http, Transport};

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
    /// What it was called with, which the checks' facts carry.
    pub params: serde_json::Value,
    /// The operation's own sentence about what it did.
    pub result: String,
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
        .filter_map(|e| {
            Some(AgentEdit {
                seq: e.seq,
                op: e.op,
                operation_id: e.operation_id,
                params: e.params,
                result: e.result?,
            })
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

/// **What the after-edit checks made of one agent operation.**
#[derive(Clone, Debug, PartialEq)]
pub struct EditReport {
    pub seq: u64,
    pub op: String,
    /// Every check that ran and answered, found or not.
    pub findings: Vec<Finding>,
    /// Checks that needed pictures or facts the editor could not give, by name. A check never answers
    /// from less, so these are said rather than guessed.
    pub skipped: Vec<String>,
    /// Checks that could not be asked (the endpoint did not answer), each with the reason.
    pub errors: Vec<String>,
}

impl EditReport {
    /// **The journal lines, advisory and one per thing worth saying.** A clear check is silent.
    pub fn journal_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .findings
            .iter()
            .filter_map(|f| f.message().map(|m| format!("{m} (after {})", self.op)))
            .collect();
        if !self.skipped.is_empty() {
            lines.push(format!(
                "after {}: {} not asked, the editor cannot capture what they need yet",
                self.op,
                self.skipped.join(", ")
            ));
        }
        lines.extend(self.errors.iter().map(|e| format!("after {}: {e}", self.op)));
        lines
    }
}

/// **The facts a check may carry for an operation**: what was run and what it said.
fn facts_of(edit: &AgentEdit) -> serde_json::Value {
    serde_json::json!({
        "operation_id": edit.operation_id,
        "operation": edit.op,
        "parameters": edit.params,
        "result": edit.result,
    })
}

/// **Run every after-edit check on each operation.** `capture` supplies the pictures by name; today the
/// editor supplies none, so any check that needs one is reported as skipped. Blocking: call it off the
/// main thread.
pub fn run_due(
    edits: &[AgentEdit],
    checks: &Checks,
    transport: &dyn Transport,
    capture: &dyn Fn(&AgentEdit) -> BTreeMap<String, String>,
) -> Vec<EditReport> {
    edits
        .iter()
        .map(|edit| {
            let evidence = Evidence {
                images: capture(edit),
                facts: Some(facts_of(edit)),
            };
            let mut report = EditReport {
                seq: edit.seq,
                op: edit.op.clone(),
                findings: Vec::new(),
                skipped: Vec::new(),
                errors: Vec::new(),
            };
            for check in checks.checks.iter().filter(|c| c.after_edit) {
                match check.run(transport, &evidence) {
                    Ok(f) => report.findings.push(f),
                    Err(DecideError::MissingEvidence { .. }) => report.skipped.push(check.name.clone()),
                    Err(e) => report.errors.push(format!("{}: {e}", check.name)),
                }
            }
            report
        })
        .collect()
}

/// Reports coming back from the threads that ask.
#[derive(Resource)]
struct Reports {
    tx: Mutex<Sender<Vec<EditReport>>>,
    rx: Mutex<Receiver<Vec<EditReport>>>,
}

impl Default for Reports {
    fn default() -> Self {
        let (tx, rx) = channel();
        Reports { tx: Mutex::new(tx), rx: Mutex::new(rx) }
    }
}

/// **Drain the queue onto a thread that asks the checks**: a model call never runs on the frame.
fn start_checks(
    project: Option<Res<crate::project::Project>>,
    mut queue: ResMut<AfterEditQueue>,
    reports: Res<Reports>,
    mut journal: ResMut<crate::chrome::Journal>,
) {
    if queue.pending.is_empty() {
        return;
    }
    let Some(project) = project else {
        return;
    };
    let edits = std::mem::take(&mut queue.pending);
    let checks = match Checks::for_project(&project.root) {
        Ok(c) => c,
        Err(e) => {
            journal.record(&format!("after-edit checks not run: {e}"), 1);
            return;
        }
    };
    let Ok(tx) = reports.tx.lock().map(|t| t.clone()) else {
        return;
    };
    std::thread::spawn(move || {
        let http = Http::new(Config::from_env());
        let done = run_due(&edits, &checks, &http, &|_| BTreeMap::new());
        let _ = tx.send(done);
    });
}

/// **Findings into the journal**, where Cmd+E shows them. Advisory: nothing is blocked or undone.
fn collect_reports(reports: Res<Reports>, mut journal: ResMut<crate::chrome::Journal>) {
    let Ok(rx) = reports.rx.lock() else {
        return;
    };
    while let Ok(done) = rx.try_recv() {
        for report in done {
            for line in report.journal_lines() {
                journal.record(&line, 1);
            }
        }
    }
}

pub struct AfterEditPlugin;

impl Plugin for AfterEditPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AfterEditQueue>()
            .init_resource::<Reports>()
            .add_systems(
                Update,
                (queue_after_edit, start_checks, collect_reports).chain(),
            );
    }
}
