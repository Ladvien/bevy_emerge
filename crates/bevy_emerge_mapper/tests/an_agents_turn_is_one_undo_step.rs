//! **An agent's turn is one undo step (E-071).**
//!
//! The brief asks for this in as many words: *"An agent's multi-step action is grouped into one undo
//! step, so a single undo reverts the whole agent step."* It was not true. Every dispatched operation
//! became its own `Undo::Logged` entry, so an eight-operation turn was eight Cmd+Z presses — and the
//! person pressing the key had no way to know where the turn ended.
//!
//! # What already existed
//!
//! `Undo::Group` — *"several reversals that are one act"* — is the mechanism `generate` uses so that
//! undoing one `G` press strips the solver rows AND puts the removed sketch back. It is closed under
//! inversion, walked by `leaves`, and honoured by `apply`. So the shape was there and nothing had to
//! be invented; what was missing is that **a turn had no boundary**, so there was nothing to group.
//!
//! # Where the boundary comes from
//!
//! `emerge_present` already takes `operation_ids` — *"the operations this covers"* — and `Undo all`
//! walks them. So the agent already declares the turn; it just was not used as a group. This turns
//! that list into one entry on the undo stack, which is what makes Cmd+Z and the presentation's
//! `Undo all` agree instead of being two ways to take back different amounts.
//!
//! # What is being proved
//!
//! 1. **Three operations, then one Cmd+Z, and the map is back where it started.**
//! 2. **The group is what the presentation named**, not "everything since the last Cmd+Z": an edit
//!    the person made before the turn is left alone.
//! 3. **A presentation naming no operations groups nothing**, so a turn that changed nothing cannot
//!    swallow the person's own next edit.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy_emerge_mapper::editor::EditorState;
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::project::OpenMap;

mod fixtures;

fn editor(name: &str) -> App {
    let root = fixtures::Fixture::new(name)
        .side(16.0)
        .descriptor("crate", "alpha")
        .build("m");
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    app.add_plugins(bevy_debugger_bevy::DebuggerPlugin::default());
    app.add_plugins(bevy_emerge_mapper::ops_brp::OpsBridgePlugin);
    for _ in 0..8 {
        app.update();
    }
    app
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

/// **One operation of a turn**, and its id.
fn apply(app: &mut App, operation: &str, parameters: serde_json::Value) -> String {
    let answer = call(
        app,
        "emerge/operation_apply",
        serde_json::json!({"operation": operation, "parameters": parameters, "dry_run": false}),
    );
    assert_eq!(answer["outcome"], "applied", "{answer}");
    answer["operation_id"]
        .as_str()
        .unwrap_or_else(|| panic!("no operation id: {answer}"))
        .to_owned()
}

/// **Cmd+Z**, as a real chord through the key systems.
fn cmd_z(app: &mut App) {
    use bevy::input::ButtonState;
    let held = bevy_emerge_mapper::keys::MOD_KEYS[0];
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(held);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyZ,
            logical_key: Key::Character("z".into()),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }
    app.update();
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(held);
    app.update();
}

fn placed(app: &App) -> usize {
    app.world().resource::<OpenMap>().map.placements.len()
}

fn undo_depth(app: &App) -> usize {
    app.world().resource::<EditorState>().undo_depth()
}

/// **Three operations, one Cmd+Z, and the map is as it was.**
#[test]
fn an_agents_turn_is_one_undo_step() {
    let mut app = editor("turn_one_step");
    assert_eq!(placed(&app), 0);

    let a = apply(
        &mut app,
        "placement_add",
        serde_json::json!({"piece": "crate", "at": [2.0, 2.0]}),
    );
    let b = apply(
        &mut app,
        "placement_add",
        serde_json::json!({"piece": "crate", "at": [4.0, 2.0]}),
    );
    let c = apply(
        &mut app,
        "placement_add",
        serde_json::json!({"piece": "crate", "at": [6.0, 2.0]}),
    );
    assert_eq!(placed(&app), 3, "three operations of one turn");

    // **The presentation declares the turn**, which is where the boundary comes from.
    let answer = call(
        &mut app,
        "emerge/present",
        serde_json::json!({
            "title": "three crates",
            "summary": "laid three crates in a row",
            "operation_ids": [a, b, c],
        }),
    );
    assert!(
        !answer.to_string().contains("refused"),
        "the presentation was refused: {answer}"
    );

    cmd_z(&mut app);
    assert_eq!(
        placed(&app),
        0,
        "one Cmd+Z must take the whole turn back, not one operation of it ({} left)",
        placed(&app)
    );
    // **And nothing is left on the stack for the turn**, or a second Cmd+Z would take back a
    // `placement_add` that is no longer there.
    assert_eq!(undo_depth(&app), 0, "the turn left an entry behind");
}

/// **The group is what the presentation named.** An edit the person made before the turn is not part
/// of it, so one Cmd+Z after a turn takes the turn and leaves their edit alone.
#[test]
fn a_turn_does_not_swallow_a_persons_earlier_edit() {
    let mut app = editor("turn_keeps_person");
    // **The person's own change**, through the same registry the click path uses.
    apply(
        &mut app,
        "placement_add",
        serde_json::json!({"piece": "crate", "at": [10.0, 10.0]}),
    );
    let before_turn = placed(&app);
    assert_eq!(before_turn, 1);

    let a = apply(
        &mut app,
        "placement_add",
        serde_json::json!({"piece": "crate", "at": [2.0, 2.0]}),
    );
    let b = apply(
        &mut app,
        "placement_add",
        serde_json::json!({"piece": "crate", "at": [4.0, 2.0]}),
    );
    assert_eq!(placed(&app), 3);

    call(
        &mut app,
        "emerge/present",
        serde_json::json!({
            "title": "two crates",
            "summary": "laid two more",
            "operation_ids": [a, b],
        }),
    );
    cmd_z(&mut app);

    assert_eq!(
        placed(&app),
        before_turn,
        "one Cmd+Z must take the turn and leave the earlier edit standing"
    );
    // **And the person's edit is still undoable in its own right**, which is what "left standing"
    // means in a history rather than merely on the map.
    assert_eq!(undo_depth(&app), 1, "the person's own entry went with the turn");
}

/// **A presentation naming no operations groups nothing.** A turn that changed nothing must not
/// swallow the person's next edit into an empty group.
#[test]
fn a_presentation_with_no_operations_groups_nothing() {
    let mut app = editor("turn_empty");
    apply(
        &mut app,
        "placement_add",
        serde_json::json!({"piece": "crate", "at": [2.0, 2.0]}),
    );
    call(
        &mut app,
        "emerge/present",
        serde_json::json!({
            "title": "nothing",
            "summary": "I have not changed anything yet",
            "operation_ids": [],
        }),
    );
    assert_eq!(
        undo_depth(&app),
        1,
        "an empty presentation collapsed the person's own edit into a group"
    );
    cmd_z(&mut app);
    assert_eq!(placed(&app), 0, "the person's edit is still undoable");
}
