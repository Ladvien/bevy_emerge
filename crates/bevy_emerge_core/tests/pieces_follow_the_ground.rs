//! **Placed pieces follow the ground** (#99): a floor-standing piece rests on the terrain under its
//! footprint, a guest rides its host, and sculpting under a piece moves it.

use bevy_emerge_core::descriptor::{Descriptor, Extent, Mount};
use bevy_emerge_core::heightfield::{Brush, Heightfield, Stroke};
use bevy_emerge_core::library::{Library, LIBRARY_VERSION};
use bevy_emerge_core::map::{Map, Placed};
use bevy_emerge_core::stack::{resolve_y, resolve_y_on};

fn house(mount: Mount) -> Descriptor {
    Descriptor {
        id: "house".into(),
        extent: Extent { footprint: Some((2.0, 2.0)), height: Some(3.0) },
        mount: Some(mount),
        ..Descriptor::default()
    }
}

fn world(mount: Mount) -> (Map, Library) {
    let map = Map {
        name: "m".into(),
        placements: vec![Placed { id: "h".into(), descriptor: "house".into(), at: (5.0, 5.0), ..Placed::default() }],
        ..Map::default()
    };
    (map, Library { version: LIBRARY_VERSION, note: None, descriptors: vec![house(mount)] })
}

fn ground() -> Heightfield {
    Heightfield::flat((0.0, 0.0), 1.0, 11, 11, 0.0)
}

#[test]
fn a_house_rides_up_when_the_ground_under_it_is_raised() {
    let (m, l) = world(Mount::OnFloor);
    let mut g = ground();
    assert_eq!(resolve_y_on(&m, &l, Some(&g)).unwrap()[0], 0.0);
    g.apply(&Stroke { brush: Brush::Raise, radius_m: 3.0, strength: 1.5, points: vec![(5.0, 5.0)] });
    let y = resolve_y_on(&m, &l, Some(&g)).unwrap()[0];
    assert!((y - 1.5).abs() <= 1e-4, "the house stands at the hilltop, got {y}");
}

#[test]
fn a_piece_on_a_slope_does_not_sink_a_corner() {
    let (m, l) = world(Mount::OnFloor);
    let mut g = ground();
    // A ramp rising along x: height = x * 0.5.
    for iz in 0..11 {
        for ix in 0..11 {
            g.heights[iz * 11 + ix] = ix as f32 * 0.5;
        }
    }
    let y = resolve_y_on(&m, &l, Some(&g)).unwrap()[0];
    assert!((y - 3.0).abs() < 1e-4, "the high corner (x = 6) is at 3.0 m, got {y}");
}

#[test]
fn the_walls_ceiling_and_no_terrain_keep_the_maps_own_datum() {
    let (m, l) = world(Mount::OnWall { height: 1.0 });
    let mut g = ground();
    g.heights.iter_mut().for_each(|h| *h = 4.0);
    assert_eq!(resolve_y_on(&m, &l, Some(&g)).unwrap(), resolve_y(&m, &l).unwrap());
    let (m, l) = world(Mount::OnFloor);
    assert_eq!(resolve_y_on(&m, &l, None).unwrap(), resolve_y(&m, &l).unwrap());
}
