//! **`emerge/pieces` lists the library's pieces, a page at a time.**
//!
//! `emerge/state` names the first page of library ids and says where the next page starts, but an
//! agent that wants the pieces themselves (the mesh each one names, its kinds, effects and looks)
//! has to read them one by one. `emerge/pieces` is that list: the same paging as the state's
//! `descriptor_ids`, each row carrying the fields a piece is chosen by, and `model_only` to leave out
//! the flat pieces that name no mesh.

use bevy::prelude::*;
use bevy_emerge_mapper::harness;

mod fixtures;

/// The most rows one page of `emerge/pieces` names. Matches the cap written into the answer.
const PIECE_CAP: usize = 50;
const PIECES: usize = 60;

fn editor(name: &str) -> App {
    let mut fixture = fixtures::Fixture::new(name);
    for i in 0..PIECES {
        fixture = fixture.descriptor(&format!("site/p{i:02}"), "alpha");
    }
    let root = fixture.build("m");
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

#[test]
fn emerge_pieces_names_the_first_page_and_says_where_the_next_starts() {
    let mut app = editor("pieces_first_page");
    let answer = call(&mut app, "emerge/pieces", serde_json::json!({}));

    assert_eq!(
        answer["total"],
        serde_json::json!(PIECES),
        "the true count is reported: {answer}"
    );
    let pieces = answer["pieces"].as_array().cloned().unwrap_or_default();
    assert_eq!(pieces.len(), PIECE_CAP, "one page names the cap, not the library: {answer}");
    assert_eq!(pieces[0]["id"], "site/p00", "the page starts at the start: {answer}");
    assert!(
        pieces[0]["mesh"].as_str().is_some_and(|m| m.ends_with(".glb")),
        "each row names the mesh its piece is drawn with: {answer}"
    );
    assert_eq!(
        answer["pieces_next"],
        serde_json::json!(PIECE_CAP),
        "the next page starts where this one stopped: {answer}"
    );
}

#[test]
fn emerge_pieces_pages_past_the_cap() {
    let mut app = editor("pieces_pages");
    let answer = call(&mut app, "emerge/pieces", serde_json::json!({"pieces_from": PIECE_CAP}));
    let pieces = answer["pieces"].as_array().cloned().unwrap_or_default();
    assert_eq!(pieces.len(), PIECES - PIECE_CAP, "the second page holds the rest: {answer}");
    assert_eq!(
        pieces.last().and_then(|p| p["id"].as_str()),
        Some("site/p59"),
        "the last piece is on the last page: {answer}"
    );
    assert_eq!(answer["pieces_next"], serde_json::Value::Null, "nothing follows: {answer}");
}

#[test]
fn model_only_leaves_out_the_flat_pieces_that_name_no_mesh() {
    let root = fixtures::Fixture::new("pieces_model_only")
        .descriptor("site/house", "alpha")
        .descriptor("site/barn", "alpha")
        .descriptor("site/tree", "alpha")
        .flat_descriptor("site/road", 1.0, 1.0)
        .flat_descriptor("site/field", 1.0, 1.0)
        .build("m");
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    app.add_plugins(bevy_debugger_bevy::DebuggerPlugin::default());
    app.add_plugins(bevy_emerge_mapper::ops_brp::OpsBridgePlugin);
    for _ in 0..8 {
        app.update();
    }

    let all = call(&mut app, "emerge/pieces", serde_json::json!({}));
    assert_eq!(all["total"], serde_json::json!(5), "every piece is counted: {all}");

    let models = call(&mut app, "emerge/pieces", serde_json::json!({"model_only": true}));
    assert_eq!(models["total"], serde_json::json!(3), "only the three meshed pieces: {models}");
    let pieces = models["pieces"].as_array().cloned().unwrap_or_default();
    assert!(
        pieces.iter().all(|p| p["mesh"].is_string()),
        "no row with no mesh survives model_only: {models}"
    );
    assert!(
        pieces.iter().all(|p| p["id"] != "site/road" && p["id"] != "site/field"),
        "the flat pieces are gone: {models}"
    );
}
