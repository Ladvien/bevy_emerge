//! **An agent can tip a placement, and `Y` and `U` are the same operation (E-002, #129 slice 3).**
//!
//! Tipping a piece was `tip_under_cursor` writing `Placed::tip` and an `Undo::Tipped` entry, named
//! by no operation, so an agent could not lay a piece on its side and the tip reached the file only
//! with the next save. It is now the registered operation `placement_set_tip`. These go through the
//! registered BRP methods, the way an agent's request goes, and assert on the map the editor holds
//! and the file on disk.

use bevy::prelude::*;
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::project::OpenMap;

use crate::fixtures;

/// **A crate standing on a table, and a barrel on its own**, so one piece can tip and one holds
/// another up.
fn editor(name: &str) -> App {
    let root = fixtures::Fixture::new(name)
        .descriptor("table", "alpha")
        .descriptor("crate", "alpha")
        .descriptor("barrel", "alpha")
        .place_as("table@0", "table", (0.0, 0.0))
        .place_on("crate", "table@0", (0.0, 0.0))
        .place_as("barrel@0", "barrel", (3.0, 3.0))
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

fn tip(app: &mut App, parameters: serde_json::Value) -> serde_json::Value {
    call(
        app,
        "emerge/operation_apply",
        serde_json::json!({"operation": "placement_set_tip", "parameters": parameters, "dry_run": false}),
    )
}

fn tip_of(app: &App, id: &str) -> (u8, u8) {
    app.world()
        .resource::<OpenMap>()
        .map
        .placements
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("no placement `{id}`"))
        .tip
}

fn file(app: &App) -> String {
    std::fs::read_to_string(&app.world().resource::<OpenMap>().map_path).expect("the map file")
}

#[test]
fn an_agent_tips_a_piece_the_file_follows_and_undo_puts_it_back() {
    let mut app = editor("tip-agent");
    let before = bevy_emerge_core::map::Map::parse(&file(&app)).expect("the fixture's map parses");

    let a = tip(&mut app, serde_json::json!({"id": "barrel@0", "tip": [1, 0]}));
    assert_eq!(a["outcome"], "applied", "the tip was not applied: {a}");
    assert_eq!(tip_of(&app, "barrel@0"), (1, 0), "the map the editor holds is not tipped");
    let saved = bevy_emerge_core::map::Map::parse(&file(&app)).expect("the map file parses");
    let barrel = saved.placements.iter().find(|p| p.id == "barrel@0").expect("the barrel row");
    assert_eq!(barrel.tip, (1, 0), "the map file does not hold the tip, so a reopen loses it");

    let id = a["operation_id"].as_str().expect("an apply returns its id").to_owned();
    let back = call(
        &mut app,
        "emerge/operation_undo",
        serde_json::json!({"operation_id": id, "dry_run": false}),
    );
    assert_eq!(back["outcome"], "applied", "undo was refused: {back}");
    assert_eq!(tip_of(&app, "barrel@0"), (0, 0));
    let undone = bevy_emerge_core::map::Map::parse(&file(&app)).expect("the map file parses");
    assert_eq!(undone.placements, before.placements, "undo must put the file's rows back");
}

#[test]
fn a_host_an_unknown_piece_and_a_fifth_quarter_are_refused_and_nothing_is_written() {
    let mut app = editor("tip-refused");
    let before = file(&app);

    let host = tip(&mut app, serde_json::json!({"id": "table@0", "tip": [0, 1]}));
    assert_eq!(host["outcome"], "refused", "a piece holding another up was tipped: {host}");
    assert!(
        host.to_string().contains("holds 1 piece"),
        "the refusal does not say what the table holds up: {host}"
    );
    let unknown = tip(&mut app, serde_json::json!({"id": "barel@0", "tip": [1, 0]}));
    assert_eq!(unknown["outcome"], "refused", "a typo was applied: {unknown}");
    let wrapped = tip(&mut app, serde_json::json!({"id": "barrel@0", "tip": [4, 0]}));
    assert_eq!(wrapped["outcome"], "refused", "a fifth quarter turn was applied: {wrapped}");

    assert_eq!(tip_of(&app, "table@0"), (0, 0));
    assert_eq!(tip_of(&app, "barrel@0"), (0, 0));
    assert_eq!(file(&app), before, "a refusal wrote the map file");
}
