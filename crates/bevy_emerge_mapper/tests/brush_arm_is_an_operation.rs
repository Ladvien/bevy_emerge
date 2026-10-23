//! **An agent can arm a brush piece, and the palette click is the same operation.**
//!
//! E-037: arming a piece was a click handler in `editor.rs` that no operation named, so an agent
//! could place pieces but could not choose one. `brush_arm` is the registered operation; the
//! palette click dispatches it. These tests go through the registered BRP method, the way an
//! agent's request goes, and assert on the editor's own state and on the log.

use bevy::prelude::*;
use bevy_emerge_mapper::editor::{EditorState, arm_piece};
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::ops;
use bevy_emerge_mapper::project::Project;

mod fixtures;

fn editor(name: &str) -> App {
    let root = fixtures::Fixture::new(name)
        .descriptor("site/wall", "alpha")
        .descriptor("site/gate", "beta")
        .build("m");
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    app.add_plugins(bevy_debugger_bevy::DebuggerPlugin::default());
    app.add_plugins(bevy_emerge_mapper::ops_brp::OpsBridgePlugin);
    for _ in 0..8 {
        app.update();
    }
    app
}

/// **Through the registered method**, as an agent's request goes.
fn call(app: &mut App, method: &str, params: serde_json::Value) -> serde_json::Value {
    use bevy::remote::{RemoteMethodSystemId, RemoteMethods};
    let Some(RemoteMethodSystemId::Instant(id)) =
        app.world().resource::<RemoteMethods>().get(method).cloned()
    else {
        panic!("no method `{method}` is registered, so an agent cannot reach it");
    };
    app.world_mut()
        .run_system_with(id, Some(params))
        .unwrap_or_else(|e| panic!("{method}: {e:?}"))
        .unwrap_or_else(|e| panic!("{method}: {e:?}"))
}

fn arm(app: &mut App, descriptor: &str) -> serde_json::Value {
    call(
        app,
        "emerge/operation_apply",
        serde_json::json!({
            "operation": "brush_arm",
            "parameters": {"descriptor": descriptor},
            "dry_run": false,
        }),
    )
}

/// **The piece the editor holds armed**, by id: the brush is an index into the library.
fn armed(app: &App) -> Option<String> {
    let project = app.world().resource::<Project>();
    let state = app.world().resource::<EditorState>();
    let ix = state.brush?;
    project.library.descriptors.get(ix).map(|d| d.id.clone())
}

fn log_lines(app: &App) -> usize {
    let project = app.world().resource::<Project>();
    std::fs::read_to_string(ops::oplog_path(project))
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

#[test]
fn an_agent_arms_a_piece_and_the_editor_holds_it() {
    let mut app = editor("arm_known");
    assert_ne!(
        armed(&app).as_deref(),
        Some("site/gate"),
        "the start must hold a different piece, or arming it proves nothing"
    );

    let answer = arm(&mut app, "site/gate");
    assert_eq!(answer["outcome"], "applied", "{answer}");
    assert_eq!(
        armed(&app).as_deref(),
        Some("site/gate"),
        "the editor's brush is the piece the agent named"
    );
}

#[test]
fn an_unknown_piece_is_refused_and_nothing_changes() {
    let mut app = editor("arm_unknown");
    arm(&mut app, "site/gate");
    let before_brush = armed(&app);
    let before_log = log_lines(&app);

    let answer = arm(&mut app, "site/no_such_piece");
    assert_eq!(answer["outcome"], "refused", "{answer}");
    let why = answer["refusal"].as_str().unwrap_or_default();
    assert!(
        why.contains("site/no_such_piece") && why.contains("library"),
        "the refusal names the id and the library it was not found in: {why}"
    );
    assert_eq!(armed(&app), before_brush, "a refused arm leaves the brush as it was");
    assert_eq!(log_lines(&app), before_log, "a refused arm writes no log line");
}

/// **A refusal names the pieces that exist (gap 2).** "The palette lists the library's pieces" sent
/// an agent to a palette it cannot read: `emerge_state` gave the count of descriptors and no ids. The
/// refusal now lists the closest ids and the total, so the agent can retry with a real one.
#[test]
fn a_refusal_names_the_pieces_an_agent_can_arm() {
    let mut app = editor("arm_names");
    let answer = arm(&mut app, "site/wal");
    assert_eq!(answer["outcome"], "refused", "{answer}");
    let why = answer["refusal"].as_str().unwrap_or_default();
    assert!(
        why.contains("site/wall") && why.contains("site/gate"),
        "the refusal lists the library's ids so the agent can retry: {why}"
    );
    assert!(why.contains("2"), "the refusal states how many pieces the library holds: {why}");
}

#[test]
fn the_palette_click_and_the_operation_leave_the_same_state() {
    // The palette path calls `arm_piece`, which dispatches `brush_arm` and applies its outcome.
    // The operation path is an agent's `emerge/operation_apply`. Both must leave the same brush
    // and the same tool.
    let mut by_click = editor("arm_click");
    {
        let project = by_click.world().resource::<Project>().clone();
        let mut state = by_click.world_mut().resource_mut::<EditorState>();
        arm_piece(&mut state, &project, "site/wall");
    }

    let mut by_op = editor("arm_op");
    let answer = arm(&mut by_op, "site/wall");
    assert_eq!(answer["outcome"], "applied", "{answer}");

    assert_eq!(armed(&by_click), armed(&by_op));
    assert_eq!(armed(&by_click).as_deref(), Some("site/wall"));
    assert_eq!(
        by_click.world().resource::<EditorState>().tool,
        by_op.world().resource::<EditorState>().tool,
        "both paths leave the same tool"
    );
}

/// **An arm that could not be logged and changed nothing says so.** The editor took the arm back
/// before the refusal, and the refusal must not say the change is still in place when there was no
/// change to keep.
#[test]
fn an_arm_that_cannot_be_logged_says_nothing_was_changed() {
    let mut app = editor("arm_unlogged");
    let before = armed(&app);
    let log = {
        let project = app.world().resource::<Project>();
        ops::oplog_path(project)
    };
    // A folder where the log file goes: every append is refused.
    let _ = std::fs::remove_file(&log);
    std::fs::create_dir_all(&log).expect("the log path is made a folder");

    let answer = arm(&mut app, "site/gate");
    let text = answer.to_string();
    assert!(
        text.contains("nothing was changed"),
        "an arm that changed nothing must say so: {answer}"
    );
    assert!(
        !text.contains("still in place"),
        "there is no change to keep in place: {answer}"
    );
    assert_eq!(armed(&app), before, "the held piece is unchanged");
    let _ = std::fs::remove_dir_all(&log);
}
