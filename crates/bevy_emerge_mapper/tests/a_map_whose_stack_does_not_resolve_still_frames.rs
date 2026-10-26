//! **A map whose stacking does not resolve still frames on the whole map** (backlog B-018).
//!
//! `resolve_frame` read `editor::heights` before it looked at the target, so *every* target paid for
//! a stack resolution — including [`FrameTarget::Map`], whose box is `Map::floor_rect` and
//! `Map::height_span` and asks the stack nothing. A map holding one guest on a host that offers no
//! such surface opens (the map is well formed: the host exists and there is no cycle) and then
//! refuses to be framed, leaving the camera at the rig's 18 m default over the origin.
//!
//! The refusal itself is right and stays: a *piece's* box cannot be known without its height, so
//! `Selection`, `Placements` and `Rect` still refuse. What the whole map means does not depend on
//! that.

use bevy::prelude::*;
use bevy_emerge_mapper::editor::{FrameTarget, resolve_frame};
use bevy_emerge_mapper::harness;
use bevy_emerge_mapper::project::{OpenMap, Project};
use bevy_emerge_mapper::view::MainCamera;

mod fixtures;

/// **A lamp that needs a `worktop` surface, resting on a desk that offers none.** The map is well
/// formed — the host exists, nothing is cyclic — and its heights do not resolve.
///
/// The library needs a `worktop` offer somewhere for the load to admit the lamp at all
/// (`lamp` unhostable-in-principle is refused when the library is read), so an unused `table`
/// offers it and the desk does not.
fn unresolvable(tag: &str, side: f32) -> (App, std::path::PathBuf) {
    let root = fixtures::Fixture::new(tag)
        .descriptor("floor", "alpha")
        .descriptor("desk", "alpha")
        .surface_descriptor("table", "alpha", "worktop")
        .mounted_descriptor("lamp", "alpha", "worktop")
        .place("floor", (0.0, 0.0))
        .place("desk", (4.0, 0.0))
        .place_on("lamp", "desk@1", (4.0, 0.0))
        .side(side)
        .build("m");
    let app = harness::build_headless(&root, "m", None).unwrap_or_else(|e| panic!("{e}"));
    (app, root)
}

/// The eight corners of a world-space box.
fn corners(centre: Vec3, size: Vec3) -> [Vec3; 8] {
    let half = size * 0.5;
    let mut out = [Vec3::ZERO; 8];
    let mut i = 0;
    for sx in [-1.0, 1.0] {
        for sy in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                out[i] = centre + half * Vec3::new(sx, sy, sz);
                i += 1;
            }
        }
    }
    out
}

/// Are all eight corners of the box inside the MainCamera's own orthographic area?
fn inside_the_view(app: &mut App, centre: Vec3, size: Vec3) -> bool {
    let world = app.world_mut();
    let mut q = world.query_filtered::<(&GlobalTransform, &Projection), With<MainCamera>>();
    let (tf, proj) = q.single(world).expect("the editor has one main camera");
    let Projection::Orthographic(ortho) = proj else {
        panic!("the Map's camera is orthographic")
    };
    let area = ortho.area;
    let inverse = tf.affine().inverse();
    let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
    for corner in corners(centre, size) {
        let local = inverse.transform_point3(corner);
        lo = lo.min(local.xy());
        hi = hi.max(local.xy());
    }
    lo.x >= area.min.x && hi.x <= area.max.x && lo.y >= area.min.y && hi.y <= area.max.y
}

/// **The whole map has a frame on a map whose stack does not resolve**, because its box is the floor
/// plan and the map's own height span — neither of which asks the stack anything.
///
/// Failed before the change: `resolve_frame` answered
/// *"map: `lamp@2` needs a `worktop` surface and rests on `desk@1`, which offers none…"* — a
/// sentence about one piece standing in the way of a question about the map.
#[test]
fn the_whole_map_frames_even_when_the_stack_does_not_resolve() {
    let (mut app, root) = unresolvable("frames-unresolvable", 1120.0);
    for _ in 0..6 {
        app.update();
    }
    {
        let project = app.world().resource::<Project>();
        let open = app.world().resource::<OpenMap>();
        let framing = resolve_frame(&FrameTarget::Map, project, open, None).unwrap_or_else(|e| {
            panic!(
                "the whole map has no frame, though its box is the floor plan and the map's own \
                 height span: {e}"
            )
        });
        assert_eq!(framing.what, "the whole map");
    }

    // And the opening frame reaches the camera, not just the resolver.
    let (centre, size) = bevy_emerge_mapper::editor::map_bounds(app.world().resource::<OpenMap>());
    assert!(
        inside_the_view(&mut app, centre, size),
        "opened on a map whose stack does not resolve, a corner of the 1120 m map is outside the \
         view, so the person sees the 18 m close-up of the origin instead of the map"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A piece's box still refuses**, because it cannot be placed without its height — the half of the
/// item that is correct, kept so the fix cannot quietly widen into "framing never fails".
#[test]
fn a_piece_on_an_unresolvable_stack_still_refuses_a_frame() {
    let (mut app, root) = unresolvable("refuses-a-piece", 1120.0);
    for _ in 0..6 {
        app.update();
    }
    let project = app.world().resource::<Project>();
    let open = app.world().resource::<OpenMap>();
    let err = resolve_frame(
        &FrameTarget::Placements(vec!["lamp@2".to_owned()]),
        project,
        open,
        None,
    )
    .err()
    .expect("an unseatible piece has no box to frame");
    assert!(
        err.contains("worktop") && err.contains("offers none"),
        "the refusal must still be the stack's own sentence, got: {err}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
