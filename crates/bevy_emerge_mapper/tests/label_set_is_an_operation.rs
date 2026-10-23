//! **An agent can set a piece's label axis, through the vocabulary gate, and the write is attributed
//! to the agent.**
//!
//! E-042: an agent could read a piece's labels but could not write one. `label_set` is the
//! registered operation. It refuses an unknown token with the nearest ones, refuses a duplicate,
//! refuses a piece the measured library does not hold, writes `library.ron` through the same path a
//! chip click uses, stamps the axis `Agent` (not `Model`, which is a VLM's word, and not `Human`,
//! which is a person's confirmation), and undoes to the exact prior record. These tests go through the
//! registered BRP methods, as an agent's request goes, and assert on the editor's own state and on
//! the file it wrote.

use bevy::prelude::*;
use bevy_emerge_core::descriptor::{Axis, By};
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::ops;
use bevy_emerge_mapper::project::Project;
use bevy_emerge_mapper::tiles::Mode;

mod fixtures;

fn editor_at(name: &str) -> App {
    let root = fixtures::Fixture::new(name)
        .descriptor("site/wall", "alpha")
        .descriptor("site/gate", "beta")
        .look_tokens(&["worn"])
        .build("m");
    let mut app =
        harness::build_headless_at(&root, "m", None, Mode::Meshes).unwrap_or_else(|e| panic!("{e}"));
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

fn label_set(app: &mut App, id: &str, axis: &str, tokens: &[&str]) -> serde_json::Value {
    call(
        app,
        "emerge/operation_apply",
        serde_json::json!({
            "operation": "label_set",
            "parameters": {"id": id, "axis": axis, "tokens": tokens},
            "dry_run": false,
        }),
    )
}

/// The piece as the editor holds it in memory — the live resource, not a copy.
fn live_piece(app: &App, id: &str) -> bevy_emerge_core::descriptor::Descriptor {
    app.world()
        .resource::<Project>()
        .measured
        .descriptors
        .iter()
        .find(|d| d.id == id)
        .unwrap_or_else(|| panic!("`{id}` is not in the live measured library"))
        .clone()
}

fn look_of(app: &App, id: &str) -> Vec<String> {
    live_piece(app, id).look
}

fn look_by(app: &App, id: &str) -> Option<By> {
    live_piece(app, id).labels.and_then(|l| l.by.get(Axis::Look))
}

#[test]
fn the_registry_has_label_set() {
    let op = ops::find("label_set").expect("label_set is a registered operation");
    assert_eq!(op.undo(), ops::Undo::Inverse, "a label change is put back by its inverse");
}

#[test]
fn an_unknown_token_is_refused_with_the_nearest_tokens() {
    let mut app = editor_at("label-set-unknown");
    let answer = label_set(&mut app, "site/wall", "look", &["wrn"]);
    assert_eq!(answer["outcome"], "refused", "{answer}");
    let why = answer["refusal"].as_str().unwrap_or_default();
    assert!(why.contains("worn"), "the refusal names the close token: {why}");
    assert_eq!(look_of(&app, "site/wall"), vec!["plain".to_string()], "a refused label moves nothing");
}

#[test]
fn a_duplicate_token_is_refused() {
    let mut app = editor_at("label-set-duplicate");
    let answer = label_set(&mut app, "site/wall", "look", &["plain", "plain"]);
    assert_eq!(answer["outcome"], "refused", "{answer}");
    assert_eq!(look_of(&app, "site/wall"), vec!["plain".to_string()]);
}

#[test]
fn a_piece_the_measured_library_does_not_hold_is_refused() {
    let mut app = editor_at("label-set-unmeasured");
    let answer = label_set(&mut app, "site/nope", "look", &["plain"]);
    assert_eq!(answer["outcome"], "refused", "{answer}");
}

#[test]
fn a_change_is_sorted_stamped_agent_and_reaches_the_editor_and_the_file() {
    let mut app = editor_at("label-set-agent");
    // Given out of vocabulary order: the stored list follows the vocabulary's order.
    let answer = label_set(&mut app, "site/wall", "look", &["worn", "plain"]);
    assert_eq!(answer["outcome"], "applied", "{answer}");
    assert_eq!(
        look_of(&app, "site/wall"),
        vec!["plain".to_string(), "worn".to_string()],
        "the live library holds the write, so the next save cannot lose it"
    );
    assert_eq!(look_by(&app, "site/wall"), Some(By::Agent), "an agent's write is not a model's or a person's");
    let path = app.world().resource::<Project>().library_path.clone();
    let on_disk = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
    assert!(on_disk.contains("worn"), "the file holds the new token: {on_disk}");
    assert!(on_disk.contains("Agent"), "the file holds the provenance: {on_disk}");
}

#[test]
fn undo_restores_the_prior_record_exactly() {
    let mut app = editor_at("label-set-undo");
    let before = live_piece(&app, "site/wall");
    let applied = label_set(&mut app, "site/wall", "look", &["plain", "worn"]);
    assert_eq!(applied["outcome"], "applied", "{applied}");
    let line = applied["operation_id"].as_str().unwrap_or_else(|| panic!("no operation id: {applied}"));
    let undone = call(
        &mut app,
        "emerge/operation_undo",
        serde_json::json!({"operation_id": line, "dry_run": false}),
    );
    assert_ne!(undone["outcome"], "refused", "{undone}");
    let after = live_piece(&app, "site/wall");
    assert_eq!(after.look, before.look, "the list comes back");
    assert_eq!(
        serde_json::to_value(&after.labels).unwrap(),
        serde_json::to_value(&before.labels).unwrap(),
        "the labels record comes back exactly, including the absence of one"
    );
}

#[test]
fn a_save_that_fails_refuses_and_changes_nothing() {
    let mut app = editor_at("label-set-save-fails");
    let path = app.world().resource::<Project>().library_path.clone();
    let dir = path.parent().expect("a library has a directory").to_path_buf();
    let mut locked = std::fs::metadata(&dir).expect("the kit directory exists").permissions();
    let open_perms = locked.clone();
    use std::os::unix::fs::PermissionsExt;
    locked.set_mode(0o555);
    std::fs::set_permissions(&dir, locked).expect("make the kit directory read-only");

    let answer = label_set(&mut app, "site/wall", "look", &["plain", "worn"]);
    std::fs::set_permissions(&dir, open_perms).expect("restore the kit directory");

    assert_eq!(answer["outcome"], "refused", "{answer}");
    assert_eq!(look_of(&app, "site/wall"), vec!["plain".to_string()], "the editor holds what the file holds");
    assert_eq!(look_by(&app, "site/wall"), None);
}
