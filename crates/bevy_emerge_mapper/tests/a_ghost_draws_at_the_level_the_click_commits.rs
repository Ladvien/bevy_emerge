//! **A ghost is drawn at the draw level the committed piece gets** — E-020's first half.
//!
//! # What was wrong
//!
//! `drive_ghost` spawned its preview through `spawn_piece`, which carries no [`bevy_emerge::Paint`],
//! and `apply_paint` reads the paint off the piece root. A brush ghost therefore drew at level 0
//! always, while the piece the click commits is *appended* to the map — and the draw order breaks
//! ties on placement order, earliest first, so a new flat piece draws above every equal-paint,
//! equal-area piece it overlaps. Hovering a tile over a field previewed two coplanar top faces, the
//! z-fight `roads_and_fields_do_not_z_fight` exists to prevent, and placing it produced a picture the
//! preview had not shown.
//!
//! # What is asserted
//!
//! The ghost's paint and the committed placement's paint are the **same number**, and that number is
//! the level the map's own order gives a row appended on top of the field. Under the defect the
//! ghost carried no paint at all (0) against a committed 1, so the first assertion fails at the base
//! commit.
//!
//! The pointer is injected the way `a_piece_lands_where_the_pointer_is` injects it: the window's
//! cursor is set in `PreUpdate`, which is what `view::sense_pointer` reads.

use bevy::input::{ButtonInput, InputSystems};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window};
use bevy_emerge_mapper::editor::{EditorState, Ghost, Placement};
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::project::{OpenMap, Project};
use bevy_emerge_mapper::view::MainCamera;

mod fixtures;

/// One frame's worth of person: the pointer, and a mouse press or release.
#[derive(Default)]
struct Frame {
    pointer: Option<Vec2>,
    mouse: Option<bool>,
}

#[derive(Resource, Default)]
struct Script {
    next: Option<Frame>,
}

fn drive(
    mut script: ResMut<Script>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Some(frame) = script.next.take() else {
        return;
    };
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

/// The pixel a person would point at for the map-space spot `at`.
fn surface_pixel_over(app: &mut App, at: (f32, f32)) -> Vec2 {
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

/// The paint on the one entity in the world matching `filter`.
fn paint_of(app: &mut App, ghosts_only: bool) -> Option<i8> {
    let world = app.world_mut();
    if ghosts_only {
        let mut q = world.query_filtered::<Option<&bevy_emerge::Paint>, With<Ghost>>();
        let paints: Vec<_> = q.iter(world).collect();
        assert_eq!(paints.len(), 1, "exactly one brush ghost is on screen");
        paints[0].map(|p| p.0)
    } else {
        let mut q = world.query_filtered::<Option<&bevy_emerge::Paint>, With<Placement>>();
        let paints: Vec<_> = q.iter(world).collect();
        assert_eq!(paints.len(), 1, "exactly one placement is drawn");
        paints[0].map(|p| p.0)
    }
}

/// **Hover the armed tile over a field, then click it down: the ghost and the piece are drawn at one
/// level, and that level is the one the order gives the new row.**
#[test]
fn a_ghost_draws_at_the_level_the_click_commits() {
    let root = fixtures::Fixture::new("ghost-level")
        .flat_descriptor("field", 12.0, 12.0)
        .flat_descriptor("tile", 4.0, 4.0)
        .place("field", (0.0, 0.0))
        .side(20.0)
        .build("m");
    let mut app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    app.init_resource::<Script>();
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.add_systems(PreUpdate, drive.after(InputSystems));
    for _ in 0..8 {
        app.update();
    }

    let index = app
        .world()
        .resource::<Project>()
        .library
        .descriptors
        .iter()
        .position(|d| d.id == "tile")
        .expect("the fixture defines tile");
    app.world_mut().resource_mut::<EditorState>().brush = Some(index);
    app.update();

    let logical = surface_pixel_over(&mut app, (3.5, 1.5));
    step(
        &mut app,
        Frame {
            pointer: Some(logical),
            ..default()
        },
    );

    let ghost = paint_of(&mut app, true);
    assert_eq!(
        ghost,
        Some(1),
        "the ghost is appended after the field among equal-paint pieces of different area, so the \
         committed row draws one level up; a ghost drawn at {ghost:?} is previewing a coplanar top \
         face the click will not produce"
    );

    step(
        &mut app,
        Frame {
            pointer: Some(logical),
            mouse: Some(true),
        },
    );
    step(
        &mut app,
        Frame {
            pointer: Some(logical),
            mouse: Some(false),
        },
    );
    app.update();

    let committed = paint_of(&mut app, false);
    assert_eq!(
        committed,
        Some(1),
        "the placed tile is one level above the field it lands on, and the preview said the same; \
         got {committed:?}"
    );
    assert_eq!(
        ghost, committed,
        "the preview and the piece must be drawn at one level — a preview computing either \
         differently is a promise about a landing that will not happen"
    );
}
