//! **Cmd+Z takes back the newest change, and a map resize is a change.**
//!
//! B-008. The size field dispatches `map_set_settings` through the registry, and its change went to
//! the operation log but never onto the editor's undo stack. So after a person laid a run of fence
//! and then resized the map, Cmd+Z skipped the resize and took back the fence: an older, unrelated
//! edit vanished and the change the person meant to undo stayed.
//!
//! These drive the human front end end to end: the size field is opened through its own `Activate`,
//! the digits and Enter arrive as `KeyboardInput`, and Cmd+Z / Cmd+Shift+Z are real key presses.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy_emerge_mapper::project::OpenMap;

mod fixtures;

fn editor(tag: &str) -> App {
    let root = fixtures::Fixture::new(tag)
        .sized_descriptor("fence", "alpha", 2.064, 0.12)
        .build("m");
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None)
        .unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..6 {
        app.update();
    }
    app
}

fn tap(app: &mut App, logical: Key, code: KeyCode) {
    for state in [
        bevy::input::ButtonState::Pressed,
        bevy::input::ButtonState::Released,
    ] {
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key: logical.clone(),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }
    app.update();
}

/// **A chord**: the platform modifier (and Shift, when asked) held while `key` is pressed.
fn chord(app: &mut App, key: KeyCode, shift: bool) {
    let held: Vec<KeyCode> = std::iter::once(bevy_emerge_mapper::keys::MOD_KEYS[0])
        .chain(shift.then_some(KeyCode::ShiftLeft))
        .collect();
    for k in &held {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(*k);
    }
    tap(
        app,
        Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
        key,
    );
    for k in &held {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(*k);
    }
    app.update();
}

/// **Resize one axis through the size field**, as a person does: open it, type, Enter.
fn resize(app: &mut App, axis: char, metres: &str) {
    bevy_emerge_mapper::editor::size_field_for_test(app.world_mut(), axis);
    app.update();
    for c in metres.chars() {
        let code = match c {
            '0' => KeyCode::Digit0,
            '1' => KeyCode::Digit1,
            '2' => KeyCode::Digit2,
            '3' => KeyCode::Digit3,
            '4' => KeyCode::Digit4,
            '5' => KeyCode::Digit5,
            '6' => KeyCode::Digit6,
            '7' => KeyCode::Digit7,
            '8' => KeyCode::Digit8,
            '9' => KeyCode::Digit9,
            _ => panic!("not a digit: {c}"),
        };
        tap(app, Key::Character(c.to_string().into()), code);
    }
    tap(app, Key::Enter, KeyCode::Enter);
}

fn map(app: &App) -> bevy_emerge_core::map::Map {
    app.world().resource::<OpenMap>().map.clone()
}

fn on_disk(app: &App) -> bevy_emerge_core::map::Map {
    let path = app.world().resource::<OpenMap>().map_path.clone();
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    bevy_emerge_core::map::Map::parse(&text).unwrap_or_else(|e| panic!("{e}"))
}

/// **Run, then resize, then Cmd+Z: the resize goes, the run stays.** Then Cmd+Z again takes the
/// run, and Cmd+Shift+Z twice puts both back in order.
#[test]
fn cmd_z_after_a_resize_takes_back_the_resize_and_not_an_older_edit() {
    let mut app = editor("undoresize");
    let start = map(&app);
    bevy_emerge_mapper::editor::run_for_test(app.world_mut(), "fence", (0.0, 0.0), (8.0, 0.0));
    app.update();
    let with_run = map(&app).placements.len();
    assert!(
        with_run > start.placements.len(),
        "the run laid nothing, so there is nothing to compare"
    );

    resize(&mut app, 'y', "44");
    assert_eq!(
        map(&app).bounds.1,
        44.0,
        "the size field did not resize the map, so the test drives nothing"
    );

    chord(&mut app, KeyCode::KeyZ, false);
    assert_eq!(
        (map(&app).bounds.1, map(&app).placements.len()),
        (start.bounds.1, with_run),
        "Cmd+Z after a resize must take back the resize and leave the fence; it took back the \
         fence and left the resize, which is an older, unrelated edit disappearing"
    );
    assert_eq!(
        on_disk(&app).bounds.1,
        start.bounds.1,
        "the file still holds the resize"
    );

    chord(&mut app, KeyCode::KeyZ, false);
    assert_eq!(
        map(&app).placements.len(),
        start.placements.len(),
        "the second Cmd+Z did not take the run back"
    );

    chord(&mut app, KeyCode::KeyZ, true);
    assert_eq!(
        map(&app).placements.len(),
        with_run,
        "the first redo did not put the run back"
    );
    chord(&mut app, KeyCode::KeyZ, true);
    assert_eq!(
        map(&app).bounds.1,
        44.0,
        "the second redo did not put the resize back"
    );
    assert_eq!(
        on_disk(&app).bounds.1,
        44.0,
        "the file did not get the redone resize"
    );
}

/// **Cmd+Z and an agent's undo are one history.** If the agent already took the resize back, Cmd+Z
/// does not take it back twice, and says so where the author looks.
#[test]
fn cmd_z_does_not_undo_a_resize_the_agent_already_took_back() {
    let mut app = editor("undoresizeagent");
    let start = map(&app);
    resize(&mut app, 'x', "61");
    assert_eq!(map(&app).bounds.0, 61.0);

    // **The agent's undo, through the same registry function the MCP tool reaches.**
    let mut ctx = bevy_emerge_mapper::ops::Ctx {
        project: Some(
            app.world()
                .resource::<bevy_emerge_mapper::project::Project>()
                .clone(),
        ),
        open: Some(app.world().resource::<OpenMap>().clone()),
        source: bevy_emerge_mapper::ops::Source::Agent,
        session: bevy_emerge_mapper::ops::session_id(),
    };
    bevy_emerge_mapper::ops::undo(&mut ctx, None)
        .unwrap_or_else(|e| panic!("the agent's undo: {e}"));
    let fresh = ctx.open.take().expect("the map comes back");
    *app.world_mut().resource_mut::<OpenMap>() = fresh;
    assert_eq!(map(&app).bounds.0, start.bounds.0);

    chord(&mut app, KeyCode::KeyZ, false);
    assert_eq!(
        map(&app).bounds.0,
        start.bounds.0,
        "Cmd+Z re-applied or double-undid the resize"
    );
    let problems: Vec<String> = app
        .world()
        .resource::<bevy_emerge_mapper::editor::EditorState>()
        .status
        .problems()
        .iter()
        .map(|p| p.line())
        .collect();
    assert!(
        problems.iter().any(|p| p.contains("took it back")),
        "the refusal did not reach the author's problem slot: {problems:?}"
    );
}
