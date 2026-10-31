//! **A piece authored inside a carved cave stands on the cave's floor** (#101): the authored height
//! (floor + lift) is where the drop starts; open air under the terrain falls to the first solid.

use bevy_grimoire_core::descriptor::{Descriptor, Extent, Mount};
use bevy_grimoire_core::heightfield::Heightfield;
use bevy_grimoire_core::library::{Library, LIBRARY_VERSION};
use bevy_grimoire_core::map::{Map, Placed};
use bevy_grimoire_core::stack::resolve_y_in;
use bevy_grimoire_core::volume::{Edit, Mode, Shape, Volume};

fn world(at: (f32, f32), lift: f32) -> (Map, Library) {
    let d = Descriptor {
        id: "crate".into(),
        extent: Extent { footprint: Some((1.0, 1.0)), height: Some(1.0) },
        mount: Some(Mount::OnFloor),
        ..Descriptor::default()
    };
    let map = Map {
        name: "m".into(),
        placements: vec![Placed { id: "c".into(), descriptor: "crate".into(), at, lift, ..Placed::default() }],
        ..Map::default()
    };
    (map, Library { version: LIBRARY_VERSION, note: None, descriptors: vec![d] })
}

/// A 4 m hill with a tunnel open between y = 0.5 and y = 2.5 under x, z in 2..8.
fn hill() -> (Heightfield, Volume) {
    let g = Heightfield::flat((0.0, 0.0), 1.0, 11, 11, 4.0);
    let mut v = Volume::default();
    v.push(Edit { shape: Shape::Box { center: [5.0, 1.5, 5.0], half_m: [3.0, 1.0, 3.0] }, mode: Mode::Carve, smooth_m: 0.0 }).unwrap();
    (g, v)
}

fn y_of(at: (f32, f32), lift: f32, with_volume: bool) -> f32 {
    let (m, l) = world(at, lift);
    let (g, v) = hill();
    resolve_y_in(&m, &l, Some(&g), with_volume.then_some(&v)).unwrap()[0]
}

#[test]
fn a_piece_started_in_the_tunnel_falls_to_its_floor() {
    let y = y_of((5.0, 5.0), 1.5, true);
    assert!((y - 0.5).abs() < 1e-3, "tunnel floor is 0.5, got {y}");
}

#[test]
fn without_the_volume_the_piece_rests_on_the_hilltop() {
    assert!((y_of((5.0, 5.0), 1.5, false) - 5.5).abs() < 1e-3);
}

#[test]
fn a_start_in_rock_or_outside_the_tunnel_keeps_the_terrain_rest() {
    let top = y_of((5.0, 5.0), 0.0, false);
    assert_eq!(y_of((5.0, 5.0), 0.0, true), top, "start y = 0 is inside rock");
    assert_eq!(y_of((9.5, 5.0), 1.5, true), y_of((9.5, 5.0), 1.5, false), "outside the tunnel");
}
