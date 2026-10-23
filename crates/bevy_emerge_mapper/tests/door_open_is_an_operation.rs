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

#[test]
fn a_map_that_does_not_parse_is_refused_not_answered_as_applied() {
    // **The Map door crosses onto the map on disk.** A map that will not parse cannot be crossed to,
    // and the crossing used to refuse after the answer had already said "applied". The read is the one
    // the crossing makes, so it belongs in the plan, before anything is answered.
    let root = fixtures::Fixture::new("door_open_bad_map").build("m");
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    let mut project = app.world_mut().remove_resource::<Project>().expect("a project is open");
    let open = app.world_mut().remove_resource::<OpenMap>().expect("a map is open");
    let map_path = open.map_path.clone();
    std::fs::write(&map_path, "this is not a map").expect("the map file can be written");
    project.open_map_name = Some("m".to_owned());
    let mut ctx = Ctx {
        project: Some(project),
        open: None,
        source: Source::Human,
        session: String::new(),
    };
    let text = match ops::dispatch(&mut ctx, "door_open", &json!({"door": "map"})) {
        Err(refusal) => refusal.to_string(),
        Ok(Applied::Refused(refusal)) => refusal.to_string(),
        Ok(done) => panic!("a map that does not parse was answered as applied: {}", summary(done)),
    };
    assert!(
        text.contains(&map_path.display().to_string()),
        "the refusal names the map file that would not open: {text}"
    );
}

#[test]
fn a_dirty_map_asked_to_leave_says_it_waits_for_the_person() {
    // **Leaving unsaved edits is the person's question.** The answer is accepted and names what it
    // waits on; the editor asks the question. It must not read as "opened".
    let root = fixtures::Fixture::new("door_open_dirty").build("m");
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    let project = app.world_mut().remove_resource::<Project>().expect("a project is open");
    let mut open = app.world_mut().remove_resource::<OpenMap>().expect("a map is open");
    open.map.note = Some("an edit that is not saved".to_owned());
    assert!(open.dirty(), "the test makes the map unsaved");
    let mut ctx = Ctx {
        project: Some(project),
        open: Some(open),
        source: Source::Human,
        session: String::new(),
    };
    let applied = ops::dispatch(&mut ctx, "door_open", &json!({"door": "kit"}))
        .unwrap_or_else(|e| panic!("door_open refused a request the person decides: {e}"));
    let Applied::Done { outcome, .. } = applied else {
        panic!("door_open did not accept the request");
    };
    assert_eq!(
        outcome.after.as_ref().and_then(|a| a["waiting"].as_str()),
        Some("the map has unsaved changes"),
        "the answer says it waits on the person, and why: {:?}",
        outcome.after
    );
}

#[test]
fn a_dirty_map_asked_for_the_map_door_waits_for_nobody() {
    // **Already on the Map door, the request crosses nothing**, so nothing is waiting on the person.
    let root = fixtures::Fixture::new("door_open_dirty_map").build("m");
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    let project = app.world_mut().remove_resource::<Project>().expect("a project is open");
    let mut open = app.world_mut().remove_resource::<OpenMap>().expect("a map is open");
    open.map.note = Some("an edit that is not saved".to_owned());
    let mut ctx = Ctx {
        project: Some(project),
        open: Some(open),
        source: Source::Human,
        session: String::new(),
    };
    let applied = ops::dispatch(&mut ctx, "door_open", &json!({"door": "map"}))
        .unwrap_or_else(|e| panic!("door_open refused the Map door: {e}"));
    let Applied::Done { outcome, .. } = applied else {
        panic!("door_open did not accept the request");
    };
    assert!(
        outcome.after.as_ref().is_some_and(|a| a.get("waiting").is_none()),
        "no leaving question for a request that stays on the Map door: {:?}",
        outcome.after
    );
}
