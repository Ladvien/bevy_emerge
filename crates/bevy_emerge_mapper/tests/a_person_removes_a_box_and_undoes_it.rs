//! **A person's box removal goes through the registry, one line per piece, and one Cmd+Z takes the
//! whole box back in the order it was in.**
//!
//! The box path used to delete rows in place and record `Undo::RemovedMany`, which left no line in
//! the operation log for the pieces it took. It now dispatches `placement_remove` for each piece,
//! back to front (so no index is shifted under a later removal), and records the lines as one
//! `Undo::Group`. Undo applies the group's children in list order, so the list must run lowest index
//! first: a put-back of index 2 before index 0 would land the second piece one slot short.
//!
//! The pointer and keys are injected the way `a_person_removes_a_piece_and_undoes_it.rs` does it.

use bevy::input::{ButtonInput, InputSystems};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window};
use bevy_emerge_mapper::editor::{EditorState, Tool};
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::keys::MOD_KEYS;
use bevy_emerge_mapper::ops::{self, Entry};
use bevy_emerge_mapper::project::{OpenMap, Project};
use bevy_emerge_mapper::view::MainCamera;

mod fixtures;

/// One frame's worth of person: the keys held down, the pointer, and a mouse press or release.
#[derive(Default)]
struct Frame {
    keys: Vec<KeyCode>,
    pointer: Option<Vec2>,
    mouse: Option<bool>,
}

/// The next frame to play. `held` is what the previous frame pressed, released at the start of the
/// next so no key is left down by accident.
#[derive(Resource, Default)]
struct Script {
    next: Option<Frame>,
    held: Vec<KeyCode>,
}

/// Runs in `PreUpdate` after `InputSystems`, so the presses survive into `Update` and the window's
/// cursor is already where the person put it when `sense_pointer` reads it.
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

/// Where on the screen a person would point to hit the ground at a map-space spot.
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

fn held(app: &App) -> bevy_emerge_core::map::Map {
    app.world().resource::<OpenMap>().map.clone()
}

fn on_disk(app: &App) -> bevy_emerge_core::map::Map {
    let path = app.world().resource::<OpenMap>().map_path.clone();
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    bevy_emerge_core::map::Map::parse(&text).unwrap_or_else(|e| panic!("{e}"))
}

fn ids(map: &bevy_emerge_core::map::Map) -> Vec<String> {
    map.placements.iter().map(|p| p.id.clone()).collect()
}

fn log(app: &App) -> Vec<Entry> {
    let path = ops::oplog_path(app.world().resource::<Project>());
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Entry>(line).unwrap_or_else(|e| panic!("{e}: {line}")))
        .collect()
}

#[test]
fn a_person_removes_a_box_of_pieces_and_one_cmd_z_puts_them_all_back_in_order() {
    // Three pieces: the two on the row are inside the box, the one off it is not. The box takes the
    // first and the last, so the survivor sits between them and the restored order is checkable.
    let root = fixtures::Fixture::new("person_box_remove")
        .descriptor("site/wall", "alpha")
        .place("site/wall", (0.0, 0.0))
        .place("site/wall", (3.0, 5.0))
        .place("site/wall", (6.0, 0.0))
        .build("m");
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    app.init_resource::<Script>();
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.add_systems(PreUpdate, drive.after(InputSystems));
    for _ in 0..8 {
        app.update();
    }
    assert_eq!(
        ids(&held(&app)),
        ["site/wall@0", "site/wall@1", "site/wall@2"],
        "the fixture starts with three"
    );

    let from = pixel_over(&mut app, (-1.0, -1.0));
    let to = pixel_over(&mut app, (7.0, 1.0));

    // **X arms removal mode.**
    step(
        &mut app,
        Frame {
            keys: vec![KeyCode::KeyX],
            ..default()
        },
    );
    assert!(
        matches!(app.world().resource::<EditorState>().tool, Tool::Remove),
        "X must arm removal mode"
    );

    // **A press, a drag across the two pieces, and a release.**
    step(
        &mut app,
        Frame {
            pointer: Some(from),
            mouse: Some(true),
            ..default()
        },
    );
    step(
        &mut app,
        Frame {
            pointer: Some(to),
            ..default()
        },
    );
    step(
        &mut app,
        Frame {
            pointer: Some(to),
            mouse: Some(false),
            ..default()
        },
    );
    assert_eq!(
        ids(&held(&app)),
        ["site/wall@1"],
        "the box took the two pieces it covered and left the one off it"
    );
    assert_eq!(
        ids(&on_disk(&app)),
        ["site/wall@1"],
        "the file agrees with the editor"
    );

    // **Each piece is its own logged removal, back to front.** Removing index 0 first would shift
    // index 2 down, so the registry sees index 2 first, then 0.
    let removals: Vec<String> = log(&app)
        .into_iter()
        .filter(|e| e.op == "placement_remove")
        .map(|e| {
            assert_eq!(e.outcome, "applied", "{e:?}");
            assert!(matches!(e.source, ops::Source::Human), "{e:?}");
            e.params["id"].as_str().unwrap_or_default().to_owned()
        })
        .collect();
    assert_eq!(
        removals,
        ["site/wall@2", "site/wall@0"],
        "one logged placement_remove per piece, taken back to front"
    );

    // **One Cmd+Z takes the whole box back, in the order it was in.** Two undo entries would need
    // two presses, so one press restoring all three proves the box is one undo.
    step(
        &mut app,
        Frame {
            keys: vec![MOD_KEYS[0], KeyCode::KeyZ],
            ..default()
        },
    );
    step(&mut app, Frame::default());
    assert_eq!(
        ids(&held(&app)),
        ["site/wall@0", "site/wall@1", "site/wall@2"],
        "one Cmd+Z puts all three back at the rows they held"
    );
    assert_eq!(
        ids(&on_disk(&app)),
        ["site/wall@0", "site/wall@1", "site/wall@2"],
        "the file agrees with the editor after the undo"
    );
}

#[test]
fn a_box_over_a_stack_leaves_the_piece_between_its_rows() {
    // **The rider sits at a lower row than its host.** Row 0 is the rider, row 1 a piece apart, and
    // row 2 the host it stands on. The box covers the pair and not the piece at row 1. Removing by
    // the rows the box found takes the host, which takes the rider with it, which shifts the piece
    // at row 1 down to row 0 just before the next index is read. That index then removes the piece
    // the box never covered. It must survive.
    let root = fixtures::Fixture::new("person_box_stack_rows")
        .descriptor("site/wall", "alpha")
        .place_on("site/wall", "site/wall@2", (6.0, 0.0))
        .place("site/wall", (3.0, 5.0))
        .place("site/wall", (6.0, 0.0))
        .build("m");
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    app.init_resource::<Script>();
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.add_systems(PreUpdate, drive.after(InputSystems));
    for _ in 0..8 {
        app.update();
    }
    assert_eq!(
        held(&app).placements[0].on.as_deref(),
        Some("site/wall@2"),
        "the fixture starts with the piece at row 0 standing on the host at row 2"
    );

    let from = pixel_over(&mut app, (4.5, -1.0));
    let to = pixel_over(&mut app, (7.0, 1.0));
    step(
        &mut app,
        Frame {
            keys: vec![KeyCode::KeyX],
            ..default()
        },
    );
    step(
        &mut app,
        Frame {
            pointer: Some(from),
            mouse: Some(true),
            ..default()
        },
    );
    step(
        &mut app,
        Frame {
            pointer: Some(to),
            ..default()
        },
    );
    step(
        &mut app,
        Frame {
            pointer: Some(to),
            mouse: Some(false),
            ..default()
        },
    );
    assert_eq!(
        ids(&held(&app)),
        ["site/wall@1"],
        "the box took the stack, and the piece between its rows stayed"
    );
    assert_eq!(
        ids(&on_disk(&app)),
        ["site/wall@1"],
        "the file agrees with the editor"
    );
}
