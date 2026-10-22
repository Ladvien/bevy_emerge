//! **The pin reason stays on the piece it was typed for.**
//!
//! B-027. The reason field stored the placement's ROW, so a row removed below it while the person
//! typed shifted what Enter pinned: the piece that moved up into the row took the pin, and the one
//! the person named was left bare. The field now holds the placement's id.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::remote::{RemoteMethodSystemId, RemoteMethods};
use bevy_emerge_mapper::editor::EditorState;
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::project::OpenMap;

mod fixtures;

fn editor(name: &str) -> App {
    let root = fixtures::Fixture::new(name)
        .descriptor("site/wall", "alpha")
        .place("site/wall", (0.0, 0.0))
        .place("site/wall", (8.0, 0.0))
        .place("site/wall", (16.0, 0.0))
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

fn held(app: &App) -> bevy_emerge_core::map::Map {
    app.world().resource::<OpenMap>().map.clone()
}

/// One key press and release through the real `KeyboardInput` path.
fn press(app: &mut App, key_code: KeyCode, logical_key: Key) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: logical_key.clone(),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }
    app.update();
}

/// **Enter pins the piece the reason field was opened on, even after a lower row is removed.**
///
/// Rows are `[first, pinned, next]`. The field opens on `pinned` (row 1), the person types `x`, and
/// an agent removes `first` (row 0) through the registry before Enter. Pinned by row, Enter would
/// land on `next`, which has moved up into row 1; pinned by id it lands on `pinned`.
#[test]
fn enter_pins_the_piece_the_reason_was_opened_on_after_a_lower_row_is_removed() {
    let mut app = editor("b027_reason_row");
    let ids: Vec<String> = held(&app).placements.iter().map(|p| p.id.clone()).collect();
    let (first, pinned, next) = (ids[0].clone(), ids[1].clone(), ids[2].clone());

    app.world_mut().resource_mut::<EditorState>().pinning = Some((pinned.clone(), String::new()));
    app.update();
    press(&mut app, KeyCode::KeyX, Key::Character("x".into()));

    let removed = call(
        &mut app,
        "emerge/operation_apply",
        serde_json::json!({"operation": "placement_remove", "parameters": {"id": first}, "dry_run": false}),
    );
    assert_eq!(removed["outcome"], "applied", "{removed}");
    app.update();

    press(&mut app, KeyCode::Enter, Key::Enter);

    let map = held(&app);
    let on_pinned = map
        .placements
        .iter()
        .find(|p| p.id == pinned)
        .expect("the piece the field was opened on still stands");
    let on_next = map
        .placements
        .iter()
        .find(|p| p.id == next)
        .expect("the piece that moved up is still on the map");
    assert!(
        on_pinned.owned,
        "Enter must pin the piece the field was opened on, not the one that moved into its row"
    );
    assert_eq!(on_pinned.owned_because.as_deref(), Some("x"));
    assert!(!on_next.owned, "the piece that moved up into the row must not be pinned");
}
