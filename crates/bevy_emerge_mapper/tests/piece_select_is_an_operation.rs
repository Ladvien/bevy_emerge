//! **An agent can select a library piece on the Meshes shelf, and a row click is the same operation.**
//!
//! E-044: selecting a piece was a click handler that wrote the highlight by hand and no operation
//! named it, so an agent could arm a piece (`brush_arm`) but could not point the kit door at one.
//! `piece_select` is the registered operation; the shelf row click dispatches it. These tests go
//! through the registered BRP method, the way an agent's request goes, and assert on the editor's
//! own state.

use bevy::prelude::*;
use bevy_emerge_mapper::filter::{Filters, Pane};
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::ops;
use bevy_emerge_mapper::tiles::{ImportState, Mode};

mod fixtures;

fn editor_at(name: &str, mode: Mode) -> App {
    let root = fixtures::Fixture::new(name)
        .descriptor("site/wall", "alpha")
        .descriptor("site/gate", "beta")
        .build("m");
    let mut app =
        harness::build_headless_at(&root, "m", None, mode).unwrap_or_else(|e| panic!("{e}"));
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

fn select(app: &mut App, id: &str) -> serde_json::Value {
    call(
        app,
        "emerge/operation_apply",
        serde_json::json!({
            "operation": "piece_select",
            "parameters": {"id": id},
            "dry_run": false,
        }),
    )
}

fn selected(app: &App) -> Option<String> {
    app.world().resource::<ImportState>().selected_library_id.clone()
}

#[test]
fn the_registry_has_piece_select() {
    let op = ops::find("piece_select").expect("piece_select is a registered operation");
    assert_eq!(op.undo(), ops::Undo::None, "a selection changes no document");
}

#[test]
fn an_unknown_piece_is_refused_with_the_nearest_ids() {
    let mut app = editor_at("piece-select-unknown", Mode::Meshes);
    let answer = select(&mut app, "site/wal");
    assert_eq!(answer["outcome"], "refused", "{answer}");
    let why = answer["refusal"].as_str().unwrap_or_default();
    assert!(why.contains("site/wall"), "the refusal names the close id: {why}");
    assert_eq!(selected(&app), None, "a refused selection moves nothing");
}

#[test]
fn a_shown_piece_is_selected_and_the_highlight_moves() {
    let mut app = editor_at("piece-select-shown", Mode::Meshes);
    let answer = select(&mut app, "site/gate");
    assert_eq!(answer["outcome"], "applied", "{answer}");
    assert_eq!(selected(&app).as_deref(), Some("site/gate"));
    assert_eq!(app.world().resource::<ImportState>().focused_pack, None);
}

#[test]
fn selecting_needs_the_meshes_shelf_on_screen() {
    let mut app = editor_at("piece-select-map-door", Mode::Map);
    let answer = select(&mut app, "site/gate");
    assert_eq!(answer["outcome"], "refused", "{answer}");
    let why = answer["refusal"].as_str().unwrap_or_default();
    assert!(why.contains("meshes"), "the refusal names the shelf it needs: {why}");
    assert_eq!(selected(&app), None);
}

#[test]
fn a_piece_the_shelf_filter_hides_is_refused() {
    let mut app = editor_at("piece-select-filtered", Mode::Meshes);
    // `site/gate` contains no `w`, so a `w` filter hides it and keeps `site/wall`.
    app.world_mut().resource_mut::<Filters>().push_for_test(Pane::Candidates, 'w');
    let answer = select(&mut app, "site/gate");
    assert_eq!(answer["outcome"], "refused", "{answer}");
    let why = answer["refusal"].as_str().unwrap_or_default();
    assert!(why.contains("filter"), "the refusal says the filter hides it: {why}");
    assert_eq!(selected(&app), None);
}
