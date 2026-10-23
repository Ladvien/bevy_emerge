//! **Opening a door is a registered operation (cycle 8, item 1).**
//!
//! `door_open` names the door and, optionally, a panel of it. The answer is the target; the crossing
//! happens in the editor. These tests cover the operation's own contract: that it is registered,
//! that it refuses what it cannot do with the reason in the words, and that it accepts a request
//! with the exact target in its answer. Nothing here writes a map.

use bevy_emerge_mapper::editor::{DoorRequest, EditorState};
use bevy_emerge_mapper::tiles::{Door, Mode};
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::ops::{self, Applied, Ctx, Source};
use bevy_emerge_mapper::project::{OpenMap, Project};
use serde_json::json;

mod fixtures;

fn empty_ctx() -> Ctx {
    Ctx {
        project: None,
        open: None,
        source: Source::Human,
        session: String::new(),
    }
}

fn refusal_text(params: serde_json::Value) -> String {
    let mut ctx = empty_ctx();
    match ops::dispatch(&mut ctx, "door_open", &params) {
        Err(refusal) => refusal.to_string(),
        Ok(applied) => panic!("expected a refusal, got {:?}", summary(applied)),
    }
}

fn summary(applied: Applied) -> String {
    match applied {
        Applied::Done { outcome, .. } => outcome.summary,
        other => format!("{other:?}"),
    }
}

#[test]
fn the_registry_has_door_open() {
    assert!(
        ops::registry().iter().any(|op| op.name() == "door_open"),
        "the registry has no `door_open`, so an agent cannot open a door through an operation"
    );
}

#[test]
fn an_unknown_door_is_refused_and_the_doors_are_named() {
    let text = refusal_text(json!({"door": "attic"}));
    assert!(text.contains("attic"), "the refusal names what was asked: {text}");
    for door in ["kit", "map", "rigs"] {
        assert!(text.contains(door), "the refusal lists the doors: {text}");
    }
}

#[test]
fn a_panel_the_door_does_not_show_is_refused() {
    // The Kit door shows its own panels; a Map-door panel asked for on Kit must be refused.
    let text = refusal_text(json!({"door": "kit", "tab": "map"}));
    assert!(
        text.contains("does not show"),
        "a panel the door does not show is refused with that reason: {text}"
    );
}

#[test]
fn the_map_door_needs_a_map_to_open_onto() {
    let text = refusal_text(json!({"door": "map"}));
    assert!(
        text.contains("no map is open"),
        "the Map door with no map open is refused with the reason: {text}"
    );
}

#[test]
fn an_accepted_request_names_its_exact_target() {
    // **A project is open, as it is in the editor.** The operation writes its log line to the
    // project, so a dispatch with no project is refused before any door is named.
    let root = fixtures::Fixture::new("door_open_accepted").build("m");
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    let mut ctx = Ctx {
        project: app.world_mut().remove_resource::<Project>(),
        open: app.world_mut().remove_resource::<OpenMap>(),
        source: Source::Human,
        session: String::new(),
    };
    let applied = ops::dispatch(&mut ctx, "door_open", &json!({"door": "KIT", "tab": "Meshes"}))
        .unwrap_or_else(|e| panic!("door_open refused a valid request: {e}"));
    let Applied::Done { outcome, .. } = applied else {
        panic!("door_open did not apply");
    };
    assert_eq!(
        outcome.after,
        Some(json!({"door": "kit", "tab": "meshes"})),
        "the answer names the door and panel the editor must cross to"
    );
    assert!(
        outcome.inverse.is_none(),
        "opening a door changes no document, so there is nothing to take back"
    );
}

#[test]
fn a_stored_request_crosses_the_door_on_the_next_frame() {
    // **The BRP path stores the request; the editor crosses it in the next frame.** This is the
    // half an agent's `door_open` reaches: `request_door` puts it in `EditorState::door_request`
    // and `apply_pending_door` crosses it.
    let root = fixtures::Fixture::new("door_open_crosses").build("m");
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..4 {
        app.update();
    }
    assert_ne!(
        *app.world().resource::<Door>(),
        Door::Kit,
        "the test starts on another door, so a crossing to Kit is visible"
    );
    app.world_mut().resource_mut::<EditorState>().door_request =
        Some(DoorRequest { door: Door::Kit, tab: Some(Mode::Meshes) });
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(*app.world().resource::<Door>(), Door::Kit, "the request crossed to the Kit door");
    assert!(*app.world().resource::<Mode>() == Mode::Meshes, "and opened the Meshes panel");
    assert!(
        app.world().resource::<EditorState>().door_request.is_none(),
        "a crossed request is consumed, so it does not cross again"
    );
}
