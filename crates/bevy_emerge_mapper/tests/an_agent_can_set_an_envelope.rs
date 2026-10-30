//! **An agent gives a composition an envelope, and takes it back** (E-072 slice 3, `composition_set_envelope`).
//!
//! Through the registered BRP methods: a kept group is anchored; `bounded` fits a tile to its members,
//! an explicit size too small for a member is refused with nothing written, `anchored` takes it off
//! again, and undo restores `compositions.ron` byte for byte.

use bevy::prelude::*;
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::project::Project;

use crate::fixtures;

fn editor(name: &str) -> (std::path::PathBuf, App) {
    let root = fixtures::Fixture::new(name)
        .sized_descriptor("crate", "alpha", 1.0, 1.0)
        .sized_descriptor("barrel", "alpha", 1.0, 1.0)
        .build("m");
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    app.add_plugins(bevy_debugger_bevy::DebuggerPlugin::default());
    app.add_plugins(bevy_emerge_mapper::ops_brp::OpsBridgePlugin);
    for _ in 0..8 {
        app.update();
    }
    (root, app)
}

fn call(app: &mut App, method: &str, params: serde_json::Value) -> serde_json::Value {
    use bevy::remote::{RemoteMethodSystemId, RemoteMethods};
    let Some(RemoteMethodSystemId::Instant(id)) =
        app.world().resource::<RemoteMethods>().get(method).cloned()
    else {
        panic!("no method `{method}` is registered");
    };
    app.world_mut()
        .run_system_with(id, Some(params))
        .unwrap_or_else(|e| panic!("{method}: {e:?}"))
        .unwrap_or_else(|e| panic!("{method}: {e:?}"))
}

fn apply(app: &mut App, operation: &str, parameters: serde_json::Value) -> serde_json::Value {
    call(
        app,
        "emerge/operation_apply",
        serde_json::json!({"operation": operation, "parameters": parameters, "dry_run": false}),
    )
}

fn undo(app: &mut App, id: &str) -> serde_json::Value {
    call(app, "emerge/operation_undo", serde_json::json!({"operation_id": id, "dry_run": false}))
}

fn place(app: &mut App, piece: &str, at: [f32; 2]) -> String {
    let a = apply(app, "placement_add", serde_json::json!({"piece": piece, "at": at}));
    assert_eq!(a["outcome"], "applied", "{a}");
    a["log_entry"]["after"]["id"].as_str().expect("a placement names its row").to_owned()
}

fn comps(root: &std::path::Path) -> String {
    std::fs::read_to_string(root.join("assets/emerge/compositions.ron")).unwrap_or_default()
}

fn id_of(a: &serde_json::Value) -> String {
    assert_eq!(a["outcome"], "applied", "{a}");
    a["operation_id"].as_str().expect("an apply returns its id").to_owned()
}

#[test]
fn a_group_becomes_bounded_and_undo_takes_it_back() {
    let (root, mut app) = editor("envelope_fit");
    let a = place(&mut app, "crate", [1.5, 1.5]);
    let b = place(&mut app, "barrel", [2.5, 1.5]);
    id_of(&apply(&mut app, "composition_define", serde_json::json!({"id": "corner", "from": [a, b]})));
    id_of(&apply(&mut app, "composition_set_envelope", serde_json::json!({"id": "corner", "envelope": "anchored"})));
    let anchored = comps(&root);
    assert!(anchored.contains("Anchored"), "anchored is written: {anchored}");

    let made = id_of(&apply(&mut app, "composition_set_envelope", serde_json::json!({"id": "corner", "envelope": "bounded"})));
    let project = app.world().resource::<Project>();
    let c = project.compositions.compositions.iter().find(|c| c.id == "corner").unwrap();
    let bevy_emerge_core::composition::Envelope::Bounded { size } = c.envelope else {
        panic!("the group is bounded now: {:?}", c.envelope);
    };
    assert!(size.0 >= 2.0 && size.2 >= 1.0, "the tile encloses both 1 m pieces a metre apart: {size:?}");
    assert!(comps(&root).contains("Bounded"), "and the file says so");

    let back = undo(&mut app, &made);
    assert_eq!(back["outcome"], "applied", "{back}");
    assert_eq!(comps(&root), anchored, "undo puts the anchored group back, byte for byte");
}

#[test]
fn a_size_that_leaves_a_member_outside_is_refused_and_writes_nothing() {
    let (root, mut app) = editor("envelope_too_small");
    let a = place(&mut app, "crate", [1.5, 1.5]);
    let b = place(&mut app, "barrel", [3.5, 1.5]);
    id_of(&apply(&mut app, "composition_define", serde_json::json!({"id": "corner", "from": [a, b]})));
    let before = comps(&root);
    let r = apply(
        &mut app,
        "composition_set_envelope",
        serde_json::json!({"id": "corner", "envelope": "bounded", "size": [1.0, 2.0, 1.0]}),
    );
    assert_eq!(r["outcome"], "refused", "{r}");
    assert_eq!(comps(&root), before, "a refusal writes nothing");

    let ghost = apply(&mut app, "composition_set_envelope", serde_json::json!({"id": "ghost", "envelope": "anchored"}));
    assert_eq!(ghost["outcome"], "refused", "{ghost}");
    assert!(ghost.to_string().contains("corner"), "the refusal names the groups there are: {ghost}");
}

#[test]
fn the_compose_chip_flips_the_envelope_through_the_operation() {
    let (root, mut app) = editor("envelope_chip");
    let a = place(&mut app, "crate", [1.5, 1.5]);
    id_of(&apply(&mut app, "composition_define", serde_json::json!({"id": "corner", "from": [a]})));
    let mut project = app.world().resource::<Project>().clone();
    bevy_emerge_mapper::tiles::set_envelope(&mut project, "corner", false).expect("anchored first");
    let receipt = bevy_emerge_mapper::tiles::set_envelope(&mut project, "corner", true).expect("bounded");
    assert!(receipt.contains("bounded"), "{receipt}");
    assert!(comps(&root).contains("Bounded"));
    bevy_emerge_mapper::tiles::set_envelope(&mut project, "corner", false).expect("anchored");
    assert!(comps(&root).contains("Anchored") && !comps(&root).contains("Bounded"));
}
