//! A piece in a carved cave rests on the cave's floor, not on the hilltop above it (#101).

use bevy_grimoire_core::volume::{Edit, Mode, Shape, Volume};

/// A flat hill whose top is at y = 4, tunnelled by a capsule along x at y = 1.5, radius 1.
fn hill_with_tunnel() -> Volume {
    let mut v = Volume::default();
    v.push(Edit {
        shape: Shape::Capsule { a: [-10.0, 1.5, 0.0], b: [10.0, 1.5, 0.0], radius_m: 1.0 },
        mode: Mode::Carve,
        smooth_m: 0.0,
    })
    .unwrap();
    v
}

fn hill(p: [f32; 3]) -> f32 {
    p[1] - 4.0
}

#[test]
fn inside_the_tunnel_the_floor_is_the_tunnel_s_floor() {
    let v = hill_with_tunnel();
    let y = v.floor_below(0.0, 0.0, 1.5, 10.0, &hill).expect("a floor");
    assert!((y - 0.5).abs() < 1e-3, "floor at {y}, want 0.5");
}

#[test]
fn off_the_tunnel_the_floor_is_the_ground() {
    let v = hill_with_tunnel();
    let y = v.floor_below(0.0, 5.0, 6.0, 10.0, &hill).expect("a floor");
    assert!((y - 4.0).abs() < 1e-3, "floor at {y}, want 4.0");
}

#[test]
fn a_start_inside_solid_or_out_of_reach_has_no_floor() {
    let v = hill_with_tunnel();
    assert_eq!(v.floor_below(0.0, 5.0, 2.0, 10.0, &hill), None, "inside the hill");
    assert_eq!(v.floor_below(0.0, 0.0, 1.5, 0.5, &hill), None, "floor 1 m down, reach 0.5");
    assert_eq!(Volume::default().floor_below(0.0, 0.0, 1.0, 5.0, &|_| f32::INFINITY), None, "empty space");
}

#[test]
fn an_added_block_is_a_floor_on_empty_space() {
    let mut v = Volume::default();
    v.push(Edit { shape: Shape::Box { center: [0.0, 1.0, 0.0], half_m: [2.0, 1.0, 2.0] }, mode: Mode::Add, smooth_m: 0.0 }).unwrap();
    let y = v.floor_below(0.5, 0.5, 5.0, 10.0, &|_| f32::INFINITY).expect("a floor");
    assert!((y - 2.0).abs() < 1e-3, "floor at {y}, want 2.0");
}
