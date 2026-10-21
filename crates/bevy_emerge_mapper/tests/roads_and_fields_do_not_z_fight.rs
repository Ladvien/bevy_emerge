//! **Two flat pieces laid over one another do not draw their top faces on the same depth.**
//!
//! The shipped `town/road` and `town/field` are mesh-less descriptors: the editor draws each as a
//! footprint marker, a 0.05 m slab standing on the floor. Every one of them has its top face at the
//! same height, so where a road crosses a field the two tops are coplanar and the depth buffer hands
//! the pixels to whichever surface it happens to test first. The picture trades pixels in a stair
//! step, which is the z-fight the developer saw on 2026-10-21 (`window-default.png`).
//!
//! This asserts what decides the picture. The harness lays out but does not rasterize, so the
//! question is whether the two top faces have the same effective depth: the same world height and
//! the same depth bias. If they do, the depth test is a coin toss and the picture is wrong whatever
//! the camera does.

use bevy::prelude::*;

mod fixtures;

/// The top face of a footprint marker's slab, in world metres, and the depth bias of its material.
///
/// The slab is the first child of the piece's root (see `bevy_emerge::spawn_footprint_marker`), and
/// its centre sits `MARKER_SLAB / 2` above the root, so its top sits `MARKER_SLAB / 2` above that
/// centre. The root is the piece's own transform, so the two together are the drawn top.
fn top_face(app: &mut App, id: &str) -> (f32, f32) {
    let world = app.world_mut();
    let mut roots = world.query::<(&Name, &Children)>();
    let children: Vec<Entity> = roots
        .iter(world)
        .find(|(name, _)| name.as_str() == id)
        .unwrap_or_else(|| panic!("no piece named `{id}` was drawn"))
        .1
        .iter()
        .collect();
    let slab = children
        .into_iter()
        .find(|c| world.get::<Mesh3d>(*c).is_some())
        .unwrap_or_else(|| panic!("`{id}` has no footprint slab"));
    let top = world
        .get::<GlobalTransform>(slab)
        .unwrap_or_else(|| panic!("`{id}`'s slab has no global transform"))
        .translation()
        .y
        + 0.025;
    let handle = world
        .get::<MeshMaterial3d<StandardMaterial>>(slab)
        .unwrap_or_else(|| panic!("`{id}`'s slab has no material"))
        .0
        .clone();
    let bias = world
        .resource::<Assets<StandardMaterial>>()
        .get(&handle)
        .unwrap_or_else(|| panic!("`{id}`'s material is not in the asset store"))
        .depth_bias;
    (top, bias)
}

#[test]
fn overlapping_flat_pieces_have_distinct_top_faces() {
    // A road laid across a field, both mesh-less and both on the floor: the shape of the shipped
    // town, where a road crosses a field and both are footprint markers at the same height.
    let root = fixtures::Fixture::new("flat-overlap")
        .flat_descriptor("road", 4.877, 20.0)
        .flat_descriptor("field", 12.0, 12.0)
        .place("road", (0.0, 0.0))
        .place("field", (0.0, 0.0))
        .build("m");
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None)
        .unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..6 {
        app.update();
    }

    let (road_top, road_bias) = top_face(&mut app, "road");
    let (field_top, field_bias) = top_face(&mut app, "field");

    assert!(
        road_top != field_top || road_bias != field_bias,
        "the road and the field draw their top faces at the same depth (top {road_top} m, bias \
         {road_bias}), so where they overlap the depth test decides the pixels by coin toss — \
         the z-fight"
    );
    // The road is the smaller footprint, so it draws on top of the field it crosses.
    assert!(
        road_bias > field_bias,
        "the road is the smaller piece and must draw over the field it crosses, but its bias is \
         {road_bias} against the field's {field_bias}"
    );
}

#[test]
fn an_authored_paint_outranks_the_smaller_footprint() {
    // The field is the larger piece, so the footprint alone would draw it under the road. Its
    // authored paint is the first key of the order, so it draws over the road instead.
    let root = fixtures::Fixture::new("flat-painted")
        .flat_descriptor("road", 4.877, 20.0)
        .flat_descriptor("field", 12.0, 12.0)
        .place("road", (0.0, 0.0))
        .place_painted("field", (0.0, 0.0), 3)
        .build("m");
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None)
        .unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..6 {
        app.update();
    }

    let (_, road_bias) = top_face(&mut app, "road");
    let (_, field_bias) = top_face(&mut app, "field");
    assert!(
        field_bias > road_bias,
        "the field carries authored paint 3 and must draw over the road, but its bias is \
         {field_bias} against the road's {road_bias}"
    );
}

#[test]
fn of_two_equal_flat_pieces_the_later_placement_draws_on_top() {
    let root = fixtures::Fixture::new("flat-equal")
        .flat_descriptor("tile_a", 4.0, 4.0)
        .flat_descriptor("tile_b", 4.0, 4.0)
        .place("tile_a", (0.0, 0.0))
        .place("tile_b", (0.0, 0.0))
        .build("m");
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None)
        .unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..6 {
        app.update();
    }

    let (_, a_bias) = top_face(&mut app, "tile_a");
    let (_, b_bias) = top_face(&mut app, "tile_b");
    assert!(
        b_bias > a_bias,
        "the equal pieces overlap and the later placement must draw on top, but tile_b's bias is \
         {b_bias} against tile_a's {a_bias}"
    );
}

#[test]
fn a_larger_piece_laid_under_a_standing_road_lifts_the_road() {
    // The road stands alone, so nothing is under it and it draws at the base. Then a larger field is
    // laid beneath it. The road is now the smaller of an overlapping pair and must be redrawn above
    // the field, though the road itself was never edited.
    let root = fixtures::Fixture::new("flat-relevel")
        .flat_descriptor("road", 4.877, 20.0)
        .flat_descriptor("field", 12.0, 12.0)
        .place("road", (0.0, 0.0))
        .build("m");
    let mut app = bevy_emerge_mapper::harness::build_headless(&root, "m", None)
        .unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..6 {
        app.update();
    }
    let (_, before) = top_face(&mut app, "road");
    assert_eq!(
        before, 0.0,
        "a road alone on the floor has no paint to lift it"
    );

    // Lay the field under it the way an edit records a placement: a new entry in the open map.
    {
        let mut open = app
            .world_mut()
            .resource_mut::<bevy_emerge_mapper::project::OpenMap>();
        let mut field = open.map.placements[0].clone();
        field.id = "field".into();
        field.descriptor = "field".into();
        field.patch = None;
        open.map.placements.push(field);
    }
    for _ in 0..6 {
        app.update();
    }
    let (_, after) = top_face(&mut app, "road");
    assert!(
        after > before,
        "the field now lies under the road, so the road is the smaller piece and must be redrawn \
         above it, but its bias stayed at {after} from {before}"
    );
}
