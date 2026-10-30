//! **A carved volume meshes without cracks where chunks meet** (#101). Needs the `isomesh` feature:
//! `cargo test -p bevy_emerge_mapper --features isomesh --test editor volume_mesh_seams::`.
#![cfg(feature = "isomesh")]

use bevy_emerge_core::volume::{Edit, Mode, Shape, Volume};
use bevy_emerge_mapper::volume_mesh::{mesh_volume, report, Region};

fn block_with_a_hollow_across_the_seams() -> Volume {
    let mut v = Volume::default();
    // A rock 12 m across, centred on a corner where eight 4 m chunks meet at (8, 8, 8).
    v.push(Edit { shape: Shape::Box { center: [8.0; 3], half_m: [6.1; 3] }, mode: Mode::Add, smooth_m: 0.0 }).unwrap();
    v.push(Edit { shape: Shape::Sphere { center: [8.0; 3], radius_m: 3.3 }, mode: Mode::Carve, smooth_m: 0.0 }).unwrap();
    v
}

fn region() -> Region {
    Region { min: [0.0; 3], chunks: [4, 4, 4], chunk_cells: 8, cell_m: 0.5 }
}

#[test]
fn a_hollow_carved_across_eight_chunks_is_a_closed_manifold_with_no_seam_holes() {
    let mesh = mesh_volume(&block_with_a_hollow_across_the_seams(), |_| f32::INFINITY, region()).unwrap();
    let r = report(&mesh, 0.5).unwrap();
    assert!(mesh.triangle_count() > 0);
    assert_eq!(r.boundary_edges, 0, "a crack at a chunk seam leaves open edges");
    assert!(r.is_closed(), "outer skin and hollow: two closed surfaces ({r:?})");
    assert_eq!(r.euler_characteristic, 4, "box skin plus its hollow are two spheres");
}

#[test]
fn carving_opens_a_hole_in_a_cliff_that_the_uncarved_rock_lacks() {
    let ground = |p: [f32; 3]| p[1] - 4.0;
    let r = Region { min: [0.0; 3], chunks: [2, 1, 2], chunk_cells: 8, cell_m: 0.5 };
    let plain = mesh_volume(&Volume::default(), ground, r).unwrap();
    let mut v = Volume::default();
    v.push(Edit { shape: Shape::Sphere { center: [4.0, 4.0, 4.0], radius_m: 2.0 }, mode: Mode::Carve, smooth_m: 0.0 }).unwrap();
    let carved = mesh_volume(&v, ground, r).unwrap();
    let (a, b) = (report(&plain, 0.5).unwrap(), report(&carved, 0.5).unwrap());
    assert!(carved.triangle_count() > plain.triangle_count(), "the crater adds surface");
    assert_eq!(a.non_manifold_edges + b.non_manifold_edges, 0);
    assert_eq!(a.boundary_edges, b.boundary_edges, "the crater straddles the chunk seam yet adds no open edge");
}
