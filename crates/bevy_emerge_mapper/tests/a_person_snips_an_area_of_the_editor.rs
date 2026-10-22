//! **A person snips an area of the editor, types a note, and Enter saves it.**
//!
//! `Cmd+\` arms the snip. A press and a drag over the window mark a rectangle, and the release
//! asks for a one-line note. Enter commits; Escape cancels and writes nothing. The keys and the
//! pointer are injected the way `a_person_removes_a_piece_and_undoes_it` injects them.
//!
//! **What a headless app can prove, and what it cannot.** A headless app has no render world, so
//! no screenshot can be read back and no PNG can be written. The gesture, the pending record, the
//! typing guard and the refusal are proved here. The bytes on disk are proved by the crate's own
//! test (`bevy_emerge_snip`), which writes a real PNG and reads the record back, and by a live run.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::{ButtonInput, InputSystems};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window};
use bevy_emerge_mapper::editor::{EditorState, Tool};
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::keys::MOD_KEYS;
use bevy_emerge_mapper::project::{OpenMap, Project};
use bevy_emerge_mapper::snip::SnipEdit;
use bevy_emerge_mapper::view::MainCamera;

mod fixtures;

#[derive(Default)]
struct Frame {
    keys: Vec<KeyCode>,
    pointer: Option<Vec2>,
    mouse: Option<bool>,
}

#[derive(Resource, Default)]
struct Script {
    next: Option<Frame>,
    held: Vec<KeyCode>,
}

fn drive(
    mut script: ResMut<Script>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let frame = script.next.take();
    for key in script.held.drain(..) {
        keys.release(key);
    }
    let Some(frame) = frame else {
        return;
    };
    for key in &frame.keys {
        keys.press(*key);
    }
    script.held = frame.keys;
    if let Ok(mut window) = windows.single_mut() {
        window.set_cursor_position(frame.pointer);
    }
    match frame.mouse {
        Some(true) => mouse.press(MouseButton::Left),
        Some(false) => mouse.release(MouseButton::Left),
        None => {}
    }
}

fn step(app: &mut App, frame: Frame) {
    app.world_mut().resource_mut::<Script>().next = Some(frame);
    app.update();
}

/// One printable character, typed as a person types it: the key goes down (so the keyboard census
/// sees it) and the character arrives as a keyboard message (so the note field reads it).
fn type_char(app: &mut App, ch: char, code: KeyCode) {
    step(
        app,
        Frame {
            keys: vec![code],
            ..default()
        },
    );
    for state in [
        bevy::input::ButtonState::Pressed,
        bevy::input::ButtonState::Released,
    ] {
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key: Key::Character(ch.to_string().into()),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }
    app.update();
}

fn tap(app: &mut App, logical: Key, code: KeyCode) {
    step(
        app,
        Frame {
            keys: vec![code],
            ..default()
        },
    );
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

fn pixel_over(app: &mut App, at: (f32, f32)) -> Vec2 {
    let origin = app.world().resource::<OpenMap>().map.origin;
    let world = Vec3::new(origin.0 + at.0, 0.0, origin.2 + at.1);
    let mut cameras = app
        .world_mut()
        .query_filtered::<(&Camera, &GlobalTransform), With<MainCamera>>();
    let (camera, transform) = cameras
        .single(app.world())
        .expect("the editor has one main camera");
    camera
        .world_to_viewport(transform, world)
        .unwrap_or_else(|e| panic!("the spot is not on screen: {e:?}"))
}

fn editor(tag: &str) -> App {
    let root = fixtures::Fixture::new(tag)
        .descriptor("site/wall", "alpha")
        .place("site/wall", (0.0, 0.0))
        .place("site/wall", (3.0, 0.0))
        .build("m");
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    app.init_resource::<Script>();
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.add_systems(PreUpdate, drive.after(InputSystems));
    for _ in 0..8 {
        app.update();
    }
    app
}

/// The snip folder under the project. Nothing is written there headless, and the test says so.
fn snips_dir(app: &App) -> std::path::PathBuf {
    app.world()
        .resource::<Project>()
        .root
        .join("target/loop/captures/snips")
}

fn files_in(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| entries.count())
        .unwrap_or(0)
}

/// Arm the snip and drag a rectangle over the two pieces. Returns the rectangle in window pixels.
fn arm_and_drag(app: &mut App) -> [f32; 4] {
    let first = pixel_over(app, (0.0, 0.0));
    let last = pixel_over(app, (3.0, 0.0));
    let start = first - Vec2::new(60.0, 60.0);
    let end = last + Vec2::new(60.0, 60.0);

    step(
        app,
        Frame {
            keys: vec![MOD_KEYS[0], KeyCode::Backslash],
            ..default()
        },
    );
    assert!(
        app.world().resource::<SnipEdit>().armed,
        "Cmd+\\ must arm the snip"
    );

    step(
        app,
        Frame {
            pointer: Some(start),
            mouse: Some(true),
            ..default()
        },
    );
    step(
        app,
        Frame {
            pointer: Some(end),
            ..default()
        },
    );
    step(
        app,
        Frame {
            pointer: Some(end),
            mouse: Some(false),
            ..default()
        },
    );
    [start.x, start.y, end.x, end.y]
}

#[test]
fn a_person_snips_an_area_and_types_a_note_and_enter_asks_to_save_it() {
    let mut app = editor("snip_gesture");
    let rect = arm_and_drag(&mut app);

    let pending = app
        .world()
        .resource::<SnipEdit>()
        .pending
        .clone()
        .expect("the release must leave a snip waiting for its note");
    assert!(
        (pending.rect[0] - rect[0]).abs() < 1.0
            && (pending.rect[1] - rect[1]).abs() < 1.0
            && (pending.rect[2] - rect[2]).abs() < 1.0
            && (pending.rect[3] - rect[3]).abs() < 1.0,
        "the snip must hold the dragged rectangle: {pending:?} vs {rect:?}"
    );
    assert_eq!(pending.note, "", "the note starts empty");

    // **Typing in the note is typing, not tools.** `X` arms removal mode everywhere else.
    type_char(&mut app, 'x', KeyCode::KeyX);
    assert!(
        !matches!(app.world().resource::<EditorState>().tool, Tool::Remove),
        "a letter typed into the note must not arm removal mode"
    );
    assert_eq!(
        app.world()
            .resource::<SnipEdit>()
            .pending
            .as_ref()
            .map(|p| p.note.as_str()),
        Some("x"),
        "the letter goes into the note"
    );

    // **Enter commits.** Headless there is no render world, so nothing can be written: the
    // refusal goes in `Status::problem`, and no snip folder is created.
    tap(&mut app, Key::Enter, KeyCode::Enter);
    assert!(
        app.world().resource::<SnipEdit>().pending.is_none(),
        "Enter must end the snip"
    );
    let problem = app.world().resource::<EditorState>().status.problem_text();
    assert!(
        problem.contains("render"),
        "a headless snip must be refused with a reason in the problem slot, got: {problem:?}"
    );
    assert_eq!(
        files_in(&snips_dir(&app)),
        0,
        "a refused snip writes no file"
    );
}

#[test]
fn a_person_cancels_a_snip_with_escape_and_nothing_is_written() {
    let mut app = editor("snip_cancel");
    arm_and_drag(&mut app);
    type_char(&mut app, 'q', KeyCode::KeyQ);
    tap(&mut app, Key::Escape, KeyCode::Escape);

    assert!(
        app.world().resource::<SnipEdit>().pending.is_none(),
        "Escape must drop the snip"
    );
    assert!(
        !app.world().resource::<SnipEdit>().armed,
        "Escape must leave the snip disarmed"
    );
    assert_eq!(
        files_in(&snips_dir(&app)),
        0,
        "a cancelled snip writes no file"
    );
}

fn placed_count(app: &App) -> usize {
    app.world().resource::<OpenMap>().map.placements.len()
}

/// Arm the snip and drag from `start` to `end`, the way a person does. Returns with the release done.
fn drag_snip(app: &mut App, start: Vec2, end: Vec2) {
    step(
        app,
        Frame {
            keys: vec![MOD_KEYS[0], KeyCode::Backslash],
            ..default()
        },
    );
    step(
        app,
        Frame {
            pointer: Some(start),
            mouse: Some(true),
            ..default()
        },
    );
    step(
        app,
        Frame {
            pointer: Some(end),
            ..default()
        },
    );
    step(
        app,
        Frame {
            pointer: Some(end),
            mouse: Some(false),
            ..default()
        },
    );
}

/// **A snip is a capture, so its drag belongs to the snip and never to the piece tool under it.**
/// Place mode commits a piece on a click; a drag that is really a snip must place nothing.
#[test]
fn a_snip_drag_in_place_mode_places_nothing() {
    let mut app = editor("snip_place_gate");
    let before = placed_count(&app);
    let start = pixel_over(&mut app, (6.0, 0.0)) - Vec2::new(40.0, 40.0);
    let end = pixel_over(&mut app, (9.0, 0.0)) + Vec2::new(40.0, 40.0);
    drag_snip(&mut app, start, end);

    assert!(
        app.world().resource::<SnipEdit>().pending.is_some(),
        "the drag must still have been taken as a snip"
    );
    assert_eq!(
        placed_count(&app),
        before,
        "a snip drag in Place mode placed a piece"
    );
}

/// **A snip drag that starts on a piece in Remove mode removes nothing.** The removal box would
/// otherwise delete everything the snip covers.
#[test]
fn a_snip_drag_in_remove_mode_removes_nothing() {
    let mut app = editor("snip_remove_gate");
    app.world_mut().resource_mut::<EditorState>().tool = Tool::Remove;
    let before = placed_count(&app);
    let start = pixel_over(&mut app, (0.0, 0.0)) - Vec2::new(60.0, 60.0);
    let end = pixel_over(&mut app, (3.0, 0.0)) + Vec2::new(60.0, 60.0);
    drag_snip(&mut app, start, end);

    assert!(
        app.world().resource::<SnipEdit>().pending.is_some(),
        "the drag must still have been taken as a snip"
    );
    assert_eq!(
        placed_count(&app),
        before,
        "a snip drag in Remove mode removed a piece"
    );
}
